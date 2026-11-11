use hera_node_rs::bpm_engine::{analyze_bpm_consensus, ExternalBpmHint};
use std::f32::consts::TAU;

/// Phase-wrapped sine helper to guarantee IEEE-754 precision across platforms.
#[inline]
fn phase_sin(freq_hz: f32, time_sec: f32) -> f32 {
    (TAU * (freq_hz * time_sec).fract()).sin()
}

/// Deterministic 64-bit LCG pseudo-random generator for synthetic acoustic noise.
struct DeterministicLcg {
    state: u64,
}

impl DeterministicLcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f32(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (((self.state >> 32) as u32 as f32) / (u32::MAX as f32)) * 2.0 - 1.0
    }
}

/// Generates a realistic synthetic Salsa track mono PCM:
/// - 4/4 meter at `bpm`
/// - 15.0s rubato piano intro (chords with no steady percussion)
/// - Percussive body with NO kick on beat 1
/// - Baby-bass syncopated tumbao on beats 2.5 and 4 (220 Hz low body)
/// - Campana / bongo martillo pattern in 800-3500 Hz band
/// - 16th-note maraca / guiro background sizzle
fn generate_synthetic_salsa_pcm(bpm: f32, duration_sec: f32, sample_rate: u32) -> Vec<f32> {
    let total_frames = (duration_sec * sample_rate as f32) as usize;
    let mut mono = Vec::with_capacity(total_frames);
    let mut lcg = DeterministicLcg::new(0x5A15_A001_CAFE_BABE);
    let beat_sec = 60.0 / bpm;
    let bar_sec = beat_sec * 4.0;

    for i in 0..total_frames {
        let t = (i as f32) / (sample_rate as f32);

        // 0.0s..15.0s: Rubato piano intro without percussion
        if t < 15.0 {
            let piano = 0.35 * phase_sin(261.63, t) // C4
                + 0.25 * phase_sin(329.63, t) // E4
                + 0.20 * phase_sin(392.00, t) // G4
                + 0.15 * phase_sin(523.25, t); // C5
            let tremolo = 0.8 + 0.2 * phase_sin(4.0, t);
            mono.push(piano * tremolo * 0.4);
            continue;
        }

        let body_t = t - 15.0;
        let bar_pos = (body_t / bar_sec).fract();
        let beat_num = bar_pos * 4.0; // 0.0 .. 4.0

        // Baby-bass tumbao: strong hits on beat 2.5 and beat 4.0
        let dist_to_tumbao_2_5 = (beat_num - 2.5).abs().min((beat_num - 6.5).abs());
        let dist_to_tumbao_4 = (beat_num - 3.95).abs().min((beat_num + 0.05).abs());

        let bass_tumbao_env = if dist_to_tumbao_2_5 < 0.20 {
            let x = 1.0 - (dist_to_tumbao_2_5 / 0.20);
            x * x
        } else if dist_to_tumbao_4 < 0.20 {
            let x = 1.0 - (dist_to_tumbao_4 / 0.20);
            x * x
        } else {
            0.0
        };
        let bass_wave = 0.65 * phase_sin(110.0, t) + 0.35 * phase_sin(220.0, t);
        let bass_signal = bass_wave * bass_tumbao_env * 0.60;

        // Campana (cowbell) on beats 1, 2, 3, 4 with syncopated accents (1.5, 2.5, 3.5)
        let beat_subpos = (beat_num * 2.0).fract();
        let is_campana_hit = beat_subpos < 0.18;
        let campana_accent = if (beat_num - 2.0).abs() < 0.2 || (beat_num - 4.0).abs() < 0.2 {
            1.0
        } else {
            0.6
        };
        let campana_env = if is_campana_hit {
            (1.0 - (beat_subpos / 0.18)).powi(3) * campana_accent
        } else {
            0.0
        };
        let campana_signal = (0.5 * phase_sin(980.0, t) + 0.5 * phase_sin(1420.0, t))
            * campana_env
            * 0.45;

        // Guiro / Maraca 16th-note rhythm in 4-10 kHz band
        let sixteenth_pos = (beat_num * 4.0).fract();
        let guiro_env = if sixteenth_pos < 0.35 {
            1.0 - (sixteenth_pos / 0.35)
        } else {
            0.05
        };
        let guiro_noise = lcg.next_f32() * guiro_env * 0.22;

        let sample = bass_signal + campana_signal + guiro_noise;
        mono.push(sample.clamp(-1.0, 1.0));
    }

    mono
}

/// Generates a synthetic Tecnomerengue track mono PCM:
/// - 148.0 or 158.0 BPM
/// - High-speed 16th-note güira pattern (4.5–11 kHz)
/// - Binary tambora pattern with kick-like low hits and syncopated slap
/// - Brass synth stabs on offbeats
fn generate_synthetic_tecnomerengue_pcm(
    bpm: f32,
    duration_sec: f32,
    sample_rate: u32,
) -> Vec<f32> {
    let total_frames = (duration_sec * sample_rate as f32) as usize;
    let mut mono = Vec::with_capacity(total_frames);
    let mut lcg = DeterministicLcg::new(0x7EC0_9E2E_0001_1337);
    let beat_sec = 60.0 / bpm;

    for i in 0..total_frames {
        let t = (i as f32) / (sample_rate as f32);
        let beat_frac = (t / beat_sec).fract();
        let half_beat_frac = ((t / beat_sec) * 2.0).fract();
        let sixteenth_frac = ((t / beat_sec) * 4.0).fract();

        // High-frequency steady 16th-note güira sizzle
        let guiro_env = if sixteenth_frac < 0.4 {
            1.0 - (sixteenth_frac / 0.4)
        } else {
            0.08
        };
        let guiro_sound = lcg.next_f32() * guiro_env * 0.45;

        // Tambora low hit on beat 1 and beat 2.5
        let tambora_env = if beat_frac < 0.20 {
            1.0 - (beat_frac / 0.20)
        } else if half_beat_frac > 0.45 && half_beat_frac < 0.65 {
            1.0 - ((half_beat_frac - 0.45) / 0.20)
        } else {
            0.0
        };
        let tambora_sub = phase_sin(78.0, t) * tambora_env * 0.55;

        // Brass synth stab on offbeats
        let offbeat_env = if (half_beat_frac - 0.5).abs() < 0.15 {
            1.0 - ((half_beat_frac - 0.5).abs() / 0.15)
        } else {
            0.0
        };
        let brass = (0.5 * phase_sin(440.0, t) + 0.5 * phase_sin(554.37, t)) * offbeat_env * 0.35;

        let sample = guiro_sound + tambora_sub + brass;
        mono.push(sample.clamp(-1.0, 1.0));
    }

    mono
}

/// Generates a live-band recording with variable BPM (drifting from start_bpm to end_bpm).
fn generate_variable_bpm_pcm(
    start_bpm: f32,
    end_bpm: f32,
    duration_sec: f32,
    sample_rate: u32,
) -> Vec<f32> {
    let total_frames = (duration_sec * sample_rate as f32) as usize;
    let mut mono = Vec::with_capacity(total_frames);
    let mut phase: f32 = 0.0;
    let mut lcg = DeterministicLcg::new(0xB411_D12F_7001_0007);

    for i in 0..total_frames {
        let t = (i as f32) / (sample_rate as f32);
        let progress = (t / duration_sec).clamp(0.0, 1.0);
        // Smooth S-curve acceleration from start_bpm to end_bpm
        let smooth_t = progress * progress * (3.0 - 2.0 * progress);
        let instant_bpm = start_bpm + (end_bpm - start_bpm) * smooth_t;

        let instant_freq = instant_bpm / 60.0;
        phase += instant_freq / (sample_rate as f32);
        if phase >= 1.0 {
            phase -= 1.0;
        }

        // Quarter-note percussive hit envelope
        let hit_env = if phase < 0.18 {
            (1.0 - (phase / 0.18)).powi(2)
        } else {
            0.02
        };

        // Offbeat ride / hi-hat
        let offbeat_phase = (phase + 0.5).fract();
        let offbeat_env = if offbeat_phase < 0.15 {
            1.0 - (offbeat_phase / 0.15)
        } else {
            0.0
        };

        let kick = phase_sin(65.0, t) * hit_env * 0.60;
        let snare = lcg.next_f32() * hit_env * 0.35;
        let ride = lcg.next_f32() * offbeat_env * 0.25;

        mono.push((kick + snare + ride).clamp(-1.0, 1.0));
    }

    mono
}

#[test]
fn test_salsa_tumbao_and_campana_resolves_to_82_108_bpm() {
    let sample_rate = 44100;
    let pcm = generate_synthetic_salsa_pcm(94.0, 48.0, sample_rate);

    let report = analyze_bpm_consensus(&pcm, sample_rate, None);

    assert!(
        (report.bpm - 94.0).abs() <= 0.6,
        "Salsa at 94.0 BPM must detect within ±0.6 BPM (got {}, expected ~94.0). Must NOT alias to 125.3 (4/3x) or 188 (2x)",
        report.bpm
    );
    assert_eq!(
        report.detected_rhythm_family, "Salsa",
        "Rhythm family must be identified as Salsa due to clave/campana and tumbao"
    );
    assert!(
        report.bpm >= 82.0 && report.bpm <= 108.0,
        "Salsa BPM must lie strictly within canonical 82..108 BPM scale"
    );
}

#[test]
fn test_variable_bpm_live_band_drift_tracks_entry_and_exit_bpm() {
    let sample_rate = 44100;
    let pcm = generate_variable_bpm_pcm(90.0, 96.5, 60.0, sample_rate);

    // Provide an erroneous static VirtualDJ tag hint of 129.0 to test resilience
    let hint = ExternalBpmHint {
        bpm: 129.0,
        genre: Some("Salsa Romantica Live".to_string()),
        source: "virtualdj".to_string(),
    };

    let report = analyze_bpm_consensus(&pcm, sample_rate, Some(hint));

    assert!(
        report.is_variable_bpm,
        "Engine must flag live band accelerating from 90.0 to 96.5 BPM as is_variable_bpm == true"
    );
    assert!(
        (report.entry_bpm - 90.0).abs() <= 1.2,
        "Entry BPM must track intro verse tempo ~90.0 BPM (got {})",
        report.entry_bpm
    );
    assert!(
        (report.exit_bpm - 96.5).abs() <= 1.2,
        "Exit BPM must track montuno/outro tempo ~96.5 BPM (got {})",
        report.exit_bpm
    );
    assert!(
        report.tempo_drift_bpm >= 5.0,
        "Detected drift must be >= 5.0 BPM (got {})",
        report.tempo_drift_bpm
    );
}

#[test]
fn test_tecnomerengue_132_to_160_bpm_not_folded_in_half() {
    let sample_rate = 44100;

    // Test 1: 148.0 BPM
    let pcm_148 = generate_synthetic_tecnomerengue_pcm(148.0, 35.0, sample_rate);
    let report_148 = analyze_bpm_consensus(&pcm_148, sample_rate, None);
    assert!(
        (report_148.bpm - 148.0).abs() <= 0.8,
        "Tecnomerengue at 148 BPM must detect ~148 BPM (got {}). NEVER fold in half to 74 BPM",
        report_148.bpm
    );
    assert_eq!(
        report_148.detected_rhythm_family, "Tecnomerengue",
        "Must classify as Tecnomerengue from high-speed guira and tambora"
    );

    // Test 2: 158.0 BPM (classic bug where > 155 BPM was folded to 79 BPM)
    let pcm_158 = generate_synthetic_tecnomerengue_pcm(158.0, 35.0, sample_rate);
    let report_158 = analyze_bpm_consensus(&pcm_158, sample_rate, None);
    assert!(
        (report_158.bpm - 158.0).abs() <= 0.8,
        "Tecnomerengue at 158 BPM must detect ~158 BPM (got {}). NEVER fold in half to 79 BPM",
        report_158.bpm
    );
}

#[test]
fn test_fractional_vinyl_tempo_not_falsely_integer_snapped() {
    let sample_rate = 44100;
    // Steady analog turntable tempo of exactly 92.35 BPM
    let pcm_analog = generate_synthetic_salsa_pcm(92.35, 40.0, sample_rate);

    let report = analyze_bpm_consensus(&pcm_analog, sample_rate, None);

    assert!(
        (report.bpm - 92.35).abs() <= 0.15,
        "Analog vinyl tempo of 92.35 BPM must preserve fractional value (got {}, expected 92.35 ± 0.15, NOT snapped to 92.00)",
        report.bpm
    );
}

#[test]
fn test_consensus_arbiter_resolves_octave_mismatch_with_genre_prior() {
    let sample_rate = 44100;
    let pcm = generate_synthetic_salsa_pcm(92.0, 35.0, sample_rate);

    // External tag erroneously marked as double-time 184.0 BPM with Salsa genre
    let hint = ExternalBpmHint {
        bpm: 184.0,
        genre: Some("Salsa Dura".to_string()),
        source: "id3".to_string(),
    };

    let report = analyze_bpm_consensus(&pcm, sample_rate, Some(hint));

    assert!(
        (report.bpm - 92.0).abs() <= 0.6,
        "Consensus arbiter must de-alias 184.0 BPM double-time tag back to 92.0 BPM for Salsa (got {})",
        report.bpm
    );
}
