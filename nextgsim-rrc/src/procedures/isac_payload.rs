//! Experimental, versioned ISAC payload carried in the RRC
//! MeasurementReportAppLayer-r17 container.
//!
//! This is a simulator-specific application payload, not a 3GPP-defined
//! ISAC payload. All numeric fields use network byte order (big-endian).

use std::convert::TryInto;

/// Magic prefix identifying this simulator-specific payload: "ISAC".
const MAGIC: &[u8; 4] = b"ISAC";

/// Version of the experimental payload format.
const VERSION: u8 = 1;

/// Payload kind for a geometry-derived range and RTT observation.
const RANGE_RTT_KIND: u8 = 1;

/// Fixed encoded length: header (6) + peer ID (8) + range (8)
/// + RTT (8) + timestamp (8) = 38 bytes.
const ENCODED_LEN: usize = 38;

/// Geometry-derived UE ranging observation.
#[derive(Debug, Clone, PartialEq)]
pub struct IsacRangeRttPayload {
    /// Configured peer UE ID used by the ranging simulator.
    pub peer_ue_id: u64,
    /// Geometry-derived range in metres.
    pub range_m: f64,
    /// Modelled round-trip time in nanoseconds.
    pub rtt_ns: u64,
    /// Measurement timestamp in milliseconds since Unix epoch.
    pub timestamp_ms: u64,
}

/// Encode one observation into the experimental application-layer payload.
pub fn encode_isac_range_rtt(payload: &IsacRangeRttPayload) -> Result<Vec<u8>, &'static str> {
    if !payload.range_m.is_finite() || payload.range_m < 0.0 {
        return Err("range_m must be finite and non-negative");
    }

    let mut bytes = Vec::with_capacity(ENCODED_LEN);
    bytes.extend_from_slice(MAGIC);
    bytes.push(VERSION);
    bytes.push(RANGE_RTT_KIND);
    bytes.extend_from_slice(&payload.peer_ue_id.to_be_bytes());
    bytes.extend_from_slice(&payload.range_m.to_bits().to_be_bytes());
    bytes.extend_from_slice(&payload.rtt_ns.to_be_bytes());
    bytes.extend_from_slice(&payload.timestamp_ms.to_be_bytes());

    Ok(bytes)
}

/// Decode one observation, rejecting unknown versions, kinds, lengths,
/// invalid magic, and non-finite or negative ranges.
pub fn decode_isac_range_rtt(bytes: &[u8]) -> Result<IsacRangeRttPayload, &'static str> {
    if bytes.len() != ENCODED_LEN {
        return Err("invalid ISAC payload length");
    }
    if &bytes[0..4] != MAGIC {
        return Err("invalid ISAC payload magic");
    }
    if bytes[4] != VERSION {
        return Err("unsupported ISAC payload version");
    }
    if bytes[5] != RANGE_RTT_KIND {
        return Err("unsupported ISAC payload kind");
    }

    let peer_ue_id = u64::from_be_bytes(bytes[6..14].try_into().map_err(|_| "invalid peer ID")?);
    let range_bits = u64::from_be_bytes(bytes[14..22].try_into().map_err(|_| "invalid range")?);
    let range_m = f64::from_bits(range_bits);
    let rtt_ns = u64::from_be_bytes(bytes[22..30].try_into().map_err(|_| "invalid RTT")?);
    let timestamp_ms =
        u64::from_be_bytes(bytes[30..38].try_into().map_err(|_| "invalid timestamp")?);

    if !range_m.is_finite() || range_m < 0.0 {
        return Err("decoded range_m must be finite and non-negative");
    }

    Ok(IsacRangeRttPayload {
        peer_ue_id,
        range_m,
        rtt_ns,
        timestamp_ms,
    })
}

/// Encode a geometry-derived ISAC observation inside an actual UPER
/// `MeasurementReportAppLayer-r17` UL-DCCH message.
///
/// The inner ISAC payload is simulator-specific; the surrounding RRC
/// message uses the generated 3GPP ASN.1 types.
pub fn encode_isac_range_rtt_app_layer(
    payload: &IsacRangeRttPayload,
    config_id: u8,
) -> Result<Vec<u8>, String> {
    use crate::codec::{encode_rrc, generated::*};

    let container = encode_isac_range_rtt(payload).map_err(str::to_owned)?;

    let message = UL_DCCH_Message {
        message: UL_DCCH_MessageType::MessageClassExtension(
            UL_DCCH_MessageType_messageClassExtension::C2(
                UL_DCCH_MessageType_messageClassExtension_c2::MeasurementReportAppLayer_r17(
                    MeasurementReportAppLayer_r17 {
                        critical_extensions:
                            MeasurementReportAppLayer_r17CriticalExtensions::MeasurementReportAppLayer_r17(
                                MeasurementReportAppLayer_r17_IEs {
                                    measurement_report_app_layer_list_r17:
                                        MeasurementReportAppLayerList_r17(vec![
                                            MeasReportAppLayer_r17 {
                                                meas_config_app_layer_id_r17:
                                                    MeasConfigAppLayerId_r17(config_id),
                                                meas_report_app_layer_container_r17: Some(
                                                    MeasReportAppLayer_r17MeasReportAppLayerContainer_r17(
                                                        container,
                                                    ),
                                                ),
                                                app_layer_session_status_r17: None,
                                                ran_visible_measurements_r17: None,
                                            },
                                        ]),
                                    late_non_critical_extension: None,
                                    non_critical_extension: None,
                                },
                            ),
                    },
                ),
            ),
        ),
    };

    encode_rrc(&message).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_layer_uper_round_trips_through_dispatch() {
        use crate::procedures::dcch_dispatch::{dispatch_ul_dcch, UlDcchMessage};

        let original = IsacRangeRttPayload {
            peer_ue_id: 42,
            range_m: 12.5,
            rtt_ns: 83,
            timestamp_ms: 1_800_000_000_000,
        };

        let encoded =
            encode_isac_range_rtt_app_layer(&original, 1).expect("encode full UPER RRC message");

        match dispatch_ul_dcch(&encoded).expect("dispatch encoded RRC message") {
            UlDcchMessage::MeasurementReportAppLayer(reports) => {
                assert_eq!(reports.len(), 1);
                assert_eq!(reports[0].config_id, 1);
                let container = reports[0]
                    .container
                    .as_deref()
                    .expect("ISAC application container");
                let decoded =
                    decode_isac_range_rtt(container).expect("decode ISAC application payload");
                assert_eq!(decoded, original);
            }
            other => panic!("unexpected UL-DCCH message: {other:?}"),
        }
    }

    #[test]
    fn range_rtt_payload_round_trips() {
        let original = IsacRangeRttPayload {
            peer_ue_id: 42,
            range_m: 12.5,
            rtt_ns: 83,
            timestamp_ms: 1_800_000_000_000,
        };

        let encoded = encode_isac_range_rtt(&original).expect("encode payload");
        let decoded = decode_isac_range_rtt(&encoded).expect("decode payload");

        assert_eq!(decoded, original);
        assert_eq!(encoded.len(), ENCODED_LEN);
    }

    #[test]
    fn decoder_rejects_wrong_magic_version_kind_and_length() {
        let original = IsacRangeRttPayload {
            peer_ue_id: 1,
            range_m: 1.0,
            rtt_ns: 7,
            timestamp_ms: 9,
        };
        let valid = encode_isac_range_rtt(&original).expect("encode payload");

        assert!(decode_isac_range_rtt(&valid[..valid.len() - 1]).is_err());

        let mut bad_magic = valid.clone();
        bad_magic[0] = b'X';
        assert!(decode_isac_range_rtt(&bad_magic).is_err());

        let mut bad_version = valid.clone();
        bad_version[4] = VERSION + 1;
        assert!(decode_isac_range_rtt(&bad_version).is_err());

        let mut bad_kind = valid;
        bad_kind[5] = RANGE_RTT_KIND + 1;
        assert!(decode_isac_range_rtt(&bad_kind).is_err());
    }

    #[test]
    fn encoder_rejects_invalid_range() {
        for range_m in [-1.0, f64::NAN, f64::INFINITY] {
            let payload = IsacRangeRttPayload {
                peer_ue_id: 1,
                range_m,
                rtt_ns: 1,
                timestamp_ms: 1,
            };
            assert!(encode_isac_range_rtt(&payload).is_err());
        }
    }

    #[test]
    fn decoder_rejects_non_finite_range() {
        let payload = IsacRangeRttPayload {
            peer_ue_id: 1,
            range_m: 1.0,
            rtt_ns: 1,
            timestamp_ms: 1,
        };
        let mut bytes = encode_isac_range_rtt(&payload).expect("encode payload");
        bytes[14..22].copy_from_slice(&f64::NAN.to_bits().to_be_bytes());

        assert!(decode_isac_range_rtt(&bytes).is_err());
    }
}
