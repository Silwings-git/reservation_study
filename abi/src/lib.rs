mod pb;
use chrono::{DateTime, Utc};
pub use pb::*;
use prost_types::Timestamp;

pub fn convert_to_utc_time(ts: Timestamp) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(ts.seconds, ts.nanos as _).unwrap()
}
