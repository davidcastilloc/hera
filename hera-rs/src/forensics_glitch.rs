use serde::{Deserialize, Serialize};

/// Report characterizing scratched-CD skips, buffer stutters, micro-dropouts, and packet errors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GlitchScanReport {
    pub has_audio_skips: bool,
    pub skip_glitch_count: u32,
    pub decode_error_packets: u32,
    pub micro_dropout_count: u32,
    pub buffer_repeat_stutter_count: u32,
    pub skip_timestamps_seconds: Vec<f32>,
    pub warning_summary: Option<String>,
}

/// Analyze decoded mono PCM samples for scratched-CD skips, phase cliffs, and buffer repeats.
pub fn scan_pcm_for_cd_skips(
    mono: &[f32],
    sample_rate: u32,
    decode_error_packets: u32,
) -> GlitchScanReport {
    let sr = sample_rate.max(8_000) as usize;
    let sr_f = sr as f32;
    let total_samples = mono.len();

    let mut micro_dropout_count = 0u32;
    let mut buffer_repeat_stutter_count = 0u32;
    let mut timestamps = Vec::new();

    if total_samples >= sr / 4 {
        // 1. Detect micro-dropouts bounded by sharp phase/amplitude cliffs
        // Window of 500ms for local background RMS
        let bg_win = (sr / 2).min(total_samples);
        let mut bg_energy = 0.0_f32;
        for &s in &mono[..bg_win] {
            bg_energy += s * s;
        }

        let min_drop_samples = (sr_f * 0.015) as usize; // 15ms
        let max_drop_samples = (sr_f * 0.350) as usize; // 350ms

        let mut in_zero_run = false;
        let mut zero_start = 0usize;

        let mut i = 0usize;
        while i < total_samples {
            // Sliding energy update
            if i >= bg_win {
                let old_s = mono[i - bg_win];
                let new_s = mono[i];
                bg_energy = (bg_energy - old_s * old_s + new_s * new_s).max(0.0);
            }
            let local_rms = (bg_energy / bg_win as f32).sqrt();

            let sample = mono[i].abs();
            if sample <= 1e-4 {
                if !in_zero_run {
                    in_zero_run = true;
                    zero_start = i;
                }
            } else if in_zero_run {
                in_zero_run = false;
                let drop_len = i.saturating_sub(zero_start);
                if (min_drop_samples..=max_drop_samples).contains(&drop_len) && local_rms >= 0.05 {
                    // Check for step discontinuity cliff immediately after dropout
                    let step = (mono[i] - mono[zero_start.saturating_sub(1)]).abs();
                    if step >= 0.50 || mono[i].abs() >= 0.45 {
                        micro_dropout_count += 1;
                        let t = (zero_start as f32) / sr_f;
                        let rounded_t = (t * 100.0).round() / 100.0;
                        if !timestamps.contains(&rounded_t) {
                            timestamps.push(rounded_t);
                        }
                    }
                }
            }
            i += 1;
        }

        // 2. Detect repeated buffer stutter (CD optical head skip looping 40ms..120ms chunk)
        let stutter_chunk_len = (sr_f * 0.060) as usize; // 60ms
        let hop = stutter_chunk_len;
        if stutter_chunk_len > 0 && total_samples > stutter_chunk_len * 3 {
            let mut pos = 0usize;
            while pos + stutter_chunk_len * 2 <= total_samples {
                let chunk1 = &mono[pos..pos + stutter_chunk_len];
                let chunk2 = &mono[pos + stutter_chunk_len..pos + stutter_chunk_len * 2];

                // Quick normalized cross-correlation
                let mut sum1 = 0.0_f32;
                let mut sum2 = 0.0_f32;
                let mut prod = 0.0_f32;
                for k in 0..stutter_chunk_len {
                    let a = chunk1[k];
                    let b = chunk2[k];
                    sum1 += a * a;
                    sum2 += b * b;
                    prod += a * b;
                }
                let norm = (sum1 * sum2).sqrt();
                if norm > 1e-3 {
                    let corr = prod / norm;
                    if corr >= 0.985 && sum1 / (stutter_chunk_len as f32) >= 0.01 {
                        buffer_repeat_stutter_count += 1;
                        let t = (pos as f32) / sr_f;
                        let rounded_t = (t * 100.0).round() / 100.0;
                        if !timestamps.contains(&rounded_t) {
                            timestamps.push(rounded_t);
                        }
                        pos += stutter_chunk_len; // Skip ahead after finding stutter
                    }
                }
                pos += hop;
            }
        }
    }

    let skip_glitch_count = micro_dropout_count + buffer_repeat_stutter_count;
    let has_audio_skips = skip_glitch_count > 0 || decode_error_packets >= 3;

    let warning_summary = if has_audio_skips {
        let mut parts = Vec::new();
        if micro_dropout_count > 0 {
            parts.push(format!("{micro_dropout_count} micro-corte(s)"));
        }
        if buffer_repeat_stutter_count > 0 {
            parts.push(format!("{buffer_repeat_stutter_count} bucle(s) CD rayado"));
        }
        if decode_error_packets > 0 {
            parts.push(format!("{decode_error_packets} paquete(s) corrupto(s)"));
        }
        Some(format!(
            "⚠️ Detectados saltos de audio ({})",
            parts.join(", ")
        ))
    } else {
        None
    };

    GlitchScanReport {
        has_audio_skips,
        skip_glitch_count,
        decode_error_packets,
        micro_dropout_count,
        buffer_repeat_stutter_count,
        skip_timestamps_seconds: timestamps,
        warning_summary,
    }
}

/// Decoded audio stream containing stereo/mono PCM frames, packet error count, and duration.
#[derive(Debug, Clone)]
pub struct DecodedAudioStreamWithStats {
    pub left: Vec<f32>,
    pub right: Option<Vec<f32>>,
    pub mono: Vec<f32>,
    pub sample_rate: u32,
    pub duration_seconds: f32,
    pub decode_error_packets: u32,
    pub was_cancelled: bool,
}

/// Decode encoded audio from a media file up to `max_seconds` (up to 600s),
/// tracking corrupted packets and supporting cooperative cancellation via `AtomicBool`.
#[cfg(not(target_arch = "wasm32"))]
pub fn decode_audio_file_cancellable(
    path: &std::path::Path,
    max_seconds: f32,
    cancel_flag: Option<&std::sync::atomic::AtomicBool>,
) -> Result<DecodedAudioStreamWithStats, String> {
    use std::fs::File;
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = File::open(path)
        .map_err(|e| format!("Failed to open media file {}: {e}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    let mut hint = Hint::new();
    if let Some(ref ext_str) = ext {
        hint.with_extension(ext_str.trim_start_matches('.'));
    }

    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Unsupported or corrupted media container: {e}"))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "No decodable audio track found in media container".to_string())?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let total_frames = track.codec_params.n_frames;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Codec initialization failed: {e}"))?;

    let max_frames = (sample_rate as f32 * max_seconds.max(5.0)) as usize;
    let mut left = Vec::with_capacity(max_frames.min(sample_rate as usize * 60));
    let mut right: Vec<f32> = Vec::new();
    let mut is_stereo = false;
    let mut decode_error_packets = 0u32;
    let mut was_cancelled = false;

    while let Ok(packet) = format.next_packet() {
        if let Some(flag) = cancel_flag {
            if flag.load(std::sync::atomic::Ordering::Relaxed) {
                was_cancelled = true;
                break;
            }
        }
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
            Err(_) => {
                decode_error_packets += 1;
                // If corrupted packet storm exceeds threshold, break cleanly without crashing
                if decode_error_packets > 100 {
                    break;
                }
            }
        }
    }

    if left.is_empty() {
        return Err("Decoded media stream contains zero audio samples".to_string());
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

    Ok(DecodedAudioStreamWithStats {
        left,
        right: if is_stereo { Some(right) } else { None },
        mono,
        sample_rate,
        duration_seconds,
        decode_error_packets,
        was_cancelled,
    })
}
