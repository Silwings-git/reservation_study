use chrono::{DateTime, FixedOffset, Utc};
use sqlx::postgres::types::PgRange;

use crate::{
    Error, Reservation, ReservationStatus,
    utils::{convert_to_timestamp, convert_to_utc_time},
};

impl Reservation {
    pub fn new_pending(
        uid: impl Into<String>,
        rid: impl Into<String>,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
        note: impl Into<String>,
    ) -> Self {
        Reservation {
            id: 0,
            user_id: uid.into(),
            status: ReservationStatus::Pending as i32,
            resource_id: rid.into(),
            start: Some(convert_to_timestamp(start.with_timezone(&Utc))),
            end: Some(convert_to_timestamp(end.with_timezone(&Utc))),
            note: note.into(),
        }
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.user_id.is_empty() {
            return Err(Error::InvalidUserId(self.user_id.clone()));
        }

        if self.resource_id.is_empty() {
            return Err(Error::InvalidResourceId(self.resource_id.clone()));
        }

        if self.start.is_none() || self.end.is_none() {
            return Err(Error::InvalidTime);
        }

        let start = convert_to_utc_time(self.start.unwrap());
        let end = convert_to_utc_time(self.end.unwrap());

        if start >= end {
            return Err(Error::InvalidTime);
        }

        Ok(())
    }

    pub fn get_timespan(&self) -> PgRange<DateTime<Utc>> {
        let start = convert_to_utc_time(self.start.unwrap());
        let end = convert_to_utc_time(self.end.unwrap());
        (start..end).into()
    }
}
