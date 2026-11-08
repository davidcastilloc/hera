use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Complete zero-trust spectral & phase audit report for an audio master.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicAuditReport {
    pub is_authentic_lossless: bool,
    pub is_synthetic_tone: bool,
    pub effective_bandwidth_hz: f32,
    pub nyquist_hz: f32,
    pub suspected_source_codec: String,
    pub spectral_flatness: f32,
    pub peak_to_mean_ratio: f32,
    pub crest_factor_db: f32,
    pub clipped_samples: usize,
    pub clipping_ratio: f32,
    pub sub_bass_phase_correlation: f32,
    pub mono_compatible_sub: bool,
    pub quality_score: f32,
    pub forensic_verdict: String,
    pub warnings: Vec<String>,
}

/// Decoded audio buffer capped to a lightweight window (< 25 MB RAM for Cloudflare Workers).
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub left: Vec<f32>,
    pub right: Option<Vec<f32>>,
    pub mono: Vec<f32>,
    pub sample_rate: u32,
    pub duration_seconds: f32,
}

/// Decode encoded audio bytes (WAV, FLAC, MP3, OGG) into mono + stereo channels using pure-Rust Symphonia.
/// Caps decoded samples to `max_seconds` (default 45s) so memory stays well inside Cloudflare Workers' 128 MB limit.
pub fn decode_audio_bytes(
    bytes: &[u8],
    ext_hint: Option<&str>,
    max_seconds: f32,
) -> Result<DecodedAudio, String> {
    let owned = bytes.to_vec();
    let cursor = Cursor::new(owned);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = ext_hint {
        hint.with_extension(ext.trim_start_matches('.'));
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Unsupported or corrupted audio container: {e}"))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "No decodable audio track found in container".to_string())?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let total_frames = track.codec_params.n_frames;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Codec initialization failed: {e}"))?;

    let max_frames = (sample_rate as f32 * max_seconds.max(5.0)) as usize;
    let mut left = Vec::with_capacity(max_frames.min(sample_rate as usize * 30));
    let mut right: Vec<f32> = Vec::new();
    let mut is_stereo = false;

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                let spec = *audio_buf.spec();
                let channels = spec.channels.count();
                let frames = audio_buf.frames();
                let mut sample_buf = SampleBuffer::<f32>::new(frames as u64, spec);
                sample_buf.copy_interleaved_ref(audio_buf);
                let samples = sample_buf.samples();

                if channels >= 2 {
                    is_stereo = true;
                    for frame in samples.chunks(channels) {
                        left.push(frame[0]);
                        right.push(frame[1]);
                        if left.len() >= max_frames {
                            break;
                        }
                    }
                } else if channels == 1 {
                    for &s in samples {
                        left.push(s);
                        if left.len() >= max_frames {
                            break;
                        }
                    }
                }
                if left.len() >= max_frames {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if left.is_empty() {
        return Err("Decoded audio stream contains zero samples".to_string());
    }

    let mono: Vec<f32> = if is_stereo && right.len() == left.len() {
        left.iter()
            .zip(right.iter())
            .map(|(&l, &r)| 0.5 * (l + r))
            .collect()
    } else {
        left.clone()
    };

    let duration_seconds = match total_frames {
        Some(nf) if nf > 0 => (nf as f32) / (sample_rate as f32),
        _ => (mono.len() as f32) / (sample_rate as f32),
    };

    Ok(DecodedAudio {
        left,
        right: if is_stereo { Some(right) } else { None },
        mono,
        sample_rate,
        duration_seconds,
    })
}

/// Run the zero-trust forensic spectral & club-phase audit on decoded PCM channels.
pub fn audit_pcm_channels(
    left: &[f32],
    right: Option<&[f32]>,
    sample_rate: u32,
    claimed_ext: &str,
) -> ForensicAuditReport {
    let mono: Vec<f32> = if let Some(r) = right {
        let n = left.len().min(r.len());
        left[..n]
            .iter()
            .zip(r[..n].iter())
            .map(|(&l, &rv)| 0.5 * (l + rv))
            .collect()
    } else {
        left.to_vec()
    };

    let nyquist = sample_rate as f32 * 0.5;
    if mono.len() < 2048 {
        return ForensicAuditReport {
            is_authentic_lossless: false,
            is_synthetic_tone: false,
            effective_bandwidth_hz: 0.0,
            nyquist_hz: nyquist,
            suspected_source_codec: "corrupted_or_empty".to_string(),
            spectral_flatness: 0.0,
            peak_to_mean_ratio: 0.0,
            crest_factor_db: 0.0,
            clipped_samples: 0,
            clipping_ratio: 0.0,
            sub_bass_phase_correlation: 1.0,
            mono_compatible_sub: true,
            quality_score: 0.0,
            forensic_verdict: "REJECT_CORRUPTED".to_string(),
            warnings: vec!["Audio buffer too short for spectral verification".to_string()],
        };
    }

    // 1. Dynamic Range, Crest Factor & Clipping Audit
    let mut peak = 0.0_f32;
    let mut sum_sq = 0.0_f64;

    let count_clip = |slice: &[f32]| -> usize {
        slice.iter().filter(|&&s| s.abs() >= 0.999).count()
    };

    let clipped_samples = if let Some(r) = right {
        count_clip(left) + count_clip(r)
    } else {
        count_clip(&mono)
    };

    for &s in &mono {
        let a = s.abs();
        if a > peak {
            peak = a;
        }
        sum_sq += (s as f64) * (s as f64);
    }
    let rms = ((sum_sq / mono.len() as f64).sqrt() as f32).max(1e-9);
    let crest_factor_db = (20.0 * (peak.max(1e-9) / rms).log10()).max(0.0);
    let total_channel_samples = if right.is_some() {
        left.len() * 2
    } else {
        mono.len()
    };
    let clipping_ratio = clipped_samples as f32 / (total_channel_samples.max(1) as f32);

    // 2. Welch Averaged Power Spectrum via 2048-point Hann FFT
    let n_fft = 2048usize;
    let hop = 1024usize;
    let n_bins = n_fft / 2 + 1;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n_fft);

    let hann: Vec<f32> = (0..n_fft)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * (i as f32) / (n_fft as f32)).cos())
        })
        .collect();

    let mut mean_power = vec![0.0_f64; n_bins];
    let mut frame_count = 0usize;
    let mut buf = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; n_fft];

    let mut pos = 0usize;
    while pos + n_fft <= mono.len() {
        for i in 0..n_fft {
            buf[i] = Complex {
                re: mono[pos + i] * hann[i],
                im: 0.0,
            };
        }
        fft.process(&mut buf);
        for b in 0..n_bins {
            let mag_sq = (buf[b].re * buf[b].re + buf[b].im * buf[b].im) as f64;
            mean_power[b] += mag_sq;
        }
        frame_count += 1;
        pos += hop;
    }

    let inv_frames = 1.0 / (frame_count.max(1) as f64);
    for p in &mut mean_power {
        *p = (*p * inv_frames) + 1e-20;
    }

    // 3. Synthetic Sine Tone Detector (Peak-to-Mean Spectral Ratio + Wiener Spectral Flatness)
    let total_power: f64 = mean_power.iter().sum();
    let max_bin_power: f64 = mean_power.iter().copied().fold(0.0_f64, f64::max);
    let peak_to_mean_ratio = (max_bin_power / total_power.max(1e-20)) as f32;

    let log_mean: f64 = mean_power.iter().map(|&p| p.ln()).sum::<f64>() / (n_bins as f64);
    let geom_mean = log_mean.exp();
    let arith_mean = total_power / (n_bins as f64);
    let spectral_flatness = (geom_mean / arith_mean.max(1e-20)) as f32;

    let is_synthetic_tone = (peak_to_mean_ratio > 0.35 && spectral_flatness < 1e-4)
        || peak_to_mean_ratio > 0.65;

    // 4. Effective Bandwidth & Fake Lossless (Transcode) Detection
    let bin_hz = sample_rate as f32 / (n_fft as f32);
    let power_db: Vec<f32> = mean_power
        .iter()
        .map(|&p| 10.0 * (p as f32).log10())
        .collect();

    // Reference level in midband (500 Hz - 6000 Hz)
    let mut mid_vals: Vec<f32> = (0..n_bins)
        .filter_map(|b| {
            let f = b as f32 * bin_hz;
            if (500.0..=6000.0).contains(&f) {
                Some(power_db[b])
            } else {
                None
            }
        })
        .collect();
    mid_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let ref_db = if !mid_vals.is_empty() {
        mid_vals[(mid_vals.len() * 3) / 4]
    } else {
        power_db.iter().copied().fold(-120.0_f32, f32::max)
    };

    let active_threshold_db = ref_db - 48.0;
    let mut effective_bandwidth_hz = 0.0_f32;
    for b in 0..n_bins {
        let f = b as f32 * bin_hz;
        if f >= 1000.0 && power_db[b] >= active_threshold_db {
            effective_bandwidth_hz = f;
        }
    }

    // Check high-frequency brickwall drop (> 16.5 kHz vs midband)
    let hf_mean_db = {
        let mut sum = 0.0_f32;
        let mut cnt = 0usize;
        for b in 0..n_bins {
            let f = b as f32 * bin_hz;
            if (16800.0..=20500.0).contains(&f) {
                sum += power_db[b];
                cnt += 1;
            }
        }
        if cnt > 0 {
            sum / (cnt as f32)
        } else {
            ref_db
        }
    };
    let brickwall_drop_db = ref_db - hf_mean_db;

    let ext_lower = claimed_ext.trim_start_matches('.').to_ascii_lowercase();
    let is_lossless_container = matches!(
        ext_lower.as_str(),
        "flac" | "wav" | "aiff" | "aif" | "alac" | ""
    );

    let (is_authentic_lossless, suspected_source_codec) = if sample_rate >= 44100 {
        if effective_bandwidth_hz < 15800.0
            || (effective_bandwidth_hz < 16800.0 && brickwall_drop_db > 32.0)
        {
            (!is_lossless_container, "mp3_128kbps_transcode".to_string())
        } else if effective_bandwidth_hz < 17400.0 && brickwall_drop_db > 28.0 {
            (!is_lossless_container, "mp3_192kbps_transcode".to_string())
        } else if effective_bandwidth_hz < 18600.0 && brickwall_drop_db > 26.0 {
            (!is_lossless_container, "aac_or_mp3_256kbps".to_string())
        } else {
            (true, "authentic_pcm_or_320plus".to_string())
        }
    } else {
        (true, "low_samplerate_master".to_string())
    };

    // 5. Sub-Bass (< 120 Hz) Stereo Phase Correlation Audit
    let sub_bass_phase_correlation = if let Some(r) = right {
        compute_sub_bass_correlation(left, r, sample_rate)
    } else {
        1.0
    };
    let mono_compatible_sub = sub_bass_phase_correlation >= 0.25;

    // 6. Composite Quality Score & Warnings
    let mut warnings = Vec::new();
    let mut score = 100.0_f32;

    if is_synthetic_tone {
        score = 0.0;
        warnings.push(
            "CRITICAL: Synthetic sine/test tone detected (Peak Ratio > 0.35, Flatness < 1e-4)."
                .to_string(),
        );
    }
    if !is_authentic_lossless {
        score -= 35.0;
        warnings.push(format!(
            "FAKE LOSSLESS: Container claims .{ext_lower} but spectrum brickwalls at {:.0} Hz ({suspected_source_codec}).",
            effective_bandwidth_hz
        ));
    }
    if !mono_compatible_sub {
        score -= 25.0;
        warnings.push(format!(
            "PHASE ALERT: Sub-bass (<120 Hz) correlation is {:.2}. Will lose low-end punch on mono club subwoofers.",
            sub_bass_phase_correlation
        ));
    }
    if clipping_ratio > 0.002 {
        score -= (clipping_ratio * 2500.0).min(20.0);
        warnings.push(format!(
            "CLIPPING: {clipped_samples} clipped samples detected ({:.2}%).",
            clipping_ratio * 100.0
        ));
    }
    if crest_factor_db < 5.5 && !is_synthetic_tone {
        score -= 10.0;
        warnings.push(format!(
            "OVER-COMPRESSED: Crest factor is {:.1} dB (hyper-brickwalled master).",
            crest_factor_db
        ));
    }

    score = score.clamp(0.0, 100.0);

    let forensic_verdict = if is_synthetic_tone {
        "REJECT_SYNTHETIC_TONE"
    } else if !is_authentic_lossless {
        "WARN_FAKE_LOSSLESS_TRANSCODE"
    } else if !mono_compatible_sub {
        "WARN_SUB_BASS_PHASE_CANCELLATION"
    } else {
        "VERIFIED_STUDIO_MASTER"
    }
    .to_string();

    ForensicAuditReport {
        is_authentic_lossless,
        is_synthetic_tone,
        effective_bandwidth_hz: (effective_bandwidth_hz * 10.0).round() / 10.0,
        nyquist_hz: nyquist,
        suspected_source_codec,
        spectral_flatness,
        peak_to_mean_ratio: (peak_to_mean_ratio * 10000.0).round() / 10000.0,
        crest_factor_db: (crest_factor_db * 100.0).round() / 100.0,
        clipped_samples,
        clipping_ratio: (clipping_ratio * 100000.0).round() / 100000.0,
        sub_bass_phase_correlation: (sub_bass_phase_correlation * 1000.0).round() / 1000.0,
        mono_compatible_sub,
        quality_score: (score * 10.0).round() / 10.0,
        forensic_verdict,
        warnings,
    }
}

/// First-order 2-pass low-pass filter at 120 Hz followed by Pearson correlation between Left & Right.
fn compute_sub_bass_correlation(left: &[f32], right: &[f32], sample_rate: u32) -> f32 {
    let n = left.len().min(right.len());
    if n < 512 {
        return 1.0;
    }
    let dt = 1.0 / (sample_rate as f32);
    let rc = 1.0 / (2.0 * std::f32::consts::PI * 120.0);
    let alpha = dt / (rc + dt);

    let mut lp_l = 0.0_f32;
    let mut lp_r = 0.0_f32;
    let mut sum_l2 = 0.0_f64;
    let mut sum_r2 = 0.0_f64;
    let mut sum_lr = 0.0_f64;

    for i in 0..n {
        lp_l += alpha * (left[i] - lp_l);
        lp_r += alpha * (right[i] - lp_r);
        let l64 = lp_l as f64;
        let r64 = lp_r as f64;
        sum_l2 += l64 * l64;
        sum_r2 += r64 * r64;
        sum_lr += l64 * r64;
    }

    let denom = (sum_l2 * sum_r2).sqrt();
    if denom < 1e-12 {
        1.0
    } else {
        (sum_lr / denom).clamp(-1.0, 1.0) as f32
    }
}
