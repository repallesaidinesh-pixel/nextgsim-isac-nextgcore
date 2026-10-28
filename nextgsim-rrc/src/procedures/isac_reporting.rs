//! Experimental ISAC range reporting used by the simulator.
//! The inner payload is private to this simulator, not a standardized 3GPP format.

use crate::codec::{decode_rrc, generated::*};

const PAYLOAD_VERSION: u8 = 1;
const PAYLOAD_LEN: usize = 30;

#[derive(Debug, Clone, PartialEq)]
pub struct IsacRangeRttReport {
    pub config_id: u8,
    pub peer_ue_id: u64,
    pub distance_m: f64,
    pub accuracy_m: f64,
    pub measurement_count: u32,
    pub method: String,
}

pub fn decode_isac_range_rtt_report(bytes: &[u8]) -> Result<IsacRangeRttReport, String> {
    let message: UL_DCCH_Message = decode_rrc(bytes).map_err(|e| format!("{e:?}"))?;

    let extension = match message.message {
        UL_DCCH_MessageType::MessageClassExtension(extension) => extension,
        _ => return Err("not an application-layer measurement report".into()),
    };

    let c2 = match extension {
        UL_DCCH_MessageType_messageClassExtension::C2(c2) => c2,
        _ => return Err("unsupported message extension".into()),
    };

    let report = match c2 {
        UL_DCCH_MessageType_messageClassExtension_c2::MeasurementReportAppLayer_r17(report) => {
            report
        }
        _ => return Err("not a MeasurementReportAppLayer-r17 message".into()),
    };

    let ies = match report.critical_extensions {
        MeasurementReportAppLayer_r17CriticalExtensions::MeasurementReportAppLayer_r17(ies) => ies,
        _ => return Err("unsupported report critical extension".into()),
    };

    if ies.measurement_report_app_layer_list_r17.0.len() != 1 {
        return Err("expected exactly one application-layer report".into());
    }

    let entry = &ies.measurement_report_app_layer_list_r17.0[0];
    let container = entry
        .meas_report_app_layer_container_r17
        .as_ref()
        .ok_or_else(|| "missing application-layer container".to_string())?;
    let payload = &container.0;

    if payload.len() != PAYLOAD_LEN {
        return Err(format!(
            "invalid experimental ISAC payload length: {}",
            payload.len()
        ));
    }
    if payload[0] != PAYLOAD_VERSION {
        return Err(format!(
            "unsupported experimental ISAC payload version: {}",
            payload[0]
        ));
    }

    let peer_ue_id = u64::from_be_bytes(payload[1..9].try_into().map_err(|_| "invalid peer ID")?);
    let distance_mm =
        u64::from_be_bytes(payload[9..17].try_into().map_err(|_| "invalid distance")?);
    let accuracy_mm =
        u64::from_be_bytes(payload[17..25].try_into().map_err(|_| "invalid accuracy")?);
    let measurement_count = u32::from_be_bytes(
        payload[25..29]
            .try_into()
            .map_err(|_| "invalid measurement count")?,
    );
    let method = match payload[29] {
        0 => "rtt",
        1 => "carrier_phase",
        code => return Err(format!("invalid experimental ISAC method code: {code}")),
    };

    if peer_ue_id == 0 {
        return Err("peer UE ID must be nonzero".into());
    }

    Ok(IsacRangeRttReport {
        config_id: entry.meas_config_app_layer_id_r17.0,
        peer_ue_id,
        distance_m: distance_mm as f64 / 1000.0,
        accuracy_m: accuracy_mm as f64 / 1000.0,
        measurement_count,
        method: method.to_string(),
    })
}
