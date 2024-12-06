use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, OrgId};

#[derive(Debug)]
pub struct OrgBenchmarkQuery {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub call_sign: CallSign,
    pub org_id: OrgId,
}
