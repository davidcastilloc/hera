//! Shared non-exclusive OS audio backend (`cpal`) with automatic `HeadlessClockAudioBackend` fallback.
//!
//! Opens `cpal::default_host().default_output_device()` in shared OS mixer mode (`BufferSize::Default`),
//! never requesting WASAPI/ALSA exclusive lock so VirtualDJ / Rekordbox / Serato run side-by-side.

use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig};

use super::backend::{
    AudioBackend, AudioBackendKind, AudioTransportSnapshot, HeadlessClockAudioBackend,
};
use super::dsp_chain::AudioTelemetryFrame;
use super::hub::{create_controller_and_worker, AudioControllerHub, RealtimeRenderWorker};
use super::{load_deck_buffer_from_bytes, load_deck_buffer_from_file, DeckId, TransportCommand};

const SCRATCH_STEREO_FRAMES: usize = 4096;

/// Shared non-exclusive OS output stream backend powered by `cpal`.
pub struct CpalSharedAudioBackend {
    hub: Arc<AudioControllerHub>,
    shutdown_tx: Option<mpsc::SyncSender<()>>,
    stream_thread: Option<JoinHandle<()>>,
}

impl CpalSharedAudioBackend {
    /// Attempt to open the default OS output device in shared non-exclusive mode.
    pub fn try_new() -> Result<Self, String> {
        let (init_tx, init_rx) = mpsc::sync_channel::<Result<Arc<AudioControllerHub>, String>>(1);
        let (shutdown_tx, shutdown_rx) = mpsc::sync_channel::<()>(1);

        let stream_thread = thread::Builder::new()
            .name("hera-cpal-shared-out".to_string())
            .spawn(move || {
                let host = cpal::default_host();
                let Some(device) = host.default_output_device() else {
                    let _ = init_tx.send(Err("No default OS audio output device".to_string()));
                    return;
                };
                let supported = match device.default_output_config() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("Default output config error: {e}")));
                        return;
                    }
                };

                let sample_format = supported.sample_format();
                let channels = supported.channels().max(1) as usize;
                let sample_rate = supported.sample_rate();
                let config = StreamConfig {
                    channels: supported.channels(),
                    sample_rate,
                    buffer_size: BufferSize::Default,
                };

                let (hub, worker) =
                    create_controller_and_worker(AudioBackendKind::CpalSharedDevice, sample_rate);
                let hub = Arc::new(hub);

                let stream_res = match sample_format {
                    SampleFormat::F32 => build_typed_stream::<f32>(&device, &config, channels, worker),
                    SampleFormat::I16 => build_typed_stream::<i16>(&device, &config, channels, worker),
                    SampleFormat::U16 => build_typed_stream::<u16>(&device, &config, channels, worker),
                    other => Err(format!("Unsupported cpal sample format: {other:?}")),
                };

                match stream_res {
                    Ok(stream) => {
                        if let Err(e) = stream.play() {
                            let _ = init_tx.send(Err(format!("Failed to start cpal stream: {e}")));
                            return;
                        }
                        let _ = init_tx.send(Ok(hub));
                        let _ = shutdown_rx.recv();
                        drop(stream);
                    }
                    Err(err) => {
                        let _ = init_tx.send(Err(err));
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn cpal owner thread: {e}"))?;

        match init_rx.recv() {
            Ok(Ok(hub)) => Ok(Self {
                hub,
                shutdown_tx: Some(shutdown_tx),
                stream_thread: Some(stream_thread),
            }),
            Ok(Err(err)) => {
                let _ = stream_thread.join();
                Err(err)
            }
            Err(e) => {
                let _ = stream_thread.join();
                Err(format!("cpal stream thread aborted during init: {e}"))
            }
        }
    }
}

fn build_typed_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    mut worker: RealtimeRenderWorker,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let mut stereo_scratch = vec![0.0_f32; SCRATCH_STEREO_FRAMES * 2];
    device
        .build_output_stream(
            config.clone(),
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let total_frames = data.len() / channels;
                let mut offset_frames = 0usize;
                while offset_frames < total_frames {
                    let chunk = (total_frames - offset_frames).min(SCRATCH_STEREO_FRAMES);
                    let stereo_slice = &mut stereo_scratch[..chunk * 2];
                    worker.render_stereo_block(stereo_slice);
                    for f in 0..chunk {
                        let l = stereo_slice[f * 2];
                        let r = stereo_slice[f * 2 + 1];
                        let out_base = (offset_frames + f) * channels;
                        if channels == 1 {
                            data[out_base] = T::from_sample(0.5 * (l + r));
                        } else {
                            data[out_base] = T::from_sample(l);
                            data[out_base + 1] = T::from_sample(r);
                            for extra in 2..channels {
                                data[out_base + extra] = T::EQUILIBRIUM;
                            }
                        }
                    }
                    offset_frames += chunk;
                }
            },
            |err| {
                eprintln!("[hera-audio] cpal stream error: {err}");
            },
            None,
        )
        .map_err(|e| format!("Failed to build shared cpal output stream: {e}"))
}

impl Drop for CpalSharedAudioBackend {
    fn drop(&mut self) {
        drop(self.shutdown_tx.take());
        if let Some(handle) = self.stream_thread.take() {
            let _ = handle.join();
        }
    }
}

impl AudioBackend for CpalSharedAudioBackend {
    fn backend_kind(&self) -> AudioBackendKind {
        AudioBackendKind::CpalSharedDevice
    }
    fn output_sample_rate(&self) -> u32 {
        self.hub.atomics.sample_rate
    }
    fn load_file(&self, deck: DeckId, path: &Path) -> Result<AudioTransportSnapshot, String> {
        let raw = load_deck_buffer_from_file(path)?;
        self.hub
            .install_buffer(deck, raw, Some(path.to_string_lossy().to_string()))
    }
    fn load_bytes(
        &self,
        deck: DeckId,
        bytes: &[u8],
        ext_hint: Option<&str>,
    ) -> Result<AudioTransportSnapshot, String> {
        let raw = load_deck_buffer_from_bytes(bytes, ext_hint)?;
        self.hub.install_buffer(deck, raw, None)
    }
    fn send_command(&self, cmd: TransportCommand) -> AudioTransportSnapshot {
        self.hub.send_command(cmd)
    }
    fn set_master_volume(&self, volume: f32) -> AudioTransportSnapshot {
        self.hub.set_master_volume(volume)
    }
    fn snapshot(&self) -> AudioTransportSnapshot {
        self.hub.snapshot()
    }
    fn poll_latest_telemetry(&self) -> Option<AudioTelemetryFrame> {
        self.hub.poll_latest_telemetry()
    }
}

/// Create the shared OS `cpal` audio backend, falling back cleanly to [`HeadlessClockAudioBackend`]
/// when running on a headless machine or VM with no soundcard.
pub fn create_default_audio_backend() -> Arc<dyn AudioBackend> {
    match CpalSharedAudioBackend::try_new() {
        Ok(cpal_backend) => Arc::new(cpal_backend),
        Err(_) => Arc::new(HeadlessClockAudioBackend::default()),
    }
}
