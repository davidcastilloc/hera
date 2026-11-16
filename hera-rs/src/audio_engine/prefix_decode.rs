//! Bounded Symphonia + FFmpeg stereo chunk decoder with OOM-safe frame reservation.

use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Hard upper bound (`15 min`) for background in-memory full-track decoding so multi-hour DJ mixes
/// (`> 1h`–`3h`) never exhaust process heap via `Vec::with_capacity` or `rubato` resampling.
pub(crate) const MAX_FULL_IN_MEMORY_DECODE_SECS: f64 = 900.0;

pub(crate) struct DecodedPcmChunk {
    pub interleaved_stereo: Vec<f32>,
    pub sample_rate: u32,
    pub eof_reached: bool,
    pub container_duration_sec: Option<f64>,
}

pub(crate) fn clamp_reserve_frames(
    max_seconds: Option<f64>,
    n_frames: Option<u64>,
    sample_rate: u32,
) -> usize {
    let sr = f64::from(sample_rate.max(1));
    let ceiling = (MAX_FULL_IN_MEMORY_DECODE_SECS * sr).ceil() as usize;
    let requested = max_seconds
        .map(|s| (s.min(MAX_FULL_IN_MEMORY_DECODE_SECS) * sr).ceil() as usize)
        .unwrap_or_else(|| {
            n_frames
                .map(|n| usize::try_from(n).unwrap_or(ceiling))
                .unwrap_or_else(|| (30.0 * sr) as usize)
        });
    requested.min(ceiling)
}

pub(crate) fn decode_audio_chunk(
    path: &Path,
    max_seconds: Option<f64>,
    cancel: Option<&AtomicBool>,
) -> Result<DecodedPcmChunk, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
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
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
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
    let container_duration_sec = track.codec_params.n_frames.and_then(|n| {
        track
            .codec_params
            .sample_rate
            .filter(|&sr| sr > 0 && n > 0)
            .map(|sr| (n as f64) / f64::from(sr))
    });
    let reserve_frames =
        clamp_reserve_frames(max_seconds, track.codec_params.n_frames, sample_rate);

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
                    slot => {
                        slot.insert(SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec))
                    }
                };
                sb.copy_interleaved_ref(audio_buf);
                append_interleaved_stereo(sb.samples(), channels, frames, &mut stereo);
                if let Some(limit_sec) = max_seconds {
                    let limit_frames =
                        ((limit_sec.max(0.5)) * f64::from(sample_rate)).ceil() as usize;
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
