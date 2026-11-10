use std::sync::Arc;

use super::{
    CommandQueue, DeckId, DualDeckEngine, HotCueSampleMarker, PreallocatedDeckBuffer,
    TransportCommand, HOT_CUE_LABELS, MAX_SEEK_FRAME,
};

fn encode_stereo_pcm16_wav(left: &[f32], right: &[f32], sample_rate: u32) -> Vec<u8> {
    let n = left.len().min(right.len());
    let num_channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * u32::from(num_channels) * (u32::from(bits_per_sample) / 8);
    let block_align = num_channels * (bits_per_sample / 8);
    let data_size = (n as u32) * u32::from(block_align);
    let riff_size = 36 + data_size;

    let mut buf = Vec::with_capacity((44 + data_size) as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVEfmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&num_channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&bits_per_sample.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());

    for i in 0..n {
        let l_i16 = (left[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        let r_i16 = (right[i].clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&l_i16.to_le_bytes());
        buf.extend_from_slice(&r_i16.to_le_bytes());
    }
    buf
}

#[test]
fn test_lock_free_command_queue_pack_unpack_and_ring_wraparound() {
    let queue = CommandQueue::new();
    assert_eq!(queue.try_pop(), None);

    // 1. Verify exact roundtrip fidelity across all 6 variants + 54-bit SeekToSample clamp.
    let roundtrip_cases = [
        TransportCommand::Play(DeckId::DeckA),
        TransportCommand::Pause(DeckId::DeckB),
        TransportCommand::SeekToSample(DeckId::DeckA, 4_233_600),
        TransportCommand::JumpToHotCue(DeckId::DeckB, 3),
        TransportCommand::SetTempoRatio(DeckId::DeckA, 1.125),
        TransportCommand::SetGain(DeckId::DeckB, 0.85),
    ];
    for cmd in roundtrip_cases {
        assert!(queue.try_push(cmd));
    }
    for expected in roundtrip_cases {
        assert_eq!(queue.try_pop(), Some(expected));
    }
    assert_eq!(queue.try_pop(), None);

    // Clamp u64::MAX frame seek to 54-bit MAX_SEEK_FRAME.
    assert!(queue.try_push(TransportCommand::SeekToSample(DeckId::DeckB, u64::MAX)));
    assert_eq!(
        queue.try_pop(),
        Some(TransportCommand::SeekToSample(
            DeckId::DeckB,
            MAX_SEEK_FRAME
        ))
    );

    // 2. Fill all 64 slots and verify the 65th push returns false without overwriting.
    for i in 0..64u64 {
        assert!(queue.try_push(TransportCommand::SeekToSample(DeckId::DeckA, i * 100)));
    }
    assert!(!queue.try_push(TransportCommand::Play(DeckId::DeckB)));

    for i in 0..64u64 {
        assert_eq!(
            queue.try_pop(),
            Some(TransportCommand::SeekToSample(DeckId::DeckA, i * 100))
        );
    }
    assert_eq!(queue.try_pop(), None);

    // 3. Push/pop 180 commands in staggered batches across multiple 64-slot ring wraparounds.
    let mut next_push = 0u64;
    let mut next_pop = 0u64;
    for _batch in 0..9 {
        for _ in 0..20 {
            let deck = if (next_push & 1) == 0 {
                DeckId::DeckA
            } else {
                DeckId::DeckB
            };
            assert!(queue.try_push(TransportCommand::SeekToSample(deck, next_push * 17)));
            next_push += 1;
        }
        for _ in 0..20 {
            let expected_deck = if (next_pop & 1) == 0 {
                DeckId::DeckA
            } else {
                DeckId::DeckB
            };
            assert_eq!(
                queue.try_pop(),
                Some(TransportCommand::SeekToSample(expected_deck, next_pop * 17))
            );
            next_pop += 1;
        }
    }
    assert_eq!(next_push, 180);
    assert_eq!(next_pop, 180);
    assert_eq!(queue.try_pop(), None);
}

#[test]
fn test_sample_accurate_jump_to_four_hot_cues() {
    let total_frames = 12_000usize;
    let mut stereo = vec![0.01_f32; total_frames * 2];
    let cue_frames: [u64; 4] = [100, 1_200, 4_800, 9_600];
    let sentinels: [(f32, f32); 4] = [
        (0.111, -0.111),
        (0.222, -0.222),
        (0.333, -0.333),
        (0.444, -0.444),
    ];

    for (idx, &frame) in cue_frames.iter().enumerate() {
        let base = (frame as usize) * 2;
        stereo[base] = sentinels[idx].0;
        stereo[base + 1] = sentinels[idx].1;
    }

    let buffer = Arc::new(
        PreallocatedDeckBuffer::from_stereo_frames(stereo, 48_000, cue_frames)
            .expect("valid stereo buffer"),
    );
    assert_eq!(
        buffer.hot_cues,
        [
            HotCueSampleMarker {
                num: 1,
                label: "1:Intro",
                frame_index: 100
            },
            HotCueSampleMarker {
                num: 2,
                label: "2:Breakdown",
                frame_index: 1_200
            },
            HotCueSampleMarker {
                num: 3,
                label: "3:Drop",
                frame_index: 4_800
            },
            HotCueSampleMarker {
                num: 4,
                label: "4:Outro",
                frame_index: 9_600
            },
        ]
    );

    let queue = CommandQueue::new();
    let mut engine = DualDeckEngine::new();
    engine.load_deck(DeckId::DeckA, buffer);

    let mut out_stereo = [0.0_f32; 8]; // 4 stereo frames per render pass
    for (i, &(expected_l, expected_r)) in sentinels.iter().enumerate() {
        let cue_num = (i as u8) + 1;
        assert!(queue.try_push(TransportCommand::JumpToHotCue(DeckId::DeckA, cue_num)));
        assert!(queue.try_push(TransportCommand::Play(DeckId::DeckA)));

        let rendered_frames = engine.render_interleaved_stereo(&queue, &mut out_stereo);
        assert_eq!(rendered_frames, 4);
        assert!((out_stereo[0] - expected_l).abs() < 1e-6);
        assert!((out_stereo[1] - expected_r).abs() < 1e-6);
        assert_eq!(engine.deck_playhead_frame(DeckId::DeckA), cue_frames[i] + 4);
        assert!(engine.is_deck_playing(DeckId::DeckA));
    }
}

#[test]
fn test_dual_deck_mix_tempo_ratio_and_eof_clamp() {
    let queue = CommandQueue::new();
    let mut engine = DualDeckEngine::new();

    // DeckA: 16 frames of linear ramp (frame i => L = i * 0.04, R = -i * 0.02)
    let mut stereo_a = Vec::with_capacity(32);
    for i in 0..16 {
        stereo_a.push((i as f32) * 0.04);
        stereo_a.push(-(i as f32) * 0.02);
    }
    // DeckB: 16 frames of constant pad (L = 0.25, R = 0.30)
    let mut stereo_b = Vec::with_capacity(32);
    for _ in 0..16 {
        stereo_b.push(0.25);
        stereo_b.push(0.30);
    }

    let buf_a = Arc::new(
        PreallocatedDeckBuffer::from_stereo_frames(stereo_a, 44_100, [0, 4, 8, 12]).unwrap(),
    );
    let buf_b = Arc::new(
        PreallocatedDeckBuffer::from_stereo_frames(stereo_b, 44_100, [0, 4, 8, 12]).unwrap(),
    );
    engine.load_deck(DeckId::DeckA, buf_a);
    engine.load_deck(DeckId::DeckB, buf_b);

    // 1. Test fractional linear interpolation at 1.25x on DeckA + simultaneous summing with DeckB.
    assert!(queue.try_push(TransportCommand::SetTempoRatio(DeckId::DeckA, 1.25)));
    assert!(queue.try_push(TransportCommand::SetTempoRatio(DeckId::DeckA, f32::NAN))); // Must ignore NaN!
    assert!(queue.try_push(TransportCommand::SetGain(DeckId::DeckB, f32::INFINITY))); // Must ignore Inf!
    assert!(queue.try_push(TransportCommand::JumpToHotCue(DeckId::DeckA, 0))); // Out-of-range cue 0 ignored
    assert!(queue.try_push(TransportCommand::JumpToHotCue(DeckId::DeckA, 9))); // Out-of-range cue 9 ignored
    assert!(queue.try_push(TransportCommand::Play(DeckId::DeckA)));
    assert!(queue.try_push(TransportCommand::Play(DeckId::DeckB)));

    let mut out = [0.0_f32; 8]; // 4 stereo frames: playhead_a = 0.0, 1.25, 2.50, 3.75
    let frames = engine.render_interleaved_stereo(&queue, &mut out);
    assert_eq!(frames, 4);
    assert!((engine.deck_tempo_ratio(DeckId::DeckA) - 1.25).abs() < 1e-6);

    for f in 0..4 {
        let pos_a = (f as f32) * 1.25;
        let expected_l = pos_a * 0.04 + 0.25;
        let expected_r = -pos_a * 0.02 + 0.30;
        assert!((out[f * 2] - expected_l).abs() < 1e-5);
        assert!((out[f * 2 + 1] - expected_r).abs() < 1e-5);
    }

    // 2. Test hot-sum soft-clamping to [-1.0, 1.0] and range clamping of gain/tempo.
    assert!(queue.try_push(TransportCommand::SetTempoRatio(DeckId::DeckA, 9.0)));
    assert!(queue.try_push(TransportCommand::SetGain(DeckId::DeckB, 5.0))); // Clamped to 2.0 -> 0.30 * 2 = 0.60
    assert!(queue.try_push(TransportCommand::SetGain(DeckId::DeckA, 2.0)));
    engine.render_interleaved_stereo(&queue, &mut out);
    assert!((engine.deck_tempo_ratio(DeckId::DeckA) - 2.0).abs() < 1e-6);
    assert!(out
        .iter()
        .all(|s| s.is_finite() && (-1.0..=1.0).contains(s)));

    // 3. Seek past EOF clamps cleanly to total_frames (16) and pauses DeckA without panic or NaN.
    assert!(queue.try_push(TransportCommand::Pause(DeckId::DeckB)));
    assert!(queue.try_push(TransportCommand::SeekToSample(DeckId::DeckA, 999_999)));
    engine.render_interleaved_stereo(&queue, &mut out);
    assert_eq!(engine.deck_playhead_frame(DeckId::DeckA), 16);
    assert!(!engine.is_deck_playing(DeckId::DeckA));
    assert!(out.iter().all(|&s| s == 0.0));
}

#[test]
fn test_decode_wav_bytes_into_preallocated_deck_buffer() {
    let sr = 44_100u32;
    let dur_sec = 4.0_f32;
    let n = (sr as f32 * dur_sec) as usize;
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);

    for i in 0..n {
        let t = (i as f32) / (sr as f32);
        let beat_env = (1.0 - (t % 0.5) * 2.0).max(0.0);
        let kick = (2.0 * std::f32::consts::PI * 60.0 * t).sin() * beat_env * 0.55;
        let synth = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.20;
        left.push(kick + synth);
        right.push(kick - synth);
    }

    let wav_bytes = encode_stereo_pcm16_wav(&left, &right, sr);
    let decoded =
        PreallocatedDeckBuffer::decode_from_bytes(&wav_bytes, Some("wav")).expect("decode WAV");
    assert_eq!(decoded.sample_rate, sr);
    assert_eq!(decoded.total_frames(), n as u64);
    assert_eq!(decoded.interleaved_stereo().len(), n * 2);

    for (idx, cue) in decoded.hot_cues.iter().enumerate() {
        assert_eq!(cue.num, (idx as u8) + 1);
        assert_eq!(cue.label, HOT_CUE_LABELS[idx]);
        assert!(cue.frame_index < decoded.total_frames());
    }
    assert!(decoded.hot_cues[0].frame_index < decoded.hot_cues[1].frame_index);
    assert!(decoded.hot_cues[1].frame_index < decoded.hot_cues[2].frame_index);
    assert!(decoded.hot_cues[2].frame_index < decoded.hot_cues[3].frame_index);

    // Also verify streaming decode_from_file via temp WAV on disk.
    let tmp_wav = std::env::temp_dir().join(format!("hera_deck_test_{}.wav", std::process::id()));
    std::fs::write(&tmp_wav, &wav_bytes).expect("write temp wav");
    let from_file = PreallocatedDeckBuffer::decode_from_file(&tmp_wav).expect("decode file WAV");
    let _ = std::fs::remove_file(&tmp_wav);
    assert_eq!(from_file.total_frames(), decoded.total_frames());
    assert_eq!(from_file.hot_cues, decoded.hot_cues);

    // Render audio block through DualDeckEngine from Cue 3 (Drop).
    let queue = CommandQueue::new();
    let mut engine = DualDeckEngine::new();
    engine.load_deck(DeckId::DeckA, Arc::new(decoded));
    assert!(queue.try_push(TransportCommand::JumpToHotCue(DeckId::DeckA, 3)));
    assert!(queue.try_push(TransportCommand::Play(DeckId::DeckA)));

    let mut out = [0.0_f32; 256];
    let rendered = engine.render_interleaved_stereo(&queue, &mut out);
    assert_eq!(rendered, 128);
    assert!(out
        .iter()
        .all(|s| s.is_finite() && (-1.0..=1.0).contains(s)));
    assert!(out.iter().any(|s| s.abs() > 0.01));
}
