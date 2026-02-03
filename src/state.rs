use mediasoup::prelude::*;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub worker_manager: WorkerManager,
    pub worker_pool: Vec<Worker>,
    pub rooms: Arc<RwLock<HashMap<String, Room>>>,
    pub listen_ip: IpAddr, // e.g., "0.0.0.0".parse().unwrap()
}

#[derive(Clone)]
pub struct Room {
    pub id: String,
    pub router: Router,
    pub peers: HashMap<String, Peer>,
    pub data_producer: Option<DataProducer>, // Broadcaster's data producer (pubSvr only)
    pub producers: HashMap<ProducerId, Producer>, // Media producers
    pub pipe_transport: Option<PipeTransport>, // For inter-server piping
}

#[derive(Clone)]
pub struct Peer {
    pub id: String,
    pub send_transport: Option<WebRtcTransport>, // For broadcaster producing
    pub recv_transport: Option<WebRtcTransport>, // For client consuming
    pub data_consumer: Option<DataConsumer>,
    pub consumers: HashMap<ConsumerId, Consumer>,
}
