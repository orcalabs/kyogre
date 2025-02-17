use std::{any::TypeId, pin::Pin, task::Poll};

use actix_web::{
    body::BoxBody, http::header::ContentType, web::Bytes, HttpRequest, HttpResponse, Responder,
};
use chrono::{DateTime, Duration, Utc};
use futures::{stream, Stream};
use kyogre_core::WebApiResult;
use oasgen::{OaSchema, ObjectType, RefOr, Schema, SchemaData, SchemaKind, Type};
use pin_project_lite::pin_project;
use serde::Serialize;
use tokio::sync::mpsc::Receiver;
use tokio_stream::{once, wrappers::ReceiverStream};
use tracing::error;

use crate::{
    error::Result,
    routes::v1::{ais::AisPosition, ais_vms::AisVmsPosition, vms::VmsPosition},
};

pub static AIS_DETAILS_INTERVAL: Duration = Duration::minutes(30);
pub static MISSING_DATA_DURATION: Duration = Duration::minutes(70);

pub struct StreamResponse<T> {
    pub rx: Receiver<WebApiResult<T>>,
}

impl<T> StreamResponse<T> {
    pub fn new(rx: Receiver<WebApiResult<T>>) -> Self {
        Self { rx }
    }
}

impl<T: OaSchema + 'static> OaSchema for StreamResponse<T> {
    fn schema_ref() -> oasgen::ReferenceOr<Schema> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            RefOr::Item(Schema {
                data: SchemaData::default(),
                kind: SchemaKind::Type(Type::Object(ObjectType::default())),
            })
        } else {
            Vec::<T>::schema_ref()
        }
    }

    fn schema() -> Schema {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            Schema {
                data: SchemaData::default(),
                kind: SchemaKind::Type(Type::Object(ObjectType::default())),
            }
        } else {
            Vec::<T>::schema()
        }
    }
}

#[macro_export]
macro_rules! stream_response {
    ($stream:expr) => {{
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            use futures::StreamExt;

            let mut stream = $stream;
            while let Some(next) = stream.next().await {
                if (tx.send(next).await).is_err() {
                    return;
                }
            }
        });

        StreamResponse::new(rx)
    }};
}

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct Interleaved<S, T> {
        #[pin]
        stream: S,
        current: Option<T>,
        delimiter: T,
        delimiter_flag: bool,
    }
}

pub trait Interleave<T> {
    fn interleave(self, delimiter: T) -> Interleaved<Self, T>
    where
        Self: Sized;
}

impl<S: Sized, T> Interleave<T> for S {
    fn interleave(self, delimiter: T) -> Interleaved<Self, T> {
        Interleaved {
            stream: self,
            current: None,
            delimiter,
            delimiter_flag: false,
        }
    }
}

impl<S> Stream for Interleaved<S, S::Item>
where
    S: Stream,
    S::Item: Clone,
{
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        if *this.delimiter_flag {
            *this.delimiter_flag = false;
            return Poll::Ready(Some(this.delimiter.clone()));
        }

        match this.stream.poll_next(cx) {
            Poll::Ready(Some(v)) => match this.current.replace(v) {
                Some(v) => {
                    *this.delimiter_flag = true;
                    Poll::Ready(Some(v))
                }
                None => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
            Poll::Ready(None) => Poll::Ready(this.current.take()),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn to_bytes<T: Serialize>(value: &T) -> Result<Bytes> {
    Ok(Bytes::from(serde_json::to_vec(value)?))
}

impl<T> Responder for StreamResponse<T>
where
    T: Serialize + 'static,
{
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        use tokio_stream::StreamExt;

        let stream = once(Bytes::from_static(b"["))
            .chain(
                ReceiverStream::new(self.rx)
                    .filter_map(|v| match v {
                        Ok(v) => match to_bytes(&v) {
                            Ok(v) => Some(v),
                            Err(e) => {
                                error!(error = true, "failed to serialize streaming item: {e:?}");
                                None
                            }
                        },
                        Err(e) => {
                            error!(error = true, "failed to retrieve streaming item: {e:?}");
                            None
                        }
                    })
                    .interleave(Bytes::from_static(b",")),
            )
            .chain(once(Bytes::from_static(b"]")))
            .map(Ok::<_, String>);

        HttpResponse::Ok()
            .content_type(ContentType::json())
            .streaming(stream)
    }
}

pub trait Position: Send + 'static {
    fn timestamp(&self) -> DateTime<Utc>;
    fn has_details(&self) -> bool;
    fn clear_details(&mut self);
    fn set_missing_data(&mut self);
}

pub fn ais_unfold<'a, T: Position>(
    stream: impl Stream<Item = WebApiResult<T>> + Send + Unpin + 'a,
) -> Pin<Box<dyn Stream<Item = WebApiResult<T>> + Send + 'a>> {
    use futures::StreamExt;

    struct State<S, P> {
        stream: S,
        missing_flag: bool,
        is_first: bool,
        pos_and_prev_det_ts: Option<(P, DateTime<Utc>)>,
    }

    let state = State {
        stream,
        missing_flag: false,
        is_first: true,
        pos_and_prev_det_ts: None::<(T, _)>,
    };

    stream::unfold(state, |mut state| async move {
        if state.pos_and_prev_det_ts.is_none() {
            match state.stream.next().await {
                Some(Ok(v)) => {
                    let ts = v.timestamp();
                    state.pos_and_prev_det_ts = Some((v, ts));
                }
                Some(e) => return Some((Some(e), state)),
                None => return None,
            }
        };

        match state.stream.next().await {
            Some(Ok(mut next)) => {
                // SAFETY: `unwrap_unchecked` is safe because of `is_none` check above
                let (pos, prev_det_ts) =
                    unsafe { state.pos_and_prev_det_ts.as_mut().unwrap_unchecked() };

                if next.timestamp() - pos.timestamp() >= MISSING_DATA_DURATION {
                    pos.set_missing_data();
                    state.missing_flag = true;
                } else {
                    if !state.missing_flag
                        && pos.timestamp() - *prev_det_ts < AIS_DETAILS_INTERVAL
                        && !state.is_first
                    {
                        pos.clear_details();
                    }
                    state.missing_flag = false;
                }

                if pos.has_details() {
                    *prev_det_ts = pos.timestamp();
                }

                state.is_first = false;
                std::mem::swap(&mut next, pos);

                Some((Some(Ok(next)), state))
            }
            Some(e) => Some((Some(e), state)),
            None => state
                .pos_and_prev_det_ts
                .take()
                .map(|(pos, _)| (Some(Ok(pos)), state)),
        }
    })
    .filter_map(|v| async move { v })
    .boxed()
}

impl Position for AisPosition {
    #[inline]
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    #[inline]
    fn has_details(&self) -> bool {
        self.det.is_some()
    }
    #[inline]
    fn clear_details(&mut self) {
        self.det = None;
    }
    #[inline]
    fn set_missing_data(&mut self) {
        if let Some(ref mut det) = self.det {
            det.missing_data = true;
        }
    }
}

impl Position for VmsPosition {
    #[inline]
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    #[inline]
    fn has_details(&self) -> bool {
        false
    }
    #[inline]
    fn clear_details(&mut self) {}
    #[inline]
    fn set_missing_data(&mut self) {}
}

impl Position for AisVmsPosition {
    #[inline]
    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    #[inline]
    fn has_details(&self) -> bool {
        self.det.is_some()
    }
    #[inline]
    fn clear_details(&mut self) {
        self.det = None;
    }
    #[inline]
    fn set_missing_data(&mut self) {
        if let Some(ref mut det) = self.det {
            det.missing_data = true;
        }
    }
}
