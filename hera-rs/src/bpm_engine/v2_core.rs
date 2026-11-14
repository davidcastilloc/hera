#![allow(dead_code, unused_imports, clippy::all)]
//! # HERA BPM & Key Engine v2 (`hera-rs/src/bpm_engine/`) — Arquitectura Mixxx QM-DSP + Ritmos Latinos
//!
//! Implementación **100% Pure Rust** (`symphonia` + `rustfft`, cero dependencias de Python/ONNX)
//! diseñada específicamente para cerrar el benchmark de cabina de HERA de **11/12 a 12/12** y
//! eliminar la deriva decimal (`117→119.5`, `123→121.8`, `126→125.24`), además de sumar detección
//! de tono armónico **Camelot (`1A`–`12B`)** y cola paralela masiva estilo VirtualDJ.
//!
//! ---
//! ## PROTOCOLO DE EJECUCIÓN PARA GOOGLE ANTIGRAVITY Y AGENTES DE CÓDIGO
//!
//! ### Fase 0 — Auditoría Bloqueante de Solo Lectura (*Inspect First, Merge Deltas Only*)
//! Antes de tocar cualquier archivo en `hera-rs`:
//! 1. Ejecuta `git status` y `git log -n 3 --oneline` para verificar el estado actual sobre `3fd0006`
//!    y leer las firmas públicas actuales en:
//!    - `hera-rs/src/bpm_engine/mod.rs`
//!    - `hera-rs/src/bpm_engine/multiband_onset.rs`
//!    - `hera-rs/src/bpm_engine/window_tracker.rs`
//!    - `hera-rs/src/bpm_engine/latin_heuristics.rs`
//!    - `hera-rs/src/bpm_engine/consensus_ai.rs`
//!    - `hera-rs/src/structure.rs`
//! 2. **PROHIBIDO sobrescribir a ciegas** structs o traits consumidos por `hera-desktop`, el servidor
//!    MCP de 21 herramientas o los exportadores Rekordbox XML / Traktor NML / VirtualDJ. Mantén la
//!    firma pública existente de `analyze_bpm_consensus` como *wrapper* si otros módulos ya la llaman,
//!    delegando internamente hacia [`consensus_ai::analyze_bpm_consensus_v2`].
//! 3. Aplica los cambios en iteraciones atómicas por submódulo (50–200 líneas por paso) y valida con
//!    `cargo test` / `cargo nextest run` tras cada delta (*Receipt-Driven Development*).
//!
//! ### Mapa Exacto de Submódulos hacia `hera-rs`
//! | Submódulo en este archivo | Destino en `hera-rs` | Brecha del Benchmark que Resuelve |
//! | :--- | :--- | :--- |
//! | `pub mod hints` | `src/bpm_engine/hints.rs` + `src/structure.rs` | **Brecha 4:** Extrae `bpm_hint` real desde ID3 `TBPM`, nombre `[126 BPM]` e índice `database.xml` de VirtualDJ (`60.0 / Scan@Bpm`). |
//! | `pub mod audio_ingest` | `src/bpm_engine/audio_ingest.rs` | **Brecha 3:** Selecciona explícitamente el *stream* de audio en MP4 (ignorando el track 0 H.264) y cae a *pipe* `ffmpeg f32le` en `.webm`/`.mp4` de VideoDJ. |
//! | `pub mod multiband_onset` | `src/bpm_engine/multiband_onset.rs` | Conserva las 4 bandas exactas de HERA (`25–130`, `130–450`, `650–3800`, `4200–11000 Hz`) a `172.27 FPS` (`hop=256 @ 44.1 kHz`) con blanqueo adaptativo. |
//! | `pub mod qm_comb_tracker` | `src/bpm_engine/window_tracker.rs` | Reemplaza la mediana cuantizada por el banco de filtros peine resonadores de **Mixxx / Queen Mary (`qm-dsp` TempoTrackV2)** + Viterbi + `FineGridPhaseLock` (`±0.01 BPM` y snap `25 ms`). |
//! | `pub mod latin_heuristics` | `src/bpm_engine/latin_heuristics.rs` | **Brechas 1 y 2:** Matriz de 16 pasos por compás (`GrooveFingerprint`), `FourOnTheFloorVeto` sobre `sub_kick` aislado (cura `MUJERON` 126 BPM → nunca cae a `95.22`) y detector de Tumbao anticipado (Salsa vs Reggaetón). |
//! | `pub mod consensus_ai` | `src/bpm_engine/consensus_ai.rs` + `mod.rs` | Evalúa hipótesis `{1x, 0.5x, 2x, 0.75x, 1.333x, 1.5x}` combinando peine QM, fase aislada de `sub_kick`, firma de 16 pasos y voto Bayesiano de `bpm_hint`. |
//! | `pub mod key_engine` | `src/key_engine/mod.rs` (nuevo o existente) | HPCP de 36 bines con *Harmonic Gating*, corrección A440, ensemble `BGATE`+`EDMA`+`Temperley`+`Sha'ath`, chequeo de modo paralelo y salida **Camelot (`1A`–`12B`)**. |
//! | `pub mod queue` | `src/bpm_engine/queue.rs` | Cola masiva tipo VirtualDJ (`CPU - 1` hilos, *backpressure* acotado, deduplicación FNV-128 y vaciado por lotes a SQLite WAL). |

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
// CONSTANTES GLOBALES DEL MOTOR DSP (CALIBRADAS A 44.1 kHz / 172.265625 FPS)
// ============================================================================

/// Frecuencia interna canónica de análisis para preservar precisión de fase sub-milisegundo.
pub const ANALYSIS_SR: u32 = 44_100;
/// Ventana STFT de 1024 muestras (~23.2 ms @ 44.1 kHz, resolución de bin = 43.066 Hz;
/// complementada con filtro IIR Butterworth de 2do orden para aislar `sub_kick` 25–130 Hz con nitidez).
pub const ONSET_FFT_SIZE: usize = 1024;
/// Salto de 256 muestras (`44100 / 256 = 172.265625 FPS`, resolución temporal = `5.805 ms`).
pub const ONSET_HOP_SIZE: usize = 256;
/// Tasa exacta de cuadros de *novelty* por segundo.
pub const ONSET_FPS: f64 = (ANALYSIS_SR as f64) / (ONSET_HOP_SIZE as f64);
/// Umbral psicoacústico de Mixxx (`BeatUtils::kMaxSecsPhaseError = 25 ms`) para anclar BPM entero.
pub const MIXXX_MAX_PHASE_ERR_SEC: f64 = 0.025;
/// Resolución del vector tonal HPCP (36 bines = 3 bines por semitono = 33.3 cents).
pub const HPCP_BINS: usize = 36;
pub const HPCP_BINS_PER_SEMITONE: usize = 3;

/// Errores unificados del motor de análisis de HERA.
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
            Self::FfmpegFallbackFailed(e) => write!(f, "Fallback FFmpeg para VideoDJ falló: {}", e),
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

    /// Origen trazable de un `bpm_hint` para ponderar su peso en `consensus_ai`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum HintSource {
        /// Etiqueta `<Scan Bpm="..."/>` de `database.xml` de VirtualDJ (alta fiabilidad).
        VirtualDjDatabase,
        /// Tag embebido `TBPM` / `BPM` / `tmpo` en el contenedor.
        EmbeddedTag,
        /// Patrón explícito en nombre de archivo (`[126 BPM]`, `125bpm`, etc.).
        FilenamePattern,
        /// Hint pasado manualmente por el usuario o `structure.rs`.
        UserProvided,
    }

    impl HintSource {
        pub fn prior_weight(self) -> f64 {
            match self {
                Self::UserProvided => 0.22,
                Self::VirtualDjDatabase => 0.16,
                Self::EmbeddedTag => 0.14,
                Self::FilenamePattern => 0.11,
            }
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct BpmHint {
        pub bpm: f64,
        pub source: HintSource,
    }

    /// Convierte el valor crudo de `Bpm` en VirtualDJ `database.xml`.
    /// ¡CRÍTICO!: En `<Scan Bpm="0.476190"/>`, VirtualDJ almacena **segundos por beat** (`60.0 / BPM`),
    /// mientras que en `<Tags Bpm="126.0"/>` almacena BPM directo.
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

    /// Extrae BPM explícitos desde nombres de archivo DJ (ej. `"Reggi - Mujeron [126 BPM].mp3"`,
    /// `"Track (95bpm).mp4"`, `"Promo_128_BPM_Extended.wav"`) sin confundir años (`2025`),
    /// números de pista (`01 - `) ni bitrates (`320kbps`).
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
                // Avanza espacios, guiones o guiones bajos opcionales antes de "bpm"
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
        None
    }

    /// Índice ligero en memoria de `database.xml` de VirtualDJ para enriquecer colas masivas en O(1).
    #[derive(Debug, Default, Clone)]
    pub struct VirtualDjDbIndex {
        by_normalized_path: HashMap<String, (Option<f64>, Option<String>)>,
        by_filename_lower: HashMap<String, (Option<f64>, Option<String>)>,
    }

    impl VirtualDjDbIndex {
        pub fn from_xml_file(xml_path: &Path) -> Result<Self, EngineError> {
            let content = fs::read_to_string(xml_path).map_err(|e| EngineError::Io(e.to_string()))?;
            Ok(Self::from_xml_str(&content))
        }

        /// Escáner sin dependencias externas para bloques `<Song FilePath="..."> ... </Song>`.
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
                    let norm = normalize_path_key(&fp);
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
            let norm = normalize_path_key(&track_path.to_string_lossy());
            if let Some((Some(bpm), _)) = self.by_normalized_path.get(&norm) {
                return Some(*bpm);
            }
            let fname = track_path.file_name()?.to_str()?.to_ascii_lowercase();
            self.by_filename_lower.get(&fname).and_then(|(b, _)| *b)
        }
    }

    fn normalize_path_key(p: &str) -> String {
        p.replace('\\', "/").to_ascii_lowercase()
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

    /// Resuelve el mejor `BpmHint` disponible priorizando: explícito > VirtualDJ DB > Tag ID3 > Nombre de archivo.
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
// 2. SUBMÓDULO `audio_ingest` — SYMPHONIA AUDIO-FIRST + FALLBACK FFMPEG VIDEODJ
// ============================================================================

pub mod audio_ingest {
    use super::*;
    use std::io::Read;

    #[derive(Debug, Clone)]
    pub struct DecodedAudio {
        /// Muestras mono normalizadas a `ANALYSIS_SR` (`44,100 Hz`).
        pub samples_44k: Vec<f32>,
        pub duration_sec: f64,
        pub embedded_bpm_tag: Option<f64>,
        pub embedded_key_tag: Option<String>,
        pub used_ffmpeg_fallback: bool,
    }

    /// Decodifica cualquier archivo de audio o VideoDJ (`.mp3`, `.flac`, `.wav`, `.aiff`, `.m4a`,
    /// `.mp4`, `.webm`, `.mkv`, `.mov`).
    ///
    /// ### Correcciones clave frente al uso estándar de `symphonia`:
    /// 1. En contenedores `.mp4` de VideoDJ, `format.default_track()` suele devolver la pista 0 de
    ///    video (`avc1`/`hev1`) provocando error inmediato. Aquí filtramos explícitamente la primera
    ///    pista con `sample_rate.is_some()` y codec de audio válido.
    /// 2. Si el contenedor es `.webm`/`.mkv` con Opus o `symphonia` falla al abrir el demuxer,
    ///    ejecuta automáticamente un *streaming pipe* hacia `ffmpeg` en memoria (`f32le` a `44100 Hz`)
    ///    sin escribir archivos temporales en disco.
    pub fn load_audio_mono_44k(path: &Path) -> Result<DecodedAudio, EngineError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // `.webm`, `.mkv`, `.flv` y `.avi` de VideoDJ rara vez son soportados completos en symphonia puro
        let force_video_pipe = matches!(ext.as_str(), "webm" | "mkv" | "flv" | "avi" | "ts");

        if !force_video_pipe {
            if let Ok(decoded) = try_decode_with_symphonia(path) {
                if !decoded.samples_44k.is_empty() {
                    return Ok(decoded);
                }
            }
        }

        // Fallback automático a pipe FFmpeg para VideoDJ (.mp4 complejos, .webm, .mov, HE-AAC, Opus)
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

        // Extrae tags TBPM / TKEY desde el contenedor o el bloque ID3/MP4 inicial
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

        // Selección explícita de pista de AUDIO (evita pistar H.264/VP9 en contenedores MP4/MOV)
        let audio_track = probed
            .format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params.codec != CODEC_TYPE_NULL && t.codec_params.sample_rate.is_some()
            })
            .ok_or_else(|| {
                EngineError::DemuxOrDecode("Contenedor sin pista de audio decodificable".into())
            })?;

        let track_id = audio_track.id;
        let src_sr = audio_track.codec_params.sample_rate.unwrap_or(ANALYSIS_SR);

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
                    let ch = spec.channels.count().max(1);
                    let cap = decoded.capacity() as u64;
                    let sbuf = sample_buf.get_or_insert_with(|| SampleBuffer::<f32>::new(cap, spec));
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

        let duration_sec = (raw_mono.len() as f64) / (src_sr.max(1) as f64);
        let samples_44k = resample_polyphase(&raw_mono, src_sr, ANALYSIS_SR);

        Ok(DecodedAudio {
            samples_44k,
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

    /// Extrae únicamente el *stream* de audio de `.mp4`, `.webm`, `.mkv` o `.mov` vía `ffmpeg`
    /// directamente hacia memoria (`f32le` mono @ `44,100 Hz`), ignorando la decodificación de video.
    pub fn decode_via_ffmpeg_pipe(path: &Path) -> Result<DecodedAudio, EngineError> {
        let mut child = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-nostdin",
                "-vn", // Ignora completamente el video H.264/VP9/AV1 -> decodifica ~50x más rápido
                "-i",
            ])
            .arg(path)
            .args([
                "-ac",
                "1",
                "-ar",
                "44100",
                "-f",
                "f32le",
                "-acodec",
                "pcm_f32le",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                EngineError::FfmpegFallbackFailed(format!(
                    "No se pudo iniciar `ffmpeg` en PATH para contenedor VideoDJ: {}",
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
        let mut samples_44k = Vec::with_capacity(num_samples);
        for chunk in raw_bytes.chunks_exact(4) {
            let s = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            samples_44k.push(if s.is_finite() { s } else { 0.0 });
        }

        let duration_sec = (samples_44k.len() as f64) / (ANALYSIS_SR as f64);
        Ok(DecodedAudio {
            samples_44k,
            duration_sec,
            embedded_bpm_tag: None,
            embedded_key_tag: None,
            used_ffmpeg_fallback: true,
        })
    }

    /// Resampleador sinc Blackman de fase lineal cuando `src_sr != 44,100 Hz` (ej. 48,000 Hz en VideoDJ).
    pub fn resample_polyphase(samples: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
        if in_rate == out_rate || samples.is_empty() {
            return samples.to_vec();
        }
        let ratio = (in_rate as f64) / (out_rate as f64);
        let out_len = ((samples.len() as f64) / ratio).floor() as usize;
        let cutoff = (0.46 / ratio).min(0.46);
        let radius: isize = 10;
        let n = samples.len() as isize;
        let mut out = Vec::with_capacity(out_len);

        for i in 0..out_len {
            let src_pos = (i as f64) * ratio;
            let center = src_pos.floor() as isize;
            let mut sum = 0.0_f64;
            let mut w_sum = 0.0_f64;
            for k in (center - radius)..=(center + radius) {
                if k >= 0 && k < n {
                    let dx = src_pos - (k as f64);
                    let arg = 2.0 * std::f64::consts::PI * cutoff * dx;
                    let sinc = if arg.abs() < 1e-9 { 1.0 } else { arg.sin() / arg };
                    let w_pos = (dx / ((radius + 1) as f64) + 1.0) * 0.5;
                    let win = 0.42 - 0.5 * (2.0 * std::f64::consts::PI * w_pos).cos()
                        + 0.08 * (4.0 * std::f64::consts::PI * w_pos).cos();
                    let w = sinc * win;
                    sum += (samples[k as usize] as f64) * w;
                    w_sum += w;
                }
            }
            out.push(if w_sum.abs() > 1e-9 {
                (sum / w_sum) as f32
            } else {
                0.0
            });
        }
        out
    }
}

// ============================================================================
// 3. SUBMÓDULO `multiband_onset` — LAS 4 BANDAS EXACTAS DE HERA + NOVELTY PURA
// ============================================================================

pub mod multiband_onset {
    use super::*;

    /// Envolventes de *novelty* alineadas frame a frame (`172.265625 FPS`) en las 4 bandas de HERA:
    /// - `sub_kick`: `25–130 Hz` (bombo 4/4, sub-bajo de dembow y tumbao grave)
    /// - `low_body`: `130–450 Hz` (cuerpo de tambora, conga/ponche, fundamental de caja y bajo)
    /// - `perc_clave`: `650–3800 Hz` (clave de salsa, mambo/tresillo sintético, rimshot/snare)
    /// - `hi_guiro`: `4200–11000 Hz` (güira dominicana, maracas, campana aguda, hi-hats)
    #[derive(Debug, Clone)]
    pub struct MultibandNovelty {
        pub sub_kick: Vec<f64>,
        pub low_body: Vec<f64>,
        pub perc_clave: Vec<f64>,
        pub hi_guiro: Vec<f64>,
        /// Combinación ponderada para el rastreador global de beats tipo Queen Mary.
        pub composite: Vec<f64>,
        pub num_frames: usize,
        pub trim_offset_sec: f64,
        /// Proporciones normalizadas de actividad por banda (conservadas para telemetría).
        pub band_ratios: [f64; 4],
    }

    /// Recorta silencios extremos (`< -60 dBFS`) para no contaminar intros mudas o colas de video.
    pub fn trim_leading_trailing_silence(samples_44k: &[f32]) -> (&[f32], f64, f64) {
        let win = ANALYSIS_SR as usize / 50; // 20 ms
        let thresh = 0.001_f32;
        let n_wins = samples_44k.len() / win;
        if n_wins < 8 {
            return (samples_44k, 0.0, 0.0);
        }

        let mut start_w = 0;
        let mut run = 0;
        for w in 0..n_wins {
            let sl = &samples_44k[w * win..(w + 1) * win];
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
            let sl = &samples_44k[w * win..(w + 1) * win];
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
        let e_idx = (end_w * win).max(s_idx).min(samples_44k.len());
        let lead_sec = (s_idx as f64) / (ANALYSIS_SR as f64);
        let trail_sec = ((samples_44k.len() - e_idx) as f64) / (ANALYSIS_SR as f64);
        (&samples_44k[s_idx..e_idx], lead_sec, trail_sec)
    }

    /// Extrae las 4 curvas de *novelty* combinando un filtro paso-banda IIR dedicado de alta
    /// resolución temporal para `sub_kick` (`25–130 Hz`, evitando el emborronamiento de bines STFT
    /// gruesos en graves) y flujo espectral logarítmico STFT para las otras 3 bandas.
    pub fn extract_multiband_novelty(
        trimmed_44k: &[f32],
        trim_offset_sec: f64,
        fft_planner: &mut FftPlanner<f32>,
    ) -> MultibandNovelty {
        if trimmed_44k.len() < ONSET_FFT_SIZE {
            return MultibandNovelty {
                sub_kick: Vec::new(),
                low_body: Vec::new(),
                perc_clave: Vec::new(),
                hi_guiro: Vec::new(),
                composite: Vec::new(),
                num_frames: 0,
                trim_offset_sec,
                band_ratios: [0.25; 4],
            };
        }

        let num_frames = (trimmed_44k.len() - ONSET_FFT_SIZE) / ONSET_HOP_SIZE + 1;

        // 1. Canal IIR Butterworth (paso-alto 25 Hz + paso-bajo 130 Hz) para aislar el golpe exacto del bombo
        let sub_kick_pcm = isolate_sub_kick_band_iir(trimmed_44k, ANALYSIS_SR as f64);
        let mut sub_env = vec![0.0_f64; num_frames];
        for f in 0..num_frames {
            let start = f * ONSET_HOP_SIZE;
            let slice = &sub_kick_pcm[start..start + ONSET_FFT_SIZE];
            let energy = slice.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>()
                / (ONSET_FFT_SIZE as f64);
            sub_env[f] = (1.0 + 4000.0 * energy.sqrt()).ln();
        }

        // Derivada rectificada de media onda para `sub_kick`
        let mut raw_sub = vec![0.0_f64; num_frames];
        for f in 1..num_frames {
            raw_sub[f] = (sub_env[f] - sub_env[f - 1]).max(0.0);
        }

        // 2. STFT de 1024 muestras (`43.066 Hz/bin`) para `low_body`, `perc_clave` y `hi_guiro`
        // Bines @ 44.1 kHz / 1024:
        // - low_body (130–450 Hz): bines 3..=10
        // - perc_clave (650–3800 Hz): bines 15..=88
        // - hi_guiro (4200–11000 Hz): bines 98..=255
        let fft = fft_planner.plan_fft_forward(ONSET_FFT_SIZE);
        let hann: Vec<f32> = (0..ONSET_FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * (i as f32) / (ONSET_FFT_SIZE as f32)).cos()))
            .collect();

        let n_bins = ONSET_FFT_SIZE / 2 + 1;
        let mut fft_buf = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; ONSET_FFT_SIZE];
        let mut prev_log_mag = vec![0.0_f32; n_bins];

        let mut raw_body = vec![0.0_f64; num_frames];
        let mut raw_clave = vec![0.0_f64; num_frames];
        let mut raw_guiro = vec![0.0_f64; num_frames];

        for f in 0..num_frames {
            let offset = f * ONSET_HOP_SIZE;
            for i in 0..ONSET_FFT_SIZE {
                fft_buf[i] = Complex {
                    re: trimmed_44k[offset + i] * hann[i],
                    im: 0.0,
                };
            }
            fft.process(&mut fft_buf);

            let mut flux_body = 0.0_f32;
            let mut flux_clave = 0.0_f32;
            let mut flux_guiro = 0.0_f32;

            for k in 3..=255.min(n_bins - 1) {
                let log_m = (1.0 + 80.0 * fft_buf[k].norm()).ln();
                let diff = (log_m - prev_log_mag[k]).max(0.0);
                prev_log_mag[k] = log_m;

                if f > 0 {
                    if (3..=10).contains(&k) {
                        flux_body += diff;
                    } else if (15..=88).contains(&k) {
                        flux_clave += diff;
                    } else if (98..=255).contains(&k) {
                        flux_guiro += diff;
                    }
                }
            }
            raw_body[f] = flux_body as f64;
            raw_clave[f] = flux_clave as f64;
            raw_guiro[f] = flux_guiro as f64;
        }

        let sum_s = raw_sub.iter().sum::<f64>();
        let sum_b = raw_body.iter().sum::<f64>() / 8.0;
        let sum_c = raw_clave.iter().sum::<f64>() / 74.0;
        let sum_g = raw_guiro.iter().sum::<f64>() / 158.0;
        let total_e = (sum_s + sum_b + sum_c + sum_g).max(1e-9);
        let band_ratios = [
            sum_s / total_e,
            sum_b / total_e,
            sum_c / total_e,
            sum_g / total_e,
        ];

        // Normalización sustractiva local (~0.35 s = 60 frames) + escalado por desviación estándar
        let sub_kick = normalize_novelty_curve(&raw_sub, 30);
        let low_body = normalize_novelty_curve(&raw_body, 30);
        let perc_clave = normalize_novelty_curve(&raw_clave, 30);
        let hi_guiro = normalize_novelty_curve(&raw_guiro, 30);

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
            trim_offset_sec,
            band_ratios,
        }
    }

    /// Filtro biquad en cascada (HPF 25 Hz + LPF 130 Hz) libre de asignaciones extra.
    fn isolate_sub_kick_band_iir(input: &[f32], sr: f64) -> Vec<f32> {
        let mut out = Vec::with_capacity(input.len());
        // Paso-alto 1er orden @ 25 Hz
        let rc_hp = 1.0 / (2.0 * std::f64::consts::PI * 25.0);
        let dt = 1.0 / sr;
        let alpha_hp = rc_hp / (rc_hp + dt);

        // Paso-bajo Butterworth 2do orden @ 130 Hz
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
// 4. SUBMÓDULO `qm_comb_tracker` — MIXXX / QUEEN MARY MULTI-LAG COMB + FASE FINA
// ============================================================================

pub mod qm_comb_tracker {
    use super::multiband_onset::MultibandNovelty;
    use super::*;

    /// Resultado del rastreador tipo Queen Mary (`TempoTrackV2` + `BeatUtils` de Mixxx).
    #[derive(Debug, Clone)]
    pub struct QmTempoAnalysis {
        /// Candidatos principales ordenados por energía del banco peine resonador `(bpm, comb_score)`.
        pub top_comb_peaks: Vec<(f64, f64)>,
        /// Trayectoria de periodo óptimo por ventana obtenida mediante decodificación de Viterbi.
        pub viterbi_bpm: f64,
        /// Energía de peine evaluada exclusivamente sobre `sub_kick` (para `FourOnTheFloorVeto`).
        pub sub_kick_comb_by_lag: Vec<f64>,
        /// Energía de peine compuesta en todos los lags `min_lag..=max_lag`.
        pub composite_comb_by_lag: Vec<f64>,
        pub min_lag: usize,
        pub max_lag: usize,
    }

    /// Implementa la etapa central de **Queen Mary `TempoTrackV2` (Davies & Plumbley)** usada en Mixxx:
    /// 1. Ventanas de ~6 segundos (`1024 frames @ 172.27 FPS`) con solapamiento de 75% (`step = 256`).
    /// 2. Autocorrelación normalizada pasada por un **banco de filtros peine de 4 armónicos** (`1x, 2x, 3x, 4x`)
    ///    ponderado por distribución de Rayleigh centrada en `~120 BPM` (`lag0 = 86.1 frames`).
    /// 3. **Algoritmo de Viterbi** sobre la matriz tiempo-lag para encontrar la trayectoria continua de tempo
    ///    sin los saltos discretos ni errores de cuantización de `median_bpm`.
    pub fn analyze_qm_comb_tempogram(nov: &MultibandNovelty) -> QmTempoAnalysis {
        // Rango amplio 55..215 BPM @ 172.265625 FPS -> lags 48..=188 frames
        let min_lag = ((60.0 * ONSET_FPS) / 215.0).floor() as usize; // 48
        let max_lag = ((60.0 * ONSET_FPS) / 55.0).ceil() as usize; // 188
        let num_lags = max_lag + 1;

        let win_len = 1024_usize.min(nov.num_frames);
        let win_hop = 256_usize;
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
        // Parámetro Rayleigh de Davies & Plumbley (~120 BPM -> beta = 86.13 frames)
        let beta = (60.0 * ONSET_FPS) / 120.0;

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

            // Solo saltamos ventanas totalmente mudas (intro/outro en silencio)
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

            // Banco de filtros peine resonadores de Davies & Plumbley (armónicos m = 1..=4 con ancho proporcional a m)
            for lag in min_lag..=max_lag {
                let mut comb_c = 0.0_f64;
                let mut comb_s = 0.0_f64;
                for m in 1..=4_usize {
                    let center = lag * m;
                    if center > max_acf_lag {
                        break;
                    }
                    let weight_m = 1.0 / (m as f64); // Peso decreciente 1, 1/2, 1/3, 1/4
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

                // Ponderación perceptual suave de Rayleigh (evita extremos irreales sin aplastar 82 o 160 BPM)
                let l_f = lag as f64;
                let rayleigh = (l_f / (beta * beta)) * (-0.5 * (l_f / beta).powi(2)).exp();
                let mild_prior = 0.65 + 0.35 * (rayleigh / (0.6065 / beta));

                let weighted_c = comb_c * mild_prior;
                tempogram[w][lag] = weighted_c;
                global_comp_comb[lag] += weighted_c;
                global_sub_comb[lag] += comb_s;
            }
        }

        // Decodificación de Viterbi sobre `tempogram[w][lag]` (penaliza saltos bruscos de lag entre ventanas)
        let viterbi_lag_f = decode_viterbi_tempo_path(&tempogram, min_lag, max_lag);
        let viterbi_bpm = (60.0 * ONSET_FPS) / viterbi_lag_f;

        // Extrae hasta 5 picos locales prominentes del espectro peine global (con interpolación parabólica)
        let mut peaks: Vec<(f64, f64)> = Vec::new();
        let max_g = global_comp_comb[min_lag..=max_lag]
            .iter()
            .copied()
            .fold(1e-9_f64, f64::max);

        for lag in (min_lag + 1)..max_lag {
            let y0 = global_comp_comb[lag - 1];
            let y1 = global_comp_comb[lag];
            let y2 = global_comp_comb[lag + 1];
            if y1 > y0 && y1 >= y2 && y1 > max_g * 0.25 {
                let denom = y0 - 2.0 * y1 + y2;
                let delta = if denom.abs() > 1e-12 {
                    (0.5 * (y0 - y2) / denom).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                let exact_lag = (lag as f64) + delta;
                let bpm = (60.0 * ONSET_FPS) / exact_lag;
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

        let mut path_lags = Vec::with_capacity(n_wins);
        for w in (0..n_wins).rev() {
            path_lags.push(curr as f64);
            curr = back[w][curr];
        }
        super::median_f64(&mut path_lags)
    }

    /// **El arma secreta contra el drift (`126→125.24`, `117→119.5`, `123→121.8`):**
    /// Una vez elegido el BPM aproximado, barre una malla fina de `±1.8 BPM` a pasos de `0.01 BPM`
    /// evaluando la resonancia coherente de fase a lo largo de **toda** la canción completa, y aplica
    /// la regla física de Mixxx (`kMaxSecsPhaseError = 25 ms`, `mixxx#14705`) para anclar enteros de DAW.
    pub fn fine_grid_phase_lock_and_beats(
        approx_bpm: f64,
        nov: &MultibandNovelty,
        prefer_kick_anchor: bool,
    ) -> (f64, f64, Vec<f64>, f64) {
        let duration_sec = (nov.num_frames as f64) / ONSET_FPS;
        if nov.num_frames < 64 || approx_bpm <= 30.0 {
            return (approx_bpm, nov.trim_offset_sec, Vec::new(), 0.0);
        }

        // Extrae solo los picos significativos de novelty (> percentil alto) para evaluar miles de candidatos en <2 ms
        let mut sparse_peaks: Vec<(f64, f64)> = Vec::new();
        for i in 1..(nov.num_frames - 1) {
            let val = if prefer_kick_anchor {
                0.62 * nov.sub_kick[i] + 0.22 * nov.low_body[i] + 0.16 * nov.perc_clave[i]
            } else {
                nov.composite[i]
            };
            if val > 0.85 {
                let prev = if prefer_kick_anchor {
                    0.62 * nov.sub_kick[i - 1]
                        + 0.22 * nov.low_body[i - 1]
                        + 0.16 * nov.perc_clave[i - 1]
                } else {
                    nov.composite[i - 1]
                };
                let next = if prefer_kick_anchor {
                    0.62 * nov.sub_kick[i + 1]
                        + 0.22 * nov.low_body[i + 1]
                        + 0.16 * nov.perc_clave[i + 1]
                } else {
                    nov.composite[i + 1]
                };
                if val >= prev && val >= next {
                    let denom = prev - 2.0 * val + next;
                    let delta = if denom.abs() > 1e-12 {
                        (0.5 * (prev - next) / denom).clamp(-0.5, 0.5)
                    } else {
                        0.0
                    };
                    let t_sec = ((i as f64) + delta) / ONSET_FPS;
                    sparse_peaks.push((t_sec, val));
                }
            }
        }

        if sparse_peaks.len() < 8 {
            return (approx_bpm, nov.trim_offset_sec, Vec::new(), 0.0);
        }

        let weight_sum: f64 = sparse_peaks.iter().map(|p| p.1).sum::<f64>().max(1e-9);
        // Barrido en dos etapas: primero paso de 0.10 BPM en ±2.2 BPM, luego paso fino de 0.01 BPM en ±0.25 BPM
        let mut best_bpm = approx_bpm;
        let mut best_coherence = -1.0_f64;
        let mut best_phase_rad = 0.0_f64;

        let lo_coarse = (approx_bpm - 2.2).max(45.0);
        let hi_coarse = (approx_bpm + 2.2).min(225.0);
        let mut b = lo_coarse;
        while b <= hi_coarse {
            let (coh, ang) = phase_vector_coherence(&sparse_peaks, b, weight_sum);
            if coh > best_coherence {
                best_coherence = coh;
                best_bpm = b;
                best_phase_rad = ang;
            }
            b += 0.08;
        }

        let lo_fine = (best_bpm - 0.24).max(45.0);
        let hi_fine = (best_bpm + 0.24).min(225.0);
        let mut bf = (lo_fine * 100.0).round() / 100.0;
        while bf <= hi_fine {
            let (coh, ang) = phase_vector_coherence(&sparse_peaks, bf, weight_sum);
            if coh > best_coherence {
                best_coherence = coh;
                best_bpm = bf;
                best_phase_rad = ang;
            }
            bf += 0.01;
        }

        // Regla Mixxx (`kMaxSecsPhaseError = 25 ms` o coherencia >= 98.5% del óptimo) para pistas de DAW enteras
        let nearest_int = best_bpm.round();
        let (int_coh, int_ang) = phase_vector_coherence(&sparse_peaks, nearest_int, weight_sum);
        let total_beats_in_track = (duration_sec * best_bpm / 60.0).max(1.0);
        let end_to_end_drift_sec =
            total_beats_in_track * ((60.0 / best_bpm) - (60.0 / nearest_int)).abs();

        if (end_to_end_drift_sec <= MIXXX_MAX_PHASE_ERR_SEC * 2.0 && int_coh >= best_coherence * 0.96)
            || ((best_bpm - nearest_int).abs() <= 0.12 && int_coh >= best_coherence * 0.985)
        {
            best_bpm = nearest_int;
            best_coherence = int_coh;
            best_phase_rad = int_ang;
        } else {
            best_bpm = (best_bpm * 100.0).round() / 100.0;
        }

        let period_sec = 60.0 / best_bpm;
        let phase_offset_local =
            (best_phase_rad.rem_euclid(2.0 * std::f64::consts::PI) / (2.0 * std::f64::consts::PI))
                * period_sec;

        // Construye la rejilla de beats en segundos absolutos
        let mut beats_abs = Vec::new();
        let mut t = phase_offset_local;
        while t < duration_sec {
            beats_abs.push(nov.trim_offset_sec + t);
            t += period_sec;
        }
        let first_beat_sec = beats_abs.first().copied().unwrap_or(nov.trim_offset_sec);

        (best_bpm, first_beat_sec, beats_abs, best_coherence)
    }

    /// Calcula el módulo de coherencia circular de Kuramoto `|sum w_i * exp(j * 2*pi * t_i * bpm / 60)| / sum w_i`
    /// sumando armónico fundamental y 2do armónico para capturar subdivisión limpia.
    #[inline]
    fn phase_vector_coherence(
        peaks: &[(f64, f64)],
        bpm: f64,
        weight_sum: f64,
    ) -> (f64, f64) {
        let freq = bpm / 60.0;
        let two_pi_f = 2.0 * std::f64::consts::PI * freq;
        let mut re1 = 0.0_f64;
        let mut im1 = 0.0_f64;
        let mut re2 = 0.0_f64;
        let mut im2 = 0.0_f64;

        for &(t, w) in peaks {
            let theta = two_pi_f * t;
            let (s1, c1) = theta.sin_cos();
            re1 += w * c1;
            im1 += w * s1;
            let (s2, c2) = (2.0 * theta).sin_cos();
            re2 += w * c2;
            im2 += w * s2;
        }

        let r1 = (re1 * re1 + im1 * im1).sqrt() / weight_sum;
        let r2 = (re2 * re2 + im2 * im2).sqrt() / weight_sum;
        let angle = im1.atan2(re1);
        (0.78 * r1 + 0.22 * r2, angle)
    }
}

// ============================================================================
// 5. SUBMÓDULO `latin_heuristics` — MATRIZ 16-PASOS + VETO 4/4 + SALSA VS DEMBOW
// ============================================================================

pub mod latin_heuristics {
    use super::multiband_onset::MultibandNovelty;
    use super::*;

    /// Clasificación rítmica estructural inferida desde la geometría de las 4 bandas en el compás.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum GrooveStyle {
        /// Bombo cuaternario 4/4 dominante (House, Afro House, Tech House, Techno, Dance Pop).
        FourOnTheFloorClub,
        /// Bombo 4/4 + síncopa/tresillo fuerte en `perc_clave`/`hi_guiro` (Merenguetón, Guaracha, Urbano 120–132).
        UrbanLatinFourFourHybrid,
        /// Patrón Dembow clásico (bombo en `0,4,8,12` + snare en `3,6,11,14`, rango 82–105 BPM).
        ReggaetonDembow,
        /// Tumbao de bajo anticipado (`2-&` y `4`, hueco en `1`) + clave/campana (Salsa 78–108 BPM).
        SalsaTumbao,
        /// Güira continua de 16 semicorcheas + tambora rápida (Merengue / Tecnomerengue 110–162 BPM).
        MerengueDrive,
        /// Bajo sincopado con acento agudo en tiempo 4 y `2-&` (Bachata 118–142 BPM).
        BachataGroove,
        /// Estructura general (Rock, Pop acústico, R&B, Hip-Hop, Jazz).
        GeneralGroove,
    }

    /// Diagnóstico completo de un BPM candidato proyectado sobre una rejilla de 16 semicorcheas.
    #[derive(Debug, Clone)]
    pub struct GrooveSignature {
        pub style: GrooveStyle,
        /// Alineación de fase evaluada **únicamente** en `sub_kick` (`25–130 Hz`).
        pub sub_kick_phase_score: f64,
        /// Fracción de los 4 tiempos de negra (`0, 4, 8, 12`) donde `sub_kick` presenta golpe real.
        pub kick_quarter_hit_ratio: f64,
        /// Índice de Tumbao de Salsa: `(bajo[6] + bajo[12]) / (2.0 * bajo[0] + eps)`.
        /// Mayor a `1.28` indica bajo anticipado sin bombo en tiempo 1.
        pub salsa_tumbao_ratio: f64,
        /// Correlación con el patrón Dembow (`snare` en pasos `3, 6, 11, 14` + `kick` en `0, 4, 8, 12`).
        pub dembow_pattern_score: f64,
        /// Uniformidad de subdivisión de semicorcheas en `hi_guiro` (`4.2–11 kHz`).
        pub guiro_sixteenth_density: f64,
        /// Energía de tresillo urbano (`0, 3, 6, 9, 12, 15` o `0, 3, 6, 8, 11, 14`) en `perc_clave`.
        pub clave_tresillo_ratio: f64,
        /// `true` si el pulso cuaternario en `sub_kick` prohíbe estrictamente aplicar `* 0.75`.
        pub veto_three_quarter_alias: bool,
    }

    /// Proyecta las 4 bandas de `MultibandNovelty` sobre un ciclo de 16 semicorcheas al `cand_bpm`
    /// alineando primero el paso `0` con el tiempo fuerte real del compás.
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

        let beat_period_frames = (60.0 * ONSET_FPS) / cand_bpm;
        let bar_frames = beat_period_frames * 4.0;
        let step_frames = bar_frames / 16.0;

        // 1. Encuentra el offset óptimo dentro de un beat `[0, beat_period_frames)` maximizando golpes compuestos
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

        // 2. Acumula las 4 bandas en 16 bines de semicorchea por compás
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

        // Evalúa impacto real de bombo beat por beat en la rejilla cuaternaria
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

        // Rotación óptima de inicio de compás entre las 4 negras `{0, 4, 8, 12}`
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

        // Métrica 1: Contraste de fase AISLADO en `sub_kick` (negras `0,4,8,12` vs huecos fuera de negra)
        let kick_on_quarters =
            (hist_sub[0] + hist_sub[4] + hist_sub[8] + hist_sub[12]) * 0.25;
        let kick_off_quarters = (0..16)
            .filter(|s| s % 4 != 0)
            .map(|s| hist_sub[s])
            .sum::<f64>()
            / 12.0;
        let sub_kick_phase_score =
            (kick_on_quarters / (kick_on_quarters + kick_off_quarters + 1e-6)).clamp(0.0, 1.0);

        // Métrica 2: Tumbao de Salsa (bajo anticipado en `6` [2-&] y `12` [4] vs tiempo `0` [1])
        // Rotamos buscando la fase de tumbao más clara en las 4 posibles negras si no hay bombo en 1
        let mut salsa_tumbao_ratio = 0.0_f64;
        for rot in [0_usize, 4, 8, 12] {
            let bass_0 = 0.55 * hist_sub[rot % 16] + 0.45 * hist_body[rot % 16];
            let bass_4 = 0.55 * hist_sub[(rot + 4) % 16] + 0.45 * hist_body[(rot + 4) % 16];
            let bass_6 = 0.55 * hist_sub[(rot + 6) % 16] + 0.45 * hist_body[(rot + 6) % 16];
            let bass_12 = 0.55 * hist_sub[(rot + 12) % 16] + 0.45 * hist_body[(rot + 12) % 16];
            // En salsa real, `2-&` (6) y `4` (12) superan ampliamente a `1` (0) y `2` (4)
            let ratio = (bass_6 + bass_12) / (bass_0 + bass_4 + 0.12);
            salsa_tumbao_ratio = salsa_tumbao_ratio.max(ratio);
        }

        // Métrica 3: Dembow (`kick` en `0,4,8,12` + `snare` en `3,6,11,14`)
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

        // Métrica 4: Tresillo urbano / Mambo (`0, 3, 6, 8, 11, 14` o `0, 3, 6, 9, 12, 15`) en `perc_clave`
        let tresillo_hits =
            (hist_clave[0] + hist_clave[3] + hist_clave[6] + hist_clave[8] + hist_clave[11] + hist_clave[14])
                / 6.0;
        let clave_mean = (hist_clave.iter().sum::<f64>() / 16.0).max(1e-6);
        let clave_tresillo_ratio = tresillo_hits / clave_mean;

        // Métrica 5: Densidad continua de güira en las 16 semicorcheas (Merengue / Merenguetón)
        let guiro_mean = hist_guiro.iter().sum::<f64>() / 16.0;
        let guiro_active_steps = hist_guiro
            .iter()
            .filter(|&&v| v >= guiro_mean * 0.55 && v > 0.12)
            .count();
        let guiro_sixteenth_density = (guiro_active_steps as f64) / 16.0;

        // REGLA CRÍTICA (`FourOnTheFloorVeto` — resuelve `MUJERON - Reggi El Autentico` 126 BPM
        // y protege Afro/Tech House 117–126 BPM):
        // Si `sub_kick` golpea consistentemente en las 4 negras (`kick_quarter_hit_ratio >= 0.62`
        // y `sub_kick_phase_score >= 0.58`), la canción ES 4/4 real; cualquier pico a `0.75x` es
        // producto del tresillo `3-3-2` en `perc_clave`/`low_body` y queda VETOADO.
        let veto_three_quarter_alias =
            kick_quarter_hit_ratio >= 0.62 && sub_kick_phase_score >= 0.58;

        // Clasificación estructural del Groove
        let style = if veto_three_quarter_alias && (112.0..=136.0).contains(&cand_bpm) {
            if clave_tresillo_ratio >= 1.22 || guiro_sixteenth_density >= 0.68 {
                GrooveStyle::UrbanLatinFourFourHybrid // Merenguetón / Guaracha / Afro-Latin House (ej. MUJERON 126)
            } else {
                GrooveStyle::FourOnTheFloorClub // Tech House / Afro House (Fisher 125, Hugel 123, Samm 117)
            }
        } else if (108.0..=164.0).contains(&cand_bpm)
            && guiro_sixteenth_density >= 0.75
            && kick_quarter_hit_ratio >= 0.50
        {
            GrooveStyle::MerengueDrive // Merengue / Tecnomerengue (Calle Ciega 126, Eddy Herrera 126, Juana La Cubana)
        } else if (78.0..=106.0).contains(&cand_bpm)
            && dembow_pattern_score >= 0.42
            && kick_quarter_hit_ratio >= 0.58
        {
            GrooveStyle::ReggaetonDembow
        } else if (76.0..=109.0).contains(&cand_bpm)
            && salsa_tumbao_ratio >= 1.25
            && kick_quarter_hit_ratio < 0.58
        {
            GrooveStyle::SalsaTumbao // Salsa Baúl (Tony Vega 82, Eddie Santiago 83, Jerry Rivera 86)
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
    use super::qm_comb_tracker::{analyze_qm_comb_tempogram, fine_grid_phase_lock_and_beats};
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

    /// Reemplazo drop-in mejorado para `analyze_bpm_consensus` en `hera-rs/src/bpm_engine/`.
    ///
    /// ### Flujo de decisión:
    /// 1. Ejecuta el banco de filtros peine multi-lag de Queen Mary (`qm_comb_tracker`) + Viterbi.
    /// 2. Genera el abanico completo de candidatos métricos (`1x`, `0.75x`, `1.3333x`, `0.5x`, `2x`, `0.6667x`, `1.5x`)
    ///    y agrega `bpm_hint` si está presente (`VirtualDJ XML`, `ID3 TBPM` o `[126 BPM]` en el nombre).
    /// 3. Evalúa `GrooveSignature` de 16 pasos en cada candidato aplicando:
    ///    - **`FourOnTheFloorVeto`**: Bloquea `0.75x` si el candidato `1x` (`112–138 BPM`) tiene bombo 4/4 real
    ///      en `sub_kick` (impidiendo que `MUJERON` 126 BPM o Afro/Tech House caigan a `88–95 BPM`).
    ///    - **Rescate Salsa `3:4`**: Si el peine detectó el armónico de clave `4:3` (`108–144 BPM`), pero
    ///      en ese tempo NO hay bombo 4/4 y en `0.75x` (`80–108 BPM`) emerge la firma de **Tumbao anticipado**
    ///      (`salsa_tumbao_ratio >= 1.25`), promueve limpiamente el BPM de Salsa real.
    ///    - **Desambiguación 0.5x / 2x**: Evita doblar Reggaetón/Dembow (`85–98 BPM` → `170–196 BPM`) y evita
    ///      partir a la mitad Merengue/Tecnomerengue (`124–156 BPM` → `62–78 BPM`) gracias a `guiro_sixteenth_density`.
    /// 4. Cierra con `fine_grid_phase_lock_and_beats` para clavar el BPM exacto (`±0.01 BPM` / entero Mixxx `25 ms`).
    pub fn analyze_bpm_consensus_v2(
        nov: &MultibandNovelty,
        bpm_hint: Option<BpmHint>,
    ) -> BpmConsensusResult {
        let qm = analyze_qm_comb_tempogram(nov);

        // Recolecta semillas base desde los picos del peine QM + trayectoria Viterbi + hint
        let mut base_seeds: Vec<f64> = qm.top_comb_peaks.iter().take(3).map(|p| p.0).collect();
        base_seeds.push(qm.viterbi_bpm);
        if let Some(h) = bpm_hint {
            base_seeds.push(h.bpm);
        }

        // Expande relaciones armónicas rítmicas (incluyendo 3:4 y 4:3 para polirritmia latina)
        let ratios = [1.0_f64, 0.75, 4.0 / 3.0, 0.5, 2.0, 2.0 / 3.0, 1.5];
        let mut candidates: Vec<(f64, f64, &'static str)> = Vec::new();

        for &seed in &base_seeds {
            for &r in &ratios {
                let c = seed * r;
                if (55.0..=210.0).contains(&c) {
                    // Evita duplicados dentro de ±0.8 BPM
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
                        candidates.push((c, seed, tag));
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

        // Precalcula firmas de 16 pasos para todos los candidatos
        let evals: Vec<(f64, &'static str, GrooveSignature, f64, f64)> = candidates
            .iter()
            .map(|&(bpm, _seed, label)| {
                let sig = evaluate_groove_at_bpm(nov, bpm);
                let lag = ((60.0 * ONSET_FPS) / bpm)
                    .round()
                    .clamp(qm.min_lag as f64, qm.max_lag as f64) as usize;
                let comp_e = qm.composite_comb_by_lag[lag] / max_comp_comb;
                let sub_e = qm.sub_kick_comb_by_lag[lag] / max_sub_comb;
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

            // 1. VETO 4/4 (`MUJERON` 126 BPM / Afro House / Tech House):
            // Comprueba si existe un candidato hermano a `4/3 * bpm` (~112..145 BPM) que posea un
            // pulso cuaternario firme en `sub_kick`. Si existe, este candidato (`0.75x`, ~84..108 BPM)
            // es un falso alias de tresillo y recibe penalización severa.
            let parent_4_3 = bpm * (4.0 / 3.0);
            if let Some((_, _, parent_sig, _, parent_sub_e)) = evals
                .iter()
                .find(|e| (e.0 - parent_4_3).abs() <= 1.6)
            {
                if parent_sig.veto_three_quarter_alias && *parent_sub_e >= *sub_e * 0.90 {
                    score -= 0.55; // Bloquea caída a 95.22 BPM en híbridos con tresillo
                }
            }

            // 2. Bonificación estructural según estilo rítmico verificado
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
                    // Verifica que el armónico `4/3` superior NO sea un 4/4 de club antes de premiar Salsa
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

            // 3. Protección anti-doble tiempo (ej. Reggaetón 92 -> 184 BPM) y anti-mitad (Merengue 126 -> 63 BPM)
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

            // 4. Voto Bayesiano de `BpmHint` (`VirtualDJ XML`, `ID3 TBPM`, `[126 BPM]` en filename)
            if let Some(hint) = bpm_hint {
                let rel_err = ((bpm - hint.bpm).abs()) / hint.bpm.max(1.0);
                if rel_err <= 0.025 {
                    score += hint.source.prior_weight();
                } else if ((bpm * 2.0 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
                    || ((bpm * 0.5 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
                {
                    // Penaliza suavemente la octava opuesta al hint cuando el audio es compatible
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
// 7. SUBMÓDULO `key_engine` — HPCP 36-BIN + A440 FIX + ENSEMBLE CAMELOT 1A–12B
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
        /// Ejemplo: `"A minor"`, `"C major"`.
        pub standard_name: String,
        /// Notación Camelot DJ (`"1A"`..`"12B"`).
        pub camelot: String,
        /// Notación Traktor Open Key (`"1m"`..`"12d"`).
        pub open_key: String,
        pub confidence: f32,
        /// Desviación global de afinación respecto a A440 en centésimas (`-50..+50 cents`).
        pub tuning_cents: f32,
    }

    // Perfiles empíricos multi-género publicados (índice 0 = Tónica):
    // BGATE & EDMA (Faraldo et al. 2016/2017 - EDM/Urbano/Pop), Temperley (Rock/Clásica), Sha'ath (DJ).
    const BGATE_MAJ: [f64; 12] = [1.00, 0.00, 0.42, 0.00, 0.53, 0.37, 0.00, 0.77, 0.00, 0.38, 0.21, 0.30];
    const BGATE_MIN: [f64; 12] = [1.00, 0.00, 0.36, 0.39, 0.00, 0.38, 0.00, 0.74, 0.27, 0.00, 0.42, 0.23];
    const EDMA_MAJ: [f64; 12] = [1.00, 0.29, 0.50, 0.40, 0.60, 0.56, 0.32, 0.80, 0.31, 0.45, 0.42, 0.39];
    const EDMA_MIN: [f64; 12] = [1.00, 0.31, 0.44, 0.58, 0.33, 0.49, 0.29, 0.78, 0.43, 0.29, 0.53, 0.32];
    const TEMP_MAJ: [f64; 12] = [0.748, 0.060, 0.488, 0.082, 0.670, 0.460, 0.096, 0.715, 0.104, 0.366, 0.057, 0.400];
    const TEMP_MIN: [f64; 12] = [0.712, 0.084, 0.474, 0.618, 0.049, 0.460, 0.105, 0.747, 0.404, 0.067, 0.133, 0.330];
    const SHAATH_MAJ: [f64; 12] = [6.6, 2.0, 3.5, 2.3, 4.6, 4.0, 2.5, 5.2, 2.4, 3.7, 2.3, 3.4];
    const SHAATH_MIN: [f64; 12] = [6.5, 2.7, 3.5, 5.4, 2.6, 3.5, 2.5, 5.2, 4.0, 2.7, 4.3, 3.2];

    /// Calcula el tono musical filtrando ventanas percusivas (*Harmonic Gating*), corrigiendo
    /// grabaciones fuera de 440 Hz y desambiguando modos paralelos (ej. Sol menor vs Sol mayor).
    pub fn detect_song_key(
        samples_44k: &[f32],
        fft_planner: &mut FftPlanner<f32>,
    ) -> MusicalKeyResult {
        let n_fft: usize = 4096;
        let hop: usize = 2048;
        if samples_44k.len() < n_fft {
            return build_key_result(PitchClass::C, KeyScale::Major, 0.0, 0.0);
        }

        let fft = fft_planner.plan_fft_forward(n_fft);
        let window: Vec<f32> = (0..n_fft)
            .map(|i| {
                let p = 2.0 * PI * (i as f32) / ((n_fft - 1) as f32);
                0.35875 - 0.48829 * p.cos() + 0.14128 * (2.0 * p).cos() - 0.01168 * (3.0 * p).cos()
            })
            .collect();

        let bin_hz = (ANALYSIS_SR as f64) / (n_fft as f64);
        let min_bin = ((55.0 / bin_hz).ceil() as usize).max(2);
        let max_bin = ((3520.0 / bin_hz).floor() as usize).min(n_fft / 2 - 2);

        let num_frames = (samples_44k.len() - n_fft) / hop + 1;
        let step = (num_frames / 550).max(1);

        let mut buf = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; n_fft];
        let mut mag = vec![0.0_f64; n_fft / 2 + 1];
        let mut hpcp = [0.0_f64; HPCP_BINS];
        let mut peaks_12 = [0.0_f64; 12];

        for f_idx in (0..num_frames).step_by(step) {
            let off = f_idx * hop;
            for i in 0..n_fft {
                buf[i] = Complex {
                    re: samples_44k[off + i] * window[i],
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
            // Ignora *intros/outros* de pura percusión o ruido de güira/platillos
            if flatness > 0.54 || (lin_s / cnt) < 1e-4 {
                continue;
            }
            let h_weight = (1.0 - flatness / 0.54).powi(2);
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
                    let amp = m1.powf(0.65) * h_weight; // Blanqueo espectral por compresión

                    let midi_p = 69.0 + 12.0 * (freq / 440.0).log2();
                    peaks_12[(midi_p.round() as isize).rem_euclid(12) as usize] += amp;

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

        // Corrección global de desafinación respecto a A440
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

        let mut scored: Vec<(PitchClass, KeyScale, f64)> = Vec::with_capacity(24);
        for root in 0..12 {
            let maj = 0.35 * corr_12(&pcp_12, &BGATE_MAJ, root)
                + 0.25 * corr_12(&pcp_12, &EDMA_MAJ, root)
                + 0.25 * corr_12(&pcp_12, &TEMP_MAJ, root)
                + 0.15 * corr_12(&pcp_12, &SHAATH_MAJ, root);
            scored.push((PitchClass::from_semitone(root), KeyScale::Major, maj));

            let min = 0.35 * corr_12(&pcp_12, &BGATE_MIN, root)
                + 0.25 * corr_12(&pcp_12, &EDMA_MIN, root)
                + 0.25 * corr_12(&pcp_12, &TEMP_MIN, root)
                + 0.15 * corr_12(&pcp_12, &SHAATH_MIN, root);
            scored.push((PitchClass::from_semitone(root), KeyScale::Minor, min));
        }
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));

        // Verificación de modo paralelo sobre la tercera real (+4 mayor vs +3 menor)
        let (mut win_tonic, mut win_scale, mut win_score) = scored[0];
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

        build_key_result(win_tonic, win_scale, conf, tuning_cents)
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
// 8. SUBMÓDULO `queue` — COLA PARALELA MASIVA TIPO VIRTUALDJ (BACKPRESSURE)
// ============================================================================

pub mod queue {
    use super::audio_ingest::{load_audio_mono_44k, resample_polyphase};
    use super::consensus_ai::{analyze_bpm_consensus_v2, BpmConsensusResult};
    use super::hints::{resolve_best_hint, VirtualDjDbIndex};
    use super::key_engine::{detect_song_key, MusicalKeyResult};
    use super::multiband_onset::{extract_multiband_novelty, trim_leading_trailing_silence};
    use super::*;
    use crossbeam_channel::{bounded, Receiver, Sender};

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

    /// Analizador de cola multinúcleo (`CPU - 1` trabajadores, memoria acotada para laptops de 8 GB).
    pub struct HeraBatchQueue {
        pub worker_threads: usize,
        pub flush_batch_size: usize,
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
                flush_batch_size: 50,
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
            let (in_tx, in_rx): (Sender<QueueTrackInput>, Receiver<QueueTrackInput>) =
                bounded(bound);
            let (out_tx, out_rx) = bounded(bound * 2);

            let seen_hashes = Arc::new(Mutex::new(HashSet::<String>::with_capacity(total)));
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
                let rx = in_rx.clone();
                let tx = out_tx.clone();
                let vdj = self.vdj_db.clone();
                let seen = Arc::clone(&seen_hashes);
                let cancel_w = Arc::clone(&cancel_flag);

                workers.push(thread::spawn(move || {
                    let mut planner = FftPlanner::<f32>::new();
                    while let Ok(job) = rx.recv() {
                        if cancel_w.load(AtomicOrdering::Relaxed) {
                            break;
                        }
                        let res = analyze_single_job(job, vdj.as_deref(), &seen, &mut planner);
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
                    WorkerOutcome::Analyzed(analysis) => {
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
                    WorkerOutcome::Duplicate { track_id, hash } => {
                        on_progress(QueueProgressEvent::DuplicateSkipped {
                            track_id,
                            audio_hash_hex: hash,
                        });
                    }
                    WorkerOutcome::Failed { track_id, error } => {
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

    enum WorkerOutcome {
        Analyzed(FullTrackAnalysis),
        Duplicate { track_id: String, hash: String },
        Failed { track_id: String, error: String },
    }

    fn analyze_single_job(
        job: QueueTrackInput,
        vdj_db: Option<&VirtualDjDbIndex>,
        seen: &Mutex<HashSet<String>>,
        planner: &mut FftPlanner<f32>,
    ) -> WorkerOutcome {
        let start_t = Instant::now();
        let track_id = job.track_id.clone();

        let (samples_44k, duration_sec, tag_bpm, used_ffmpeg) =
            if let Some((pcm, sr)) = job.inline_pcm {
                let dur = (pcm.len() as f64) / (sr.max(1) as f64);
                let s44 = resample_polyphase(&pcm, sr, ANALYSIS_SR);
                (s44, dur, None, false)
            } else if let Some(ref p) = job.path {
                match load_audio_mono_44k(p) {
                    Ok(dec) => (
                        dec.samples_44k,
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

        let hash = fast_pcm_hash_128(&samples_44k);
        if let Ok(mut g) = seen.lock() {
            if !g.insert(hash.clone()) {
                return WorkerOutcome::Duplicate { track_id, hash };
            }
        }

        let resolved_hint = resolve_best_hint(
            job.explicit_bpm_hint,
            job.path.as_deref(),
            tag_bpm,
            vdj_db,
        );

        let (trimmed, lead_sec, _trail_sec) = trim_leading_trailing_silence(&samples_44k);
        let nov = extract_multiband_novelty(trimmed, lead_sec, planner);
        let bpm_result = analyze_bpm_consensus_v2(&nov, resolved_hint);
        let key_result = detect_song_key(trimmed, planner);
        drop(samples_44k); // Libera inmediatamente el audio antes de despachar el resultado

        WorkerOutcome::Analyzed(FullTrackAnalysis {
            track_id,
            audio_hash_hex: hash,
            duration_sec,
            bpm_result,
            key_result,
            used_ffmpeg_video_fallback: used_ffmpeg,
            elapsed_ms: start_t.elapsed().as_millis() as u64,
        })
    }

    fn fast_pcm_hash_128(samples: &[f32]) -> String {
        let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
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
// 9. SUITE DE TESTS DEL BENCHMARK REAL DE CABINA DE HERA (12/12 ESCENARIOS)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::consensus_ai::analyze_bpm_consensus_v2;
    use super::hints::{extract_bpm_from_filename, parse_virtualdj_bpm_value, VirtualDjDbIndex};
    use super::key_engine::detect_song_key;
    use super::latin_heuristics::GrooveStyle;
    use super::multiband_onset::extract_multiband_novelty;
    use super::*;

    /// Sintetiza el caso crítico `MUJERON - Reggi El Autentico` (`126.00 BPM`):
    /// - Bombo 4/4 en `sub_kick` (`58 Hz`) en los tiempos `0, 4, 8, 12`
    /// - Tresillo urbano intenso (`0, 3, 6, 9, 12, 15` / mambo `1200–2400 Hz`) que antes engañaba
    ///   al scorer forzando `126 * 0.75 = 94.5..95.22 BPM`
    /// - Güira constante en `6500 Hz`.
    fn synth_merengueton_triplet_hybrid_126(duration_sec: f64) -> Vec<f32> {
        let n = (duration_sec * (ANALYSIS_SR as f64)) as usize;
        let mut out = vec![0.0_f32; n];
        let beat_sec = 60.0 / 126.0;
        let bar_sec = beat_sec * 4.0;
        let step_sec = bar_sec / 16.0;

        for (i, sample) in out.iter_mut().enumerate() {
            let t = (i as f64) / (ANALYSIS_SR as f64);
            let bar_pos = t.rem_euclid(bar_sec);
            let step = (bar_pos / step_sec).floor() as usize;
            let step_phase = bar_pos - (step as f64) * step_sec;

            // 1. Sub-kick 4/4 en negras (pasos 0, 4, 8, 12)
            let kick = if step % 4 == 0 && step_phase < 0.085 {
                let env = (-step_phase * 40.0).exp();
                let f = 55.0 + 75.0 * (-step_phase * 70.0).exp();
                0.70 * env * (2.0 * std::f64::consts::PI * f * step_phase).sin()
            } else {
                0.0
            };

            // 2. Tresillo / polirritmia 3:4 agresiva en `perc_clave` (3 golpes exactos por compás = 94.5 BPM!)
            let triplet_period = bar_sec / 3.0; // Exactamente el alias 0.75x (94.5 BPM)!
            let tri_phase = bar_pos.rem_euclid(triplet_period);
            let mambo_stab = if tri_phase < 0.06 {
                let env = (-tri_phase * 55.0).exp();
                0.55 * env
                    * ((2.0 * std::f64::consts::PI * 1350.0 * tri_phase).sin()
                        + 0.7 * (2.0 * std::f64::consts::PI * 2100.0 * tri_phase).sin())
            } else {
                0.0
            };

            // 3. Güira en semicorcheas (6.5 kHz)
            let guiro = if step_phase < 0.022 {
                let noise = ((i as f64 * 12.9898).sin() * 43_758.545_3).fract() * 2.0 - 1.0;
                0.15 * (-step_phase * 120.0).exp()
                    * noise
                    * (2.0 * std::f64::consts::PI * 6500.0 * t).sin()
            } else {
                0.0
            };

            *sample = (kick + mambo_stab + guiro).clamp(-0.98, 0.98) as f32;
        }
        out
    }

    /// Sintetiza Salsa Baúl real (`83.00 BPM`):
    /// - SIN bombo en tiempo 1 (`step 0` vacío en graves)
    /// - Bajo Tumbao anticipado en `step 6` (`2-&`) y `step 12` (`4`) a `82–165 Hz`
    /// - Clave de Son 2-3 (`steps 2, 4, 8, 11, 14`) en `1800 Hz` + Maracas en corcheas.
    fn synth_salsa_baul_83(duration_sec: f64) -> Vec<f32> {
        let n = (duration_sec * (ANALYSIS_SR as f64)) as usize;
        let mut out = vec![0.0_f32; n];
        let beat_sec = 60.0 / 83.0;
        let bar_sec = beat_sec * 4.0;
        let step_sec = bar_sec / 16.0;

        for (i, sample) in out.iter_mut().enumerate() {
            let t = (i as f64) / (ANALYSIS_SR as f64);
            let bar_pos = t.rem_euclid(bar_sec);
            let step = (bar_pos / step_sec).floor() as usize;
            let sp = bar_pos - (step as f64) * step_sec;

            // Tumbao de baby bass en `2-&` (step 6) y `4` (step 12)
            let tumbao = if matches!(step, 6 | 12) && sp < 0.16 {
                let env = (-sp * 18.0).exp();
                let f = if step == 6 { 98.0 } else { 110.0 };
                0.68 * env
                    * ((2.0 * std::f64::consts::PI * f * sp).sin()
                        + 0.5 * (2.0 * std::f64::consts::PI * (f * 2.0) * sp).sin())
            } else {
                0.0
            };

            // Clave + Campana en `perc_clave` (2200 Hz)
            let clave = if matches!(step, 0 | 4 | 8 | 11 | 14) && sp < 0.035 {
                0.48 * (-sp * 95.0).exp() * (2.0 * std::f64::consts::PI * 2350.0 * sp).sin()
            } else {
                0.0
            };

            // Maracas en corcheas
            let maraca = if step % 2 == 0 && sp < 0.025 {
                let noise = ((i as f64 * 78.233).sin() * 43_758.545_3).fract() * 2.0 - 1.0;
                0.14 * (-sp * 110.0).exp() * noise
            } else {
                0.0
            };

            *sample = (tumbao + clave + maraca).clamp(-0.98, 0.98) as f32;
        }
        out
    }

    #[test]
    fn test_brecha1_mujeron_126_hybrid_never_falls_to_95() {
        let pcm = synth_merengueton_triplet_hybrid_126(20.0);
        let mut planner = FftPlanner::<f32>::new();
        let nov = extract_multiband_novelty(&pcm, 0.0, &mut planner);
        let res = analyze_bpm_consensus_v2(&nov, None);

        assert!(
            (res.bpm - 126.0).abs() <= 0.15,
            "MUJERON híbrido 126 BPM cayó en alias: obtenido {:.2} BPM (estilo {:?})",
            res.bpm,
            res.detected_style
        );
    }

    #[test]
    fn test_brecha2_salsa_baul_83_exact_without_breaking_club() {
        let pcm = synth_salsa_baul_83(22.0);
        let mut planner = FftPlanner::<f32>::new();
        let nov = extract_multiband_novelty(&pcm, 0.0, &mut planner);
        let res = analyze_bpm_consensus_v2(&nov, None);

        assert!(
            (res.bpm - 83.0).abs() <= 0.25,
            "Salsa Baúl esperada ~83.00 BPM, obtenido {:.2} BPM",
            res.bpm
        );
    }

    #[test]
    fn test_brecha4_virtualdj_xml_and_filename_hints() {
        // En VirtualDJ database.xml, 126.00 BPM se guarda como 60 / 126 = 0.476190 seg/beat
        let vdj_xml = r#"
        <VirtualDJ_Database Version="2026">
          <Song FilePath="/Music/Latin/Reggi El Autentico - MUJERON.mp4" FileSize="14200123">
            <Tags Author="Reggi El Autentico" Title="MUJERON" />
            <Scan Version="801" Bpm="0.476190" AltBpm="0.630120" Key="Am" />
          </Song>
        </VirtualDJ_Database>
        "#;
        let db = VirtualDjDbIndex::from_xml_str(vdj_xml);
        let looked = db
            .lookup_bpm(Path::new("/otra/carpeta/Reggi El Autentico - MUJERON.mp4"))
            .expect("Debe encontrar por nombre de archivo normalizado");
        assert!((looked - 126.0).abs() < 0.05);

        assert_eq!(parse_virtualdj_bpm_value("0.480000"), Some(125.0));
        assert_eq!(
            extract_bpm_from_filename(Path::new("01 - Fisher - Losing It [125 BPM].mp3")),
            Some(125.0)
        );
        assert_eq!(
            extract_bpm_from_filename(Path::new("2025_Promo_320kbps.mp3")),
            None
        );
    }

    #[test]
    fn test_key_detection_off_a440_camelot_8a() {
        // Acorde La menor (A3=57, C4=60, E4=64) desafinado +26 cents (~446.6 Hz)
        let n = ANALYSIS_SR as usize * 8;
        let mut pcm = vec![0.0_f32; n];
        for (i, s) in pcm.iter_mut().enumerate() {
            let t = (i as f64) / (ANALYSIS_SR as f64);
            let mut acc = 0.0_f64;
            for &(midi, amp) in &[(57.26_f64, 1.0_f64), (60.26, 0.78), (64.26, 0.82)] {
                let f = 440.0 * 2.0_f64.powf((midi - 69.0) / 12.0);
                acc += amp
                    * ((2.0 * std::f64::consts::PI * f * t).sin()
                        + 0.4 * (4.0 * std::f64::consts::PI * f * t).sin());
            }
            *s = (0.3 * acc) as f32;
        }
        let mut planner = FftPlanner::<f32>::new();
        let key = detect_song_key(&pcm, &mut planner);
        assert_eq!(key.camelot, "8A");
    }
}