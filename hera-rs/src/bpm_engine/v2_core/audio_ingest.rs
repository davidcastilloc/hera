    use super::*;

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

    #[cfg(not(target_arch = "wasm32"))]
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
                } else if tag.key.eq_ignore_ascii_case("TKEY")
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

    #[cfg(target_arch = "wasm32")]
    fn try_decode_with_symphonia(_path: &Path) -> Result<DecodedAudio, EngineError> {
        Err(EngineError::DemuxOrDecode(
            "Symphonia no disponible en wasm32".into(),
        ))
    }

    pub use super::ffmpeg_hash::{decode_via_ffmpeg_pipe, fast_partial_file_hash};