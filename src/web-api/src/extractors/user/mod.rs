use super::{AcceptedIssuer, Auth0Profile, BwProfile};
use crate::error::{Error, Result};
use crate::extractors::BearerToken;
use crate::states::BwState;
use actix_web::http::header::AUTHORIZATION;
use actix_web::{FromRequest, web::Data};
use http_client::HttpClient;
use kyogre_core::{AisPermission, BarentswatchUserId};
use oasgen::{
    HeaderStyle, OaParameter, OaSchema, Parameter, ParameterData, ParameterKind,
    ParameterSchemaOrContent, RefOr,
};
use std::future::ready;
use std::pin::Pin;
use tracing::warn;

#[derive(Debug)]
pub enum UserAuth {
    Bw(BwProfile),
    Orca(Auth0Profile),
    NoUser,
}

impl UserAuth {
    pub fn id(&self) -> Option<BarentswatchUserId> {
        match self {
            UserAuth::Bw(v) => Some(v.user.id),
            UserAuth::Orca(_) | UserAuth::NoUser => None,
        }
    }
    pub fn ais_permission(&self) -> AisPermission {
        match self {
            UserAuth::NoUser => AisPermission::Above15m,
            UserAuth::Bw(bw_profile) => bw_profile.into(),
            UserAuth::Orca(auth0_profile) => auth0_profile.into(),
        }
    }
}

impl OaParameter for UserAuth {
    fn parameters() -> Vec<RefOr<Parameter>> {
        vec![RefOr::Item(Parameter {
            data: ParameterData {
                name: AUTHORIZATION.to_string(),
                description: None,
                required: false,
                deprecated: None,
                format: ParameterSchemaOrContent::Schema(String::schema_ref()),
                example: None,
                examples: Default::default(),
                explode: None,
                extensions: Default::default(),
            },
            kind: ParameterKind::Header {
                style: HeaderStyle::Simple,
            },
        })]
    }
}

impl FromRequest for UserAuth {
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        match BearerToken::from_request_owned(req) {
            Err(e) => Box::pin(ready(Err(e))),
            Ok(bearer) => {
                match bearer {
                    None => Box::pin(ready(Ok(UserAuth::NoUser))),
                    Some(bearer) => match bearer.issuer() {
                        Err(e) => Box::pin(ready(Err(e))),
                        Ok(issuer) => match issuer {
                            AcceptedIssuer::OrcaDev => Box::pin(ready(
                                Auth0Profile::from_request_and_bearer(req, bearer)
                                    .map(UserAuth::Orca)
                                    .inspect_err(|e| {
                                        warn!("failed to extract auth0 profile: {e:?}")
                                    }),
                            )),
                            AcceptedIssuer::Barentswatch => {
                                // `HttpClient` should be provided on startup, so `unwrap` is safe
                                let client = req.app_data::<Data<HttpClient>>().unwrap().clone();
                                // `BwState` should be provided on startup, so `unwrap` is safe
                                let state = req.app_data::<Data<BwState>>().unwrap().clone();

                                let query_string = req.query_string().to_string();
                                Box::pin(async move {
                                    BwProfile::extract_impl(state, client, bearer, query_string)
                                        .await
                                        .inspect_err(|e| {
                                            warn!("failed to extract barentswatch profile: {e:?}")
                                        })
                                        .map(UserAuth::Bw)
                                })
                            }
                        },
                    },
                }
            }
        }
    }
}
