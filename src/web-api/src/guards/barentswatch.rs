use actix_web::guard::Guard;
use tracing::warn;

use crate::states::BwState;

#[derive(Debug, Clone)]
pub struct BwGuard {
    state: BwState,
}

impl BwGuard {
    pub fn new(state: BwState) -> Self {
        Self { state }
    }
}

impl Guard for BwGuard {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        match ctx.head().headers.get("bw-token") {
            Some(token) => match token.to_str() {
                Ok(token) => self
                    .state
                    .decode::<serde_json::Value>(token)
                    .map_err(|e| {
                        warn!("failed to decode token: {token}, err: {e:?}");
                        e
                    })
                    .is_ok(),
                Err(_) => false,
            },
            None => false,
        }
    }
}
