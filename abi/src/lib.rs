mod error;
mod pager;
mod pb;
mod types;
mod utils;

pub use error::*;
pub use pager::*;
pub use pb::*;
pub use utils::*;

pub type ReservationId = i64;
pub type UserId = String;
pub type ResourceId = String;

/// validate the data structure, raise error if invalid
pub trait Validator {
    fn validate(&self) -> Result<(), Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "rsvp.reservation_status", rename_all = "lowercase")]
pub enum RsvpStatus {
    Unknown,
    Pending,
    Confirmed,
    Blocked,
}

impl Validator for ReservationId {
    fn validate(&self) -> Result<(), Error> {
        if *self <= 0 {
            Err(Error::InvalidReservationId(*self))
        } else {
            Ok(())
        }
    }
}
