use prost_types::Timestamp;

use crate::{ReservationQuery, convert_to_utc_time};

impl ReservationQuery {
    pub fn pg_start_time_string(&self) -> String {
        dbg!(get_time_string(self.start.as_ref(), true))
    }

    pub fn pg_end_time_string(&self) -> String {
        dbg!(get_time_string(self.end.as_ref(), false))
    }
}

fn get_time_string(ts: Option<&Timestamp>, start: bool) -> String {
    match ts {
        Some(ts) => convert_to_utc_time(ts).to_rfc3339(),
        None => (if start { "-infinity" } else { "infinity" }).into(),
    }
}
