mod state;
mod types;
use crate::state::{AppState, Peer, Room};
use crate::types::{ConnectTransportRequest, ConsumeRequest, DataConsumeRequest, EnterRequest};

use crate::types::media_codecs;
use actix_web::{App, HttpResponse, HttpServer, web};
use mediasoup::prelude::*;
use mediasoup::types::data_structures::TransportTuple;
use reqwest::Client;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::ops::RangeInclusive;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;
// Endpoint: POST /enter {roomId, token}
async fn enter(req: web::Json<EnterRequest>, data: web::Data<AppState>) -> HttpResponse {
    let mut rooms = data.rooms.write().unwrap();
    let room_id = &req.room_id;

    let worker = data
        .worker_manager
        .create_worker(WorkerSettings::default())
        .await
        .unwrap();
    let router = worker
        .create_router(RouterOptions::new(media_codecs()))
        .await
        .unwrap();
    let mut room = Room {
        id: room_id.clone(),
        router,
        peers: HashMap::new(),
        data_producer: None,
        producers: HashMap::new(),
        pipe_transport: None,
    };

    let peer_id = Uuid::new_v4().to_string();

    // Create recv transport for consumption
    let recv_transport = room
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
            send_transport: None,
            recv_transport: Some(recv_transport.clone()),
            data_consumer: None,
            consumers: HashMap::new(),
        },
    );

    // Fetch from pubSvr: DataProducerID (diagram: subSvr->pubSvr:GetRoom)
    let client = Client::new();
    let pub_res = client
        .post("http://pubsvr:3000/getRoom") // Implement /getRoom on pubSvr
        .json(&serde_json::json!({"roomId": room_id}))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let data_producer_id = pub_res["dataProducerId"].as_str().unwrap();
    let cap = room.router.rtp_capabilities().clone();
    rooms.insert(room_id.to_string(), room);
    HttpResponse::Ok().json(serde_json::json!({
        "rtpCapabilities":cap,
        "peerId": peer_id,
        "consumerOptions": recv_transport.dump().await.unwrap(),  // ICE/DTLS for client
        "dataProducerId": data_producer_id,
    }))
}

// Endpoint: POST /dataConsume {roomId, token, peerId, dataProducerId}
async fn data_consume(
    req: web::Json<DataConsumeRequest>,
    data: web::Data<AppState>,
) -> HttpResponse {
    let room_id = &req.room_id;
    let mut rooms = data.rooms.write().unwrap();
    let room = rooms.get_mut(room_id).unwrap();

    // Create PipeTransport locally
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

    let local_tuple = pipe_transport.tuple();

    // Send to pubSvr to create/connect its pipe (diagram: subSvr->pubSvr:createPipeTransport)
    let client = Client::new();
    let pub_res = client
        .post("http://pubsvr:3000/createPipeTransport")
        .json(&serde_json::json!({
            "roomId": room_id,
            "tuple": local_tuple,
        }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let remote_tuple: TransportTuple = serde_json::from_value(pub_res["tuple"].clone()).unwrap();

    // Connect our pipe to pub's tuple
    pipe_transport
        .connect(PipeTransportRemoteParameters {
            ip: remote_tuple.remote_ip().unwrap(),
            port: remote_tuple.remote_port().unwrap(),
            srtp_parameters: None,
        })
        .await
        .unwrap();

    room.pipe_transport = Some(pipe_transport.clone());

    // Consume data from pub's data_producer via pipe
    let data_consumer = pipe_transport
        .consume_data(DataConsumerOptions::new_direct(req.data_producer_id, None))
        .await
        .unwrap();

    // Store in peer; assume peer_id from req
    let peer = room.peers.get_mut(&req.peer_id).unwrap();
    peer.data_consumer = Some(data_consumer.clone());

    HttpResponse::Ok().json(serde_json::json!({ "dataConsumerId": data_consumer.id() }))
}

// Endpoint: POST /createRecvTransport {dtlsParameters, peerId} - Client connects recv transport
async fn create_recv_transport(
    req: web::Json<ConnectTransportRequest>,
    data: web::Data<AppState>,
) -> HttpResponse {
    // Similar to connect_send_transport; locate recv_transport and call .connect()
    HttpResponse::Ok().json("ok")
}

// Endpoint: POST /dataConsumeResume {dataConsumerId, peerId}
async fn data_consume_resume(// req with dataConsumerId, peerId
    // data: web::Data<AppState>,
) -> HttpResponse {
    // Locate consumer and call consumer.resume().await
    HttpResponse::Ok().json("ok")
}

// Endpoint: POST /consume {producerId}
async fn consume(req: web::Json<ConsumeRequest>, data: web::Data<AppState>) -> HttpResponse {
    // Locate room/peer/recv_transport
    // Create consumer = recv_transport.consume(ConsumerOptions::new(req.producer_id, rtp_capabilities))
    // For piping media: if producer is on pub, use pipe_transport.consume() instead, but diagram uses recv_transport
    // Diagram shows Consume after pipeProducer notification
    HttpResponse::Ok().json("ok") // Return consumer params for client
}

// Endpoint: POST /consumeResume {producerId}
async fn consume_resume(// Similar: consumer.resume()
) -> HttpResponse {
    HttpResponse::Ok().json("ok")
}

pub async fn run_sub_server() -> std::io::Result<()> {
    // Similar to run_pub_server, on port 4000
    // Add routes for above endpoints
    env_logger::init();

    let worker_manager = WorkerManager::new();
    let listen_ip = IpAddr::from_str("0.0.0.0").unwrap();
    let state = web::Data::new(AppState {
        worker_manager,
        rooms: Arc::new(RwLock::new(HashMap::new())),
        listen_ip,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/enter", web::post().to(enter))
            .route("/dataConsume", web::post().to(data_consume))
            .route("/dataConsumeResume", web::post().to(data_consume_resume))
            .route(
                "/createRecvTransport",
                web::post().to(create_recv_transport),
            )
            .route("/consume", web::post().to(consume))
            .route("/consumeResume", web::post().to(consume_resume))
    })
    .bind(("0.0.0.0", 4000))?
    .run()
    .await?;

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    run_sub_server().await
}
