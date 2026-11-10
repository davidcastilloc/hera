//! Shadow & DSP unit tests for `rubato` resampling, `fundsp` master bus safety, `realfft` 32-bin
//! spectrum detection, `rtrb` wait-free telemetry, and `HeadlessClockAudioBackend` 4-Hot-Cue parity.

use super::backend::{AudioBackend, AudioBackendKind, HeadlessClockAudioBackend};
use super::dsp_chain::{
    create_telemetry_ring, resample_deck_buffer_to_rate, MasterDspBus, RealTimeSpectrumAnalyzer,
    SPECTRUM_BIN_COUNT,
};
use super::{
    create_default_audio_backend, DeckId, PreallocatedDeckBuffer, TransportCommand, HOT_CUE_LABELS,
};

fn build_pcm16_wav(left: &[f32], right: &[f32], sample_rate: u32) -> Vec<u8> {
    let n = left.len().min(right.len());
    let data_size = (n as u32) * 4;
    let mut wav = Vec::with_capacity(44 + data_size as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 4).to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for i in 0..n {
        let l = (left[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        let r = (right[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        wav.extend_from_slice(&l.to_le_bytes());
        wav.extend_from_slice(&r.to_le_bytes());
    }
    wav
}

#[test]
fn test_rubato_resample_44100_to_48000_preserves_duration_and_scales_hot_cues() {
    let in_sr = 44_100u32;
    let out_sr = 48_000u32;
    let frames_in = 44_100usize; // 1.0 second
    let mut stereo = Vec::with_capacity(frames_in * 2);
    for i in 0..frames_in {
        let t = (i as f32) / (in_sr as f32);
        let l = (std::f32::consts::TAU * 440.0 * t).sin() * 0.6;
        let r = (std::f32::consts::TAU * 880.0 * t).sin() * 0.5;
        stereo.push(l);
        stereo.push(r);
    }

    let cues_44k = [4_410, 11_025, 22_050, 33_075];
    let raw = PreallocatedDeckBuffer::from_stereo_frames(stereo, in_sr, cues_44k)
        .expect("valid 44.1k buffer");
    let resampled = resample_deck_buffer_to_rate(&raw, out_sr).expect("rubato 44.1k -> 48k");

    assert_eq!(resampled.sample_rate, out_sr);
    let diff_frames = (resampled.total_frames() as i64 - i64::from(out_sr)).abs();
    assert!(
        diff_frames <= 64,
        "Expected ~48000 frames after 1.0s resample, got {}",
        resampled.total_frames()
    );
    assert_eq!(resampled.hot_cues[0].frame_index, 4_800);
    assert_eq!(resampled.hot_cues[1].frame_index, 12_000);
    assert_eq!(resampled.hot_cues[2].frame_index, 24_000);
    assert_eq!(resampled.hot_cues[3].frame_index, 36_000);
    assert!(resampled
        .interleaved_stereo()
        .iter()
        .all(|s| s.is_finite() && (-1.0..=1.0).contains(s)));
}

#[test]
fn test_fundsp_master_bus_soft_clips_hot_signals_and_sanitizes_nans() {
    let mut bus = MasterDspBus::new(48_000);
    let mut block = Vec::with_capacity(512);
    for i in 0..256 {
        if i == 17 {
            block.push(f32::NAN);
            block.push(f32::INFINITY);
        } else {
            let phase = (i as f32) * 0.15;
            block.push(phase.sin() * 3.5 + 0.25); // Hot signal + DC bias
            block.push(phase.cos() * -3.5 - 0.25);
        }
    }

    bus.process_interleaved_in_place(&mut block, 1.25);
    assert!(block
        .iter()
        .all(|s| s.is_finite() && (-1.0..=1.0).contains(s)));
    assert!(block.iter().any(|s| s.abs() > 0.2));
}

#[test]
fn test_realfft_32_bin_spectrum_and_rtrb_telemetry_on_124_bpm_kick_plus_sheen() {
    let sr = 48_000u32;
    let mut analyzer = RealTimeSpectrumAnalyzer::new(sr);
    let (mut tx, mut rx) = create_telemetry_ring(16);

    let frames = 2048usize;
    let beat_period = 60.0_f32 / 124.0_f32;
    let mut stereo = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = (i as f32) / (sr as f32);
        let beat_env = (1.0 - ((t % beat_period) / beat_period) * 1.5).max(0.25);
        let sub_kick = (std::f32::consts::TAU * 62.0 * t).sin() * 0.65 * beat_env;
        let air_sheen = (std::f32::consts::TAU * 9_000.0 * t).sin() * 0.35;
        stereo.push(sub_kick + air_sheen);
        stereo.push(sub_kick - air_sheen);
    }

    let telemetry = analyzer.analyze_block(&stereo, 1.5, 36.0, true);
    assert_eq!(telemetry.spectrum_bins.len(), SPECTRUM_BIN_COUNT);
    assert!(telemetry.sub_bass > 0.08, "sub_bass={}", telemetry.sub_bass);
    assert!(
        telemetry.air_presence > 0.04,
        "air_presence={}",
        telemetry.air_presence
    );
    assert!(
        telemetry.sub_bass > telemetry.low_mid,
        "Expected 62Hz kick sub_bass ({}) > empty low_mid ({})",
        telemetry.sub_bass,
        telemetry.low_mid
    );
    assert!(telemetry.rms_left > 0.1 && telemetry.rms_right > 0.1);
    assert!(telemetry.peak_db > -12.0 && telemetry.peak_db <= 0.0);

    assert!(tx.push(telemetry).is_ok());
    let popped = rx.pop().expect("wait-free pop from rtrb");
    assert_eq!(popped.sequence, 1);
    assert_eq!(popped.spectrum_bins, telemetry.spectrum_bins);
}

#[test]
fn test_headless_backend_4_hot_cue_seek_shadow_parity_and_eof_completion() {
    let in_sr = 44_100u32;
    let out_sr = 48_000u32;
    let dur_sec = 2.0_f32;
    let frames_44k = (in_sr as f32 * dur_sec) as usize;
    let mut left = Vec::with_capacity(frames_44k);
    let mut right = Vec::with_capacity(frames_44k);
    for i in 0..frames_44k {
        let t = (i as f32) / (in_sr as f32);
        let kick = (std::f32::consts::TAU * 60.0 * t).sin() * 0.5;
        let lead = (std::f32::consts::TAU * 440.0 * t).sin() * 0.25;
        left.push(kick + lead);
        right.push(kick - lead);
    }
    let wav = build_pcm16_wav(&left, &right, in_sr);

    // Shadow reference: raw 44.1k decode + rubato 48k cue positions.
    let raw_44k = PreallocatedDeckBuffer::decode_from_bytes(&wav, Some("wav")).unwrap();
    let expected_ratio = f64::from(out_sr) / f64::from(in_sr);

    let backend = HeadlessClockAudioBackend::new_deterministic(out_sr);
    assert_eq!(backend.backend_kind(), AudioBackendKind::HeadlessClockFallback);
    let snap = backend
        .load_bytes(DeckId::DeckA, &wav, Some("wav"))
        .expect("load & resample WAV into HeadlessClockAudioBackend");
    assert!(snap.track_loaded);
    assert_eq!(snap.sample_rate, out_sr);

    for (idx, label) in HOT_CUE_LABELS.iter().enumerate() {
        assert_eq!(raw_44k.hot_cues[idx].label, *label);
        let expected_48k = ((raw_44k.hot_cues[idx].frame_index as f64) * expected_ratio)
            .round() as u64;
        assert_eq!(snap.hot_cue_frames[idx], expected_48k);

        let cue_num = (idx as u8) + 1;
        let after_jump = backend.send_command(TransportCommand::JumpToHotCue(DeckId::DeckA, cue_num));
        assert_eq!(after_jump.playhead_frame, expected_48k);
    }

    // Play from Cue 4 (Outro) through to EOF and verify telemetry + `ended` flag.
    backend.send_command(TransportCommand::JumpToHotCue(DeckId::DeckA, 4));
    let playing_snap = backend.send_command(TransportCommand::Play(DeckId::DeckA));
    assert!(playing_snap.is_playing);
    assert!(!playing_snap.ended);

    let remaining = (snap.total_frames - snap.hot_cue_frames[3] + 256) as usize;
    let rendered = backend.render_deterministic_frames(remaining);
    assert!(rendered.iter().any(|s| s.abs() > 0.01));

    let telemetry = backend
        .poll_latest_telemetry()
        .expect("telemetry frame emitted via rtrb");
    assert_eq!(telemetry.sample_rate, out_sr);

    let eof_snap = backend.snapshot();
    assert!(eof_snap.ended);
    assert!(!eof_snap.is_playing);
    assert_eq!(eof_snap.playhead_frame, eof_snap.total_frames);

    // Also verify default factory never panics even on headless systems without ALSA hardware.
    let default_backend = create_default_audio_backend();
    assert!(default_backend.output_sample_rate() >= 8_000);
}
