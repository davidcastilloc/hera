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
