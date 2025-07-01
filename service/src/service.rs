use abi::{
    CancelRequest, CancelResponse, Config, ConfirmRequest, ConfirmResponse, FilterRequest,
    FilterResponse, GetRequest, GetResponse, ListenRequest, QueryRequest, ReserveRequest,
    ReserveResponse, UpdateRequest, UpdateResponse, reservation_service_server::ReservationService,
};
use reservation::{ReservationManager, Rsvp};
use tonic::{Response, Status, async_trait};

use crate::{ReservationStream, RsvpService};

impl RsvpService {
    pub async fn from_config(config: &Config) -> Result<Self, anyhow::Error> {
        Ok(Self {
            manager: ReservationManager::from_config(&config.db).await?,
        })
    }
}

#[async_trait]
impl ReservationService for RsvpService {
    /// make a reservation
    async fn reserve(
        &self,
        request: tonic::Request<ReserveRequest>,
    ) -> std::result::Result<tonic::Response<ReserveResponse>, tonic::Status> {
        let request = request.into_inner();
        if request.reservation.is_none() {
            return Err(Status::invalid_argument("missing reservation"));
        }
        let reservation = self.manager.reserve(request.reservation.unwrap()).await?;
        Ok(Response::new(ReserveResponse {
            reservation: Some(reservation),
        }))
    }
    /// confirm a pending reservation, if reservation is not pending, do nothing
    async fn confirm(
        &self,
        _request: tonic::Request<ConfirmRequest>,
    ) -> std::result::Result<tonic::Response<ConfirmResponse>, tonic::Status> {
        todo!()
    }
    /// update the reservation note
    async fn update(
        &self,
        _request: tonic::Request<UpdateRequest>,
    ) -> std::result::Result<tonic::Response<UpdateResponse>, tonic::Status> {
        todo!()
    }
    /// cancel a reservation
    async fn cancel(
        &self,
        _request: tonic::Request<CancelRequest>,
    ) -> std::result::Result<tonic::Response<CancelResponse>, tonic::Status> {
        todo!()
    }
    /// get a reservation by id
    async fn get(
        &self,
        _request: tonic::Request<GetRequest>,
    ) -> std::result::Result<tonic::Response<GetResponse>, tonic::Status> {
        todo!()
    }
    /// Server streaming response type for the query method.
    type queryStream = ReservationStream;
    /// query reservations by resource id, user id, status, start time, end time
    async fn query(
        &self,
        _request: tonic::Request<QueryRequest>,
    ) -> std::result::Result<tonic::Response<Self::queryStream>, tonic::Status> {
        todo!()
    }
    /// filter reservations, order by reservation id
    async fn filter(
        &self,
        _request: tonic::Request<FilterRequest>,
    ) -> std::result::Result<tonic::Response<FilterResponse>, tonic::Status> {
        todo!()
    }
    /// Server streaming response type for the listen method.
    type listenStream = ReservationStream;
    /// another system could monitor newly added/confirmed/cancelled reservations
    async fn listen(
        &self,
        _request: tonic::Request<ListenRequest>,
    ) -> std::result::Result<tonic::Response<Self::listenStream>, tonic::Status> {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use std::{
        ops::Deref,
        sync::Arc,
        thread::{self},
    };

    use abi::Reservation;
    use lazy_static::lazy_static;
    use sqlx::{Connection, Executor, types::Uuid};
    use tokio::runtime::Runtime;

    use super::*;

    lazy_static! {
        static ref RT: Runtime = Runtime::new().unwrap();
    }

    struct TestConfig {
        config: Arc<Config>,
    }

    impl Deref for TestConfig {
        type Target = Config;

        fn deref(&self) -> &Self::Target {
            &self.config
        }
    }

    impl TestConfig {
        pub fn new() -> Self {
            let mut config = Config::load("../service/fixtures/config.yml").unwrap();
            let db_name = format!("test_{}", Uuid::new_v4());
            config.db.dbname = db_name.clone();
            let url = config.db.url();
            let server_url = config.db.server_url();

            let _ = thread::spawn(move || {
                // create database dbname
                RT.block_on(async {
                    let mut conn = sqlx::PgConnection::connect(&server_url).await.unwrap();
                    conn.execute(format!(r#"CREATE DATABASE "{}";"#, &db_name).as_str())
                        .await
                        .expect("Error while querying the drop database.");

                    let mut conn = sqlx::PgConnection::connect(&url).await.unwrap();
                    sqlx::migrate!("../migrations")
                        .run(&mut conn)
                        .await
                        .unwrap();
                });
            })
            .join()
            .expect("failed to create database.");

            Self {
                config: Arc::new(config),
            }
        }
    }

    impl Drop for TestConfig {
        fn drop(&mut self) {
            let server_url = self.config.db.server_url();
            let db_name = self.config.db.dbname.clone();
            let _ = thread::spawn(move || {
                RT.block_on(async move {
                    let mut conn = sqlx::PgConnection::connect(&server_url).await.unwrap();
                    sqlx::query(&format!(r#"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = '{}'"#,db_name))
                    .execute(&mut conn)
                    .await
                    .expect("Terminate all other connections");
                    conn.execute(format!(r#"DROP DATABASE "{}";"#, db_name).as_str())
                        .await
                        .expect("Error while querying the drop database.");
                });
            })
            .join()
            .expect("failed to drop database.");
        }
    }

    #[tokio::test]
    async fn rpc_reserve_should_work() {
        let config = TestConfig::new();
        let service = RsvpService::from_config(&config).await.unwrap();
        let reservation = Reservation::new_pending(
            "silwings",
            "ixia-3230",
            "2025-12-26T15:00:00+0800".parse().unwrap(),
            "2025-12-30T12:00:00+0800".parse().unwrap(),
            "test device reservation",
        );
        let request = tonic::Request::new(ReserveRequest {
            reservation: Some(reservation.clone()),
        });
        let response = service.reserve(request).await.unwrap();
        let reservation_res = response.into_inner().reservation;
        assert!(reservation_res.is_some());
        let reservation_res = reservation_res.unwrap();
        assert_eq!(reservation_res.user_id, reservation.user_id);
        assert_eq!(reservation_res.resource_id, reservation.resource_id);
        assert_eq!(reservation_res.start, reservation.start);
        assert_eq!(reservation_res.end, reservation.end);
        assert_eq!(reservation_res.note, reservation.note);
        assert_eq!(reservation_res.status, reservation.status);
    }
}
