//! OS kernel worker priority lowering (`BELOW_NORMAL` / `nice(10)`), `catch_unwind`
//! isolation, and `CORRUPT_FILE` quarantine job execution for HERA's background DSP scanner.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use lofty::file::AudioFile;
use log::warn;

use hera_node_rs::bpm_engine::v2_core::audio_ingest::ProcessLimiter;
use hera_node_rs::scanner::fast_seek::run_fast_scan_on_file;
use hera_node_rs::scanner::full_pass::{FullHotCue, run_full_pass_on_file};
use hera_node_rs::scanner::scheduler::ScanPriority;

use crate::libs::db_scanner_queries::ScannerRowUpdate;
use crate::libs::track::HotCuePin;

#[cfg(windows)]
unsafe extern "system" {
    fn GetCurrentThread() -> *mut core::ffi::c_void;
    fn SetThreadPriority(hThread: *mut core::ffi::c_void, nPriority: i32) -> i32;
}

#[cfg(unix)]
unsafe extern "C" {
    fn nice(inc: i32) -> i32;
}

/// Lowers the current OS worker thread scheduling priority so background DSP analysis never
/// starves live DJ booth audio callbacks, UI rendering, or VirtualDJ/Ableton real-time threads.
pub fn lower_current_worker_thread_priority() {
    #[cfg(windows)]
    {
        const THREAD_PRIORITY_BELOW_NORMAL: i32 = -1;
        // SAFETY: `GetCurrentThread` returns a pseudo-handle valid for the calling thread,
        // and `SetThreadPriority` is safe to invoke with `THREAD_PRIORITY_BELOW_NORMAL`.
        unsafe {
            let handle = GetCurrentThread();
            let _ = SetThreadPriority(handle, THREAD_PRIORITY_BELOW_NORMAL);
        }
    }

    #[cfg(unix)]
    {
        // SAFETY: `nice(10)` adjusts the calling thread/process scheduling niceness upward
        // without dereferencing pointers or mutating Rust memory.
        unsafe {
            let _ = nice(10);
        }
    }
}

/// Executes a DSP scan job inside `catch_unwind`, converting any decoder or container parser
/// panic into a deterministic `Err("PANIC_IN_SCAN_WORKER: ...")` string.
pub fn run_isolated_scan_job<F, T>(job: F) -> Result<T, String>
where
    F: FnOnce() -> T,
{
    catch_unwind(AssertUnwindSafe(job)).map_err(|payload| {
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_owned()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_owned()
        };
        format!("PANIC_IN_SCAN_WORKER: {msg}")
    })
}

pub(super) struct ProcessedTrackOutcome {
    pub update: ScannerRowUpdate,
    pub hot_cues: Option<Vec<HotCuePin>>,
    pub corrupt_issue: Option<(String, String)>,
}

fn check_unreadable_or_zero_byte_file(path: &Path) -> Option<String> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.len() == 0 => {
            Some("ZERO_BYTE_FILE: audio container has 0 bytes".to_owned())
        }
        Ok(_) => None,
        Err(err) => Some(format!("UNREADABLE_FILE: {err}")),
    }
}

fn probe_duration_fast_secs(path: &Path) -> u32 {
    let is_video = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "webm" | "mkv" | "mp4" | "mov" | "avi" | "flv" | "m4v"
            )
        });
    if !is_video {
        return lofty::probe::Probe::open(path)
            .ok()
            .and_then(|p| {
                p.options(lofty::config::ParseOptions::new().read_cover_art(false))
                    .read()
                    .ok()
            })
            .map(|tf| tf.properties().duration().as_secs() as u32)
            .unwrap_or(0);
    }
    let mut cmd = std::process::Command::new("ffprobe");
    cmd.args([
        "-v",
        "quiet",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
    ])
    .arg(path)
    .stdin(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<f64>()
                .ok()
        })
        .filter(|d| d.is_finite() && *d > 0.0)
        .map(|d| d.round() as u32)
        .unwrap_or(0)
}

fn empty_failure_update(
    track_id: String,
    duration_sec: Option<u32>,
    target_level: i64,
) -> ScannerRowUpdate {
    ScannerRowUpdate {
        track_id,
        bpm: None,
        camelot: None,
        energy: None,
        hot_cues_json: None,
        waveform_peaks_blob: None,
        scan_level: target_level,
        is_transition_candidate: false,
        is_fake_320kbps: false,
        duration_sec,
    }
}

fn map_full_cues(cues: Vec<FullHotCue>) -> Vec<HotCuePin> {
    cues.into_iter()
        .take(4)
        .map(|c| {
            let (color, role) = match c.num {
                1 => ("#22c55e", "Intro"),
                2 => ("#38bdf8", "Breakdown"),
                3 => ("#ef4444", "Drop"),
                _ => ("#f59e0b", "Outro"),
            };
            HotCuePin {
                num: c.num,
                name: c.name,
                seconds: ((c.seconds * 10.0).round()) / 10.0,
                color: color.to_string(),
                role: role.to_string(),
            }
        })
        .collect()
}

pub(super) fn execute_isolated_worker_job(
    track_id: String,
    path: String,
    current_level: i64,
    cached_dur: u32,
    prio: ScanPriority,
    limiter: &ProcessLimiter,
) -> Option<ProcessedTrackOutcome> {
    let path_obj = Path::new(&path);
    if let Some(reason) = check_unreadable_or_zero_byte_file(path_obj) {
        warn!("Quarantining unreadable/zero-byte track {path}: {reason}");
        return Some(ProcessedTrackOutcome {
            update: empty_failure_update(track_id, None, -1),
            hot_cues: None,
            corrupt_issue: Some((path, reason)),
        });
    }
    let is_level_2 = matches!(
        prio,
        ScanPriority::P0Immediate | ScanPriority::P3FullElastic
    );
    if (is_level_2 && current_level >= 2) || (!is_level_2 && current_level >= 1) {
        return None;
    }
    let measured_dur = if cached_dur > 0 {
        0
    } else {
        probe_duration_fast_secs(path_obj)
    };
    let duration_opt = (measured_dur > 0).then_some(measured_dur);

    if is_level_2 {
        match run_isolated_scan_job(|| run_full_pass_on_file(path_obj, None, Some(limiter))) {
            Ok(Ok(res)) => {
                let pins = map_full_cues(res.hot_cues);
                let hot_cues_json = serde_json::to_string(&pins).ok();
                Some(ProcessedTrackOutcome {
                    update: ScannerRowUpdate {
                        track_id,
                        bpm: Some(res.bpm),
                        camelot: Some(res.camelot_key),
                        energy: None,
                        hot_cues_json,
                        waveform_peaks_blob: Some(res.waveform_peaks_blob),
                        scan_level: 2,
                        is_transition_candidate: false,
                        is_fake_320kbps: false,
                        duration_sec: duration_opt,
                    },
                    hot_cues: Some(pins),
                    corrupt_issue: None,
                })
            }
            Ok(Err(e)) => {
                warn!("Full pass failed for {path}: {e}");
                let quarantine = current_level <= 0;
                Some(ProcessedTrackOutcome {
                    update: empty_failure_update(
                        track_id,
                        duration_opt,
                        if quarantine { -1 } else { 2 },
                    ),
                    hot_cues: None,
                    corrupt_issue: quarantine.then(|| (path, format!("DECODE_ERROR: {e}"))),
                })
            }
            Err(panic_err) => {
                warn!("Full pass panicked for {path}: {panic_err}");
                Some(ProcessedTrackOutcome {
                    update: empty_failure_update(track_id, duration_opt, -1),
                    hot_cues: None,
                    corrupt_issue: Some((path, panic_err)),
                })
            }
        }
    } else {
        match run_isolated_scan_job(|| run_fast_scan_on_file(path_obj, None, 320, Some(limiter))) {
            Ok(Ok(res)) => Some(ProcessedTrackOutcome {
                update: ScannerRowUpdate {
                    track_id,
                    bpm: Some(res.bpm),
                    camelot: Some(res.camelot_key),
                    energy: None,
                    hot_cues_json: None,
                    waveform_peaks_blob: None,
                    scan_level: 1,
                    is_transition_candidate: res.is_transition_candidate,
                    is_fake_320kbps: res.is_fake_320kbps,
                    duration_sec: duration_opt,
                },
                hot_cues: None,
                corrupt_issue: None,
            }),
            Ok(Err(e)) => {
                warn!("Fast scan failed for {path}: {e}");
                let quarantine = current_level <= 0;
                Some(ProcessedTrackOutcome {
                    update: empty_failure_update(
                        track_id,
                        duration_opt,
                        if quarantine { -1 } else { 1 },
                    ),
                    hot_cues: None,
                    corrupt_issue: quarantine.then(|| (path, format!("DECODE_ERROR: {e}"))),
                })
            }
            Err(panic_err) => {
                warn!("Fast scan panicked for {path}: {panic_err}");
                Some(ProcessedTrackOutcome {
                    update: empty_failure_update(track_id, duration_opt, -1),
                    hot_cues: None,
                    corrupt_issue: Some((path, panic_err)),
                })
            }
        }
    }
}
