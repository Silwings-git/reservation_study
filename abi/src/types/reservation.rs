use std::ops::Bound;

use chrono::{DateTime, FixedOffset, Utc};
use sqlx::{
    FromRow, Row,
    postgres::{PgRow, types::PgRange},
};

use crate::{
    Error, Id, Reservation, ReservationStatus, RsvpStatus,
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

        let start = convert_to_utc_time(&self.start.unwrap());
        let end = convert_to_utc_time(&self.end.unwrap());

        if start >= end {
            return Err(Error::InvalidTime);
        }

        Ok(())
    }

    pub fn get_timespan(&self) -> PgRange<DateTime<Utc>> {
        let start = convert_to_utc_time(&self.start.unwrap());
        let end = convert_to_utc_time(&self.end.unwrap());
        (start..end).into()
    }
}

impl FromRow<'_, sqlx::postgres::PgRow> for Reservation {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let id: i64 = row.get("id");
        let timaspan: PgRange<DateTime<Utc>> = row.try_get("timespan")?;

        let window = NaiveRange::from(timaspan);

        let status: RsvpStatus = row.get("status");

        Ok(Reservation {
            id,
            user_id: row.get("user_id"),
            status: ReservationStatus::from(status) as i32,
            resource_id: row.get("resource_id"),
            start: window.start.map(convert_to_timestamp),
            end: window.end.map(convert_to_timestamp),
            note: row.get("note"),
        })
    }
}

struct NaiveRange<T> {
    start: Option<T>,
    end: Option<T>,
}

impl<T> From<PgRange<T>> for NaiveRange<T> {
    fn from(value: PgRange<T>) -> Self {
        let convert = |b: Bound<T>| match b {
            Bound::Included(v) | Bound::Excluded(v) => Some(v),
            _ => None,
        };

        let start = convert(value.start);
        let end = convert(value.end);

        NaiveRange { start, end }
    }
}

impl Id for Reservation {
    fn id(&self) -> i64 {
        self.id
    }
}
