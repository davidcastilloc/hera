    use super::audio_ingest::{fast_partial_file_hash, load_audio_mono_native_sr, ProcessLimiter};
    use super::consensus_ai::analyze_bpm_consensus_v2;
    use super::hints::{resolve_best_hint, VirtualDjDbIndex};
    use super::key_engine::detect_song_key;
    use super::multiband_onset::{extract_multiband_novelty, trim_leading_trailing_silence};
    use super::queue::{FullTrackAnalysis, QueueTrackInput};
    use super::*;

    pub enum WorkerOutcome {
        Analyzed(FullTrackAnalysis),
        Duplicate { track_id: String, hash: String },
        Failed { track_id: String, error: String },
    }

    pub fn analyze_single_job(
        job: QueueTrackInput,
        vdj_db: Option<&VirtualDjDbIndex>,
        seen: &Mutex<HashSet<String>>,
        ffmpeg_limiter: &ProcessLimiter,
        excerpt_mode: bool,
        planner: &mut FftPlanner<f32>,
    ) -> WorkerOutcome {
        let start_t = Instant::now();
        let track_id = job.track_id.clone();

        // Deduplicación O(1) antes de decodificar si viene de archivo en disco
        let mut pre_hash: Option<String> = None;
        if let Some(ref p) = job.path {
            if let Ok(fh) = fast_partial_file_hash(p) {
                if let Ok(mut g) = seen.lock() {
                    if !g.insert(fh.clone()) {
                        return WorkerOutcome::Duplicate {
                            track_id,
                            hash: fh,
                        };
                    }
                }
                pre_hash = Some(fh);
            }
        }

        let (samples, sr, duration_sec, tag_bpm, used_ffmpeg) =
            if let Some((pcm, in_sr)) = job.inline_pcm {
                let dur = (pcm.len() as f64) / (in_sr.max(1) as f64);
                ((*pcm).clone(), in_sr.max(8_000), dur, None, false)
            } else if let Some(ref p) = job.path {
                match load_audio_mono_native_sr(p, Some(ffmpeg_limiter)) {
                    Ok(dec) => (
                        dec.samples,
                        dec.sample_rate,
                        dec.duration_sec,
                        dec.embedded_bpm_tag,
                        dec.used_ffmpeg_fallback,
                    ),
                    Err(e) => {
                        return WorkerOutcome::Failed {
                            track_id,
                            error: e.to_string(),
                        }
                    }
                }
            } else {
                return WorkerOutcome::Failed {
                    track_id,
                    error: "Entrada sin archivo ni buffer PCM".into(),
                };
            };

        if duration_sec < 3.0 {
            return WorkerOutcome::Failed {
                track_id,
                error: EngineError::TrackTooShort { duration_sec }.to_string(),
            };
        }

        let hash = pre_hash.unwrap_or_else(|| fast_pcm_hash_128(&samples, sr));

        let resolved_hint = resolve_best_hint(
            job.explicit_bpm_hint,
            job.path.as_deref(),
            tag_bpm,
            vdj_db,
        );

        let (trimmed, lead_sec, _trail_sec) = trim_leading_trailing_silence(&samples, sr);

        // Si la canción supera 150 s, concatena 3 ventanas de 22 s (25%, 50%, 75%) para acelerar DSP 3x
        // y evitar intros ambientales; el bloqueo de fase por bloques de 12 s maneja las juntas sin error!
        let working_slice: Vec<f32>;
        let analysis_pcm: &[f32] = if excerpt_mode && trimmed.len() > (sr as usize) * 150 {
            let win_samples = (sr as usize) * 22;
            let total_s = trimmed.len();
            let centers = [total_s / 4, total_s / 2, (total_s * 3) / 4];
            let mut joined = Vec::with_capacity(win_samples * 3);
            for &c in &centers {
                let s = c.saturating_sub(win_samples / 2);
                let e = (s + win_samples).min(total_s);
                joined.extend_from_slice(&trimmed[s..e]);
            }
            working_slice = joined;
            &working_slice
        } else {
            trimmed
        };

        let nov = extract_multiband_novelty(analysis_pcm, sr, lead_sec, planner);
        let bpm_result = analyze_bpm_consensus_v2(&nov, resolved_hint);
        let key_result = detect_song_key(analysis_pcm, sr, planner);
        drop(samples);

        WorkerOutcome::Analyzed(FullTrackAnalysis {
            track_id,
            audio_hash_hex: hash,
            sample_rate: sr,
            duration_sec,
            bpm_result,
            key_result,
            used_ffmpeg_video_fallback: used_ffmpeg,
            elapsed_ms: start_t.elapsed().as_millis() as u64,
        })
    }

    fn fast_pcm_hash_128(samples: &[f32], sr: u32) -> String {
        let mut h1: u64 = 0xcbf2_9ce4_8422_2325 ^ (sr as u64);
        let mut h2: u64 = 0x8422_2325_cbf2_9ce4;
        let step = (samples.len() / 8192).max(1);
        for (i, &s) in samples.iter().step_by(step).enumerate() {
            let q = (s.clamp(-1.0, 1.0) * 32767.0) as i16 as u16;
            h1 = (h1 ^ ((q & 0xFF) as u64)).wrapping_mul(0x0100_0000_01b3);
            h2 = (h2 ^ ((q >> 8) as u64) ^ (i as u64 & 0xFF)).wrapping_mul(0x0100_0000_01b3);
        }
        h2 ^= samples.len() as u64;
        format!("{:016x}{:016x}", h1, h2)
    }

#[inline]
fn median_f64(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        0.5 * (values[mid - 1] + values[mid])
    }