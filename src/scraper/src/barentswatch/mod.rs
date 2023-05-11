use std::sync::Arc;

mod fishing_facility_historic;

pub use fishing_facility_historic::*;

use crate::wrapped_http_client::WrappedHttpClient;

pub struct BarentswatchSource {
    pub client: Arc<WrappedHttpClient>,
}

impl BarentswatchSource {
    pub fn new(client: Arc<WrappedHttpClient>) -> BarentswatchSource {
        BarentswatchSource { client }
    }
}
