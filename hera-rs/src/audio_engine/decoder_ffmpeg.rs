use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

pub const FFMPEG_STEREO_SR: u32 = 48_000;

/// Decodes video/audio containers (`.webm`, `.mkv`, `.flv`, or Symphonia fallbacks)
/// into interleaved stereo `f32` PCM at `48_000 Hz` using `ffmpeg` with `CREATE_NO_WINDOW`.
pub fn decode_stereo_via_ffmpeg(path: &Path) -> Result<(Vec<f32>, u32), String> {
    decode_stereo_via_ffmpeg_bounded(path, None, None)
}

/// Bounded `ffmpeg` stereo decoder (`-t <max_seconds>` when set) with null stdin/stderr
/// (preventing Windows dual-pipe deadlocks) and cooperative cancellation via `cancel`.
pub fn decode_stereo_via_ffmpeg_bounded(
    path: &Path,
    max_seconds: Option<f64>,
    cancel: Option<&AtomicBool>,
) -> Result<(Vec<f32>, u32), String> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-nostdin"]);
    if let Some(secs) = max_seconds.filter(|s| s.is_finite() && *s > 0.0) {
        cmd.args(["-t", &format!("{secs:.3}")]);
    }
    cmd.args(["-vn", "-sn", "-dn", "-i"])
        .arg(path)
        .args([
            "-ac",
            "2",
            "-ar",
            "48000",
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

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("No se pudo iniciar `ffmpeg` en PATH: {e}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Pipe stdout vacío al decodificar con ffmpeg".to_string())?;

    let reserve = max_seconds
        .map(|s| ((s.clamp(1.0, 600.0) * 48_000.0) as usize) * 8)
        .unwrap_or(48_000 * 2 * 4 * 180);
    let mut raw_bytes = Vec::with_capacity(reserve);
    let mut chunk = [0_u8; 8192];
    loop {
        if cancel.is_some_and(|c| c.load(Ordering::Acquire)) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Decodificación ffmpeg cancelada por cambio de pista".to_string());
        }
        match stdout.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => raw_bytes.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Error leyendo stdout de ffmpeg: {e}"));
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("Error esperando a ffmpeg: {e}"))?;

    if !status.success() || raw_bytes.len() < 8 {
        return Err(format!(
            "FFmpeg retornó código {:?} en {:?}",
            status.code(),
            path
        ));
    }

    let num_samples = raw_bytes.len() / 4;
    let mut interleaved = Vec::with_capacity(num_samples);
    for c in raw_bytes.chunks_exact(4) {
        let s = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        interleaved.push(if s.is_finite() { s } else { 0.0 });
    }
    if interleaved.len() % 2 != 0 {
        interleaved.pop();
    }

    Ok((interleaved, FFMPEG_STEREO_SR))
}
