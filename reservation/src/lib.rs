use abi::DbConfig;
use abi::Error;
use abi::FilterPager;
use abi::ReservationId;
use abi::ReservationQuery;
use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

mod db;
mod manager;

pub use db::*;

#[derive(Debug)]
pub struct ReservationManager {
    pool: PgPool,
}

impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn from_config(config: &DbConfig) -> Result<Self, abi::Error> {
        let url = config.url();
        let pool = PgPoolOptions::default()
            .max_connections(config.max_connections)
            .connect(&url)
            .await?;
        Ok(Self::new(pool))
    }
}

#[async_trait]
pub trait Rsvp {
    /// make a reservation
    async fn reserve(&self, rsvp: abi::Reservation) -> Result<abi::Reservation, Error>;
    /// change reservation status(if current status is pending , change it to confirmed)
    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, Error>;
    /// update note
    async fn update_note(&self, id: ReservationId, note: String)
    -> Result<abi::Reservation, Error>;
    /// delete reservation
    async fn delete(&self, id: ReservationId) -> Result<(), Error>;
    /// get reservation by id
    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, Error>;
    /// query reservations
    async fn query(&self, query: ReservationQuery) -> Result<Vec<abi::Reservation>, Error>;
    /// filter reservations
    async fn filter(
        &self,
        filter: abi::ReservationFilter,
    ) -> Result<(FilterPager, Vec<abi::Reservation>), abi::Error>;
}
