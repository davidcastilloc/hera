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

