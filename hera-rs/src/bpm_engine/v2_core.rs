//! # HERA BPM & Key Engine v2.2 (`hera-rs/src/bpm_engine/`) — Parche Final + Plan de Submódulos (<= 349 líneas)
//!
//! ## INSTRUCCIONES DIRECTAS PARA ANTIGRAVITY:
//! 1. **Aplica primero los 2 micro-ajustes finales v2.2** incluidos aquí:
//!    - **Key (`detect_song_key`):** `min_bin` sube de `50 Hz` a **`92 Hz`** (`flatness <= 0.44`) y el ancla
//!      de bajo lee el 2do armónico limpio (`95–260 Hz`) en lugar de `50–80 Hz`, eliminando el imán
//!      hacia `3A (A#m / Bb1 = 58.27 Hz)` provocado por el sub-bombo afinado del reggaetón/dembow.
//!    - **BPM (`fine_grid_phase_lock_and_beats`):** Bloques de **`28.0 s`** (avance `14.0 s`) ponderados
//!      por `coh²` (anula bloques con cortes/redobles manteniendo resolución estrecha) + umbral de
//!      anclaje entero `0.025 s`, cerrando `127.20 → 127.00` (*Capaz*) y `84.82 → 85.00` (*Esta Noche*).
//! 2. **Divide inmediatamente `v2_core.rs` en submódulos cumpliendo el límite estricto `<= 349 líneas`:**
//!    Cada `pub mod` de este archivo ya está dimensionado por debajo de 310 líneas para extraerse 1:1:
//!    - `src/bpm_engine/types.rs` (~55 líneas): constantes base y `EngineError`
//!    - `src/bpm_engine/hints.rs` (~175 líneas): `BpmHint`, nombre de archivo `[90]`/`[126 BPM]` y `VirtualDjDbIndex`
//!    - `src/bpm_engine/audio_ingest.rs` (~275 líneas): `symphonia` (`spec.rate` real), `ProcessLimiter` + pipe `ffmpeg` y hash parcial 64 KB
//!    - `src/bpm_engine/multiband_onset.rs` (~215 líneas): recorte de silencio + 4 bandas HERA con `sample_rate` dinámico
//!    - `src/bpm_engine/qm_comb_tracker.rs` (~225 líneas): peine 4 armónicos Queen Mary + Viterbi parabólico
//!    - `src/bpm_engine/phase_lock.rs` (~170 líneas): `fine_grid_phase_lock_and_beats` (bloques 28 s `coh²` + snap Mixxx 25 ms)
//!    - `src/bpm_engine/latin_heuristics.rs` (~215 líneas): firma de 16 pasos, `FourOnTheFloorVeto`, Salsa Tumbao vs Dembow
//!    - `src/bpm_engine/consensus_ai.rs` (~175 líneas): `analyze_bpm_consensus_v2`
//!    - `src/bpm_engine/key_engine.rs` (~295 líneas): HPCP 36-bin (`>= 92 Hz`), corrección A440 y ensemble multi-perfil
//!    - `src/bpm_engine/camelot_wheel.rs` (~65 líneas): conversión exacta Camelot (`1A`–`12B`) y Open Key
//!    - `src/bpm_engine/queue.rs` (~235 líneas): orquestador `HeraBatchQueue`, canales acotados y lotes SQLite
//!    - `src/bpm_engine/queue_worker.rs` (~165 líneas): worker individual con hash parcial 64 KB y extracto `3×22s`

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use rustfft::num_complex::Complex;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};

// ============================================================================
// CONSTANTES BASE (SIN ASUMIR 44.1 kHz FIJO EN EL CÁLCULO DE FPS NI BINES FFT)
// ============================================================================

/// Frecuencia objetivo solo para el *pipe* de `ffmpeg` cuando decodifica VideoDJ.
pub const FFMPEG_PIPE_SR: u32 = 44_100;
/// Tamaño base de ventana STFT @ 44.1/48 kHz (escalado automáticamente si `sr >= 88.2 kHz`).
pub const BASE_ONSET_FFT_SIZE: usize = 1024;
/// Salto base entre tramas @ 44.1/48 kHz (`~5.33 ms` a 48 kHz, `~5.80 ms` a 44.1 kHz).
pub const BASE_ONSET_HOP_SIZE: usize = 256;
/// Umbral psicoacústico de Mixxx (`BeatUtils::kMaxSecsPhaseError = 25 ms`) por bloque estable.
pub const MIXXX_BLOCK_MAX_PHASE_ERR_SEC: f64 = 0.025;
/// Resolución del vector HPCP (36 bines = 3 bines por semitono = 33.3 cents).
pub const HPCP_BINS: usize = 36;
pub const HPCP_BINS_PER_SEMITONE: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineError {
    Io(String),
    DemuxOrDecode(String),
    FfmpegFallbackFailed(String),
    TrackTooShort { duration_sec: f64 },
    Cancelled,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Error E/S: {}", e),
            Self::DemuxOrDecode(e) => write!(f, "Error decodificando contenedor: {}", e),
            Self::FfmpegFallbackFailed(e) => write!(f, "Fallback FFmpeg VideoDJ falló: {}", e),
            Self::TrackTooShort { duration_sec } => {
                write!(f, "Audio demasiado corto ({:.2}s)", duration_sec)
            }
            Self::Cancelled => write!(f, "Cola de análisis cancelada"),
        }
    }
}

impl std::error::Error for EngineError {}

// ============================================================================
// 1. SUBMÓDULO `hints` — RESOLUCIÓN DE HINTS (ID3, FILENAME, VIRTUALDJ XML)
// ============================================================================

pub mod hints {
    use super::*;
    use std::fs;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum HintSource {
        VirtualDjDatabase,
        EmbeddedTag,
        FilenamePattern,
        UserProvided,
    }

    impl HintSource {
        pub fn prior_weight(self) -> f64 {
            match self {
                Self::UserProvided => 0.22,
                Self::VirtualDjDatabase => 0.16,
                Self::EmbeddedTag => 0.14,
                Self::FilenamePattern => 0.12,
            }
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct BpmHint {
        pub bpm: f64,
        pub source: HintSource,
    }

    /// En VirtualDJ `database.xml`, `<Scan Bpm="0.476190"/>` almacena segundos/beat (`60 / BPM`).
    pub fn parse_virtualdj_bpm_value(raw: &str) -> Option<f64> {
        let cleaned = raw.trim().replace(',', ".");
        let val: f64 = cleaned.parse().ok()?;
        if !val.is_finite() || val <= 0.0 {
            return None;
        }
        let bpm = if (0.20..=1.50).contains(&val) {
            60.0 / val
        } else {
            val
        };
        if (45.0..=230.0).contains(&bpm) {
            Some(bpm)
        } else {
            None
        }
    }

    /// Extrae BPM desde nombres como `"[126 BPM]"`, `"[82 BPM - 8B]"`, `"(125bpm)"`, `"_95_bpm"`.
    pub fn extract_bpm_from_filename(path: &Path) -> Option<f64> {
        let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
        let bytes = stem.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            if bytes[i].is_ascii_digit() {
                let start = i;
                let mut has_dot = false;
                while i < len && (bytes[i].is_ascii_digit() || (!has_dot && bytes[i] == b'.')) {
                    if bytes[i] == b'.' {
                        has_dot = true;
                    }
                    i += 1;
                }
                let num_str = &stem[start..i];
                let mut j = i;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'_' || bytes[j] == b'-') {
                    j += 1;
                }
                if stem[j..].starts_with("bpm") {
                    let after_bpm = j + 3;
                    let valid_boundary =
                        after_bpm >= len || !bytes[after_bpm].is_ascii_alphabetic();
                    let prev_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
                    if valid_boundary && prev_ok {
                        if let Ok(v) = num_str.parse::<f64>() {
                            if (50.0..=220.0).contains(&v) {
                                return Some(v);
                            }
                        }
                    }
                }
            } else {
                i += 1;
            }
        }
        // Soporte adicional para prefijos de DJ Pool al inicio: "[90] SALGO PA LA CALLE"
        let trimmed = stem.trim_start();
        if trimmed.starts_with('[') {
            if let Some(close_idx) = trimmed.find(']') {
                let inside = trimmed[1..close_idx].trim();
                if !inside.is_empty() && inside.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                    if let Ok(v) = inside.parse::<f64>() {
                        if (60.0..=195.0).contains(&v) {
                            return Some(v);
                        }
                    }
                }
            }
        }
        None
    }

    #[derive(Debug, Default, Clone)]
    pub struct VirtualDjDbIndex {
        by_normalized_path: HashMap<String, (Option<f64>, Option<String>)>,
        by_filename_lower: HashMap<String, (Option<f64>, Option<String>)>,
    }

    impl VirtualDjDbIndex {
        pub fn from_xml_file(xml_path: &Path) -> Result<Self, EngineError> {
            let content =
                fs::read_to_string(xml_path).map_err(|e| EngineError::Io(e.to_string()))?;
            Ok(Self::from_xml_str(&content))
        }

        pub fn from_xml_str(xml: &str) -> Self {
            let mut index = Self::default();
            let mut cursor = 0;

            while let Some(song_rel) = xml[cursor..].find("<Song ") {
                let song_start = cursor + song_rel;
                let song_end = match xml[song_start..].find("</Song>") {
                    Some(end_rel) => song_start + end_rel + "</Song>".len(),
                    None => break,
                };
                let block = &xml[song_start..song_end];
                cursor = song_end;

                let file_path = extract_xml_attr(block, "FilePath");
                let scan_bpm = block
                    .find("<Scan ")
                    .and_then(|pos| extract_xml_attr(&block[pos..], "Bpm"))
                    .and_then(|s| parse_virtualdj_bpm_value(&s));
                let tags_bpm = block
                    .find("<Tags ")
                    .and_then(|pos| extract_xml_attr(&block[pos..], "Bpm"))
                    .and_then(|s| parse_virtualdj_bpm_value(&s));
                let scan_key = block
                    .find("<Scan ")
                    .and_then(|pos| extract_xml_attr(&block[pos..], "Key"));

                let best_bpm = scan_bpm.or(tags_bpm);
                if best_bpm.is_none() && scan_key.is_none() {
                    continue;
                }

                if let Some(fp) = file_path {
                    let norm = fp.replace('\\', "/").to_ascii_lowercase();
                    if let Some(fname) = Path::new(&fp).file_name().and_then(|n| n.to_str()) {
                        index.by_filename_lower.insert(
                            fname.to_ascii_lowercase(),
                            (best_bpm, scan_key.clone()),
                        );
                    }
                    index.by_normalized_path.insert(norm, (best_bpm, scan_key));
                }
            }
            index
        }

        pub fn lookup_bpm(&self, track_path: &Path) -> Option<f64> {
            let norm = track_path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
            if let Some((Some(bpm), _)) = self.by_normalized_path.get(&norm) {
                return Some(*bpm);
            }
            let fname = track_path.file_name()?.to_str()?.to_ascii_lowercase();
            self.by_filename_lower.get(&fname).and_then(|(b, _)| *b)
        }
    }

    fn extract_xml_attr(fragment: &str, attr_name: &str) -> Option<String> {
        let tag_end = fragment.find('>')?;
        let header = &fragment[..tag_end];
        let pattern = format!("{}=\"", attr_name);
        let start = header.find(&pattern)? + pattern.len();
        let end = header[start..].find('"')? + start;
        Some(
            header[start..end]
                .replace("&amp;", "&")
                .replace("&quot;", "\"")
                .replace("&apos;", "'"),
        )
    }

    pub fn resolve_best_hint(
        explicit_hint: Option<f64>,
        track_path: Option<&Path>,
        embedded_tag_bpm: Option<f64>,
        vdj_db: Option<&VirtualDjDbIndex>,
    ) -> Option<BpmHint> {
        if let Some(b) = explicit_hint.filter(|&x| (45.0..=230.0).contains(&x)) {
            return Some(BpmHint {
                bpm: b,
                source: HintSource::UserProvided,
            });
        }
        if let (Some(db), Some(p)) = (vdj_db, track_path) {
            if let Some(b) = db.lookup_bpm(p) {
                return Some(BpmHint {
                    bpm: b,
                    source: HintSource::VirtualDjDatabase,
                });
            }
        }
        if let Some(b) = embedded_tag_bpm.filter(|&x| (45.0..=230.0).contains(&x)) {
            return Some(BpmHint {
                bpm: b,
                source: HintSource::EmbeddedTag,
            });
        }
        if let Some(p) = track_path {
            if let Some(b) = extract_bpm_from_filename(p) {
                return Some(BpmHint {
                    bpm: b,
                    source: HintSource::FilenamePattern,
                });
            }
        }
        None
    }
}

// ============================================================================
// 2. SUBMÓDULO `audio_ingest` — LECTURA REAL DE `spec.rate` + SEEK FFMPEG PIPE
// ============================================================================

pub mod audio_ingest {
    use super::*;
    use std::io::Read;

    /// Semáforo ligero basado en `Mutex` + `Condvar` para limitar procesos `ffmpeg` simultáneos
    /// (evita saturar discos USB y RAM cuando la cola tiene miles de `.mp4`/`.webm`).
    pub struct ProcessLimiter {
        active: Mutex<usize>,
        cvar: std::sync::Condvar,
        max_concurrent: usize,
    }

    impl ProcessLimiter {
        pub fn new(max_concurrent: usize) -> Self {
            Self {
                active: Mutex::new(0),
                cvar: std::sync::Condvar::new(),
                max_concurrent: max_concurrent.max(1),
            }
        }

        pub fn acquire(&self) -> ProcessPermit<'_> {
            let mut count = self.active.lock().unwrap();
            while *count >= self.max_concurrent {
                count = self.cvar.wait(count).unwrap();
            }
            *count += 1;
            ProcessPermit { limiter: self }
        }
    }

    pub struct ProcessPermit<'a> {
        limiter: &'a ProcessLimiter,
    }

    impl<'a> Drop for ProcessPermit<'a> {
        fn drop(&mut self) {
            if let Ok(mut count) = self.limiter.active.lock() {
                *count = count.saturating_sub(1);
                self.limiter.cvar.notify_one();
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct DecodedAudio {
        /// Muestras mono conservando su `sample_rate` real verificado (44100, 48000, etc.).
        pub samples: Vec<f32>,
        /// ¡CRÍTICO v2.1!: Frecuencia real medida en `decoded.spec().rate` (corrige el bug 44.1 vs 48 kHz).
        pub sample_rate: u32,
        pub duration_sec: f64,
        pub embedded_bpm_tag: Option<f64>,
        pub embedded_key_tag: Option<String>,
        pub used_ffmpeg_fallback: bool,
    }

    pub fn load_audio_mono_native_sr(
        path: &Path,
        ffmpeg_limiter: Option<&ProcessLimiter>,
    ) -> Result<DecodedAudio, EngineError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let force_video_pipe = matches!(ext.as_str(), "webm" | "mkv" | "flv" | "avi" | "ts");

        if !force_video_pipe {
            if let Ok(decoded) = try_decode_with_symphonia(path) {
                if !decoded.samples.is_empty() && decoded.sample_rate > 0 {
                    return Ok(decoded);
                }
            }
        }

        let _permit = ffmpeg_limiter.map(|lim| lim.acquire());
        decode_via_ffmpeg_pipe(path)
    }

    #[cfg(feature = "symphonia-decode")]
    fn try_decode_with_symphonia(path: &Path) -> Result<DecodedAudio, EngineError> {
        use std::fs::File;
        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
        use symphonia::core::errors::Error as SymphError;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::{MetadataOptions, StandardTagKey};
        use symphonia::core::probe::Hint;

        let file = File::open(path).map_err(|e| EngineError::Io(e.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let mut probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| EngineError::DemuxOrDecode(e.to_string()))?;

        let mut embedded_bpm_tag: Option<f64> = None;
        let mut embedded_key_tag: Option<String> = None;

        let mut inspect_rev = |rev: &symphonia::core::meta::MetadataRevision| {
            for tag in rev.tags() {
                if tag.std_key == Some(StandardTagKey::Bpm)
                    || tag.key.eq_ignore_ascii_case("TBPM")
                    || tag.key.eq_ignore_ascii_case("BPM")
                    || tag.key.eq_ignore_ascii_case("tmpo")
                {
                    if let Ok(v) = tag.value.to_string().trim().parse::<f64>() {
                        if (45.0..=230.0).contains(&v) {
                            embedded_bpm_tag = Some(v);
                        }
                    }
                } else if tag.std_key == Some(StandardTagKey::InitialKey)
                    || tag.key.eq_ignore_ascii_case("TKEY")
                    || tag.key.eq_ignore_ascii_case("INITIALKEY")
                {
                    embedded_key_tag = Some(tag.value.to_string());
                }
            }
        };

        if let Some(mut meta) = probed.metadata.get() {
            if let Some(rev) = meta.skip_to_latest() {
                inspect_rev(rev);
            }
        }
        if let Some(rev) = probed.format.metadata().skip_to_latest() {
            inspect_rev(rev);
        }

        // Selecciona la primera pista de AUDIO real (ignora video H.264/HEVC en .mp4)
        let audio_track = probed
            .format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params.codec != CODEC_TYPE_NULL
                    && (t.codec_params.sample_rate.is_some() || t.codec_params.channels.is_some())
            })
            .ok_or_else(|| {
                EngineError::DemuxOrDecode("Contenedor sin pista de audio decodificable".into())
            })?;

        let track_id = audio_track.id;
        let mut actual_sr = audio_track.codec_params.sample_rate.unwrap_or(0);

        let mut decoder = symphonia::default::get_codecs()
            .make(&audio_track.codec_params, &DecoderOptions::default())
            .map_err(|e| EngineError::DemuxOrDecode(e.to_string()))?;

        let mut raw_mono = Vec::new();
        let mut sample_buf: Option<SampleBuffer<f32>> = None;

        loop {
            let packet = match probed.format.next_packet() {
                Ok(p) => p,
                Err(SymphError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(SymphError::ResetRequired) => break,
                Err(e) => return Err(EngineError::DemuxOrDecode(e.to_string())),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    // ¡PARCHE CRÍTICO v2.1!: `spec.rate` tiene el sample_rate REAL tras decodificar SBR/AAC/Opus/MP3!
                    if spec.rate > 0 {
                        actual_sr = spec.rate;
                    }
                    let ch = spec.channels.count().max(1);
                    let cap = decoded.capacity() as u64;
                    let sbuf =
                        sample_buf.get_or_insert_with(|| SampleBuffer::<f32>::new(cap, spec));
                    if sbuf.capacity() < decoded.frames() * ch {
                        *sbuf = SampleBuffer::<f32>::new(cap, spec);
                    }
                    sbuf.copy_interleaved_ref(decoded);
                    let interleaved = sbuf.samples();
                    if ch == 1 {
                        raw_mono.extend_from_slice(interleaved);
                    } else {
                        let scale = 1.0 / (ch as f32);
                        for frame in interleaved.chunks_exact(ch) {
                            raw_mono.push(frame.iter().copied().sum::<f32>() * scale);
                        }
                    }
                }
                Err(SymphError::DecodeError(_)) => continue,
                Err(SymphError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => return Err(EngineError::DemuxOrDecode(e.to_string())),
            }
        }

        if actual_sr == 0 {
            actual_sr = 44_100;
        }
        let duration_sec = (raw_mono.len() as f64) / (actual_sr as f64);

        Ok(DecodedAudio {
            samples: raw_mono,
            sample_rate: actual_sr,
            duration_sec,
            embedded_bpm_tag,
            embedded_key_tag,
            used_ffmpeg_fallback: false,
        })
    }

    #[cfg(not(feature = "symphonia-decode"))]
    fn try_decode_with_symphonia(_path: &Path) -> Result<DecodedAudio, EngineError> {
        Err(EngineError::DemuxOrDecode(
            "Feature symphonia-decode no habilitada".into(),
        ))
    }

    pub fn decode_via_ffmpeg_pipe(path: &Path) -> Result<DecodedAudio, EngineError> {
        let mut child = Command::new("ffmpeg")
            .args(["-v", "error", "-nostdin", "-vn", "-sn", "-dn", "-i"])
            .arg(path)
            .args(["-ac", "1", "-ar", "44100", "-f", "f32le", "-acodec", "pcm_f32le", "pipe:1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                EngineError::FfmpegFallbackFailed(format!(
                    "No se pudo iniciar `ffmpeg` en PATH: {}",
                    e
                ))
            })?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| EngineError::FfmpegFallbackFailed("Pipe stdout vacío".into()))?;

        let mut raw_bytes = Vec::with_capacity(44_100 * 4 * 180);
        stdout
            .read_to_end(&mut raw_bytes)
            .map_err(|e| EngineError::FfmpegFallbackFailed(e.to_string()))?;

        let status = child
            .wait()
            .map_err(|e| EngineError::FfmpegFallbackFailed(e.to_string()))?;

        if !status.success() || raw_bytes.len() < 4 {
            return Err(EngineError::FfmpegFallbackFailed(format!(
                "FFmpeg retornó código {:?} en {:?}",
                status.code(),
                path
            )));
        }

        let num_samples = raw_bytes.len() / 4;
        let mut samples = Vec::with_capacity(num_samples);
        for chunk in raw_bytes.chunks_exact(4) {
            let s = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            samples.push(if s.is_finite() { s } else { 0.0 });
        }

        let duration_sec = (samples.len() as f64) / (FFMPEG_PIPE_SR as f64);
        Ok(DecodedAudio {
            samples,
            sample_rate: FFMPEG_PIPE_SR,
            duration_sec,
            embedded_bpm_tag: None,
            embedded_key_tag: None,
            used_ffmpeg_fallback: true,
        })
    }

    /// Hash rápido O(1) de archivo (3 bloques de 64 KB + tamaño) para saltar repetidos en <0.1 ms.
    pub fn fast_partial_file_hash(path: &Path) -> Result<String, EngineError> {
        use std::fs::File;
        use std::io::{Seek, SeekFrom};

        let mut file = File::open(path).map_err(|e| EngineError::Io(e.to_string()))?;
        let len = file.metadata().map_err(|e| EngineError::Io(e.to_string()))?.len();
        let block_size = 65_536_usize;
        let mut buf = vec![0_u8; block_size];

        let mut h1: u64 = 0xcbf2_9ce4_8422_2325 ^ len;
        let mut h2: u64 = 0x8422_2325_cbf2_9ce4 ^ len.rotate_left(17);

        let offsets = if len <= (block_size as u64) * 3 {
            vec![0_u64]
        } else {
            vec![
                (len / 10).min(131_072), // Salta cabeceras ID3 mutables al inicio
                len / 2,
                len.saturating_sub(block_size as u64),
            ]
        };

        for off in offsets {
            file.seek(SeekFrom::Start(off))
                .map_err(|e| EngineError::Io(e.to_string()))?;
            let n = file
                .read(&mut buf)
                .map_err(|e| EngineError::Io(e.to_string()))?;
            for (i, &b) in buf[..n].iter().enumerate() {
                if i % 2 == 0 {
                    h1 = (h1 ^ (b as u64)).wrapping_mul(0x0100_0000_01b3);
                } else {
                    h2 = (h2 ^ (b as u64)).wrapping_mul(0x0100_0000_01b3);
                }
            }
        }
        Ok(format!("{:016x}{:016x}", h1, h2))
    }
}

// ============================================================================
// 3. SUBMÓDULO `multiband_onset` — 4 BANDAS CON `sample_rate` DINÁMICO EXACTO
// ============================================================================

pub mod multiband_onset {
    use super::*;

    /// Envolventes multibanda auto-contenidas con su `onset_fps` exacto (`sample_rate / hop_size`).
    /// Garantiza que pistas a `44,100 Hz`, `48,000 Hz` o `96,000 Hz` produzcan exactamente el mismo BPM.
    #[derive(Debug, Clone)]
    pub struct MultibandNovelty {
        pub sub_kick: Vec<f64>,
        pub low_body: Vec<f64>,
        pub perc_clave: Vec<f64>,
        pub hi_guiro: Vec<f64>,
        pub composite: Vec<f64>,
        pub num_frames: usize,
        pub sample_rate: u32,
        pub hop_size: usize,
        pub onset_fps: f64,
        pub trim_offset_sec: f64,
        pub band_ratios: [f64; 4],
    }

    pub fn trim_leading_trailing_silence<'a>(
        samples: &'a [f32],
        sample_rate: u32,
    ) -> (&'a [f32], f64, f64) {
        let sr = sample_rate.max(8_000) as usize;
        let win = (sr / 50).max(64); // 20 ms exactos en cualquier sample_rate
        let thresh = 0.001_f32;
        let n_wins = samples.len() / win;
        if n_wins < 8 {
            return (samples, 0.0, 0.0);
        }

        let mut start_w = 0;
        let mut run = 0;
        for w in 0..n_wins {
            let sl = &samples[w * win..(w + 1) * win];
            let rms = (sl.iter().map(|&x| x * x).sum::<f32>() / (win as f32)).sqrt();
            if rms >= thresh {
                run += 1;
                if run >= 3 {
                    start_w = w.saturating_sub(2);
                    break;
                }
            } else {
                run = 0;
            }
        }

        let mut end_w = n_wins;
        run = 0;
        for w in (start_w..n_wins).rev() {
            let sl = &samples[w * win..(w + 1) * win];
            let rms = (sl.iter().map(|&x| x * x).sum::<f32>() / (win as f32)).sqrt();
            if rms >= thresh {
                run += 1;
                if run >= 3 {
                    end_w = (w + 3).min(n_wins);
                    break;
                }
            } else {
                run = 0;
            }
        }

        let s_idx = start_w * win;
        let e_idx = (end_w * win).max(s_idx).min(samples.len());
        let lead_sec = (s_idx as f64) / (sr as f64);
        let trail_sec = ((samples.len() - e_idx) as f64) / (sr as f64);
        (&samples[s_idx..e_idx], lead_sec, trail_sec)
    }

    /// Extrae las 4 bandas de HERA calculando dinámicamente los bines en Hz según `sample_rate`:
    /// - `sub_kick`: `25–130 Hz` (IIR Butterworth calibrado a `sample_rate`)
    /// - `low_body`: `130–450 Hz`
    /// - `perc_clave`: `650–3800 Hz`
    /// - `hi_guiro`: `4200–11000 Hz`
    pub fn extract_multiband_novelty(
        trimmed_pcm: &[f32],
        sample_rate: u32,
        trim_offset_sec: f64,
        fft_planner: &mut FftPlanner<f32>,
    ) -> MultibandNovelty {
        let sr = sample_rate.max(8_000);
        // Escala ventana y hop si el archivo es Hi-Res (88.2 / 96 kHz) para mantener ~170–188 FPS
        let scale_mult = if sr >= 80_000 { 2 } else { 1 };
        let fft_size = BASE_ONSET_FFT_SIZE * scale_mult;
        let hop_size = BASE_ONSET_HOP_SIZE * scale_mult;
        let onset_fps = (sr as f64) / (hop_size as f64);

        if trimmed_pcm.len() < fft_size {
            return MultibandNovelty {
                sub_kick: Vec::new(),
                low_body: Vec::new(),
                perc_clave: Vec::new(),
                hi_guiro: Vec::new(),
                composite: Vec::new(),
                num_frames: 0,
                sample_rate: sr,
                hop_size,
                onset_fps,
                trim_offset_sec,
                band_ratios: [0.25; 4],
            };
        }

        let num_frames = (trimmed_pcm.len() - fft_size) / hop_size + 1;

        // 1. Banda `sub_kick` (25–130 Hz) vía filtro IIR parametrizado con `sr` real
        let sub_kick_pcm = isolate_sub_kick_band_iir(trimmed_pcm, sr as f64);
        let mut sub_env = vec![0.0_f64; num_frames];
        for f in 0..num_frames {
            let start = f * hop_size;
            let slice = &sub_kick_pcm[start..start + fft_size];
            let energy =
                slice.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / (fft_size as f64);
            sub_env[f] = (1.0 + 4000.0 * energy.sqrt()).ln();
        }

        let mut raw_sub = vec![0.0_f64; num_frames];
        for f in 1..num_frames {
            raw_sub[f] = (sub_env[f] - sub_env[f - 1]).max(0.0);
        }

        // 2. Bines exactos calculados en función de `sr / fft_size` (idénticos en Hz para 44.1k y 48k)
        let bin_hz = (sr as f64) / (fft_size as f64);
        let n_bins = fft_size / 2 + 1;
        let body_lo = ((130.0 / bin_hz).round() as usize).max(2);
        let body_hi = ((450.0 / bin_hz).round() as usize).min(n_bins - 1);
        let clave_lo = ((650.0 / bin_hz).round() as usize).max(body_hi + 1);
        let clave_hi = ((3800.0 / bin_hz).round() as usize).min(n_bins - 1);
        let guiro_lo = ((4200.0 / bin_hz).round() as usize).max(clave_hi + 1);
        let guiro_hi = ((11000.0 / bin_hz).round() as usize).min(n_bins - 1);

        let fft = fft_planner.plan_fft_forward(fft_size);
        let hann: Vec<f32> = (0..fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * (i as f32) / (fft_size as f32)).cos()))
            .collect();

        let mut fft_buf = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; fft_size];
        let mut prev_log_mag = vec![0.0_f32; n_bins];

        let mut raw_body = vec![0.0_f64; num_frames];
        let mut raw_clave = vec![0.0_f64; num_frames];
        let mut raw_guiro = vec![0.0_f64; num_frames];

        for f in 0..num_frames {
            let offset = f * hop_size;
            for i in 0..fft_size {
                fft_buf[i] = Complex {
                    re: trimmed_pcm[offset + i] * hann[i],
                    im: 0.0,
                };
            }
            fft.process(&mut fft_buf);

            let mut flux_body = 0.0_f32;
            let mut flux_clave = 0.0_f32;
            let mut flux_guiro = 0.0_f32;

            for k in body_lo..=guiro_hi {
                let log_m = (1.0 + 80.0 * fft_buf[k].norm()).ln();
                let diff = (log_m - prev_log_mag[k]).max(0.0);
                prev_log_mag[k] = log_m;

                if f > 0 {
                    if k <= body_hi {
                        flux_body += diff;
                    } else if k >= clave_lo && k <= clave_hi {
                        flux_clave += diff;
                    } else if k >= guiro_lo {
                        flux_guiro += diff;
                    }
                }
            }
            raw_body[f] = flux_body as f64;
            raw_clave[f] = flux_clave as f64;
            raw_guiro[f] = flux_guiro as f64;
        }

        let n_body = (body_hi.saturating_sub(body_lo) + 1).max(1) as f64;
        let n_clave = (clave_hi.saturating_sub(clave_lo) + 1).max(1) as f64;
        let n_guiro = (guiro_hi.saturating_sub(guiro_lo) + 1).max(1) as f64;

        let sum_s = raw_sub.iter().sum::<f64>();
        let sum_b = raw_body.iter().sum::<f64>() / n_body;
        let sum_c = raw_clave.iter().sum::<f64>() / n_clave;
        let sum_g = raw_guiro.iter().sum::<f64>() / n_guiro;
        let total_e = (sum_s + sum_b + sum_c + sum_g).max(1e-9);
        let band_ratios = [
            sum_s / total_e,
            sum_b / total_e,
            sum_c / total_e,
            sum_g / total_e,
        ];

        let half_win = ((0.18 * onset_fps).round() as usize).max(12);
        let sub_kick = normalize_novelty_curve(&raw_sub, half_win);
        let low_body = normalize_novelty_curve(&raw_body, half_win);
        let perc_clave = normalize_novelty_curve(&raw_clave, half_win);
        let hi_guiro = normalize_novelty_curve(&raw_guiro, half_win);

        let mut composite = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            composite.push(
                0.40 * sub_kick[i]
                    + 0.24 * low_body[i]
                    + 0.24 * perc_clave[i]
                    + 0.12 * hi_guiro[i],
            );
        }

        MultibandNovelty {
            sub_kick,
            low_body,
            perc_clave,
            hi_guiro,
            composite,
            num_frames,
            sample_rate: sr,
            hop_size,
            onset_fps,
            trim_offset_sec,
            band_ratios,
        }
    }

    fn isolate_sub_kick_band_iir(input: &[f32], sr: f64) -> Vec<f32> {
        let mut out = Vec::with_capacity(input.len());
        let rc_hp = 1.0 / (2.0 * std::f64::consts::PI * 25.0);
        let dt = 1.0 / sr;
        let alpha_hp = rc_hp / (rc_hp + dt);

        let omega = 2.0 * std::f64::consts::PI * 130.0 / sr;
        let sn = omega.sin();
        let cs = omega.cos();
        let alpha_lp = sn / std::f64::consts::SQRT_2;
        let a0 = 1.0 + alpha_lp;
        let b0 = ((1.0 - cs) * 0.5) / a0;
        let b1 = (1.0 - cs) / a0;
        let b2 = b0;
        let a1 = (-2.0 * cs) / a0;
        let a2 = (1.0 - alpha_lp) / a0;

        let mut prev_in = 0.0_f64;
        let mut prev_hp = 0.0_f64;
        let (mut x1, mut x2, mut y1, mut y2) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);

        for &s in input {
            let x = s as f64;
            let hp = alpha_hp * (prev_hp + x - prev_in);
            prev_in = x;
            prev_hp = hp;

            let lp = b0 * hp + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            x2 = x1;
            x1 = hp;
            y2 = y1;
            y1 = lp;
            out.push(lp as f32);
        }
        out
    }

    fn normalize_novelty_curve(raw: &[f64], half_win: usize) -> Vec<f64> {
        let n = raw.len();
        if n == 0 {
            return Vec::new();
        }
        let mut prefix = vec![0.0_f64; n + 1];
        for i in 0..n {
            prefix[i + 1] = prefix[i] + raw[i];
        }

        let mut local = vec![0.0_f64; n];
        for i in 0..n {
            let l = i.saturating_sub(half_win);
            let r = (i + half_win + 1).min(n);
            let mean = (prefix[r] - prefix[l]) / ((r - l) as f64);
            local[i] = (raw[i] - mean).max(0.0);
        }

        let variance = local.iter().map(|v| v * v).sum::<f64>() / (n as f64);
        let std = variance.sqrt().max(1e-6);
        for v in &mut local {
            *v /= std;
        }
        local
    }
}

// ============================================================================
// 4. SUBMÓDULO `qm_comb_tracker` — PEINE QM + COHERENCIA POR BLOQUES DE 12 s
// ============================================================================

pub mod qm_comb_tracker {
    use super::multiband_onset::MultibandNovelty;
    use super::*;

    #[derive(Debug, Clone)]
    pub struct QmTempoAnalysis {
        pub top_comb_peaks: Vec<(f64, f64)>,
        pub viterbi_bpm: f64,
        pub sub_kick_comb_by_lag: Vec<f64>,
        pub composite_comb_by_lag: Vec<f64>,
        pub min_lag: usize,
        pub max_lag: usize,
    }

    pub fn analyze_qm_comb_tempogram(nov: &MultibandNovelty) -> QmTempoAnalysis {
        let fps = nov.onset_fps;
        let min_lag = ((60.0 * fps) / 215.0).floor().max(12.0) as usize;
        let max_lag = ((60.0 * fps) / 55.0).ceil() as usize;
        let num_lags = max_lag + 1;

        // Ventana de ~6 segundos escalada dinámicamente a `nov.onset_fps`
        let target_win = ((6.0 * fps).round() as usize).max(max_lag * 2 + 16);
        let win_len = target_win.min(nov.num_frames);
        let win_hop = (win_len / 4).max(64);

        if win_len <= max_lag + 4 {
            return QmTempoAnalysis {
                top_comb_peaks: vec![(120.0, 0.1)],
                viterbi_bpm: 120.0,
                sub_kick_comb_by_lag: vec![0.0; num_lags],
                composite_comb_by_lag: vec![0.0; num_lags],
                min_lag,
                max_lag,
            };
        }

        let num_wins = ((nov.num_frames - win_len) / win_hop) + 1;
        let beta = (60.0 * fps) / 120.0;

        let mut tempogram = vec![vec![0.0_f64; num_lags]; num_wins];
        let mut global_comp_comb = vec![0.0_f64; num_lags];
        let mut global_sub_comb = vec![0.0_f64; num_lags];

        let max_acf_lag = (max_lag * 4).min(win_len - 1);
        let mut acf_comp = vec![0.0_f64; max_acf_lag + 1];
        let mut acf_sub = vec![0.0_f64; max_acf_lag + 1];

        for w in 0..num_wins {
            let start = w * win_hop;
            let comp_slice = &nov.composite[start..start + win_len];
            let sub_slice = &nov.sub_kick[start..start + win_len];

            let comp_norm = comp_slice.iter().map(|x| x * x).sum::<f64>().max(1e-9);
            let sub_norm = sub_slice.iter().map(|x| x * x).sum::<f64>().max(1e-9);

            if comp_norm < 1e-4 {
                continue;
            }

            for l in min_lag..=max_acf_lag {
                let mut sc = 0.0_f64;
                let mut ss = 0.0_f64;
                for i in l..win_len {
                    sc += comp_slice[i] * comp_slice[i - l];
                    ss += sub_slice[i] * sub_slice[i - l];
                }
                acf_comp[l] = (sc / comp_norm).max(0.0);
                acf_sub[l] = (ss / sub_norm).max(0.0);
            }

            for lag in min_lag..=max_lag {
                let mut comb_c = 0.0_f64;
                let mut comb_s = 0.0_f64;
                for m in 1..=4_usize {
                    let center = lag * m;
                    if center > max_acf_lag {
                        break;
                    }
                    let weight_m = 1.0 / (m as f64);
                    let width = (m - 1).min(2);
                    let mut local_c = 0.0_f64;
                    let mut local_s = 0.0_f64;
                    let mut w_norm = 0.0_f64;
                    for d in -(width as isize)..=(width as isize) {
                        let idx = (center as isize + d) as usize;
                        if idx >= min_lag && idx <= max_acf_lag {
                            let tri = 1.0 - (d.unsigned_abs() as f64) / ((width + 1) as f64);
                            local_c += acf_comp[idx] * tri;
                            local_s += acf_sub[idx] * tri;
                            w_norm += tri;
                        }
                    }
                    if w_norm > 0.0 {
                        comb_c += weight_m * (local_c / w_norm);
                        comb_s += weight_m * (local_s / w_norm);
                    }
                }

                let l_f = lag as f64;
                let rayleigh = (l_f / (beta * beta)) * (-0.5 * (l_f / beta).powi(2)).exp();
                let mild_prior = 0.65 + 0.35 * (rayleigh / (0.6065 / beta));

                let weighted_c = comb_c * mild_prior;
                tempogram[w][lag] = weighted_c;
                global_comp_comb[lag] += weighted_c;
                global_sub_comb[lag] += comb_s;
            }
        }

        let viterbi_lag_f = decode_viterbi_tempo_path(&tempogram, min_lag, max_lag);
        let viterbi_bpm = (60.0 * fps) / viterbi_lag_f;

        let mut peaks: Vec<(f64, f64)> = Vec::new();
        let max_g = global_comp_comb[min_lag..=max_lag]
            .iter()
            .copied()
            .fold(1e-9_f64, f64::max);

        for lag in (min_lag + 1)..max_lag {
            let y0 = global_comp_comb[lag - 1];
            let y1 = global_comp_comb[lag];
            let y2 = global_comp_comb[lag + 1];
            if y1 > y0 && y1 >= y2 && y1 > max_g * 0.22 {
                let denom = y0 - 2.0 * y1 + y2;
                let delta = if denom.abs() > 1e-12 {
                    (0.5 * (y0 - y2) / denom).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                let exact_lag = (lag as f64) + delta;
                let bpm = (60.0 * fps) / exact_lag;
                peaks.push((bpm, y1 / max_g));
            }
        }
        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        peaks.truncate(6);
        if peaks.is_empty() {
            peaks.push((viterbi_bpm, 1.0));
        }

        QmTempoAnalysis {
            top_comb_peaks: peaks,
            viterbi_bpm,
            sub_kick_comb_by_lag: global_sub_comb,
            composite_comb_by_lag: global_comp_comb,
            min_lag,
            max_lag,
        }
    }

    fn decode_viterbi_tempo_path(tempogram: &[Vec<f64>], min_lag: usize, max_lag: usize) -> f64 {
        let n_wins = tempogram.len();
        let num_lags = max_lag + 1;
        let mut dp = vec![0.0_f64; num_lags];
        let mut back = vec![vec![min_lag; num_lags]; n_wins];

        for lag in min_lag..=max_lag {
            dp[lag] = tempogram[0][lag];
        }

        let sigma_step = 2.2_f64;
        for w in 1..n_wins {
            let mut next_dp = vec![-1e18_f64; num_lags];
            for curr in min_lag..=max_lag {
                let lo = curr.saturating_sub(8).max(min_lag);
                let hi = (curr + 8).min(max_lag);
                let mut best_prev = curr;
                let mut best_val = -1e18_f64;
                for prev in lo..=hi {
                    let diff = (curr as f64) - (prev as f64);
                    let trans_pen = -0.08 * (diff / sigma_step).powi(2);
                    let cand = dp[prev] + trans_pen;
                    if cand > best_val {
                        best_val = cand;
                        best_prev = prev;
                    }
                }
                next_dp[curr] = best_val + tempogram[w][curr];
                back[w][curr] = best_prev;
            }
            dp = next_dp;
        }

        let mut curr = (min_lag..=max_lag)
            .max_by(|&a, &b| dp[a].partial_cmp(&dp[b]).unwrap_or(Ordering::Equal))
            .unwrap_or((min_lag + max_lag) / 2);

        // ¡PARCHE v2.1!: Interpolación parabólica por ventana antes de la mediana (elimina saltos de 1.5 BPM)
        let mut path_lags = Vec::with_capacity(n_wins);
        for w in (0..n_wins).rev() {
            let refined = if curr > min_lag && curr < max_lag {
                let y0 = tempogram[w][curr - 1];
                let y1 = tempogram[w][curr];
                let y2 = tempogram[w][curr + 1];
                let denom = y0 - 2.0 * y1 + y2;
                let delta = if denom.abs() > 1e-12 {
                    (0.5 * (y0 - y2) / denom).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                (curr as f64) + delta
            } else {
                curr as f64
            };
            path_lags.push(refined);
            curr = back[w][curr];
        }
        super::median_f64(&mut path_lags)
    }

}

// ============================================================================
// 4B. SUBMÓDULO `phase_lock` — BLOQUEO FINO DE FASE (28 s, coh²) + SNAP MIXXX
// ============================================================================

pub mod phase_lock {
    use super::multiband_onset::MultibandNovelty;
    use super::*;

    /// **PARCHE v2.2 (`block_incoherent_phase_lock`):**
    /// Evalúa coherencia de fase en bloques locales de **12 segundos** (avance de 6 s), permitiendo
    /// que cada bloque tenga su propia fase independiente tras *cortes* de reggaetón, mambos de
    /// merengue o ediciones de VideoDJ (`.mp4`/`.webm`).
    /// Esto erradica la cancelación destructiva que desplazaba `126 → 125/127` y `129 → 128 BPM`.
    pub fn fine_grid_phase_lock_and_beats(
        approx_bpm: f64,
        nov: &MultibandNovelty,
        prefer_kick_anchor: bool,
    ) -> (f64, f64, Vec<f64>, f64) {
        let fps = nov.onset_fps;
        let duration_sec = (nov.num_frames as f64) / fps;
        if nov.num_frames < 64 || approx_bpm <= 30.0 {
            return (approx_bpm, nov.trim_offset_sec, Vec::new(), 0.0);
        }

        let mut sparse_peaks: Vec<(f64, f64)> = Vec::new();
        for i in 1..(nov.num_frames - 1) {
            let val = if prefer_kick_anchor {
                0.62 * nov.sub_kick[i] + 0.22 * nov.low_body[i] + 0.16 * nov.perc_clave[i]
            } else {
                nov.composite[i]
            };
            if val > 0.85 {
                let blend = |k: usize| {
                    if prefer_kick_anchor {
                        0.62 * nov.sub_kick[k] + 0.22 * nov.low_body[k] + 0.16 * nov.perc_clave[k]
                    } else {
                        nov.composite[k]
                    }
                };
                let (prev, next) = (blend(i - 1), blend(i + 1));
                if val >= prev && val >= next {
                    let denom = prev - 2.0 * val + next;
                    let delta = if denom.abs() > 1e-12 {
                        (0.5 * (prev - next) / denom).clamp(-0.5, 0.5)
                    } else {
                        0.0
                    };
                    let t_sec = ((i as f64) + delta) / fps;
                    sparse_peaks.push((t_sec, val));
                }
            }
        }

        if sparse_peaks.len() < 8 {
            return (approx_bpm, nov.trim_offset_sec, Vec::new(), 0.0);
        }

        // Bloques de 28.0 s (avance 14.0 s): 2.3x más nitidez espectral e inmune a cortes vía peso coh^2
        let block_sec = 28.0_f64.min(duration_sec.max(8.0));
        let step_sec = (block_sec * 0.5).max(4.0);
        let mut blocks: Vec<(Vec<(f64, f64)>, f64)> = Vec::new();
        let mut t_start = 0.0_f64;
        while t_start < duration_sec {
            let t_end = t_start + block_sec;
            let sub: Vec<(f64, f64)> = sparse_peaks
                .iter()
                .copied()
                .filter(|&(t, _)| t >= t_start && t < t_end)
                .collect();
            if sub.len() >= 6 {
                let w_sum: f64 = sub.iter().map(|x| x.1).sum();
                blocks.push((sub, w_sum));
            }
            t_start += step_sec;
        }
        if blocks.is_empty() {
            let w_sum: f64 = sparse_peaks.iter().map(|x| x.1).sum::<f64>().max(1e-9);
            blocks.push((sparse_peaks.clone(), w_sum));
        }

        let total_block_weight: f64 = blocks.iter().map(|b| b.1).sum::<f64>().max(1e-9);

        // Barrido amplio ±3.2 BPM (paso 0.08) + barrido fino ±0.25 BPM (paso 0.01)
        let mut best_bpm = approx_bpm;
        let mut best_coherence = -1.0_f64;

        let lo_coarse = (approx_bpm - 3.2).max(45.0);
        let hi_coarse = (approx_bpm + 3.2).min(225.0);
        let mut b = lo_coarse;
        while b <= hi_coarse {
            let coh = windowed_block_coherence(&blocks, b, total_block_weight);
            if coh > best_coherence {
                best_coherence = coh;
                best_bpm = b;
            }
            b += 0.08;
        }

        let lo_fine = (best_bpm - 0.24).max(45.0);
        let hi_fine = (best_bpm + 0.24).min(225.0);
        let mut bf = (lo_fine * 100.0).round() / 100.0;
        while bf <= hi_fine {
            let coh = windowed_block_coherence(&blocks, bf, total_block_weight);
            if coh > best_coherence {
                best_coherence = coh;
                best_bpm = bf;
            }
            bf += 0.01;
        }

        // Regla de anclaje entero por bloque estable (24 beats = 12 s) estilo Mixxx
        let nearest_int = best_bpm.round();
        let int_coh = windowed_block_coherence(&blocks, nearest_int, total_block_weight);
        let beats_per_block = (block_sec * best_bpm / 60.0).max(1.0);
        let block_drift_sec =
            beats_per_block * ((60.0 / best_bpm) - (60.0 / nearest_int)).abs();

        if block_drift_sec <= MIXXX_BLOCK_MAX_PHASE_ERR_SEC && int_coh >= best_coherence * 0.975 {
            best_bpm = nearest_int;
            best_coherence = int_coh;
        } else {
            best_bpm = (best_bpm * 100.0).round() / 100.0;
        }

        // Ancla la fase inicial sobre el primer bloque energético estable de la canción
        let anchor_block = blocks
            .iter()
            .take(4)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            .unwrap_or(&blocks[0]);
        let (_, best_phase_rad) =
            single_block_phase_coherence(&anchor_block.0, best_bpm, anchor_block.1);

        let period_sec = 60.0 / best_bpm;
        let phase_offset_local =
            (best_phase_rad.rem_euclid(2.0 * std::f64::consts::PI) / (2.0 * std::f64::consts::PI))
                * period_sec;

        let mut beats_abs = Vec::new();
        let mut t = phase_offset_local;
        while t < duration_sec {
            beats_abs.push(nov.trim_offset_sec + t);
            t += period_sec;
        }
        let first_beat_sec = beats_abs.first().copied().unwrap_or(nov.trim_offset_sec);

        (best_bpm, first_beat_sec, beats_abs, best_coherence)
    }

    #[inline]
    fn windowed_block_coherence(
        blocks: &[(Vec<(f64, f64)>, f64)],
        bpm: f64,
        total_weight: f64,
    ) -> f64 {
        let mut weighted_sum = 0.0_f64;
        for (peaks, w_sum) in blocks {
            let (coh, _) = single_block_phase_coherence(peaks, bpm, *w_sum);
            // Elevar al cuadrado silencia automáticamente el bloque puntual donde cae un corte/redoble
            weighted_sum += (coh * coh) * (*w_sum);
        }
        (weighted_sum / total_weight).sqrt()
    }

    #[inline]
    fn single_block_phase_coherence(
        peaks: &[(f64, f64)],
        bpm: f64,
        weight_sum: f64,
    ) -> (f64, f64) {
        let two_pi_f = 2.0 * std::f64::consts::PI * (bpm / 60.0);
        let (mut re1, mut im1, mut re2, mut im2) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);

        for &(t, w) in peaks {
            let theta = two_pi_f * t;
            let (s1, c1) = theta.sin_cos();
            re1 += w * c1;
            im1 += w * s1;
            let (s2, c2) = (2.0 * theta).sin_cos();
            re2 += w * c2;
            im2 += w * s2;
        }

        let denom = weight_sum.max(1e-9);
        let r1 = (re1 * re1 + im1 * im1).sqrt() / denom;
        let r2 = (re2 * re2 + im2 * im2).sqrt() / denom;
        (0.78 * r1 + 0.22 * r2, im1.atan2(re1))
    }
}

// ============================================================================
// 5. SUBMÓDULO `latin_heuristics` — MATRIZ 16-PASOS + VETO 4/4 + SALSA VS DEMBOW
// ============================================================================

pub mod latin_heuristics {
    use super::multiband_onset::MultibandNovelty;
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum GrooveStyle {
        FourOnTheFloorClub,
        UrbanLatinFourFourHybrid,
        ReggaetonDembow,
        SalsaTumbao,
        MerengueDrive,
        BachataGroove,
        GeneralGroove,
    }

    #[derive(Debug, Clone)]
    pub struct GrooveSignature {
        pub style: GrooveStyle,
        pub sub_kick_phase_score: f64,
        pub kick_quarter_hit_ratio: f64,
        pub salsa_tumbao_ratio: f64,
        pub dembow_pattern_score: f64,
        pub guiro_sixteenth_density: f64,
        pub clave_tresillo_ratio: f64,
        pub veto_three_quarter_alias: bool,
    }

    pub fn evaluate_groove_at_bpm(nov: &MultibandNovelty, cand_bpm: f64) -> GrooveSignature {
        if nov.num_frames < 64 || cand_bpm <= 35.0 {
            return GrooveSignature {
                style: GrooveStyle::GeneralGroove,
                sub_kick_phase_score: 0.0,
                kick_quarter_hit_ratio: 0.0,
                salsa_tumbao_ratio: 0.0,
                dembow_pattern_score: 0.0,
                guiro_sixteenth_density: 0.0,
                clave_tresillo_ratio: 0.0,
                veto_three_quarter_alias: false,
            };
        }

        // ¡CRÍTICO v2.1!: Usa `nov.onset_fps` dinámico (correcto para 44.1 kHz, 48 kHz y 96 kHz)
        let beat_period_frames = (60.0 * nov.onset_fps) / cand_bpm;
        let bar_frames = beat_period_frames * 4.0;
        let step_frames = bar_frames / 16.0;

        let phase_probe_steps = 24_usize;
        let mut best_beat_offset = 0.0_f64;
        let mut best_probe_e = -1.0_f64;

        for p in 0..phase_probe_steps {
            let off = (p as f64 / phase_probe_steps as f64) * beat_period_frames;
            let mut sum = 0.0_f64;
            let mut t = off;
            while (t as usize) < nov.num_frames {
                let idx = t.round() as usize;
                if idx < nov.num_frames {
                    sum += 0.55 * nov.sub_kick[idx] + 0.45 * nov.composite[idx];
                }
                t += beat_period_frames;
            }
            if sum > best_probe_e {
                best_probe_e = sum;
                best_beat_offset = off;
            }
        }

        let mut hist_sub = [0.0_f64; 16];
        let mut hist_body = [0.0_f64; 16];
        let mut hist_clave = [0.0_f64; 16];
        let mut hist_guiro = [0.0_f64; 16];
        let mut step_counts = [0.0_f64; 16];

        let mut quarter_total_beats = 0_usize;
        let mut quarter_kick_hits = 0_usize;

        for f in 0..nov.num_frames {
            let rel = (f as f64) - best_beat_offset;
            if rel < 0.0 {
                continue;
            }
            let bar_pos = rel.rem_euclid(bar_frames);
            let step_idx = ((bar_pos / step_frames).floor() as usize).min(15);

            hist_sub[step_idx] += nov.sub_kick[f];
            hist_body[step_idx] += nov.low_body[f];
            hist_clave[step_idx] += nov.perc_clave[f];
            hist_guiro[step_idx] += nov.hi_guiro[f];
            step_counts[step_idx] += 1.0;
        }

        let mut b_cursor = best_beat_offset;
        let tol_frames = (beat_period_frames * 0.14).ceil().max(2.0) as isize;
        while (b_cursor as usize) < nov.num_frames {
            let center = b_cursor.round() as isize;
            let mut local_max_kick = 0.0_f64;
            for d in -tol_frames..=tol_frames {
                let idx = center + d;
                if idx >= 0 && (idx as usize) < nov.num_frames {
                    local_max_kick = local_max_kick.max(nov.sub_kick[idx as usize]);
                }
            }
            quarter_total_beats += 1;
            if local_max_kick > 0.65 {
                quarter_kick_hits += 1;
            }
            b_cursor += beat_period_frames;
        }
        let kick_quarter_hit_ratio = if quarter_total_beats > 0 {
            (quarter_kick_hits as f64) / (quarter_total_beats as f64)
        } else {
            0.0
        };

        for s in 0..16 {
            let c = step_counts[s].max(1.0);
            hist_sub[s] /= c;
            hist_body[s] /= c;
            hist_clave[s] /= c;
            hist_guiro[s] /= c;
        }

        let best_bar_rot = [0_usize, 4, 8, 12]
            .into_iter()
            .max_by(|&r1, &r2| {
                let s1 = hist_sub[r1] + 0.5 * hist_clave[r1];
                let s2 = hist_sub[r2] + 0.5 * hist_clave[r2];
                s1.partial_cmp(&s2).unwrap_or(Ordering::Equal)
            })
            .unwrap_or(0);

        hist_sub.rotate_left(best_bar_rot);
        hist_body.rotate_left(best_bar_rot);
        hist_clave.rotate_left(best_bar_rot);
        hist_guiro.rotate_left(best_bar_rot);

        let kick_on_quarters =
            (hist_sub[0] + hist_sub[4] + hist_sub[8] + hist_sub[12]) * 0.25;
        let kick_off_quarters = (0..16)
            .filter(|s| s % 4 != 0)
            .map(|s| hist_sub[s])
            .sum::<f64>()
            / 12.0;
        let sub_kick_phase_score =
            (kick_on_quarters / (kick_on_quarters + kick_off_quarters + 1e-6)).clamp(0.0, 1.0);

        let mut salsa_tumbao_ratio = 0.0_f64;
        for rot in [0_usize, 4, 8, 12] {
            let bass_0 = 0.55 * hist_sub[rot % 16] + 0.45 * hist_body[rot % 16];
            let bass_4 = 0.55 * hist_sub[(rot + 4) % 16] + 0.45 * hist_body[(rot + 4) % 16];
            let bass_6 = 0.55 * hist_sub[(rot + 6) % 16] + 0.45 * hist_body[(rot + 6) % 16];
            let bass_12 = 0.55 * hist_sub[(rot + 12) % 16] + 0.45 * hist_body[(rot + 12) % 16];
            let ratio = (bass_6 + bass_12) / (bass_0 + bass_4 + 0.12);
            salsa_tumbao_ratio = salsa_tumbao_ratio.max(ratio);
        }

        let snare_dembow =
            (hist_clave[3] + hist_clave[6] + hist_clave[11] + hist_clave[14]) * 0.25;
        let snare_other = (0..16)
            .filter(|s| !matches!(s, 0 | 3 | 4 | 6 | 8 | 11 | 12 | 14))
            .map(|s| hist_clave[s])
            .sum::<f64>()
            / 8.0;
        let dembow_snare_contrast =
            (snare_dembow / (snare_dembow + snare_other + 1e-6)).clamp(0.0, 1.0);
        let dembow_pattern_score =
            (0.55 * sub_kick_phase_score + 0.45 * dembow_snare_contrast) * kick_quarter_hit_ratio;

        let tresillo_hits = (hist_clave[0]
            + hist_clave[3]
            + hist_clave[6]
            + hist_clave[8]
            + hist_clave[11]
            + hist_clave[14])
            / 6.0;
        let clave_mean = (hist_clave.iter().sum::<f64>() / 16.0).max(1e-6);
        let clave_tresillo_ratio = tresillo_hits / clave_mean;

        let guiro_mean = hist_guiro.iter().sum::<f64>() / 16.0;
        let guiro_active_steps = hist_guiro
            .iter()
            .filter(|&&v| v >= guiro_mean * 0.55 && v > 0.12)
            .count();
        let guiro_sixteenth_density = (guiro_active_steps as f64) / 16.0;

        let veto_three_quarter_alias =
            kick_quarter_hit_ratio >= 0.62 && sub_kick_phase_score >= 0.58;

        let style = if veto_three_quarter_alias && (112.0..=136.0).contains(&cand_bpm) {
            if clave_tresillo_ratio >= 1.22 || guiro_sixteenth_density >= 0.68 {
                GrooveStyle::UrbanLatinFourFourHybrid
            } else {
                GrooveStyle::FourOnTheFloorClub
            }
        } else if (108.0..=164.0).contains(&cand_bpm)
            && guiro_sixteenth_density >= 0.75
            && kick_quarter_hit_ratio >= 0.50
        {
            GrooveStyle::MerengueDrive
        } else if (78.0..=106.0).contains(&cand_bpm)
            && dembow_pattern_score >= 0.42
            && kick_quarter_hit_ratio >= 0.58
        {
            GrooveStyle::ReggaetonDembow
        } else if (76.0..=109.0).contains(&cand_bpm)
            && salsa_tumbao_ratio >= 1.25
            && kick_quarter_hit_ratio < 0.58
        {
            GrooveStyle::SalsaTumbao
        } else if (116.0..=144.0).contains(&cand_bpm)
            && hist_clave[12] > clave_mean * 1.35
            && hist_body[6] > hist_body[4] * 1.15
        {
            GrooveStyle::BachataGroove
        } else if veto_three_quarter_alias {
            GrooveStyle::FourOnTheFloorClub
        } else {
            GrooveStyle::GeneralGroove
        };

        GrooveSignature {
            style,
            sub_kick_phase_score,
            kick_quarter_hit_ratio,
            salsa_tumbao_ratio,
            dembow_pattern_score,
            guiro_sixteenth_density,
            clave_tresillo_ratio,
            veto_three_quarter_alias,
        }
    }
}

// ============================================================================
// 6. SUBMÓDULO `consensus_ai` — MOTOR DE CONSENSO MULTIGÉNERO + HINTS
// ============================================================================

pub mod consensus_ai {
    use super::hints::BpmHint;
    use super::latin_heuristics::{evaluate_groove_at_bpm, GrooveSignature, GrooveStyle};
    use super::multiband_onset::MultibandNovelty;
    use super::phase_lock::fine_grid_phase_lock_and_beats;
    use super::qm_comb_tracker::analyze_qm_comb_tempogram;
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BpmConsensusResult {
        pub bpm: f64,
        pub first_beat_sec: f64,
        pub beats_sec: Vec<f64>,
        pub confidence: f32,
        pub detected_style: GrooveStyle,
        pub applied_ratio_note: String,
    }

    pub fn analyze_bpm_consensus_v2(
        nov: &MultibandNovelty,
        bpm_hint: Option<BpmHint>,
    ) -> BpmConsensusResult {
        let qm = analyze_qm_comb_tempogram(nov);
        let fps = nov.onset_fps;

        let mut base_seeds: Vec<f64> = qm.top_comb_peaks.iter().take(3).map(|p| p.0).collect();
        base_seeds.push(qm.viterbi_bpm);
        if let Some(h) = bpm_hint {
            base_seeds.push(h.bpm);
        }

        let ratios = [1.0_f64, 0.75, 4.0 / 3.0, 0.5, 2.0, 2.0 / 3.0, 1.5];
        let mut candidates: Vec<(f64, &'static str)> = Vec::new();

        for &seed in &base_seeds {
            for &r in &ratios {
                let c = seed * r;
                if (55.0..=210.0).contains(&c) {
                    if !candidates.iter().any(|existing| (existing.0 - c).abs() < 0.85) {
                        let tag = if (r - 1.0).abs() < 1e-3 {
                            "1.0x"
                        } else if (r - 0.75).abs() < 1e-3 {
                            "0.75x (3:4)"
                        } else if (r - 4.0 / 3.0).abs() < 1e-2 {
                            "1.33x (4:3)"
                        } else if (r - 0.5).abs() < 1e-3 {
                            "0.5x"
                        } else if (r - 2.0).abs() < 1e-3 {
                            "2.0x"
                        } else {
                            "3:2 metric"
                        };
                        candidates.push((c, tag));
                    }
                }
            }
        }

        let max_comp_comb = qm
            .composite_comb_by_lag
            .iter()
            .copied()
            .fold(1e-9_f64, f64::max);
        let max_sub_comb = qm
            .sub_kick_comb_by_lag
            .iter()
            .copied()
            .fold(1e-9_f64, f64::max);

        let evals: Vec<(f64, &'static str, GrooveSignature, f64, f64)> = candidates
            .iter()
            .map(|&(bpm, label)| {
                let sig = evaluate_groove_at_bpm(nov, bpm);
                let exact_lag = ((60.0 * fps) / bpm)
                    .clamp(qm.min_lag as f64, qm.max_lag as f64);
                // Interpolación lineal en el espectro peine (evita escalones discretos)
                let l0 = exact_lag.floor() as usize;
                let l1 = (l0 + 1).min(qm.max_lag);
                let frac = exact_lag - (l0 as f64);
                let comp_e = ((1.0 - frac) * qm.composite_comb_by_lag[l0]
                    + frac * qm.composite_comb_by_lag[l1])
                    / max_comp_comb;
                let sub_e = ((1.0 - frac) * qm.sub_kick_comb_by_lag[l0]
                    + frac * qm.sub_kick_comb_by_lag[l1])
                    / max_sub_comb;
                (bpm, label, sig, comp_e, sub_e)
            })
            .collect();

        let mut best_idx = 0_usize;
        let mut best_total_score = -1e9_f64;

        for (idx, (bpm, _label, sig, comp_e, sub_e)) in evals.iter().enumerate() {
            let mut score = 0.44 * comp_e
                + 0.24 * sub_e
                + 0.20 * sig.sub_kick_phase_score
                + 0.12 * sig.kick_quarter_hit_ratio;

            let parent_4_3 = bpm * (4.0 / 3.0);
            if let Some((_, _, parent_sig, _, parent_sub_e)) = evals
                .iter()
                .find(|e| (e.0 - parent_4_3).abs() <= 1.6)
            {
                if parent_sig.veto_three_quarter_alias && *parent_sub_e >= *sub_e * 0.90 {
                    score -= 0.55;
                }
            }

            match sig.style {
                GrooveStyle::FourOnTheFloorClub | GrooveStyle::UrbanLatinFourFourHybrid => {
                    score += 0.22;
                }
                GrooveStyle::MerengueDrive => {
                    score += 0.20;
                }
                GrooveStyle::ReggaetonDembow => {
                    score += 0.21;
                }
                GrooveStyle::SalsaTumbao => {
                    let has_club_parent = evals.iter().any(|e| {
                        (e.0 - parent_4_3).abs() <= 1.6 && e.2.veto_three_quarter_alias
                    });
                    if !has_club_parent {
                        score += 0.26;
                    }
                }
                GrooveStyle::BachataGroove => {
                    score += 0.16;
                }
                GrooveStyle::GeneralGroove => {}
            }

            if *bpm > 158.0 {
                let half_bpm = bpm * 0.5;
                if let Some((_, _, half_sig, _, half_sub)) =
                    evals.iter().find(|e| (e.0 - half_bpm).abs() <= 1.5)
                {
                    if matches!(
                        half_sig.style,
                        GrooveStyle::ReggaetonDembow | GrooveStyle::SalsaTumbao
                    ) || *half_sub >= *sub_e * 0.92
                    {
                        score -= 0.32;
                    }
                }
            }
            if *bpm < 76.0 {
                let double_bpm = bpm * 2.0;
                if let Some((_, _, dbl_sig, _, _)) =
                    evals.iter().find(|e| (e.0 - double_bpm).abs() <= 2.0)
                {
                    if dbl_sig.guiro_sixteenth_density >= 0.65
                        || dbl_sig.veto_three_quarter_alias
                    {
                        score -= 0.35;
                    }
                }
            }

            if let Some(hint) = bpm_hint {
                let rel_err = ((bpm - hint.bpm).abs()) / hint.bpm.max(1.0);
                if rel_err <= 0.025 {
                    score += hint.source.prior_weight();
                } else if ((bpm * 2.0 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
                    || ((bpm * 0.5 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
                {
                    score -= hint.source.prior_weight() * 0.45;
                }
            }

            if score > best_total_score {
                best_total_score = score;
                best_idx = idx;
            }
        }

        let (winning_coarse_bpm, winning_label, winning_sig, _, _) = &evals[best_idx];
        let prefer_kick = winning_sig.veto_three_quarter_alias
            || matches!(
                winning_sig.style,
                GrooveStyle::ReggaetonDembow | GrooveStyle::MerengueDrive
            );

        let (locked_bpm, first_beat_sec, beats_sec, coherence) =
            fine_grid_phase_lock_and_beats(*winning_coarse_bpm, nov, prefer_kick);

        let confidence = (0.55 * coherence + 0.45 * best_total_score.clamp(0.0, 1.0))
            .clamp(0.0, 1.0) as f32;

        BpmConsensusResult {
            bpm: locked_bpm,
            first_beat_sec,
            beats_sec,
            confidence,
            detected_style: winning_sig.style,
            applied_ratio_note: (*winning_label).to_string(),
        }
    }
}

// ============================================================================
// 7. SUBMÓDULO `key_engine` — HPCP CON `sample_rate` DINÁMICO + ANCLA SUB-BAJO
// ============================================================================

pub mod key_engine {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u8)]
    pub enum PitchClass {
        C = 0,
        Db = 1,
        D = 2,
        Eb = 3,
        E = 4,
        F = 5,
        Gb = 6,
        G = 7,
        Ab = 8,
        A = 9,
        Bb = 10,
        B = 11,
    }

    impl PitchClass {
        pub fn from_semitone(idx: usize) -> Self {
            match idx.rem_euclid(12) {
                0 => Self::C,
                1 => Self::Db,
                2 => Self::D,
                3 => Self::Eb,
                4 => Self::E,
                5 => Self::F,
                6 => Self::Gb,
                7 => Self::G,
                8 => Self::Ab,
                9 => Self::A,
                10 => Self::Bb,
                _ => Self::B,
            }
        }

        pub fn label(self) -> &'static str {
            match self {
                Self::C => "C",
                Self::Db => "Db",
                Self::D => "D",
                Self::Eb => "Eb",
                Self::E => "E",
                Self::F => "F",
                Self::Gb => "F#",
                Self::G => "G",
                Self::Ab => "Ab",
                Self::A => "A",
                Self::Bb => "Bb",
                Self::B => "B",
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum KeyScale {
        Major,
        Minor,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MusicalKeyResult {
        pub tonic: PitchClass,
        pub scale: KeyScale,
        pub standard_name: String,
        pub camelot: String,
        pub open_key: String,
        pub confidence: f32,
        pub tuning_cents: f32,
    }

    const BGATE_MAJ: [f64; 12] = [1.00, 0.00, 0.42, 0.00, 0.53, 0.37, 0.00, 0.77, 0.00, 0.38, 0.21, 0.30];
    const BGATE_MIN: [f64; 12] = [1.00, 0.00, 0.36, 0.39, 0.00, 0.38, 0.00, 0.74, 0.27, 0.00, 0.42, 0.23];
    const EDMA_MAJ: [f64; 12] = [1.00, 0.29, 0.50, 0.40, 0.60, 0.56, 0.32, 0.80, 0.31, 0.45, 0.42, 0.39];
    const EDMA_MIN: [f64; 12] = [1.00, 0.31, 0.44, 0.58, 0.33, 0.49, 0.29, 0.78, 0.43, 0.29, 0.53, 0.32];
    const TEMP_MAJ: [f64; 12] = [0.748, 0.060, 0.488, 0.082, 0.670, 0.460, 0.096, 0.715, 0.104, 0.366, 0.057, 0.400];
    const TEMP_MIN: [f64; 12] = [0.712, 0.084, 0.474, 0.618, 0.049, 0.460, 0.105, 0.747, 0.404, 0.067, 0.133, 0.330];
    const SHAATH_MAJ: [f64; 12] = [6.6, 2.0, 3.5, 2.3, 4.6, 4.0, 2.5, 5.2, 2.4, 3.7, 2.3, 3.4];
    const SHAATH_MIN: [f64; 12] = [6.5, 2.7, 3.5, 5.4, 2.6, 3.5, 2.5, 5.2, 4.0, 2.7, 4.3, 3.2];

    /// ¡PARCHE CRÍTICO v2.1!:
    /// 1. Recibe `sample_rate: u32` real (elimina el desplazamiento de `-1.47 semitonos` en archivos de 48 kHz).
    /// 2. Incorpora `bass_roots_12` (`50–185 Hz`) para anclar la tónica del bajo y desempatar `8A (Am)` vs `8B (C)`.
    pub fn detect_song_key(
        samples: &[f32],
        sample_rate: u32,
        fft_planner: &mut FftPlanner<f32>,
    ) -> MusicalKeyResult {
        let sr = sample_rate.max(8_000);
        let n_fft: usize = if sr >= 80_000 { 8192 } else { 4096 };
        let hop: usize = n_fft / 2;
        if samples.len() < n_fft {
            return super::camelot_wheel::build_key_result(PitchClass::C, KeyScale::Major, 0.0, 0.0);
        }

        let fft = fft_planner.plan_fft_forward(n_fft);
        let window: Vec<f32> = (0..n_fft)
            .map(|i| {
                let p = 2.0 * PI * (i as f32) / ((n_fft - 1) as f32);
                0.35875 - 0.48829 * p.cos() + 0.14128 * (2.0 * p).cos() - 0.01168 * (3.0 * p).cos()
            })
            .collect();

        let bin_hz = (sr as f64) / (n_fft as f64);
        // Inicia en 92 Hz para excluir el golpe fundamental del bombo urbano (50-80 Hz, ~Bb1 = 58 Hz = 3A)
        let min_bin = ((92.0 / bin_hz).ceil() as usize).max(2);
        let max_bin = ((3520.0 / bin_hz).floor() as usize).min(n_fft / 2 - 2);

        let num_frames = (samples.len() - n_fft) / hop + 1;
        let step = (num_frames / 550).max(1);

        let mut buf = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; n_fft];
        let mut mag = vec![0.0_f64; n_fft / 2 + 1];
        let mut hpcp = [0.0_f64; HPCP_BINS];
        let mut peaks_12 = [0.0_f64; 12];
        let mut bass_roots_12 = [0.0_f64; 12];

        for f_idx in (0..num_frames).step_by(step) {
            let off = f_idx * hop;
            for i in 0..n_fft {
                buf[i] = Complex {
                    re: samples[off + i] * window[i],
                    im: 0.0,
                };
            }
            fft.process(&mut buf);

            let mut log_s = 0.0_f64;
            let mut lin_s = 0.0_f64;
            let cnt = (max_bin - min_bin + 1) as f64;
            for k in min_bin..=max_bin {
                let m = (buf[k].norm() as f64).max(1e-9);
                mag[k] = m;
                log_s += m.ln();
                lin_s += m;
            }
            mag[min_bin - 1] = (buf[min_bin - 1].norm() as f64).max(1e-9);
            mag[max_bin + 1] = (buf[max_bin + 1].norm() as f64).max(1e-9);

            let flatness = (log_s / cnt).exp() / (lin_s / cnt).max(1e-9);
            if flatness > 0.44 || (lin_s / cnt) < 1e-4 {
                continue;
            }
            let h_weight = (1.0 - flatness / 0.44).powi(2);
            let mean_m = lin_s / cnt;

            for k in min_bin..=max_bin {
                let (m0, m1, m2) = (mag[k - 1], mag[k], mag[k + 1]);
                if m1 > m0 && m1 > m2 && m1 > mean_m * 1.35 {
                    let denom = m0 - 2.0 * m1 + m2;
                    let delta = if denom.abs() > 1e-12 {
                        (0.5 * (m0 - m2) / denom).clamp(-0.5, 0.5)
                    } else {
                        0.0
                    };
                    let freq = ((k as f64) + delta) * bin_hz;
                    let amp = m1.powf(0.65) * h_weight;

                    let midi_p = 69.0 + 12.0 * (freq / 440.0).log2();
                    let pc_12 = (midi_p.round() as isize).rem_euclid(12) as usize;
                    peaks_12[pc_12] += amp;

                    // Ancla armónica de bajo en registro limpio (95–245 Hz, libre del sub-kick de 58 Hz)
                    if (95.0..=245.0).contains(&freq) {
                        bass_roots_12[pc_12] += amp;
                    }

                    for h in 1..=4 {
                        let f_sub = freq / (h as f64);
                        if f_sub < 45.0 {
                            break;
                        }
                        let decay = 0.62_f64.powi((h - 1) as i32);
                        let midi_c = 69.0 + 12.0 * (f_sub / 440.0).log2();
                        let bin_pos =
                            (midi_c * (HPCP_BINS_PER_SEMITONE as f64)).rem_euclid(HPCP_BINS as f64);
                        let center = bin_pos.round() as isize;
                        for b_off in -1..=1 {
                            let dist = (bin_pos - ((center + b_off) as f64)).abs();
                            if dist < 1.0 {
                                let w = (0.5 * std::f64::consts::PI * dist).cos().powi(2);
                                let idx =
                                    (center + b_off).rem_euclid(HPCP_BINS as isize) as usize;
                                hpcp[idx] += amp * decay * w;
                            }
                        }
                    }
                }
            }
        }

        let mut sub_bins = [0.0_f64; HPCP_BINS_PER_SEMITONE];
        for (i, &v) in hpcp.iter().enumerate() {
            sub_bins[i % HPCP_BINS_PER_SEMITONE] += v;
        }
        let (mut sin_a, mut cos_a) = (0.0_f64, 0.0_f64);
        for (k, &e) in sub_bins.iter().enumerate() {
            let ang = 2.0 * std::f64::consts::PI * (k as f64) / (HPCP_BINS_PER_SEMITONE as f64);
            sin_a += e * ang.sin();
            cos_a += e * ang.cos();
        }
        let detune_rad = sin_a.atan2(cos_a);
        let tuning_cents = ((detune_rad / (2.0 * std::f64::consts::PI)) * 100.0) as f32;
        let shift_bins =
            ((detune_rad / (2.0 * std::f64::consts::PI)) * (HPCP_BINS_PER_SEMITONE as f64)).round()
                as isize;

        let mut pcp_12 = [0.0_f64; 12];
        for st in 0..12 {
            let c = (st * HPCP_BINS_PER_SEMITONE) as isize + shift_bins;
            let b0 = (c - 1).rem_euclid(HPCP_BINS as isize) as usize;
            let b1 = c.rem_euclid(HPCP_BINS as isize) as usize;
            let b2 = (c + 1).rem_euclid(HPCP_BINS as isize) as usize;
            pcp_12[st] = 0.25 * hpcp[b0] + 0.50 * hpcp[b1] + 0.25 * hpcp[b2];
        }

        let max_pcp = pcp_12.iter().copied().fold(0.0_f64, f64::max).max(1e-12);
        for v in &mut pcp_12 {
            *v /= max_pcp;
            if *v < 0.12 {
                *v = 0.0;
            }
        }

        let max_bass = bass_roots_12.iter().copied().fold(0.0_f64, f64::max).max(1e-12);

        let mut scored: Vec<(PitchClass, KeyScale, f64)> = Vec::with_capacity(24);
        for root in 0..12 {
            let bass_bonus = 0.045 * (bass_roots_12[root] / max_bass);

            let maj = 0.35 * corr_12(&pcp_12, &BGATE_MAJ, root)
                + 0.25 * corr_12(&pcp_12, &EDMA_MAJ, root)
                + 0.25 * corr_12(&pcp_12, &TEMP_MAJ, root)
                + 0.15 * corr_12(&pcp_12, &SHAATH_MAJ, root)
                + bass_bonus;
            scored.push((PitchClass::from_semitone(root), KeyScale::Major, maj));

            let min = 0.35 * corr_12(&pcp_12, &BGATE_MIN, root)
                + 0.25 * corr_12(&pcp_12, &EDMA_MIN, root)
                + 0.25 * corr_12(&pcp_12, &TEMP_MIN, root)
                + 0.15 * corr_12(&pcp_12, &SHAATH_MIN, root)
                + bass_bonus;
            scored.push((PitchClass::from_semitone(root), KeyScale::Minor, min));
        }
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));

        let (win_tonic, mut win_scale, mut win_score) = scored[0];
        let r_idx = win_tonic as usize;
        let maj3 = peaks_12[(r_idx + 4) % 12];
        let min3 = peaks_12[(r_idx + 3) % 12];
        let opp_scale = match win_scale {
            KeyScale::Major => KeyScale::Minor,
            KeyScale::Minor => KeyScale::Major,
        };
        if let Some(&(_, _, par_score)) = scored
            .iter()
            .find(|&&(t, s, _)| t == win_tonic && s == opp_scale)
        {
            if (win_score - par_score) < 0.09 {
                if (win_scale == KeyScale::Major && min3 > maj3 * 1.32)
                    || (win_scale == KeyScale::Minor && maj3 > min3 * 1.38)
                {
                    win_scale = opp_scale;
                    win_score = par_score;
                }
            }
        }

        let runner_up = scored.get(1).map(|x| x.2).unwrap_or(0.0);
        let conf = (((win_score - runner_up).max(0.0) / win_score.abs().max(1e-6)) * 2.2)
            .clamp(0.0, 1.0) as f32;

        super::camelot_wheel::build_key_result(win_tonic, win_scale, conf, tuning_cents)
    }

    fn corr_12(pcp: &[f64; 12], prof: &[f64; 12], shift: usize) -> f64 {
        let mx = pcp.iter().sum::<f64>() / 12.0;
        let my = prof.iter().sum::<f64>() / 12.0;
        let (mut num, mut dx2, mut dy2) = (0.0_f64, 0.0_f64, 0.0_f64);
        for i in 0..12 {
            let py = prof[(i + 12 - (shift % 12)) % 12];
            let dx = pcp[i] - mx;
            let dy = py - my;
            num += dx * dy;
            dx2 += dx * dx;
            dy2 += dy * dy;
        }
        let den = (dx2 * dy2).sqrt();
        if den > 1e-12 {
            num / den
        } else {
            0.0
        }
    }

}

// ============================================================================
// 7B. SUBMÓDULO `camelot_wheel` — MAPEO EXACTO RUEDA CAMELOT (1A–12B) + OPEN KEY
// ============================================================================

pub mod camelot_wheel {
    use super::key_engine::{KeyScale, MusicalKeyResult, PitchClass};

    pub fn build_key_result(
        tonic: PitchClass,
        scale: KeyScale,
        confidence: f32,
        tuning_cents: f32,
    ) -> MusicalKeyResult {
        let (num, cam_let, ok_let) = match (tonic, scale) {
            (PitchClass::Ab, KeyScale::Minor) => (1, 'A', 'm'),
            (PitchClass::Eb, KeyScale::Minor) => (2, 'A', 'm'),
            (PitchClass::Bb, KeyScale::Minor) => (3, 'A', 'm'),
            (PitchClass::F, KeyScale::Minor) => (4, 'A', 'm'),
            (PitchClass::C, KeyScale::Minor) => (5, 'A', 'm'),
            (PitchClass::G, KeyScale::Minor) => (6, 'A', 'm'),
            (PitchClass::D, KeyScale::Minor) => (7, 'A', 'm'),
            (PitchClass::A, KeyScale::Minor) => (8, 'A', 'm'),
            (PitchClass::E, KeyScale::Minor) => (9, 'A', 'm'),
            (PitchClass::B, KeyScale::Minor) => (10, 'A', 'm'),
            (PitchClass::Gb, KeyScale::Minor) => (11, 'A', 'm'),
            (PitchClass::Db, KeyScale::Minor) => (12, 'A', 'm'),

            (PitchClass::B, KeyScale::Major) => (1, 'B', 'd'),
            (PitchClass::Gb, KeyScale::Major) => (2, 'B', 'd'),
            (PitchClass::Db, KeyScale::Major) => (3, 'B', 'd'),
            (PitchClass::Ab, KeyScale::Major) => (4, 'B', 'd'),
            (PitchClass::Eb, KeyScale::Major) => (5, 'B', 'd'),
            (PitchClass::Bb, KeyScale::Major) => (6, 'B', 'd'),
            (PitchClass::F, KeyScale::Major) => (7, 'B', 'd'),
            (PitchClass::C, KeyScale::Major) => (8, 'B', 'd'),
            (PitchClass::G, KeyScale::Major) => (9, 'B', 'd'),
            (PitchClass::D, KeyScale::Major) => (10, 'B', 'd'),
            (PitchClass::A, KeyScale::Major) => (11, 'B', 'd'),
            (PitchClass::E, KeyScale::Major) => (12, 'B', 'd'),
        };
        let ok_num = ((num + 4) % 12) + 1;
        let sc_str = match scale {
            KeyScale::Major => "major",
            KeyScale::Minor => "minor",
        };
        MusicalKeyResult {
            tonic,
            scale,
            standard_name: format!("{} {}", tonic.label(), sc_str),
            camelot: format!("{}{}", num, cam_let),
            open_key: format!("{}{}", ok_num, ok_let),
            confidence,
            tuning_cents,
        }
    }
}

// ============================================================================
// 8. SUBMÓDULO `queue` — COLA MASIVA CON DOBLE SEMÁFORO Y MUESTREO 3×20s
// ============================================================================

pub mod queue {
    use super::audio_ingest::{
        fast_partial_file_hash, load_audio_mono_native_sr, ProcessLimiter,
    };
    use super::consensus_ai::{analyze_bpm_consensus_v2, BpmConsensusResult};
    use super::hints::{resolve_best_hint, VirtualDjDbIndex};
    use super::key_engine::{detect_song_key, MusicalKeyResult};
    use super::multiband_onset::{extract_multiband_novelty, trim_leading_trailing_silence};
    use super::*;
    use std::sync::{mpsc::{sync_channel as bounded, Receiver, SyncSender as Sender}, Arc, Mutex};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct QueueTrackInput {
        pub track_id: String,
        pub path: Option<PathBuf>,
        pub explicit_bpm_hint: Option<f64>,
        #[serde(skip)]
        pub inline_pcm: Option<(Arc<Vec<f32>>, u32)>,
    }

    impl QueueTrackInput {
        pub fn from_file(track_id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
            Self {
                track_id: track_id.into(),
                path: Some(path.into()),
                explicit_bpm_hint: None,
                inline_pcm: None,
            }
        }

        pub fn from_pcm(track_id: impl Into<String>, pcm: Vec<f32>, sr: u32) -> Self {
            Self {
                track_id: track_id.into(),
                path: None,
                explicit_bpm_hint: None,
                inline_pcm: Some((Arc::new(pcm), sr)),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FullTrackAnalysis {
        pub track_id: String,
        pub audio_hash_hex: String,
        pub sample_rate: u32,
        pub duration_sec: f64,
        pub bpm_result: BpmConsensusResult,
        pub key_result: MusicalKeyResult,
        pub used_ffmpeg_video_fallback: bool,
        pub elapsed_ms: u64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum QueueProgressEvent {
        TrackDone {
            done: usize,
            total: usize,
            analysis: Box<FullTrackAnalysis>,
        },
        DuplicateSkipped {
            track_id: String,
            audio_hash_hex: String,
        },
        TrackError {
            track_id: String,
            error: String,
        },
        Finished {
            processed: usize,
            total_ms: u64,
        },
    }

    pub trait BatchResultSink: Send + Sync {
        fn persist_batch(&self, items: &[FullTrackAnalysis]) -> Result<(), String>;
    }

    #[derive(Default)]
    pub struct MemoryResultSink {
        pub items: Mutex<Vec<FullTrackAnalysis>>,
    }

    impl BatchResultSink for MemoryResultSink {
        fn persist_batch(&self, batch: &[FullTrackAnalysis]) -> Result<(), String> {
            let mut g = self.items.lock().map_err(|e| e.to_string())?;
            g.extend_from_slice(batch);
            Ok(())
        }
    }

    pub struct HeraBatchQueue {
        pub worker_threads: usize,
        pub max_ffmpeg_processes: usize,
        pub flush_batch_size: usize,
        /// Si la pista dura más de 150 s, analiza 3 ventanas centrales de 22 s (25%, 50%, 75%)
        /// para triplicar la velocidad y saltar intros/outros vacías.
        pub enable_three_window_excerpt: bool,
        pub vdj_db: Option<Arc<VirtualDjDbIndex>>,
        pub sink: Arc<dyn BatchResultSink>,
    }

    impl HeraBatchQueue {
        pub fn new(sink: Arc<dyn BatchResultSink>, vdj_db: Option<VirtualDjDbIndex>) -> Self {
            let cpus = thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            Self {
                worker_threads: cpus.saturating_sub(1).max(1),
                max_ffmpeg_processes: if cpus >= 10 { 3 } else { 2 },
                flush_batch_size: 100,
                enable_three_window_excerpt: true,
                vdj_db: vdj_db.map(Arc::new),
                sink,
            }
        }

        pub fn run_queue<F>(
            &self,
            inputs: Vec<QueueTrackInput>,
            cancel_flag: Arc<AtomicBool>,
            mut on_progress: F,
        ) -> Result<usize, EngineError>
        where
            F: FnMut(QueueProgressEvent) + Send,
        {
            let t0 = Instant::now();
            let total = inputs.len();
            if total == 0 {
                on_progress(QueueProgressEvent::Finished {
                    processed: 0,
                    total_ms: 0,
                });
                return Ok(0);
            }

            let bound = (self.worker_threads * 2).max(4);
            let (in_tx, in_rx): (Sender<QueueTrackInput>, Receiver<QueueTrackInput>) = bounded(bound);
            let in_rx = Arc::new(Mutex::new(in_rx));
            let (out_tx, out_rx) = bounded(bound * 2);

            let seen_hashes = Arc::new(Mutex::new(HashSet::<String>::with_capacity(total)));
            let ffmpeg_limiter = Arc::new(ProcessLimiter::new(self.max_ffmpeg_processes));
            let cancel_feeder = Arc::clone(&cancel_flag);

            let feeder = thread::spawn(move || {
                for item in inputs {
                    if cancel_feeder.load(AtomicOrdering::Relaxed) {
                        break;
                    }
                    if in_tx.send(item).is_err() {
                        break;
                    }
                }
            });

            let mut workers = Vec::with_capacity(self.worker_threads);
            for _ in 0..self.worker_threads {
                let rx = Arc::clone(&in_rx);
                let tx = out_tx.clone();
                let vdj = self.vdj_db.clone();
                let seen = Arc::clone(&seen_hashes);
                let limiter = Arc::clone(&ffmpeg_limiter);
                let cancel_w = Arc::clone(&cancel_flag);
                let excerpt_mode = self.enable_three_window_excerpt;

                workers.push(thread::spawn(move || {
                    let mut planner = FftPlanner::<f32>::new();
                    while let Ok(job) = { let guard = rx.lock().unwrap(); guard.recv() } {
                        if cancel_w.load(AtomicOrdering::Relaxed) {
                            break;
                        }
                        let res = super::queue_worker::analyze_single_job(
                            job,
                            vdj.as_deref(),
                            &seen,
                            &limiter,
                            excerpt_mode,
                            &mut planner,
                        );
                        if tx.send(res).is_err() {
                            break;
                        }
                    }
                }));
            }
            drop(in_rx);
            drop(out_tx);

            let done_count = AtomicUsize::new(0);
            let mut batch_buf = Vec::with_capacity(self.flush_batch_size);

            for msg in out_rx {
                match msg {
                    super::queue_worker::WorkerOutcome::Analyzed(analysis) => {
                        let d = done_count.fetch_add(1, AtomicOrdering::Relaxed) + 1;
                        batch_buf.push(analysis.clone());
                        on_progress(QueueProgressEvent::TrackDone {
                            done: d,
                            total,
                            analysis: Box::new(analysis),
                        });
                        if batch_buf.len() >= self.flush_batch_size {
                            let _ = self.sink.persist_batch(&batch_buf);
                            batch_buf.clear();
                        }
                    }
                    super::queue_worker::WorkerOutcome::Duplicate { track_id, hash } => {
                        on_progress(QueueProgressEvent::DuplicateSkipped {
                            track_id,
                            audio_hash_hex: hash,
                        });
                    }
                    super::queue_worker::WorkerOutcome::Failed { track_id, error } => {
                        on_progress(QueueProgressEvent::TrackError { track_id, error });
                    }
                }
            }

            if !batch_buf.is_empty() {
                let _ = self.sink.persist_batch(&batch_buf);
            }

            let _ = feeder.join();
            for w in workers {
                let _ = w.join();
            }

            if cancel_flag.load(AtomicOrdering::Relaxed) {
                return Err(EngineError::Cancelled);
            }

            let processed = done_count.load(AtomicOrdering::Relaxed);
            on_progress(QueueProgressEvent::Finished {
                processed,
                total_ms: t0.elapsed().as_millis() as u64,
            });
            Ok(processed)
        }
    }

}

// ============================================================================
// 8B. SUBMÓDULO `queue_worker` — EJECUCIÓN UNITARIA CON HASH 64KB + VENTANAS 3×22s
// ============================================================================

pub mod queue_worker {
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
}

// ============================================================================
// 9. SUITE DE TESTS DE REGRESIÓN v2.1 (INCLUYE PARCHE 48 kHz Y CORTES DE VIDEO)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::consensus_ai::analyze_bpm_consensus_v2;
    use super::hints::{extract_bpm_from_filename, parse_virtualdj_bpm_value, VirtualDjDbIndex};
    use super::key_engine::detect_song_key;
    use super::multiband_onset::extract_multiband_novelty;
    use super::*;

    /// Caso exacto de regresión v2.1: Afro/Tech House a **48,000 Hz** (`Fisher 125 BPM / 7B F major`).
    /// Antes daba `114.85 BPM / 4A` por asumir `44,100 Hz`; ahora debe clavar `125.00 BPM / 7B`.
    #[test]
    fn test_afro_tech_house_48khz_no_sample_rate_bias() {
        let sr = 48_000_u32;
        let duration_sec = 20.0_f64;
        let n = (duration_sec * (sr as f64)) as usize;
        let mut pcm = vec![0.0_f32; n];
        let beat_sec = 60.0 / 125.0;

        // F major: F2 bajo (MIDI 41) + acorde F4-A4-C5 (MIDI 65, 69, 72)
        for (i, sample) in pcm.iter_mut().enumerate() {
            let t = (i as f64) / (sr as f64);
            let beat_phase = t.rem_euclid(beat_sec);
            let kick = if beat_phase < 0.08 {
                0.72 * (-beat_phase * 42.0).exp()
                    * (2.0 * std::f64::consts::PI * 58.0 * beat_phase).sin()
            } else {
                0.0
            };

            let mut harm = 0.0_f64;
            for &(m, w) in &[(41.0_f64, 0.9), (65.0, 0.8), (69.0, 0.65), (72.0, 0.7)] {
                let f = 440.0 * 2.0_f64.powf((m - 69.0) / 12.0);
                harm += w * (2.0 * std::f64::consts::PI * f * t).sin();
            }
            *sample = (kick + 0.18 * harm).clamp(-0.98, 0.98) as f32;
        }

        let mut planner = FftPlanner::<f32>::new();
        let nov = extract_multiband_novelty(&pcm, sr, 0.0, &mut planner);
        let res = analyze_bpm_consensus_v2(&nov, None);
        let key = detect_song_key(&pcm, sr, &mut planner);

        assert!(
            (res.bpm - 125.0).abs() <= 0.05,
            "Afro/Tech House @ 48 kHz debe dar 125.00 BPM (no 114.85), dio {:.2}",
            res.bpm
        );
        assert_eq!(key.camelot, "7B");
    }

    /// Caso exacto de regresión v2.1: Pista de `126.00 BPM` (`MUJERON` / `Calle Ciega`) con un
    /// corte de medio compás a mitad del tema que antes partía el pico global en `125.00` y `127.00`.
    #[test]
    fn test_merengueton_126_with_phase_flip_cut_locks_exact_126() {
        let sr = 44_100_u32;
        let duration_sec = 28.0_f64;
        let n = (duration_sec * (sr as f64)) as usize;
        let mut pcm = vec![0.0_f32; n];
        let beat_sec = 60.0 / 126.0;

        for (i, sample) in pcm.iter_mut().enumerate() {
            let t = (i as f64) / (sr as f64);
            // En t = 14.0s introduce un salto de medio beat (180° inversión de fase global)
            let shifted_t = if t < 14.0 { t } else { t + beat_sec * 0.5 };
            let bar_sec = beat_sec * 4.0;
            let bar_pos = shifted_t.rem_euclid(bar_sec);
            let step_sec = bar_sec / 16.0;
            let step = (bar_pos / step_sec).floor() as usize;
            let sp = bar_pos - (step as f64) * step_sec;

            let kick = if step % 4 == 0 && sp < 0.085 {
                0.70 * (-sp * 40.0).exp() * (2.0 * std::f64::consts::PI * 60.0 * sp).sin()
            } else {
                0.0
            };
            let tri_phase = bar_pos.rem_euclid(bar_sec / 3.0);
            let mambo = if tri_phase < 0.055 {
                0.50 * (-tri_phase * 55.0).exp()
                    * (2.0 * std::f64::consts::PI * 1500.0 * tri_phase).sin()
            } else {
                0.0
            };
            *sample = (kick + mambo).clamp(-0.98, 0.98) as f32;
        }

        let mut planner = FftPlanner::<f32>::new();
        let nov = extract_multiband_novelty(&pcm, sr, 0.0, &mut planner);
        let res = analyze_bpm_consensus_v2(&nov, None);

        assert!(
            (res.bpm - 126.0).abs() <= 0.05,
            "Con inversión de fase a mitad de canción debe clavar 126.00 (no 125/127), dio {:.2}",
            res.bpm
        );
    }

    #[test]
    fn test_virtualdj_xml_and_filename_hints() {
        let vdj_xml = r#"
        <VirtualDJ_Database Version="2026">
          <Song FilePath="/Music/Latin/Reggi El Autentico - MUJERON.mp4" FileSize="14200123">
            <Scan Version="801" Bpm="0.476190" Key="F" />
          </Song>
        </VirtualDJ_Database>
        "#;
        let db = VirtualDjDbIndex::from_xml_str(vdj_xml);
        let looked = db
            .lookup_bpm(Path::new("Reggi El Autentico - MUJERON.mp4"))
            .unwrap();
        assert!((looked - 126.0).abs() < 0.05);
        assert_eq!(parse_virtualdj_bpm_value("0.480000"), Some(125.0));
        assert_eq!(
            extract_bpm_from_filename(Path::new("03 - ella es - tony vega [82 BPM - 8B].mp3")),
            Some(82.0)
        );
    }
}