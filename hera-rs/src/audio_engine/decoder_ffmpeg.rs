use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

pub const FFMPEG_STEREO_SR: u32 = 48_000;

/// Decodes video/audio containers (`.webm`, `.mkv`, `.flv`, or Symphonia fallbacks)
/// into interleaved stereo `f32` PCM at `48_000 Hz` using `ffmpeg` with `CREATE_NO_WINDOW`.
pub fn decode_stereo_via_ffmpeg(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-nostdin", "-vn", "-sn", "-dn", "-i"])
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

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

    let mut raw_bytes = Vec::with_capacity(48_000 * 2 * 4 * 180);
    stdout
        .read_to_end(&mut raw_bytes)
        .map_err(|e| format!("Error leyendo stdout de ffmpeg: {e}"))?;

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
    for chunk in raw_bytes.chunks_exact(4) {
        let s = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        interleaved.push(if s.is_finite() { s } else { 0.0 });
    }
    if interleaved.len() % 2 != 0 {
        interleaved.pop();
    }

    Ok((interleaved, FFMPEG_STEREO_SR))
}
