use crate::bpm_engine::v2_core::{
    consensus_ai::analyze_bpm_consensus_v2,
    hints::BpmHint,
    key_engine::detect_song_key,
    multiband_onset::extract_multiband_novelty,
};
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastWindowPlan {
    pub use_full_file: bool,
    pub win_a_start_sec: f64,
    pub win_a_dur_sec: f64,
    pub win_b_start_sec: f64,
    pub win_b_dur_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastScanResult {
    pub bpm: f64,
    pub camelot_key: String,
    pub is_transition_candidate: bool,
    pub is_fake_320kbps: bool,
    pub bpm_window_a: f64,
    pub bpm_window_b: f64,
}

pub fn plan_fast_windows(total_duration_sec: f64) -> FastWindowPlan {
    if total_duration_sec <= 75.0 {
        return FastWindowPlan {
            use_full_file: true,
            win_a_start_sec: 0.0,
            win_a_dur_sec: total_duration_sec.max(0.0),
            win_b_start_sec: 0.0,
            win_b_dur_sec: 0.0,
        };
    }
    let start_a = (0.35 * total_duration_sec - 12.0).max(0.0);
    let start_b = (0.65 * total_duration_sec - 8.0).max(start_a + 24.0);
    FastWindowPlan {
        use_full_file: false,
        win_a_start_sec: start_a,
        win_a_dur_sec: 24.0,
        win_b_start_sec: start_b,
        win_b_dur_sec: 16.0,
    }
}

#[inline]
pub fn is_transition_candidate(bpm_a: f64, bpm_b: f64) -> bool {
    if (bpm_a - bpm_b).abs() <= 1.5 {
        return false;
    }
    let not_double_a = (bpm_a - bpm_b * 2.0).abs() > 2.0;
    let not_double_b = (bpm_b - bpm_a * 2.0).abs() > 2.0;
    not_double_a && not_double_b
}

#[inline]
pub fn is_fake_320kbps(effective_bandwidth_hz: f32, claimed_bitrate_kbps: u32) -> bool {
    claimed_bitrate_kbps >= 320 && effective_bandwidth_hz < 16_200.0
}

pub fn analyze_fast_windows(
    samples_a: &[f32],
    samples_b: &[f32],
    sample_rate: u32,
    hint: Option<BpmHint>,
    effective_bandwidth_hz: f32,
    claimed_bitrate_kbps: u32,
) -> FastScanResult {
    let mut planner = FftPlanner::<f32>::new();
    let nov_a = extract_multiband_novelty(samples_a, sample_rate, 0.0, &mut planner);
    let res_a = analyze_bpm_consensus_v2(&nov_a, hint);

    let (bpm_b, final_bpm, key_samples) = if samples_b.is_empty() {
        (res_a.bpm, res_a.bpm, samples_a.to_vec())
    } else {
        let nov_b = extract_multiband_novelty(samples_b, sample_rate, 0.0, &mut planner);
        let res_b = analyze_bpm_consensus_v2(&nov_b, hint);
        let mut merged = Vec::with_capacity(samples_a.len() + samples_b.len());
        merged.extend_from_slice(samples_a);
        merged.extend_from_slice(samples_b);

        let final_bpm = if is_transition_candidate(res_a.bpm, res_b.bpm) {
            res_a.bpm
        } else {
            let mut sub_kick = Vec::with_capacity(nov_a.sub_kick.len() + nov_b.sub_kick.len());
            sub_kick.extend_from_slice(&nov_a.sub_kick);
            sub_kick.extend_from_slice(&nov_b.sub_kick);
            let mut low_body = Vec::with_capacity(nov_a.low_body.len() + nov_b.low_body.len());
            low_body.extend_from_slice(&nov_a.low_body);
            low_body.extend_from_slice(&nov_b.low_body);
            let mut perc_clave = Vec::with_capacity(nov_a.perc_clave.len() + nov_b.perc_clave.len());
            perc_clave.extend_from_slice(&nov_a.perc_clave);
            perc_clave.extend_from_slice(&nov_b.perc_clave);
            let mut hi_guiro = Vec::with_capacity(nov_a.hi_guiro.len() + nov_b.hi_guiro.len());
            hi_guiro.extend_from_slice(&nov_a.hi_guiro);
            hi_guiro.extend_from_slice(&nov_b.hi_guiro);
            let mut composite = Vec::with_capacity(nov_a.composite.len() + nov_b.composite.len());
            composite.extend_from_slice(&nov_a.composite);
            composite.extend_from_slice(&nov_b.composite);
            let num_frames = composite.len();
            let nov_combined = crate::bpm_engine::v2_core::multiband_onset::MultibandNovelty {
                sub_kick,
                low_body,
                perc_clave,
                hi_guiro,
                composite,
                num_frames,
                sample_rate: nov_a.sample_rate,
                hop_size: nov_a.hop_size,
                onset_fps: nov_a.onset_fps,
                trim_offset_sec: nov_a.trim_offset_sec,
                band_ratios: nov_a.band_ratios,
            };
            analyze_bpm_consensus_v2(&nov_combined, hint).bpm
        };
        (res_b.bpm, final_bpm, merged)
    };

    let key = detect_song_key(&key_samples, sample_rate, &mut planner);
    FastScanResult {
        bpm: final_bpm,
        camelot_key: key.camelot,
        is_transition_candidate: !samples_b.is_empty()
            && is_transition_candidate(res_a.bpm, bpm_b),
        is_fake_320kbps: is_fake_320kbps(effective_bandwidth_hz, claimed_bitrate_kbps),
        bpm_window_a: res_a.bpm,
        bpm_window_b: bpm_b,
    }
}

pub fn estimate_effective_bandwidth_hz(
    samples: &[f32],
    sample_rate: u32,
    planner: &mut FftPlanner<f32>,
) -> f32 {
    if samples.len() < 2048 || sample_rate == 0 {
        return (sample_rate as f32) * 0.5;
    }
    let n = 2048_usize;
    let fft = planner.plan_fft_forward(n);
    let mut mag_acc = vec![0.0_f64; n / 2];
    let mut buf = vec![rustfft::num_complex::Complex::new(0.0_f32, 0.0); n];
    let mut windows = 0_usize;
    for chunk in samples.chunks_exact(n).take(32) {
        for (i, &s) in chunk.iter().enumerate() {
            let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * (i as f32) / (n as f32)).cos();
            buf[i] = rustfft::num_complex::Complex::new(s * w, 0.0);
        }
        fft.process(&mut buf);
        for (k, c) in buf.iter().take(n / 2).enumerate() {
            mag_acc[k] += (c.re * c.re + c.im * c.im) as f64;
        }
        windows += 1;
    }
    if windows == 0 {
        return (sample_rate as f32) * 0.5;
    }
    let peak = mag_acc.iter().cloned().fold(0.0_f64, f64::max).max(1e-12);
    let threshold = peak * 1e-5; // -50 dB
    let mut max_bin = 0_usize;
    for (k, &m) in mag_acc.iter().enumerate() {
        if m >= threshold {
            max_bin = k;
        }
    }
    (max_bin as f32) * (sample_rate as f32) / (n as f32)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_fast_scan_on_file(
    path: &std::path::Path,
    hint: Option<crate::bpm_engine::v2_core::hints::BpmHint>,
    claimed_bitrate_kbps: u32,
    limiter: Option<&crate::bpm_engine::v2_core::audio_ingest::ProcessLimiter>,
) -> Result<FastScanResult, crate::bpm_engine::v2_core::types::EngineError> {
    use crate::bpm_engine::v2_core::audio_ingest::load_audio_mono_native_sr;
    use crate::bpm_engine::v2_core::ffmpeg_hash::run_ffmpeg_f32le_pipe_bounded;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let use_ffmpeg = matches!(ext.as_str(), "webm" | "mkv" | "weba" | "flv");

    let (samples_a, samples_b, sample_rate) = if !use_ffmpeg {
        if let Ok((a, b, sr)) = try_symphonia_fast_windows(path) {
            (a, b, sr)
        } else {
            let dec = load_audio_mono_native_sr(path, limiter)?;
            let sr = dec.sample_rate.max(1);
            let plan = plan_fast_windows(dec.duration_sec);
            if plan.use_full_file {
                (dec.samples, Vec::new(), sr)
            } else {
                let a0 = ((plan.win_a_start_sec * sr as f64) as usize).min(dec.samples.len());
                let a1 = (a0 + (plan.win_a_dur_sec * sr as f64) as usize).min(dec.samples.len());
                let b0 = ((plan.win_b_start_sec * sr as f64) as usize).min(dec.samples.len());
                let b1 = (b0 + (plan.win_b_dur_sec * sr as f64) as usize).min(dec.samples.len());
                (dec.samples[a0..a1].to_vec(), dec.samples[b0..b1].to_vec(), sr)
            }
        }
    } else {
        let _permit = limiter.map(|l| l.acquire());
        let a = run_ffmpeg_f32le_pipe_bounded(path, Some((58.0, 24.0)), 15)?;
        let b = run_ffmpeg_f32le_pipe_bounded(path, Some((122.0, 16.0)), 15).unwrap_or_default();
        (a, b, 44_100)
    };

    let mut planner = FftPlanner::<f32>::new();
    let bw = estimate_effective_bandwidth_hz(&samples_a, sample_rate, &mut planner);
    Ok(analyze_fast_windows(&samples_a, &samples_b, sample_rate, hint, bw, claimed_bitrate_kbps))
}

#[cfg(not(target_arch = "wasm32"))]
fn try_symphonia_fast_windows(path: &std::path::Path) -> Result<(Vec<f32>, Vec<f32>, u32), ()> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use symphonia::core::units::Time;

    let file = std::fs::File::open(path).map_err(|_| ())?;
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| ())?;
    let track = probed.format.tracks().iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL && t.codec_params.sample_rate.is_some())
        .or_else(|| probed.format.default_track())
        .ok_or(())?;
    let track_id = track.id;
    let mut sr = track.codec_params.sample_rate.unwrap_or(44_100);
    let total_dur = track.codec_params.n_frames
        .map(|n| n as f64 / sr.max(1) as f64)
        .unwrap_or(0.0);
    if total_dur <= 75.0 {
        return Err(());
    }
    let plan = plan_fast_windows(total_dur);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| ())?;

    let mut read_window = |start_sec: f64, dur_sec: f64| -> Vec<f32> {
        let _ = probed.format.seek(
            SeekMode::Coarse,
            SeekTo::Time { time: Time::from(start_sec), track_id: Some(track_id) },
        );
        decoder.reset();
        let max_samples = (dur_sec * sr as f64) as usize;
        let mut out = Vec::with_capacity(max_samples);
        while out.len() < max_samples {
            let Ok(pkt) = probed.format.next_packet() else { break };
            if pkt.track_id() != track_id { continue; }
            if let Ok(abuf) = decoder.decode(&pkt) {
                let spec = *abuf.spec();
                if spec.rate > 0 { sr = spec.rate; }
                let ch = spec.channels.count().max(1);
                let mut sb = SampleBuffer::<f32>::new(abuf.capacity() as u64, spec);
                sb.copy_interleaved_ref(abuf);
                for frame in sb.samples().chunks_exact(ch) {
                    let sum: f32 = frame.iter().sum();
                    out.push(sum / ch as f32);
                    if out.len() >= max_samples { break; }
                }
            }
        }
        out
    };

    let a = read_window(plan.win_a_start_sec, plan.win_a_dur_sec);
    let b = read_window(plan.win_b_start_sec, plan.win_b_dur_sec);
    if a.is_empty() { return Err(()); }
    Ok((a, b, sr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_window_plan_and_forensic_flags() {
        let short = plan_fast_windows(60.0);
        assert!(short.use_full_file);

        let long = plan_fast_windows(200.0);
        assert!(!long.use_full_file);
        assert!((long.win_a_start_sec - 58.0).abs() < 1e-6);
        assert!((long.win_a_dur_sec - 24.0).abs() < 1e-6);
        assert!((long.win_b_start_sec - 122.0).abs() < 1e-6);
        assert!((long.win_b_dur_sec - 16.0).abs() < 1e-6);

        assert!(is_transition_candidate(100.0, 126.0));
        assert!(!is_transition_candidate(126.0, 126.5));
        assert!(!is_transition_candidate(85.0, 170.0));

        assert!(is_fake_320kbps(15_800.0, 320));
        assert!(!is_fake_320kbps(19_500.0, 320));
    }
}
