"""Validación de archivos de audio mediante ffprobe y FFmpeg."""

from pathlib import Path
import asyncio
import hashlib
import json
from pydantic import BaseModel, Field


class ValidationResult(BaseModel):
    is_valid: bool
    sha256: str
    codec: str | None = None
    duration_ms: int | None = None
    sample_rate_hz: int | None = None
    bit_depth: int | None = None
    channels: int | None = None
    bitrate_kbps: int | None = None
    file_size_bytes: int = 0
    errors: list[str] = Field(default_factory=list)


class FFmpegValidator:
    def __init__(self, ffmpeg_path: str = "ffmpeg", ffprobe_path: str = "ffprobe"):
        self.ffmpeg_path = ffmpeg_path
        self.ffprobe_path = ffprobe_path

    async def calculate_sha256(self, file_path: Path | str) -> str:
        """Calcula el hash SHA-256 del archivo en streaming."""
        sha = hashlib.sha256()
        with open(file_path, "rb") as f:
            while chunk := f.read(65536):
                sha.update(chunk)
        return sha.hexdigest()

    async def validate_media(self, file_path: Path | str) -> ValidationResult:
        """Inspecciona el contenedor y verifica que contenga un stream de audio válido."""
        path = Path(file_path)
        if not path.exists() or path.stat().st_size == 0:
            return ValidationResult(
                is_valid=False,
                sha256="",
                errors=["El archivo no existe o está vacío."],
            )

        sha256 = await self.calculate_sha256(path)
        size = path.stat().st_size

        # Intentar ejecutar ffprobe
        cmd = [
            self.ffprobe_path,
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            str(path.resolve()),
        ]

        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()

            if proc.returncode != 0:
                # Zero-Trust: Si ffprobe falla, el archivo no puede considerarse válido
                stderr_msg = stderr.decode("utf-8", errors="replace").strip()
                return ValidationResult(
                    is_valid=False,
                    sha256=sha256,
                    file_size_bytes=size,
                    codec=path.suffix.lstrip(".").lower(),
                    errors=[f"ffprobe devolvió error code {proc.returncode}: {stderr_msg or 'Stream inválido'}"],
                )

            data = json.loads(stdout.decode("utf-8", errors="replace"))
            streams = data.get("streams", [])
            audio_stream = next((s for s in streams if s.get("codec_type") == "audio"), None)

            if not audio_stream:
                return ValidationResult(
                    is_valid=False,
                    sha256=sha256,
                    file_size_bytes=size,
                    errors=["No se encontró ningún stream de audio en el archivo."],
                )

            format_info = data.get("format", {})
            duration_sec = float(format_info.get("duration", audio_stream.get("duration", 0)))
            duration_ms = int(duration_sec * 1000) if duration_sec > 0 else None

            sample_rate = int(audio_stream.get("sample_rate", 0)) or None
            channels = int(audio_stream.get("channels", 0)) or None
            bit_depth = int(audio_stream.get("bits_per_raw_sample", audio_stream.get("bits_per_sample", 0))) or None
            bitrate = int(format_info.get("bit_rate", audio_stream.get("bit_rate", 0)))
            bitrate_kbps = bitrate // 1000 if bitrate > 0 else None

            return ValidationResult(
                is_valid=True,
                sha256=sha256,
                codec=audio_stream.get("codec_name"),
                duration_ms=duration_ms,
                sample_rate_hz=sample_rate,
                bit_depth=bit_depth,
                channels=channels,
                bitrate_kbps=bitrate_kbps,
                file_size_bytes=size,
            )

        except FileNotFoundError:
            # ffprobe no está en PATH: Zero-Trust rechaza promover sin validación verificable
            return ValidationResult(
                is_valid=False,
                sha256=sha256,
                file_size_bytes=size,
                codec=path.suffix.lstrip(".").lower(),
                errors=["ffprobe no encontrado en PATH; no se puede verificar la integridad del contenedor."],
            )
