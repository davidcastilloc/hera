//! Prefix-first (`< 50 ms`) + seamless background upgrade loader for `AudioControllerHub`.

use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::structure::analyze_structure_and_style;

use super::backend::AudioTransportSnapshot;
use super::dsp_chain::resample_deck_buffer_to_rate;
use super::hub::AudioControllerHub;
use super::{DeckId, PreallocatedDeckBuffer};

const PREFIX_DECODE_SECS: f64 = 12.0;

struct DecodedPcmChunk {
    interleaved_stereo: Vec<f32>,
    sample_rate: u32,
    eof_reached: bool,
    container_duration_sec: Option<f64>,
}

/// Immediately decodes up to 12s of audio (`< 50 ms`) so deck playback and Hot Cue seek start
/// without blocking on full-track decoding/resampling; spawns a background worker for long tracks
/// that upgrades the playing deck buffer seamlessly when the full PCM decode completes.
pub(crate) fn load_file_prefix_then_full(
    hub: &Arc<AudioControllerHub>,
    deck: DeckId,
    path: &Path,
    duration_hint_sec: Option<f64>,
    known_cues_sec: Option<[f64; 4]>,
) -> Result<AudioTransportSnapshot, String> {
    let prefix = decode_audio_chunk(path, Some(PREFIX_DECODE_SECS), None)?;
    let target_sr = hub.atomics.sample_rate.max(8_000);
    let decoded_sec =
        (prefix.interleaved_stereo.len() / 2) as f64 / f64::from(prefix.sample_rate.max(1));
    let logical_dur_sec = if prefix.eof_reached {
        decoded_sec
    } else {
        prefix
            .container_duration_sec
            .filter(|d| d.is_finite() && *d > decoded_sec)
            .or_else(|| duration_hint_sec.filter(|d| d.is_finite() && *d > decoded_sec))
            .unwrap_or(decoded_sec)
    };

    let prefix_cues_sec = if let Some(cues) = known_cues_sec {
        sanitize_cue_seconds(cues)
    } else if prefix.eof_reached {
        compute_cues_seconds(&prefix.interleaved_stereo, prefix.sample_rate)
    } else {
        [0.02, 0.25, 0.50, 0.80].map(|ratio| logical_dur_sec * ratio)
    };

    let raw_prefix =
        PreallocatedDeckBuffer::from_stereo_frames(prefix.interleaved_stereo, prefix.sample_rate, [0; 4])?;
    let mut resampled_prefix = resample_deck_buffer_to_rate(&raw_prefix, target_sr)?;
    let logical_total_frames = ((logical_dur_sec * f64::from(target_sr)).round() as u64)
        .max(resampled_prefix.total_frames());
    apply_cue_seconds(&mut resampled_prefix, prefix_cues_sec, logical_total_frames);

    let source_path = Some(path.to_string_lossy().to_string());
    if prefix.eof_reached {
        return Ok(hub.install_resampled_buffer(deck, Arc::new(resampled_prefix), source_path));
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let (snap, generation) = hub.install_prefix_buffer(
        deck,
        Arc::new(resampled_prefix),
        logical_total_frames,
        source_path,
        Arc::clone(&cancel),
    );

    let hub_bg = Arc::clone(hub);
    let path_bg = path.to_path_buf();
    let _ = thread::Builder::new()
        .name("hera-deck-full-decode".to_string())
        .spawn(move || {
            if cancel.load(Ordering::Acquire) {
                return;
            }
            let Ok(full) = decode_audio_chunk(&path_bg, None, Some(&cancel)) else {
                return;
            };
            if cancel.load(Ordering::Acquire)
                || hub_bg.atomics.load_generation.load(Ordering::Acquire) != generation
            {
                return;
            }
            let cue_secs = match known_cues_sec {
                Some(cues) => sanitize_cue_seconds(cues),
                None => compute_cues_seconds(&full.interleaved_stereo, full.sample_rate),
            };
            let Ok(raw_full) = PreallocatedDeckBuffer::from_stereo_frames(
                full.interleaved_stereo,
                full.sample_rate,
                [0; 4],
            ) else {
                return;
            };
            let Ok(mut resampled_full) = resample_deck_buffer_to_rate(&raw_full, target_sr) else {
                return;
            };
            if cancel.load(Ordering::Acquire)
                || hub_bg.atomics.load_generation.load(Ordering::Acquire) != generation
            {
                return;
            }
            let full_frames = resampled_full.total_frames();
            apply_cue_seconds(&mut resampled_full, cue_secs, full_frames);
            hub_bg.upgrade_buffer_if_generation(deck, generation, Arc::new(resampled_full), &cancel);
        });

    Ok(snap)
}

fn sanitize_cue_seconds(cues: [f64; 4]) -> [f64; 4] {
    cues.map(|s| if s.is_finite() { s.max(0.0) } else { 0.0 })
}

fn apply_cue_seconds(
    buf: &mut PreallocatedDeckBuffer,
    cue_secs: [f64; 4],
    logical_total_frames: u64,
) {
    let max_frame = logical_total_frames.max(buf.total_frames()).saturating_sub(1);
    let sr = f64::from(buf.sample_rate.max(1));
    for (slot, sec) in buf.hot_cues.iter_mut().zip(cue_secs.iter()) {
        slot.frame_index = ((*sec).max(0.0) * sr).round().min(max_frame as f64) as u64;
    }
}

fn compute_cues_seconds(interleaved_stereo: &[f32], sample_rate: u32) -> [f64; 4] {
    let total_frames = interleaved_stereo.len() / 2;
    if total_frames < 4 {
        return [0.0; 4];
    }
    let mono: Vec<f32> = (0..total_frames)
        .map(|f| 0.5 * (interleaved_stereo[f * 2] + interleaved_stereo[f * 2 + 1]))
        .collect();
    let full_dur_sec = (total_frames as f32) / (sample_rate.max(1) as f32);
    let profile = analyze_structure_and_style(&mono, sample_rate.max(1), Some(full_dur_sec), None);
    [
        f64::from(profile.first_beat_seconds.max(0.0)),
        f64::from(profile.phrase_in_seconds.max(0.0)),
        f64::from(profile.main_drop_seconds.max(0.0)),
        f64::from(profile.mix_out_seconds.max(0.0)),
    ]
}

fn decode_audio_chunk(
    path: &Path,
    max_seconds: Option<f64>,
    cancel: Option<&AtomicBool>,
) -> Result<DecodedPcmChunk, String> {
    let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
    if !matches!(ext.as_deref(), Some("webm" | "mkv" | "flv")) {
        if let Ok(file) = File::open(path) {
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            if let Ok(chunk) = decode_symphonia_chunk(mss, ext.as_deref(), max_seconds, cancel) {
                return Ok(chunk);
            }
            if cancel.is_some_and(|c| c.load(Ordering::Acquire)) {
                return Err("Decodificación cancelada".to_string());
            }
        }
    }

    let (stereo, sr) =
        super::decoder_ffmpeg::decode_stereo_via_ffmpeg_bounded(path, max_seconds, cancel)?;
    let decoded_sec = (stereo.len() / 2) as f64 / f64::from(sr.max(1));
    let eof_reached = match max_seconds {
        Some(limit) => decoded_sec + 0.5 < limit,
        None => true,
    };
    Ok(DecodedPcmChunk {
        interleaved_stereo: stereo,
        sample_rate: sr,
        eof_reached,
        container_duration_sec: None,
    })
}

fn decode_symphonia_chunk(
    mss: MediaSourceStream,
    ext_hint: Option<&str>,
    max_seconds: Option<f64>,
    cancel: Option<&AtomicBool>,
) -> Result<DecodedPcmChunk, String> {
    let mut hint = Hint::new();
    if let Some(ext) = ext_hint {
        hint.with_extension(ext.trim_start_matches('.'));
    }
    let mut format = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("Contenedor de audio inválido o no soportado: {e}"))?
        .format;

    let track = format
        .tracks()
        .iter()
        .find(|t| {
            t.codec_params.codec != CODEC_TYPE_NULL
                && (t.codec_params.sample_rate.is_some() || t.codec_params.channels.is_some())
        })
        .or_else(|| format.default_track())
        .ok_or_else(|| "Sin pista decodificable en contenedor".to_string())?;

    let track_id = track.id;
    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(44_100).max(1);
    let container_duration_sec = track
        .codec_params
        .n_frames
        .and_then(|n| track.codec_params.sample_rate.filter(|&sr| sr > 0 && n > 0).map(|sr| (n as f64) / f64::from(sr)));
    let reserve_frames = max_seconds
        .map(|s| (s * f64::from(sample_rate)).ceil() as usize)
        .unwrap_or_else(|| track.codec_params.n_frames.unwrap_or(u64::from(sample_rate) * 30) as usize);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Error iniciando códec: {e}"))?;

    let mut stereo = Vec::with_capacity(reserve_frames.saturating_mul(2));
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut eof_reached = false;

    loop {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err("Decodificación cancelada".to_string());
        }
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(err)) if err.kind() == ErrorKind::UnexpectedEof => {
                eof_reached = true;
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(_) => {
                eof_reached = true;
                break;
            }
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                let spec = *audio_buf.spec();
                if spec.rate > 0 {
                    sample_rate = spec.rate;
                }
                let channels = spec.channels.count();
                let frames = audio_buf.frames();
                if frames == 0 || channels == 0 {
                    continue;
                }
                let sb = match &mut sample_buf {
                    Some(existing) if existing.capacity() >= audio_buf.capacity() => existing,
                    slot => slot.insert(SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec)),
                };
                sb.copy_interleaved_ref(audio_buf);
                append_interleaved_stereo(sb.samples(), channels, frames, &mut stereo);
                if let Some(limit_sec) = max_seconds {
                    let limit_frames = ((limit_sec.max(0.5)) * f64::from(sample_rate)).ceil() as usize;
                    if stereo.len() / 2 >= limit_frames {
                        break;
                    }
                }
            }
            Err(SymphoniaError::ResetRequired) => decoder.reset(),
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => {
                eof_reached = true;
                break;
            }
        }
    }

    if stereo.is_empty() {
        return Err("Decoded audio stream contains zero samples".to_string());
    }

    Ok(DecodedPcmChunk {
        interleaved_stereo: stereo,
        sample_rate,
        eof_reached,
        container_duration_sec,
    })
}

fn append_interleaved_stereo(samples: &[f32], channels: usize, frames: usize, out: &mut Vec<f32>) {
    if channels == 2 {
        out.extend_from_slice(&samples[..frames * 2]);
    } else if channels == 1 {
        for &s in &samples[..frames] {
            out.push(s);
            out.push(s);
        }
    } else {
        for frame in samples[..frames * channels].chunks_exact(channels) {
            out.push(frame[0]);
            out.push(frame[1]);
        }
    }
}
