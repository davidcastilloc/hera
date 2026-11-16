use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STD;
use hera_node_rs::cue_engine::CueOrigin;
use hera_node_rs::scanner::fast_seek::FastScanResult;
use hera_node_rs::scanner::full_pass::{FullHotCue, FullPassResult};

use super::track::HotCuePin;

#[derive(Debug, Clone)]
pub struct ScannerRowUpdate {
    pub track_id: String,
    pub bpm: Option<f64>,
    pub camelot: Option<String>,
    pub energy: Option<f64>,
    pub hot_cues_json: Option<String>,
    pub waveform_peaks_blob: Option<Vec<u8>>,
    pub waveform_peaks_b64: Option<String>,
    pub embedding_64d_b64: Option<String>,
    pub loudness_lufs: Option<f64>,
    pub true_peak_dbfs: Option<f64>,
    pub replay_gain_db: Option<f64>,
    pub entry_bpm: Option<f64>,
    pub exit_bpm: Option<f64>,
    pub has_intro_vocals: Option<bool>,
    pub has_outro_vocals: Option<bool>,
    pub has_audio_skips: Option<bool>,
    pub glitch_details: Option<String>,
    pub scan_level: i64,
    pub is_transition_candidate: bool,
    pub is_fake_320kbps: bool,
    pub duration_sec: Option<u32>,
}

impl ScannerRowUpdate {
    pub fn failure(track_id: String, duration_sec: Option<u32>, scan_level: i64) -> Self {
        Self {
            track_id,
            bpm: None,
            camelot: None,
            energy: None,
            hot_cues_json: None,
            waveform_peaks_blob: None,
            waveform_peaks_b64: None,
            embedding_64d_b64: None,
            loudness_lufs: None,
            true_peak_dbfs: None,
            replay_gain_db: None,
            entry_bpm: None,
            exit_bpm: None,
            has_intro_vocals: None,
            has_outro_vocals: None,
            has_audio_skips: None,
            glitch_details: None,
            scan_level,
            is_transition_candidate: false,
            is_fake_320kbps: false,
            duration_sec,
        }
    }

    pub fn from_fast_scan(
        track_id: String,
        res: FastScanResult,
        duration_sec: Option<u32>,
    ) -> Self {
        let mut upd = Self::failure(track_id, duration_sec, 1);
        upd.bpm = Some(res.bpm);
        upd.camelot = Some(res.camelot_key);
        upd.entry_bpm = res.entry_bpm;
        upd.exit_bpm = res.exit_bpm;
        upd.is_transition_candidate = res.is_transition_candidate;
        upd.is_fake_320kbps = res.is_fake_320kbps;
        upd
    }

    pub fn from_full_pass(
        track_id: String,
        res: FullPassResult,
        duration_sec: Option<u32>,
    ) -> (Self, Vec<HotCuePin>) {
        let pins = map_full_cues(res.hot_cues);
        let hot_cues_json = serde_json::to_string(&pins).ok();
        let waveform_peaks_b64 =
            (!res.rgb_waveform_blob.is_empty()).then(|| BASE64_STD.encode(&res.rgb_waveform_blob));
        let embedding_64d_b64 = (!res.embedding_64d_q8.is_empty()).then(|| {
            let bytes: Vec<u8> = res.embedding_64d_q8.iter().map(|&v| v as u8).collect();
            BASE64_STD.encode(&bytes)
        });
        let glitch_details = if res.glitch_report.has_audio_skips {
            Some(res.glitch_report.warning_summary.unwrap_or_else(|| {
                let ts: Vec<String> = res
                    .glitch_report
                    .skip_timestamps_seconds
                    .iter()
                    .take(8)
                    .map(|t| format!("{t:.2}s"))
                    .collect();
                format!(
                    "Skips detected (dropouts={}, stutters={}): {}",
                    res.glitch_report.micro_dropout_count,
                    res.glitch_report.buffer_repeat_stutter_count,
                    ts.join(", ")
                )
            }))
        } else {
            None
        };

        let upd = Self {
            track_id,
            bpm: Some(res.bpm),
            camelot: Some(res.camelot_key),
            energy: None,
            hot_cues_json,
            waveform_peaks_blob: Some(res.waveform_peaks_blob),
            waveform_peaks_b64,
            embedding_64d_b64,
            loudness_lufs: Some(f64::from(res.loudness.loudness_lufs)),
            true_peak_dbfs: Some(f64::from(res.loudness.true_peak_dbfs)),
            replay_gain_db: Some(f64::from(res.loudness.replay_gain_db)),
            entry_bpm: res.entry_bpm,
            exit_bpm: res.exit_bpm,
            has_intro_vocals: Some(res.has_intro_vocals),
            has_outro_vocals: Some(res.has_outro_vocals),
            has_audio_skips: Some(res.glitch_report.has_audio_skips),
            glitch_details,
            scan_level: 2,
            is_transition_candidate: res.is_transition_candidate,
            is_fake_320kbps: false,
            duration_sec,
        };
        (upd, pins)
    }
}

fn map_full_cues(cues: Vec<FullHotCue>) -> Vec<HotCuePin> {
    cues.into_iter()
        .take(8)
        .map(|c| {
            let (color, default_role) = match c.num {
                1 => ("#22c55e", "Intro"),
                2 => ("#38bdf8", "Breakdown"),
                3 => ("#ef4444", "Drop"),
                4 => ("#f59e0b", "Outro"),
                5 => ("#a855f7", "Phrase"),
                6 => ("#06b6d4", "Breakdown"),
                7 => ("#ec4899", "Drop"),
                _ => ("#eab308", "Outro"),
            };
            let role = if c.role.is_empty() {
                default_role.to_string()
            } else {
                c.role
            };
            let origin = match c.origin {
                CueOrigin::DjManual => "dj_manual",
                CueOrigin::HeraAuto => "hera_auto",
            };
            HotCuePin {
                num: c.num,
                name: c.name,
                seconds: ((c.seconds * 1000.0).round()) / 1000.0,
                color: color.to_string(),
                role,
                origin: Some(origin.to_string()),
            }
        })
        .collect()
}

pub(super) fn merge_preserving_manual_hot_cues(
    existing_json: Option<&str>,
    incoming_json: Option<&str>,
) -> Option<String> {
    let incoming_str = incoming_json?;
    let existing_pins: Vec<HotCuePin> = existing_json
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let has_manual = existing_pins
        .iter()
        .any(|p| p.origin.as_deref() == Some("dj_manual"));
    if !has_manual {
        return Some(incoming_str.to_string());
    }
    let incoming_pins: Vec<HotCuePin> = serde_json::from_str(incoming_str).unwrap_or_default();
    let mut merged: BTreeMap<u8, HotCuePin> = BTreeMap::new();
    for pin in existing_pins {
        if pin.origin.as_deref() == Some("dj_manual") && (1..=8).contains(&pin.num) {
            merged.insert(pin.num, pin);
        }
    }
    for pin in incoming_pins {
        if (1..=8).contains(&pin.num) {
            merged.entry(pin.num).or_insert(pin);
        }
    }
    let final_vec: Vec<HotCuePin> = merged.into_values().take(8).collect();
    serde_json::to_string(&final_vec).ok()
}
