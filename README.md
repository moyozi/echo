# echo



media server config
```json
{
  "hostIp":"127.0.0.1", //内网ip
  "announceIp":"22.33.12.192", //外网ip
  "worker":10, //worker数量，0表示机器cpu数量
  "type":"sendrecv","sendonly","recvonly" // 类型 纯发送，纯接收，发送接收
}
```

dispatch config
```json
{
  "rooms":{
  "broadcastSingle":["room1","room2"],
  "broadcastCluster":["room3","room4"],
  },
  "hosts":{
    "hostIpMap":{
      "host1":{"LAN":"ip","WAN":"ip"},
      "host2":{"LAN":"ip","WAN":"ip"},
      "host3":{"LAN":"ip","WAN":"ip"}
    };
    
    "sendrecv":[
      "host1"
    ];
    
    "sendonly":[
     "host2"
    ];
    
    "recvonly":[
     "host3"
    ]
  }

}

```
local
```rust 


type Room struct{
  RoomId:String
  SendRouter:Router
  RecvRouters:Vec<Router> 
  Producers:HashMap<PeerId,Vec<Producer>>
  Piped: HashSet<(ProdcuerId,RouterId)>
}

type Peer struct{
  PeerId:String
  UserId:String
  Room:Arc<Room>
  SendRouter:Router
  RecvRouter:Router 
  ProducerTransport:WebRtcTransport
  ConsumerTransport:WebRtcTransport
  Producers:HashMap<ProducerId,Producer>
  Consumers:HashMap<ConsumerId,Consumer>
}

```
cluster
```rust
type Room struct{

}
type PeerSend struct{
  PeerId:String
  Room:Arc<Room>
  Router:Router
  ProducerTransport:WebRtcTransport
  Producers:HashMap<ProducerId,Producer>
  PipeTransport:HashMap<HostId,PipeTransport>
  PipeConsumer: HashMap<ProducerId,HostId>
}


type PeerRecv struct{
  PeerId:String
  Room:Arc<Room>
  Router:Router
  ConsumerTransport:WebRtcTransport
  Producers:HashMap<String,Producer>
  Consumers:HashMap<String,Consumer>
}

```
