use std::pin::Pin;

use abi::{
    CancelRequest, CancelResponse, Config, ConfirmRequest, ConfirmResponse, FilterRequest,
    FilterResponse, GetRequest, GetResponse, ListenRequest, QueryRequest, Reservation,
    ReserveRequest, ReserveResponse, UpdateRequest, UpdateResponse,
    reservation_service_server::{ReservationService, ReservationServiceServer},
};
use futures::Stream;
use reservation::{ReservationManager, Rsvp};
use tonic::{Response, Status, async_trait, transport::Server};

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, tonic::Status>> + Send>>;

pub struct RsvpService {
    manager: ReservationManager,
}

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

pub async fn start_server(config: &Config) -> Result<(), anyhow::Error> {
    let addr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    let svc = RsvpService::from_config(config).await?;
    let svc = ReservationServiceServer::new(svc);

    println!("Listening on {}", addr);
    Server::builder().add_service(svc).serve(addr).await?;
    Ok(())
}
