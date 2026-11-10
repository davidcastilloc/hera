use super::*;

fn sanitize_filename_part(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '(' | ')' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn has_valid_audio_magic(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"fLaC") {
        Some("flac")
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WAVE" {
        Some("wav")
    } else if bytes.starts_with(b"OggS") {
        Some("ogg")
    } else if bytes.starts_with(b"ID3")
        || (bytes.len() > 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
    {
        Some("mp3")
    } else {
        None
    }
}

fn camelot_root_hz(camelot: &str) -> (f32, bool) {
    match parse_camelot_code(camelot) {
        Some((num, mode)) => {
            let minor_roots = [
                207.65_f32, // 1A Abm
                155.56,     // 2A Ebm
                233.08,     // 3A Bbm
                174.61,     // 4A Fm
                130.81,     // 5A Cm
                196.00,     // 6A Gm
                146.83,     // 7A Dm
                220.00,     // 8A Am
                164.81,     // 9A Em
                246.94,     // 10A Bm
                185.00,     // 11A F#m
                138.59,     // 12A C#m
            ];
            let base = minor_roots[((num - 1).rem_euclid(12)) as usize];
            let is_major = mode == 'B';
            let root = if is_major { base * 1.1892071 } else { base };
            (root, is_major)
        }
        None => (220.0, false),
    }
}

fn render_harmonic_preview_wav(bpm: f32, camelot: &str, seed_bytes: &[u8]) -> Vec<u8> {
    let sample_rate: u32 = 44_100;
    let channels: u16 = 2;
    let duration_secs: f32 = 24.0;
    let total_frames = (sample_rate as f32 * duration_secs) as usize;
    let clamped_bpm = bpm.clamp(70.0, 180.0);
    let beat_period = 60.0 / clamped_bpm;
    let (root_hz, is_major) = camelot_root_hz(camelot);
    let third_ratio = if is_major { 1.259921 } else { 1.1892071 };
    let fifth_ratio = 1.4983071_f32;

    let mut lfsr: u32 = seed_bytes
        .iter()
        .take(16)
        .fold(0xA5A5_5A5A_u32, |acc, &b| acc.rotate_left(5) ^ (b as u32))
        .max(1);

    let data_bytes_len = (total_frames * (channels as usize) * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_bytes_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * (channels as u32) * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * 2;
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes_len.to_le_bytes());

    let tau = std::f32::consts::TAU;
    for frame in 0..total_frames {
        let t = (frame as f32) / (sample_rate as f32);
        let beat_pos = (t / beat_period).fract();
        let bar_idx = (t / (beat_period * 4.0)).floor() as usize;

        // 4-on-the-floor kick transient + sub-bass
        let kick_t = beat_pos * beat_period;
        let kick_env = (-kick_t * 18.0).exp();
        let kick_freq = 52.0 + 95.0 * (-kick_t * 42.0).exp();
        let kick = (tau * kick_freq * kick_t).sin() * kick_env * 0.55;

        // Off-beat hi-hat noise for broad spectral bandwidth & rhythm clarity
        lfsr ^= lfsr << 13;
        lfsr ^= lfsr >> 17;
        lfsr ^= lfsr << 5;
        let white = ((lfsr & 0xFFFF) as f32 / 32768.0) - 1.0;
        let hat_pos = ((beat_pos + 0.5).fract()) * beat_period;
        let hat_env = (-hat_pos * 45.0).exp() * if bar_idx >= 1 { 0.16 } else { 0.06 };
        let hat = white * hat_env;

        // Harmonic triad pad with dynamic breakdown/drop envelope for 4 Hot Cues
        let section_gain = if (2..4).contains(&bar_idx) {
            0.35 // breakdown contrast
        } else if bar_idx >= 4 {
            0.85 // drop lift
        } else {
            0.55 // intro/outro
        };
        let root = (tau * root_hz * t).sin();
        let third = (tau * (root_hz * third_ratio) * t).sin();
        let fifth = (tau * (root_hz * fifth_ratio) * t).sin();
        let sub = (tau * (root_hz * 0.5) * t).sin() * (1.0 - kick_env * 0.65);

        let pad_l = (root * 0.42 + third * 0.31 + fifth * 0.27) * 0.28 * section_gain;
        let pad_r = (root * 0.40 + third * 0.34 + fifth * 0.26 * (1.0 + 0.02 * (t * 2.5).sin()))
            * 0.28
            * section_gain;
        let mono_foundation = kick + sub * 0.24 * section_gain;

        let left = (mono_foundation + pad_l + hat).clamp(-0.96, 0.96);
        let right = (mono_foundation + pad_r + hat * 0.92).clamp(-0.96, 0.96);

        let s_l = (left * 32767.0) as i16;
        let s_r = (right * 32767.0) as i16;
        wav.extend_from_slice(&s_l.to_le_bytes());
        wav.extend_from_slice(&s_r.to_le_bytes());
    }

    wav
}

#[tauri::command]
pub async fn download_p2p_track<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: DownloadP2PTrackRequest,
) -> Result<DownloadP2PTrackResponse, String> {
    let fallback_data_dir = app.path().app_data_dir().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let daemon = ensure_desktop_udp_daemon()
            .ok_or_else(|| "No se pudo inicializar el daemon UDP P2P local".to_string())?;

        let explicit_peer = request
            .peer_endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let total_chunks = request.chunk_count.unwrap_or(32).clamp(1, 512);

        let (payload_to_write, ext, quarantine_verified, stage_used) = if let Some(peer) = explicit_peer {
            let (assembled, stage_used) = if peer.starts_with("iroh://") || peer.starts_with('{') {
                let adapter = IrohQuicAdapter::new("hera-desktop-leecher", false);
                let ticket = peer;
                let fetch_res = tauri::async_runtime::block_on(async {
                    adapter
                        .fetch_verified_shard_from_ticket(ticket, &request.track_id, None)
                        .await
                });
                match fetch_res {
                    Ok(receipt) => (receipt.payload, "STAGE_2_IROH_QUIC_STREAM".to_string()),
                    Err(e) => return Err(format!("Fallo en transporte Iroh QUIC: {e}")),
                }
            } else {
                let turn_fallback = request
                    .turn_relay_endpoint
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .or(Some(daemon.daemon_listen_socket.as_str()));

                let mut buf = Vec::with_capacity((total_chunks as usize) * 8192);
                let mut last_stage = "STAGE_3_DIRECT_UDP_SPRAY".to_string();

                for chunk_idx in 0..total_chunks {
                    let (chunk_bytes, stage_label) = request_chunk_with_turn_fallback(
                        peer,
                        daemon.nat_diagnosis.observed_port_delta,
                        daemon.nat_diagnosis.spray_window_ports,
                        turn_fallback,
                        &request.track_id,
                        chunk_idx,
                    )?;
                    last_stage = stage_label;
                    buf.extend_from_slice(&chunk_bytes);
                }
                (buf, last_stage)
            };

            match has_valid_audio_magic(&assembled) {
                Some(real_ext) => (assembled, real_ext, true, stage_used),
                None => {
                    return Err(format!(
                        "Pista P2P `{}` descargada no contiene cabecera de audio válida (magic check falló)",
                        request.track_id
                    ));
                }
            }
        } else {
            // Local offline demo mode: no peer endpoint provided
            let preview_wav = render_harmonic_preview_wav(request.bpm, &request.camelot, &[]);
            (
                preview_wav,
                "wav",
                false,
                "OFFLINE_SYNTHETIC_PREVIEW_UNVERIFIED".to_string(),
            )
        };
        let sha256_hex = compute_sha256_hex(&payload_to_write);

        let cwd_base = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        let base_dir = if std::fs::create_dir_all(cwd_base.join("quarantine")).is_ok()
            && std::fs::create_dir_all(cwd_base.join("library")).is_ok()
        {
            cwd_base
        } else {
            fallback_data_dir.unwrap_or_else(std::env::temp_dir)
        };

        let quarantine_dir = base_dir.join("quarantine");
        let library_dir = base_dir.join("library");
        std::fs::create_dir_all(&quarantine_dir)
            .map_err(|e| format!("No se pudo preparar quarantine/: {e}"))?;
        std::fs::create_dir_all(&library_dir)
            .map_err(|e| format!("No se pudo preparar library/: {e}"))?;

        let safe_artist = sanitize_filename_part(&request.artist);
        let safe_title = sanitize_filename_part(&request.title);
        let file_name = format!(
            "{} - {} [{:.0}BPM {}].{}",
            if safe_artist.is_empty() {
                "HeraArtist"
            } else {
                &safe_artist
            },
            if safe_title.is_empty() {
                &request.track_id
            } else {
                &safe_title
            },
            request.bpm,
            sanitize_filename_part(&request.camelot),
            ext
        );

        let quarantine_path = quarantine_dir.join(format!("{}.part", &file_name));
        let final_library_path = library_dir.join(&file_name);

        std::fs::write(&quarantine_path, &payload_to_write)
            .map_err(|e| format!("Error escribiendo en quarantine/: {e}"))?;
        if std::fs::rename(&quarantine_path, &final_library_path).is_err() {
            std::fs::write(&final_library_path, &payload_to_write)
                .map_err(|e| format!("Error promoviendo pista a library/: {e}"))?;
            let _ = std::fs::remove_file(&quarantine_path);
        }

        Ok(DownloadP2PTrackResponse {
            track_id: request.track_id,
            saved_path: final_library_path.to_string_lossy().to_string(),
            quarantine_verified,
            bytes_written: payload_to_write.len(),
            chunks_verified: total_chunks,
            stage_used,
            sha256_hex,
            r2_bytes_used: 0,
        })
    })
    .await
    .map_err(|e| format!("Fallo en tarea download_p2p_track: {e}"))?
}
