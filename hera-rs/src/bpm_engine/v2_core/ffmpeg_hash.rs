use super::*;
use crate::bpm_engine::v2_core::audio_ingest::DecodedAudio;
use std::io::Read;

pub fn decode_via_ffmpeg_pipe(path: &Path) -> Result<DecodedAudio, EngineError> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-nostdin", "-vn", "-sn", "-dn", "-i"])
        .arg(path)
        .args(["-ac", "1", "-ar", "44100", "-f", "f32le", "-acodec", "pcm_f32le", "pipe:1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().map_err(|e| {
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
