mod service;

use std::pin::Pin;

use abi::{Config, Reservation, reservation_service_server::ReservationServiceServer};
use futures::Stream;
use reservation::ReservationManager;
use tonic::transport::Server;

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, tonic::Status>> + Send>>;

pub struct RsvpService {
    manager: ReservationManager,
}

pub async fn start_server(config: &Config) -> Result<(), anyhow::Error> {
    let addr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    let svc = RsvpService::from_config(config).await?;
    let svc = ReservationServiceServer::new(svc);

    println!("Listening on {}", addr);
    Server::builder().add_service(svc).serve(addr).await?;
    Ok(())
}
