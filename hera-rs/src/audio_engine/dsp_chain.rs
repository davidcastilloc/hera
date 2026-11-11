//! Real-time Pro-DJ DSP Chain (`rubato` sinc resampler, `fundsp` master bus, `realfft` 32-bin
//! logarithmic spectrum analyzer, and `rtrb` wait-free SPSC telemetry ring).

use std::sync::Arc;

use fundsp::prelude32::{dcblock_hz, highshelf_hz, lowshelf_hz, shape, AudioUnit, Tanh};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use rtrb::{Consumer, Producer, RingBuffer};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use serde::{Deserialize, Serialize};

use super::PreallocatedDeckBuffer;

/// Number of logarithmically spaced visual/DSP spectrum bins per telemetry frame.
pub const SPECTRUM_BIN_COUNT: usize = 32;
/// Default capacity of the lock-free `rtrb` telemetry ring buffer.
pub const TELEMETRY_RING_CAPACITY: usize = 256;
const FFT_WINDOW_SIZE: usize = 1024;

/// Copy-only real-time telemetry frame pushed wait-free over `rtrb::RingBuffer`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AudioTelemetryFrame {
    pub sequence: u64,
    pub sample_rate: u32,
    pub current_time_sec: f64,
    pub duration_sec: f64,
    pub is_playing: bool,
    pub rms_left: f32,
    pub rms_right: f32,
    pub peak_db: f32,
    pub sub_bass: f32,
    pub low_mid: f32,
    pub high_mid: f32,
    pub air_presence: f32,
    pub spectrum_bins: [f32; SPECTRUM_BIN_COUNT],
}

/// Construct a pre-allocated wait-free SPSC `rtrb` telemetry ring buffer.
pub fn create_telemetry_ring(
    capacity: usize,
) -> (Producer<AudioTelemetryFrame>, Consumer<AudioTelemetryFrame>) {
    RingBuffer::new(capacity.max(4))
}

/// Resample a decoded [`PreallocatedDeckBuffer`] to `target_sample_rate` using `rubato` windowed
/// sinc interpolation and proportionally scale all 4 `HotCueSampleMarker` frame offsets.
pub fn resample_deck_buffer_to_rate(
    buf: &PreallocatedDeckBuffer,
    target_sample_rate: u32,
) -> Result<PreallocatedDeckBuffer, String> {
    if target_sample_rate == 0 {
        return Err("target_sample_rate must be greater than zero".to_string());
    }
    if buf.sample_rate == target_sample_rate {
        return Ok(buf.clone());
    }

    let in_frames = buf.total_frames() as usize;
    let ratio = f64::from(target_sample_rate) / f64::from(buf.sample_rate.max(1));
    let params = SincInterpolationParameters {
        sinc_len: 128,
        f_cutoff: Some(0.925),
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };
    let chunk_frames = in_frames.clamp(64, 1024);
    let mut resampler =
        Async::<f32>::new_sinc(ratio, 1.1, &params, chunk_frames, 2, FixedAsync::Input)
            .map_err(|e| format!("Failed to construct rubato sinc resampler: {e}"))?;

    let input_slice = InterleavedSlice::new(buf.interleaved_stereo(), 2, in_frames)
        .map_err(|e| format!("Invalid stereo slice for rubato: {e}"))?;
    let output_owned = resampler
        .process_all(&input_slice, in_frames, None)
        .map_err(|e| format!("Rubato resampling failed: {e}"))?;

    let mut stereo = output_owned.take_data();
    for sample in &mut stereo {
        *sample = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
    }

    let scaled_cues = buf
        .hot_cues
        .map(|cue| ((cue.frame_index as f64) * ratio).round().max(0.0) as u64);
    PreallocatedDeckBuffer::from_stereo_frames(stereo, target_sample_rate, scaled_cues)
}

/// Zero-allocation stereo master bus (`fundsp::prelude32`): 12 Hz DC-blocker, subtle club
/// low/high shelves, and analog-style `Tanh` soft-clipping safety stage.
pub struct MasterDspBus {
    unit: Box<dyn AudioUnit>,
}

impl MasterDspBus {
    /// Build and pre-allocate the stereo `fundsp` master stage at `sample_rate`.
    pub fn new(sample_rate: u32) -> Self {
        let left = dcblock_hz(12.0)
            >> lowshelf_hz(90.0, 0.707, 1.02)
            >> highshelf_hz(11_500.0, 0.707, 1.02)
            >> shape(Tanh(1.0));
        let right = dcblock_hz(12.0)
            >> lowshelf_hz(90.0, 0.707, 1.02)
            >> highshelf_hz(11_500.0, 0.707, 1.02)
            >> shape(Tanh(1.0));
        let mut unit: Box<dyn AudioUnit> = Box::new(left | right);
        unit.set_sample_rate(f64::from(sample_rate.max(8_000)));
        unit.allocate();
        Self { unit }
    }

    /// Process interleaved `(L, R)` samples in-place with zero heap allocations and NaN protection.
    pub fn process_interleaved_in_place(
        &mut self,
        interleaved_stereo: &mut [f32],
        master_gain: f32,
    ) {
        let gain = if master_gain.is_finite() {
            master_gain.clamp(0.0, 2.0)
        } else {
            0.0
        };
        let mut out_pair = [0.0_f32; 2];
        let (frames_slice, _) = interleaved_stereo.as_chunks_mut::<2>();
        for frame in frames_slice {
            let in_l = if frame[0].is_finite() {
                frame[0] * gain
            } else {
                0.0
            };
            let in_r = if frame[1].is_finite() {
                frame[1] * gain
            } else {
                0.0
            };
            self.unit.tick(&[in_l, in_r], &mut out_pair);
            frame[0] = if out_pair[0].is_finite() {
                out_pair[0].clamp(-1.0, 1.0)
            } else {
                0.0
            };
            frame[1] = if out_pair[1].is_finite() {
                out_pair[1].clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }
    }
}

/// Real-time zero-allocation 32-bin logarithmic spectrum + 4-band RMS + L/R meter (`realfft`).
pub struct RealTimeSpectrumAnalyzer {
    sample_rate: u32,
    fft: Arc<dyn RealToComplex<f32>>,
    mono_ring: [f32; FFT_WINDOW_SIZE],
    ring_pos: usize,
    hann_window: [f32; FFT_WINDOW_SIZE],
    fft_input: Vec<f32>,
    fft_output: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    sequence: u64,
}

impl RealTimeSpectrumAnalyzer {
    /// Pre-allocate all `realfft` buffers off the audio thread.
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate.max(8_000);
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_WINDOW_SIZE);
        let mut hann_window = [0.0_f32; FFT_WINDOW_SIZE];
        let denom = (FFT_WINDOW_SIZE - 1) as f32;
        for (i, w) in hann_window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (std::f32::consts::TAU * (i as f32) / denom).cos());
        }
        let fft_input = fft.make_input_vec();
        let fft_output = fft.make_output_vec();
        let fft_scratch = fft.make_scratch_vec();
        Self {
            sample_rate: sr,
            fft,
            mono_ring: [0.0; FFT_WINDOW_SIZE],
            ring_pos: 0,
            hann_window,
            fft_input,
            fft_output,
            fft_scratch,
            sequence: 0,
        }
    }

    /// Analyze a rendered stereo quantum without allocating on the heap.
    pub fn analyze_block(
        &mut self,
        interleaved_stereo: &[f32],
        current_time_sec: f64,
        duration_sec: f64,
        is_playing: bool,
    ) -> AudioTelemetryFrame {
        let frames = interleaved_stereo.len() / 2;
        let (mut sum_l2, mut sum_r2, mut peak) = (0.0_f32, 0.0_f32, 0.0_f32);

        let (pairs, _) = interleaved_stereo.as_chunks::<2>();
        for pair in pairs {
            let l = if pair[0].is_finite() { pair[0] } else { 0.0 };
            let r = if pair[1].is_finite() { pair[1] } else { 0.0 };
            sum_l2 += l * l;
            sum_r2 += r * r;
            peak = peak.max(l.abs()).max(r.abs());
            self.mono_ring[self.ring_pos] = 0.5 * (l + r) + 0.35 * (l - r);
            self.ring_pos = (self.ring_pos + 1) & (FFT_WINDOW_SIZE - 1);
        }

        let inv_n = if frames > 0 {
            1.0 / (frames as f32)
        } else {
            0.0
        };
        let rms_left = (sum_l2 * inv_n).sqrt().clamp(0.0, 1.0);
        let rms_right = (sum_r2 * inv_n).sqrt().clamp(0.0, 1.0);
        let peak_db = (20.0 * peak.max(1.584_893e-5).log10()).clamp(-96.0, 0.0);

        for i in 0..FFT_WINDOW_SIZE {
            let ring_idx = (self.ring_pos + i) & (FFT_WINDOW_SIZE - 1);
            self.fft_input[i] = self.mono_ring[ring_idx] * self.hann_window[i];
        }
        let _ = self.fft.process_with_scratch(
            &mut self.fft_input,
            &mut self.fft_output,
            &mut self.fft_scratch,
        );

        let hz_per_bin = (self.sample_rate as f32) / (FFT_WINDOW_SIZE as f32);
        let max_hz = ((self.sample_rate as f32) * 0.48).clamp(4_000.0, 20_000.0);
        let min_hz = 25.0_f32;
        let log_ratio = (max_hz / min_hz).ln();
        let parseval_scale_sq = 16.0 / (3.0 * (FFT_WINDOW_SIZE as f32) * (FFT_WINDOW_SIZE as f32));

        let mut spectrum_bins = [0.0_f32; SPECTRUM_BIN_COUNT];
        for (b, slot) in spectrum_bins.iter_mut().enumerate() {
            let f0 = min_hz * (log_ratio * (b as f32) / (SPECTRUM_BIN_COUNT as f32)).exp();
            let f1 = min_hz * (log_ratio * ((b + 1) as f32) / (SPECTRUM_BIN_COUNT as f32)).exp();
            *slot = self.band_energy(f0, f1, hz_per_bin, parseval_scale_sq);
        }

        self.sequence = self.sequence.wrapping_add(1);
        AudioTelemetryFrame {
            sequence: self.sequence,
            sample_rate: self.sample_rate,
            current_time_sec,
            duration_sec,
            is_playing,
            rms_left,
            rms_right,
            peak_db,
            sub_bass: self.band_energy(20.0, 120.0, hz_per_bin, parseval_scale_sq),
            low_mid: self.band_energy(120.0, 500.0, hz_per_bin, parseval_scale_sq),
            high_mid: self.band_energy(500.0, 4_000.0, hz_per_bin, parseval_scale_sq),
            air_presence: self.band_energy(4_000.0, max_hz, hz_per_bin, parseval_scale_sq),
            spectrum_bins,
        }
    }

    fn band_energy(&self, low_hz: f32, high_hz: f32, hz_per_bin: f32, scale_sq: f32) -> f32 {
        let last_bin = self.fft_output.len().saturating_sub(1);
        let start = ((low_hz / hz_per_bin).floor() as usize).clamp(1, last_bin);
        let end = ((high_hz / hz_per_bin).ceil() as usize).clamp(start, last_bin);
        let mut sum_mag_sq = 0.0_f32;
        for c in &self.fft_output[start..=end] {
            sum_mag_sq += c.re * c.re + c.im * c.im;
        }
        ((sum_mag_sq * scale_sq).sqrt() * 1.4).clamp(0.0, 1.0)
    }
}
