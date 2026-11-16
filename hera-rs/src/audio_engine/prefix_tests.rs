//! Integration tests for prefix-first (`< 50 ms`) loading and seamless full-track upgrade.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use super::backend::{AudioBackend, HeadlessClockAudioBackend};
use super::{DeckId, TransportCommand};

fn write_test_pcm16_wav(path: &Path, sample_rate: u32, dur_sec: f32, freq_hz: f32) {
    let total_frames = ((sample_rate as f32) * dur_sec) as usize;
    let data_size = (total_frames as u32) * 4;
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

    let period = (sample_rate as f32) / freq_hz;
    for i in 0..total_frames {
        let phase = ((i as f32) % period) / period;
        let sample = ((std::f32::consts::TAU * phase).sin() * 0.4 * 32767.0) as i16;
        let bytes = sample.to_le_bytes();
        wav.extend_from_slice(&bytes);
        wav.extend_from_slice(&bytes);
    }
    fs::write(path, wav).expect("write test WAV");
}

#[test]
fn test_prefix_loader_starts_fast_and_seamlessly_upgrades_past_prefix_to_eof() {
    let wav_path = std::env::temp_dir().join(format!(
        "hera_prefix_stream_{}_{:?}.wav",
        std::process::id(),
        std::thread::current().id()
    ));
    write_test_pcm16_wav(&wav_path, 44_100, 30.0, 220.0);

    let backend = HeadlessClockAudioBackend::new_deterministic(48_000);
    let known_cues = [1.0, 8.0, 16.0, 24.0];
    let snap = backend
        .load_file_fast(DeckId::DeckA, &wav_path, Some(30.0), Some(known_cues))
        .expect("load_file_fast 30s WAV");

    assert!(snap.track_loaded);
    assert!((snap.duration_sec - 30.0).abs() < 0.05);
    for (idx, expected_sec) in known_cues.iter().enumerate() {
        assert!(
            (snap.hot_cue_seconds[idx] - expected_sec).abs() < 0.01,
            "cue {} expected {expected_sec}s, got {}",
            idx + 1,
            snap.hot_cue_seconds[idx]
        );
    }

    assert!(backend.send_command(TransportCommand::Play(DeckId::DeckA)).is_playing);

    // Render 11.5s inside the prefix window.
    let mut rendered_prefix = Vec::new();
    for _ in 0..539 {
        rendered_prefix.extend(backend.render_deterministic_frames(1024));
    }
    assert!(rendered_prefix.iter().any(|s| s.abs() > 0.01));
    assert!(backend.snapshot().is_playing);

    // Wait for background full-track decode + 48 kHz resample upgrade.
    let deadline = Instant::now() + Duration::from_secs(15);
    while !backend.is_stream_complete() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(backend.is_stream_complete(), "background full upgrade did not finish");
    assert!(backend.deck_buffer_frames(DeckId::DeckA).unwrap() >= 1_439_000);

    // Cross the 12.0s prefix boundary smoothly and confirm prefix Arc is retired off-thread.
    let mut across_boundary = Vec::new();
    for _ in 0..96 {
        across_boundary.extend(backend.render_deterministic_frames(1024));
    }
    assert!(across_boundary.iter().any(|s| s.abs() > 0.01));
    assert!(backend.drain_retired_buffers() >= 1);
    let mid_snap = backend.snapshot();
    assert!(mid_snap.is_playing);
    assert!(!mid_snap.ended);
    assert!(mid_snap.current_time_sec > 13.0);

    // Render to 30.0s EOF in blocks.
    let rem_frames = (mid_snap.total_frames.saturating_sub(mid_snap.playhead_frame) + 2048) as usize;
    let tail = backend.render_deterministic_frames(rem_frames);
    assert!(tail.iter().any(|s| s.abs() > 0.01));
    let eof_snap = backend.snapshot();
    assert!(eof_snap.ended);
    assert!(!eof_snap.is_playing);
    assert_eq!(eof_snap.playhead_frame, eof_snap.total_frames);

    let _ = fs::remove_file(&wav_path);
}

#[test]
fn test_prefix_loader_cancels_stale_generation_on_rapid_track_switch() {
    let base = std::env::temp_dir();
    let id = format!("{}_{:?}", std::process::id(), std::thread::current().id());
    let track_a = base.join(format!("hera_switch_a_{id}.wav"));
    let track_b = base.join(format!("hera_switch_b_{id}.wav"));
    write_test_pcm16_wav(&track_a, 44_100, 30.0, 110.0);
    write_test_pcm16_wav(&track_b, 44_100, 30.0, 440.0);

    let backend = HeadlessClockAudioBackend::new_deterministic(48_000);
    let _ = backend
        .load_file_fast(DeckId::DeckA, &track_a, Some(30.0), Some([1.0, 8.0, 16.0, 24.0]))
        .expect("load track A");
    let snap_b = backend
        .load_file_fast(DeckId::DeckA, &track_b, Some(30.0), Some([2.0, 9.0, 17.0, 25.0]))
        .expect("switch rapidly to track B");
    assert!(snap_b.loaded_path.as_deref().is_some_and(|p| p.contains("hera_switch_b_")));

    let deadline = Instant::now() + Duration::from_secs(15);
    while !backend.is_stream_complete() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    std::thread::sleep(Duration::from_millis(25));

    let final_snap = backend.snapshot();
    assert!(final_snap.loaded_path.as_deref().is_some_and(|p| p.contains("hera_switch_b_")));
    assert!((final_snap.hot_cue_seconds[0] - 2.0).abs() < 0.01);
    assert!((final_snap.hot_cue_seconds[3] - 25.0).abs() < 0.01);

    let _ = fs::remove_file(&track_a);
    let _ = fs::remove_file(&track_b);
}
