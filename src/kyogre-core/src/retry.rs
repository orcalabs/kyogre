use backon::{ConstantBuilder, Retryable};
use futures::Future;
use std::time::Duration;

pub trait IsTimeout {
    fn is_timeout(&self) -> bool;
}

pub async fn retry<T, Fut, FutureFn, E>(fut: FutureFn) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
    FutureFn: FnMut() -> Fut,
    E: IsTimeout,
{
    fut.retry(
        ConstantBuilder::default()
            .with_delay(Duration::from_millis(10))
            .with_max_times(3),
    )
    .when(|e| e.is_timeout())
    .await
}
