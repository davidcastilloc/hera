//! Symphonia full-stream stereo PCM decoder & 4-Hot-Cue sample-accurate frame mapper.

use std::io::Cursor;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::structure::analyze_structure_and_style;

/// Canonical 4-Hot-Cue labels across HERA (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`).
pub const HOT_CUE_LABELS: [&str; 4] = ["1:Intro", "2:Breakdown", "3:Drop", "4:Outro"];

/// Sample-accurate stereo frame marker for one of HERA's 4 structural Hot Cues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotCueSampleMarker {
    pub num: u8,
    pub label: &'static str,
    pub frame_index: u64,
}

/// Immutable pre-allocated stereo PCM buffer (`len == total_frames * 2`) with 4 Hot Cue pins.
///
/// Allocated and decoded off the audio thread before being shared into `DualDeckEngine` via `Arc`.
#[derive(Debug, Clone)]
pub struct PreallocatedDeckBuffer {
    interleaved_stereo: Vec<f32>,
    pub sample_rate: u32,
    pub hot_cues: [HotCueSampleMarker; 4],
}

impl PreallocatedDeckBuffer {
    /// Construct a validated stereo deck buffer from pre-interleaved `(L, R)` frames and 4 cue offsets.
    pub fn from_stereo_frames(
        interleaved_stereo: Vec<f32>,
        sample_rate: u32,
        hot_cue_frames: [u64; 4],
    ) -> Result<Self, String> {
        if sample_rate == 0 {
            return Err("sample_rate must be greater than zero".to_string());
        }
        if interleaved_stereo.is_empty() || (interleaved_stereo.len() & 1) != 0 {
            return Err(
                "interleaved_stereo must contain non-empty (L, R) stereo frame pairs".to_string(),
            );
        }

        let total_frames = (interleaved_stereo.len() / 2) as u64;
        let max_frame = total_frames - 1;
        let hot_cues = [
            HotCueSampleMarker {
                num: 1,
                label: HOT_CUE_LABELS[0],
                frame_index: hot_cue_frames[0].min(max_frame),
            },
            HotCueSampleMarker {
                num: 2,
                label: HOT_CUE_LABELS[1],
                frame_index: hot_cue_frames[1].min(max_frame),
            },
            HotCueSampleMarker {
                num: 3,
                label: HOT_CUE_LABELS[2],
                frame_index: hot_cue_frames[2].min(max_frame),
            },
            HotCueSampleMarker {
                num: 4,
                label: HOT_CUE_LABELS[3],
                frame_index: hot_cue_frames[3].min(max_frame),
            },
        ];

        Ok(Self {
            interleaved_stereo,
            sample_rate,
            hot_cues,
        })
    }

    /// Decode an entire encoded audio byte slice (WAV, FLAC, MP3, M4A/AAC, ALAC, OGG) into stereo
    /// `f32` PCM and compute 4 sample-accurate Hot Cue frame offsets via `analyze_structure_and_style`.
    pub fn decode_from_bytes(bytes: &[u8], ext_hint: Option<&str>) -> Result<Self, String> {
        let cursor = Cursor::new(bytes.to_vec());
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
        decode_media_source(mss, ext_hint, None)
    }

    /// Stream-decode an audio file directly from disk via `std::fs::File` and `MediaSourceStream`.
    pub fn decode_from_file(path: &Path) -> Result<Self, String> {
        Self::decode_from_file_with_cues(path, None)
    }

    /// Stream-decode an audio/video file, optionally reusing precomputed Hot Cues (`< 80 ms` load).
    pub fn decode_from_file_with_cues(
        path: &Path,
        known_cues_sec: Option<[f64; 4]>,
    ) -> Result<Self, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());

        #[cfg(not(target_arch = "wasm32"))]
        if matches!(ext.as_deref(), Some("webm" | "mkv" | "flv")) {
            let (stereo, sr) = super::decoder_ffmpeg::decode_stereo_via_ffmpeg(path)?;
            return Ok(Self::build_from_stereo(stereo, sr, known_cues_sec));
        }

        let symphonia_res = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open audio file {}: {e}", path.display()))
            .and_then(|file| {
                let mss = MediaSourceStream::new(Box::new(file), Default::default());
                decode_media_source(mss, ext.as_deref(), known_cues_sec)
            });

        #[cfg(not(target_arch = "wasm32"))]
        if symphonia_res.is_err() {
            if let Ok((interleaved_stereo, sample_rate)) =
                super::decoder_ffmpeg::decode_stereo_via_ffmpeg(path)
            {
                return Ok(Self::build_from_stereo(
                    interleaved_stereo,
                    sample_rate,
                    known_cues_sec,
                ));
            }
        }

        symphonia_res
    }

    fn build_from_stereo(
        interleaved_stereo: Vec<f32>,
        sample_rate: u32,
        known_cues_sec: Option<[f64; 4]>,
    ) -> Self {
        let total_frames = (interleaved_stereo.len() / 2) as u64;
        let hot_cue_frames = match known_cues_sec {
            Some(secs) => secs.map(|s| (s.max(0.0) * f64::from(sample_rate)).round() as u64),
            None => compute_hot_cue_frames(&interleaved_stereo, sample_rate, total_frames),
        };
        Self::from_stereo_frames(interleaved_stereo, sample_rate, hot_cue_frames)
            .expect("build_from_stereo: non-empty stereo")
    }

    /// Total number of stereo `(L, R)` sample frames in this buffer.
    #[inline]
    pub fn total_frames(&self) -> u64 {
        (self.interleaved_stereo.len() / 2) as u64
    }

    /// Exact duration of the buffer in seconds.
    #[inline]
    pub fn duration_seconds(&self) -> f64 {
        (self.total_frames() as f64) / f64::from(self.sample_rate.max(1))
    }

    /// Immutable slice of interleaved `(L, R)` stereo samples (`len == total_frames * 2`).
    #[inline]
    pub fn interleaved_stereo(&self) -> &[f32] {
        &self.interleaved_stereo
    }
}

/// Convenience loader matching the spec alias for [`PreallocatedDeckBuffer::decode_from_bytes`].
pub fn load_deck_buffer_from_bytes(
    bytes: &[u8],
    ext_hint: Option<&str>,
) -> Result<PreallocatedDeckBuffer, String> {
    PreallocatedDeckBuffer::decode_from_bytes(bytes, ext_hint)
}

/// Convenience loader matching the spec alias for [`PreallocatedDeckBuffer::decode_from_file`].
pub fn load_deck_buffer_from_file(path: &Path) -> Result<PreallocatedDeckBuffer, String> {
    PreallocatedDeckBuffer::decode_from_file(path)
}

fn decode_media_source(
    mss: MediaSourceStream,
    ext_hint: Option<&str>,
    known_cues_sec: Option<[f64; 4]>,
) -> Result<PreallocatedDeckBuffer, String> {
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
        .tracks()
        .iter()
        .find(|t| {
            t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL
                && (t.codec_params.sample_rate.is_some() || t.codec_params.channels.is_some())
        })
        .or_else(|| format.default_track())
        .ok_or_else(|| "No decodable audio track found in container".to_string())?;

    let track_id = track.id;
    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(44_100).max(1);
    let reserve_frames = track
        .codec_params
        .n_frames
        .unwrap_or(u64::from(sample_rate) * 30) as usize;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Codec initialization failed: {e}"))?;

    let mut interleaved_stereo = Vec::with_capacity(reserve_frames.saturating_mul(2));
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(_) => break,
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
                let samples = sb.samples();

                if channels == 2 {
                    interleaved_stereo.extend_from_slice(&samples[..frames * 2]);
                } else if channels == 1 {
                    for &s in &samples[..frames] {
                        interleaved_stereo.push(s);
                        interleaved_stereo.push(s);
                    }
                } else {
                    for frame in samples[..frames * channels].chunks_exact(channels) {
                        interleaved_stereo.push(frame[0]);
                        interleaved_stereo.push(frame[1]);
                    }
                }
            }
            Err(SymphoniaError::ResetRequired) => decoder.reset(),
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }

    if interleaved_stereo.is_empty() {
        return Err("Decoded audio stream contains zero samples".to_string());
    }

    Ok(PreallocatedDeckBuffer::build_from_stereo(
        interleaved_stereo,
        sample_rate,
        known_cues_sec,
    ))
}

fn compute_hot_cue_frames(
    interleaved_stereo: &[f32],
    sample_rate: u32,
    total_frames: u64,
) -> [u64; 4] {
    let mono: Vec<f32> = (0..(total_frames as usize))
        .map(|f| 0.5 * (interleaved_stereo[f * 2] + interleaved_stereo[f * 2 + 1]))
        .collect();
    let full_dur_sec = (total_frames as f32) / (sample_rate as f32);
    let profile = analyze_structure_and_style(&mono, sample_rate, Some(full_dur_sec), None);

    let sr_f64 = f64::from(sample_rate);
    let to_frame = |sec: f32| -> u64 { (f64::from(sec.max(0.0)) * sr_f64).round() as u64 };
    let raw = [
        to_frame(profile.first_beat_seconds),
        to_frame(profile.phrase_in_seconds),
        to_frame(profile.main_drop_seconds),
        to_frame(profile.mix_out_seconds),
    ];

    let last_frame = total_frames.saturating_sub(1);
    if total_frames < 4 {
        return [
            raw[0].min(last_frame),
            raw[1].min(last_frame),
            raw[2].min(last_frame),
            raw[3].min(last_frame),
        ];
    }

    let c1 = raw[0].min(last_frame.saturating_sub(3));
    let rem_after_c1 = last_frame.saturating_sub(c1);
    let c2 = if raw[1] > c1 && raw[1] <= last_frame.saturating_sub(2) {
        raw[1]
    } else {
        (c1 + (rem_after_c1 / 4).max(1)).min(last_frame.saturating_sub(2))
    };
    let c3 = if raw[2] > c2 && raw[2] <= last_frame.saturating_sub(1) {
        raw[2]
    } else {
        (c2 + ((last_frame - c2) / 2).max(1)).min(last_frame.saturating_sub(1))
    };
    let c4 = if raw[3] > c3 && raw[3] <= last_frame {
        raw[3]
    } else {
        (c3 + ((last_frame - c3) * 3 / 4).max(1)).min(last_frame)
    };

    [c1, c2, c3, c4]
}
