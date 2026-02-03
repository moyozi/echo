mod state;
mod types;
use crate::state::{AppState, Peer, Room};
use crate::types::{
    ConnectTransportRequest, EnterRequest, EnterResponse, ProduceRequest, TransportOptions,
};
use actix_web::{App, HttpResponse, HttpServer, web};
use mediasoup::prelude::*;
use mediasoup::types::data_structures::TransportTuple;
use mediasoup::worker::{WorkerLogLevel, WorkerLogTag};
use std::collections::HashMap;
use std::i16;
use std::net::{IpAddr, Ipv4Addr};
use std::ops::RangeInclusive;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;

use crate::types::media_codecs;
// Endpoint: POST /enter {roomId, token}
async fn enter(req: web::Json<EnterRequest>, data: web::Data<AppState>) -> HttpResponse {
    let mut rooms = data.rooms.write().unwrap();
    let room_id = &req.room_id;
    let peer_id = if req.peer_id.is_some() {
        req.peer_id.clone().unwrap()
    } else {
        Uuid::new_v4().to_string()
    };
    let room_opt = rooms.get_mut(room_id);
    if room_opt.is_some() {
        let room = room_opt.unwrap();
        let cap = room.router.rtp_capabilities();
        let send_transport = if room.peers.get(&peer_id).is_none() {
            let s = room
                .router
                .create_webrtc_transport(WebRtcTransportOptions::new(
                    WebRtcTransportListenInfos::new(ListenInfo {
                        protocol: Protocol::Tcp,
                        ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                        announced_address: Some("10.6.5.218".to_string()),
                        expose_internal_ip: false,
                        port: None,
                        port_range: Some(RangeInclusive::new(5170, 5172)),
                        flags: None,
                        send_buffer_size: None,
                        recv_buffer_size: None,
                    }),
                ))
                .await
                .unwrap();

            room.peers.insert(
                peer_id.clone(),
                Peer {
                    id: peer_id.clone(),
                    send_transport: Some(s.clone()),
                    recv_transport: None,
                    data_consumer: None,
                    consumers: HashMap::new(),
                },
            );
            s
        } else {
            let p = room.peers.get(&peer_id).unwrap();
            p.send_transport.clone().unwrap()
        };
        let data_producer = room.data_producer.clone().unwrap();
        HttpResponse::Ok().json(serde_json::json!(EnterResponse {
            room_id: room_id.to_string(),
            router_rtp_capabilities: cap,
            producer_transport_options: Some(TransportOptions {
                id: send_transport.id(),
                dtls_parameters: send_transport.dtls_parameters(),
                ice_candidates: send_transport.ice_candidates().clone(),
                ice_parameters: send_transport.ice_parameters().clone(),
                sctp_parameters: send_transport.sctp_parameters(),
            }),
            consumer_transport_options: None,
            data_producer_id: data_producer.id()
        }))
    } else {
        let worker = &data.worker_pool[0];
        let router = worker
            .create_router(RouterOptions::new(media_codecs()))
            .await
            .unwrap(); // Add codecs as needed
        let mut room = Room {
            id: room_id.clone(),
            router,
            peers: HashMap::new(),
            data_producer: None,
            producers: HashMap::new(),
            pipe_transport: None,
        };

        // Create DirectTransport for data producer
        let direct_transport = room
            .router
            .create_direct_transport(DirectTransportOptions::default())
            .await
            .unwrap();

        // Create DataProducer
        let data_producer = direct_transport
            .produce_data(DataProducerOptions::new_direct())
            .await
            .unwrap();

        room.data_producer = Some(data_producer.clone());

        let send_transport = room
            .router
            .create_webrtc_transport(WebRtcTransportOptions::new(
                WebRtcTransportListenInfos::new(ListenInfo {
                    protocol: Protocol::Tcp,
                    ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                    announced_address: Some("10.6.5.218".to_string()),
                    expose_internal_ip: false,
                    port: None,
                    port_range: Some(RangeInclusive::new(5170, 5172)),
                    flags: None,
                    send_buffer_size: None,
                    recv_buffer_size: None,
                }),
            ))
            .await
            .unwrap();

        room.peers.insert(
            peer_id.clone(),
            Peer {
                id: peer_id.clone(),
                send_transport: Some(send_transport.clone()),
                recv_transport: None,
                data_consumer: None,
                consumers: HashMap::new(),
            },
        );

        // Create send transport for media production
        let cap = room.router.rtp_capabilities().clone();
        rooms.insert(room_id.to_string(), room);
        HttpResponse::Ok().json(serde_json::json!(EnterResponse {
            room_id: room_id.to_string(),
            router_rtp_capabilities: cap,
            producer_transport_options: Some(TransportOptions {
                id: send_transport.id(),
                dtls_parameters: send_transport.dtls_parameters(),
                ice_candidates: send_transport.ice_candidates().clone(),
                ice_parameters: send_transport.ice_parameters().clone(),
                sctp_parameters: send_transport.sctp_parameters(),
            }),
            consumer_transport_options: None,
            data_producer_id: data_producer.id()
        }))
    }
}

// Endpoint: POST /produce {kind, rtpParameters} - Broadcaster produces media
async fn produce(req: web::Json<ProduceRequest>, data: web::Data<AppState>) -> HttpResponse {
    // Locate room/peer from query params or body; simplified
    // Assume room_id and peer_id in body for demo
    let room_id = "example".to_string(); // TODO: Extract properly
    let peer_id = "example".to_string();

    let rooms = data.rooms.read().unwrap();
    let room = rooms.get(&room_id).unwrap();
    let peer = room.peers.get(&peer_id).unwrap();
    let send_transport = peer.send_transport.as_ref().unwrap();

    let producer = send_transport
        .produce(ProducerOptions::new(req.kind, req.rtp_parameters.clone()))
        .await
        .unwrap();
    let producer_id = producer.id();

    drop(rooms); // Release read lock
    let mut rooms = data.rooms.write().unwrap();
    let room = rooms.get_mut(&room_id).unwrap();
    room.producers.insert(producer_id, producer);

    // Notify subSvr(s) of new producer (e.g., via HTTP POST to subSvr)
    // In production: use pub/sub like Redis or WebSocket

    HttpResponse::Ok().json(serde_json::json!({ "producerId": producer_id }))
}

// Endpoint: POST /connectSendTransport {dtlsParameters, peerId}
async fn connect_send_transport(
    req: web::Json<ConnectTransportRequest>,
    data: web::Data<AppState>,
) -> HttpResponse {
    // Locate transport
    let room_id = "example".to_string(); // TODO
    let rooms = data.rooms.read().unwrap();
    let room = rooms.get(&room_id).unwrap();
    let peer = room.peers.get(&req.peer_id).unwrap();
    let send_transport = peer.send_transport.as_ref().unwrap();

    send_transport
        .connect(WebRtcTransportRemoteParameters {
            dtls_parameters: req.dtls_parameters.clone(),
        })
        .await
        .unwrap();

    HttpResponse::Ok().json("ok")
}

// Endpoint for subSvr: POST /createPipeTransport {roomId} - Create and return tuple for pipe
async fn create_pipe_transport_for_sub(
    req: web::Json<serde_json::Value>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let room_id = req["roomId"].as_str().unwrap().to_string();
    let remote_tuple: TransportTuple = serde_json::from_value(req["tuple"].clone()).unwrap(); // From subSvr

    let mut rooms = data.rooms.write().unwrap();
    let room = rooms.get_mut(&room_id).unwrap();

    let pipe_transport = room
        .router
        .create_pipe_transport(PipeTransportOptions::new(ListenInfo {
            protocol: Protocol::Tcp,
            ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            announced_address: Some("10.6.5.218".to_string()),
            expose_internal_ip: false,
            port: None,
            port_range: Some(RangeInclusive::new(5170, 5172)),
            flags: None,
            send_buffer_size: None,
            recv_buffer_size: None,
        }))
        .await
        .unwrap();

    // Connect to subSvr's tuple
    pipe_transport
        .connect(PipeTransportRemoteParameters {
            ip: remote_tuple.remote_ip().unwrap(),
            port: remote_tuple.remote_port().unwrap(),
            srtp_parameters: None,
        })
        .await
        .unwrap();

    room.pipe_transport = Some(pipe_transport.clone());

    // Return our local tuple for subSvr to connect
    HttpResponse::Ok().json(serde_json::json!({
        "tuple": pipe_transport.tuple(),
        "dataProducerId": room.data_producer.as_ref().unwrap().id(),
    }))
}

// Add more: e.g., GET /getRoom for subSvr to fetch dataProducerId
const WORKERNUM: i16 = 10;
pub async fn run_pub_server() -> std::io::Result<()> {
    env_logger::init();
    let worker_manager = WorkerManager::new();
    let mut worker_pool: Vec<Worker> = vec![];

    let mut settings = WorkerSettings::default();
    settings.log_level = WorkerLogLevel::Debug;
    settings.log_tags = vec![
        WorkerLogTag::Info,
        WorkerLogTag::Ice,
        WorkerLogTag::Dtls,
        WorkerLogTag::Rtp,
        WorkerLogTag::Srtp,
        WorkerLogTag::Rtcp,
        WorkerLogTag::Rtx,
        WorkerLogTag::Bwe,
        WorkerLogTag::Score,
        WorkerLogTag::Simulcast,
        WorkerLogTag::Svc,
        WorkerLogTag::Sctp,
        WorkerLogTag::Message,
    ];

    for _x in 0..WORKERNUM {
        let w = worker_manager
            .create_worker(settings.clone())
            .await
            .unwrap();
        worker_pool.push(w);
    }
    let state = web::Data::new(AppState {
        worker_manager,
        worker_pool,
        rooms: Arc::new(RwLock::new(HashMap::new())),
        listen_ip: IpAddr::from_str("0.0.0.0").unwrap(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/enter", web::post().to(enter))
            .route("/produce", web::post().to(produce))
            .route(
                "/connectSendTransport",
                web::post().to(connect_send_transport),
            )
            .route(
                "/createPipeTransport",
                web::post().to(create_pipe_transport_for_sub),
            )
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    run_pub_server().await
}
