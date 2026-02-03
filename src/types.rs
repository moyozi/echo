use mediasoup::{prelude::*, types::sctp_parameters::SctpParameters};
use serde::{Deserialize, Serialize};

use std::num::{NonZeroU8, NonZeroU32};
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterRequest {
    pub room_id: String,
    pub token: String, // Placeholder auth
    pub peer_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterResponse {
    pub router_rtp_capabilities: RtpCapabilitiesFinalized,
    pub consumer_transport_options: Option<TransportOptions>,
    pub producer_transport_options: Option<TransportOptions>,
    pub data_producer_id: DataProducerId,
    pub room_id: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportOptions {
    pub id: TransportId,
    pub dtls_parameters: DtlsParameters,
    pub ice_candidates: Vec<IceCandidate>,
    pub ice_parameters: IceParameters,
    pub sctp_parameters: Option<SctpParameters>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectTransportRequest {
    pub dtls_parameters: DtlsParameters,
    pub peer_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProduceRequest {
    pub kind: MediaKind,
    pub rtp_parameters: RtpParameters,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumeRequest {
    pub producer_id: ProducerId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataConsumeRequest {
    pub room_id: String,
    pub token: String,
    pub peer_id: String,
    pub data_producer_id: DataProducerId,
}

pub fn media_codecs() -> Vec<RtpCodecCapability> {
    vec![
        RtpCodecCapability::Audio {
            mime_type: MimeTypeAudio::Opus,
            preferred_payload_type: None,
            clock_rate: NonZeroU32::new(48000).unwrap(),
            channels: NonZeroU8::new(2).unwrap(),
            parameters: RtpCodecParametersParameters::from([("useinbandfec", 1_u32.into())]),
            rtcp_feedback: vec![RtcpFeedback::TransportCc],
        },
        RtpCodecCapability::Video {
            mime_type: MimeTypeVideo::Vp8,
            preferred_payload_type: None,
            clock_rate: NonZeroU32::new(90000).unwrap(),
            parameters: RtpCodecParametersParameters::default(),
            rtcp_feedback: vec![
                RtcpFeedback::Nack,
                RtcpFeedback::NackPli,
                RtcpFeedback::CcmFir,
                RtcpFeedback::GoogRemb,
                RtcpFeedback::TransportCc,
            ],
        },
    ]
}
