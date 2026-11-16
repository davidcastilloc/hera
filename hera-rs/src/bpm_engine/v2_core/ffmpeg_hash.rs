use super::*;
use crate::bpm_engine::v2_core::audio_ingest::DecodedAudio;
use std::io::Read;
use std::sync::mpsc;
use std::time::Duration;

pub fn run_ffmpeg_f32le_pipe_bounded(
    path: &Path,
    window: Option<(f64, f64)>,
    timeout_secs: u64,
) -> Result<Vec<f32>, EngineError> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "quiet", "-nostdin"]);
    if let Some((start, _)) = window {
        cmd.arg("-ss").arg(format!("{start:.3}"));
    }
    cmd.arg("-i").arg(path);
    if let Some((_, dur)) = window {
        cmd.arg("-t").arg(format!("{dur:.3}"));
    } else {
        cmd.args(["-t", "420"]);
    }
    cmd.args([
        "-vn",
        "-sn",
        "-dn",
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
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let mut child = cmd.spawn().map_err(|e| {
        EngineError::FfmpegFallbackFailed(format!("No se pudo iniciar `ffmpeg`: {e}"))
    })?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| EngineError::FfmpegFallbackFailed("Pipe stdout vacío".into()))?;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::with_capacity(44_100 * 4 * 60);
        let res = stdout.read_to_end(&mut buf).map(|_| buf);
        let _ = tx.send(res);
    });

    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(2));
    let exited_ok = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(EngineError::FfmpegFallbackFailed(format!(
                    "FFmpeg timeout tras {timeout_secs}s en {path:?}"
                )));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(EngineError::FfmpegFallbackFailed(e.to_string()));
            }
        }
    };

    let raw_bytes = rx
        .recv_timeout(Duration::from_secs(2))
        .map_err(|e| EngineError::FfmpegFallbackFailed(format!("Timeout leyendo stdout: {e}")))?
        .map_err(|e| EngineError::FfmpegFallbackFailed(e.to_string()))?;

    if !exited_ok || raw_bytes.len() < 4 {
        return Err(EngineError::FfmpegFallbackFailed(format!(
            "FFmpeg no decodificó audio válido en {path:?}"
        )));
    }

    let mut samples = Vec::with_capacity(raw_bytes.len() / 4);
    for chunk in raw_bytes.chunks_exact(4) {
        let s = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        samples.push(if s.is_finite() { s } else { 0.0 });
    }
    Ok(samples)
}

pub fn decode_via_ffmpeg_pipe(path: &Path) -> Result<DecodedAudio, EngineError> {
    let samples = run_ffmpeg_f32le_pipe_bounded(path, None, 20)?;
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
    let len = file
        .metadata()
        .map_err(|e| EngineError::Io(e.to_string()))?
        .len();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_ffmpeg_pipe_returns_fast_err_on_missing_file_without_hanging() {
        let bogus = Path::new("C:/nonexistent_hera_missing_track_12345.mp3");
        let start = Instant::now();
        let res = run_ffmpeg_f32le_pipe_bounded(bogus, Some((0.0, 5.0)), 3);
        assert!(res.is_err());
        assert!(start.elapsed() < Duration::from_secs(4));
    }
}
