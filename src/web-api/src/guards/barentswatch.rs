use crate::error::Result;
use actix_web::guard::Guard;
use tracing::warn;

use crate::{extractors::BearerToken, states::BwState};

#[derive(Debug, Clone)]
pub struct BwGuard {
    state: BwState,
}

impl BwGuard {
    pub fn new(state: BwState) -> Self {
        Self { state }
    }
}

impl BwGuard {
    fn check_impl(&self, ctx: &actix_web::guard::GuardContext<'_>) -> Result<bool> {
        Ok(match BearerToken::from_guard_context(ctx)? {
            Some(bearer) => self
                .state
                .decode::<serde_json::Value>(&bearer)
                .map_err(|e| {
                    warn!("failed to decode token: {}, err: {e:?}", bearer.token());
                    e
                })
                .is_ok(),
            None => false,
        })
    }
}

impl Guard for BwGuard {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        self.check_impl(ctx).unwrap_or(false)
    }
}
