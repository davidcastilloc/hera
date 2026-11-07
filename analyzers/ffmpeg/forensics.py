"""Auditoría forense de pureza acústica para HERA (Zero-Trust Audio Forensics).

Detecta:
1. Transcodificaciones falsas (Fake FLAC / Fake 320 kbps re-escalados desde MP3 128 kbps o rips de YouTube)
   mediante análisis del corte espectral tipo ladrillo (Brickwall Roll-off en 15.5-16.2 kHz).
2. Tonos sintéticos o placeholders senoidales (Peak Ratio > 0.35 y Spectral Flatness < 1e-4).
3. Clipping digital duro (muestras consecutivas saturadas a >= 0.998 FS).
4. Correlación de fase en sub-graves (< 120 Hz) para garantizar compatibilidad mono en subwoofers de club.
"""

from __future__ import annotations
from pathlib import Path
import subprocess
import numpy as np
from pydantic import BaseModel, Field


LOSSLESS_CODECS = {"flac", "wav", "alac", "aiff", "aif", "pcm_s16le", "pcm_s24le", "pcm_f32le"}


class ForensicAuditReport(BaseModel):
    """Reporte forense de autenticidad y salud acústica de un archivo de audio."""
    verdict: str = Field(
        ...,
        description="STUDIO_MASTER | CLEAN_LOSSY | FAKE_TRANSCODE | CLIPPED_MASTER | SYNTHETIC_REJECTED | PHASE_ISSUE",
    )
    authenticity_score: float = Field(..., ge=0.0, le=100.0, description="Puntuación de pureza acústica (0..100)")
    effective_cutoff_hz: float = Field(..., ge=0.0, description="Frecuencia de corte espectral efectivo en Hz")
    estimated_true_bitrate_kbps: int = Field(..., ge=32, le=1411, description="Bitrate real estimado según espectro")
    is_fake_upscale: bool = Field(default=False, description="True si aparenta ser lossless/320k pero corta en <=16.3kHz")
    is_synthetic_tone: bool = Field(default=False, description="True si es una onda senoidal / tono sintético de prueba")
    spectral_flatness: float = Field(..., ge=0.0, le=1.0, description="Entropía de Wiener (planitud espectral)")
    peak_ratio: float = Field(..., ge=0.0, le=1.0, description="Concentración de energía en los 3 bines dominantes")
    clipped_samples_count: int = Field(default=0, ge=0, description="Muestras con saturación digital dura")
    clipping_ratio: float = Field(default=0.0, ge=0.0, le=1.0, description="Proporción de muestras saturadas")
    sub_bass_phase_correlation: float = Field(
        default=1.0, ge=-1.0, le=1.0, description="Correlación de fase L/R bajo 120 Hz (1.0 = mono perfecto)"
    )
    mono_compatible_sub: bool = Field(default=True, description="True si el sub-bajo no se cancela en sistemas mono")
    warnings: list[str] = Field(default_factory=list, description="Advertencias técnicas para el DJ / ingeniero")


class AudioForensicsAnalyzer:
    """Motor DSP forense para verificación Zero-Trust de masters de audio."""

    @classmethod
    def analyze_signal(
        cls,
        y: np.ndarray,
        sr: int = 44100,
        claimed_codec: str | None = "flac",
        claimed_bitrate_kbps: int | None = 1411,
    ) -> ForensicAuditReport:
        """Ejecuta auditoría forense sobre un array de audio (mono 1D o estéreo 2D [N, 2] / [2, N])."""
        if y is None or y.size == 0:
            return ForensicAuditReport(
                verdict="SYNTHETIC_REJECTED",
                authenticity_score=0.0,
                effective_cutoff_hz=0.0,
                estimated_true_bitrate_kbps=32,
                is_fake_upscale=False,
                is_synthetic_tone=True,
                spectral_flatness=0.0,
                peak_ratio=1.0,
                warnings=["Señal vacía o nula."],
            )

        # 1. Separar canales si es estéreo y obtener mezcla mono para análisis espectral
        if y.ndim == 2:
            if y.shape[0] == 2 and y.shape[1] > 2:
                left, right = y[0].astype(np.float32), y[1].astype(np.float32)
            elif y.shape[1] == 2:
                left, right = y[:, 0].astype(np.float32), y[:, 1].astype(np.float32)
            else:
                left = right = y.reshape(-1).astype(np.float32)
            y_mono = 0.5 * (left + right)
            has_stereo = True
        else:
            y_mono = y.astype(np.float32)
            left = right = y_mono
            has_stereo = False

        # 2. Auditoría de Clipping Digital
        abs_y = np.abs(y_mono)
        clipped_mask = abs_y >= 0.9985
        clipped_count = int(np.sum(clipped_mask))
        clip_ratio = float(clipped_count / max(1, len(y_mono)))

        # 3. Correlación de Fase en Sub-Graves (< 120 Hz) para Club Subwoofers
        sub_phase_corr = 1.0
        if has_stereo and len(left) > 2048:
            # Filtro paso-bajo rápido en dominio de frecuencia (< 120 Hz)
            n_sub = min(len(left), sr * 15)
            L_fft = np.fft.rfft(left[:n_sub])
            R_fft = np.fft.rfft(right[:n_sub])
            freqs_sub = np.fft.rfftfreq(n_sub, 1.0 / sr)
            mask_120 = (freqs_sub >= 20.0) & (freqs_sub <= 120.0)
            if np.any(mask_120):
                L_low = np.fft.irfft(L_fft * mask_120, n=n_sub)
                R_low = np.fft.irfft(R_fft * mask_120, n=n_sub)
                std_l = float(np.std(L_low))
                std_r = float(np.std(R_low))
                if std_l > 1e-6 and std_r > 1e-6:
                    sub_phase_corr = float(np.clip(np.corrcoef(L_low, R_low)[0, 1], -1.0, 1.0))

        mono_ok = sub_phase_corr >= 0.20

        # 4. Espectro STFT promediado (Welch-like) para Planitud Espectral, Peak Ratio y Frecuencia de Corte
        n_fft = 2048 if len(y_mono) >= 2048 else max(256, 1 << (len(y_mono).bit_length() - 1))
        hop = n_fft // 2
        n_frames = max(1, (len(y_mono) - n_fft) // hop)
        if n_frames > 1:
            strides = (y_mono.strides[0] * hop, y_mono.strides[0])
            frames = np.lib.stride_tricks.as_strided(y_mono, shape=(n_frames, n_fft), strides=strides)
            win = np.hanning(n_fft).astype(np.float32)
            power_spec = np.mean(np.abs(np.fft.rfft(frames * win, axis=1)) ** 2, axis=0)
        else:
            power_spec = np.abs(np.fft.rfft(y_mono[:n_fft] * np.hanning(len(y_mono[:n_fft])))) ** 2

        freqs = np.fft.rfftfreq(n_fft, 1.0 / sr)
        non_dc = power_spec[1:] if len(power_spec) > 1 else power_spec
        total_power = float(np.sum(non_dc)) + 1e-12

        # Peak Ratio (concentración en los 3 bines más fuertes) y Spectral Flatness (media geométrica / aritmética)
        sorted_bins = np.sort(non_dc)
        top3_power = float(np.sum(sorted_bins[-3:]))
        peak_ratio = float(np.clip(top3_power / total_power, 0.0, 1.0))

        pos_bins = np.maximum(non_dc, 1e-15)
        geo_mean = float(np.exp(np.mean(np.log(pos_bins))))
        arith_mean = float(np.mean(pos_bins))
        spectral_flatness = float(np.clip(geo_mean / max(arith_mean, 1e-15), 0.0, 1.0))

        # Regla invariante Zero-Trust: Peak Ratio > 0.35 y Spectral Flatness < 1e-4 (o senoidal pura concentrada)
        is_synthetic = bool(
            (peak_ratio > 0.35 and spectral_flatness < 1e-4)
            or (peak_ratio > 0.82 and spectral_flatness < 0.005)
        )

        # 5. Cálculo de Frecuencia de Corte Espectral (Brickwall Roll-off)
        psd_db = 10.0 * np.log10(np.maximum(power_spec, 1e-16))
        mid_band_mask = (freqs >= 800.0) & (freqs <= min(5500.0, sr * 0.35))
        ref_db = float(np.percentile(psd_db[mid_band_mask], 75)) if np.any(mid_band_mask) else float(np.max(psd_db))

        # Umbral de piso activo (-42 dB respecto a la referencia de medios)
        active_bins = np.where((freqs >= 1000.0) & (psd_db >= (ref_db - 42.0)))[0]
        raw_cutoff_hz = float(freqs[active_bins[-1]]) if len(active_bins) > 0 else float(sr / 2.0)

        # Si el muestreo es < 40 kHz (p.ej. buffer rápido a 22050 Hz con Nyquist = 11025 Hz),
        # escalar o verificar si llega hasta el borde de Nyquist
        nyquist = sr / 2.0
        if sr < 40000:
            if raw_cutoff_hz >= nyquist * 0.90:
                effective_cutoff_hz = 20500.0
            else:
                effective_cutoff_hz = raw_cutoff_hz * (22050.0 / nyquist)
        else:
            effective_cutoff_hz = raw_cutoff_hz

        # Verificar escalón ladrillo (brickwall) entre 13.5-15.5 kHz y 16.5-20.0 kHz cuando sr >= 40000
        brickwall_drop_db = 0.0
        if sr >= 40000:
            b_pre = (freqs >= 13000.0) & (freqs <= 15400.0)
            b_post = (freqs >= 16500.0) & (freqs <= 20200.0)
            if np.any(b_pre) and np.any(b_post):
                brickwall_drop_db = float(np.mean(psd_db[b_pre]) - np.mean(psd_db[b_post]))
                if brickwall_drop_db >= 24.0 and effective_cutoff_hz > 16200.0:
                    effective_cutoff_hz = 15800.0

        # Estimación de bitrate real equivalente
        if effective_cutoff_hz >= 19600.0:
            est_bitrate = 1411 if (claimed_codec and claimed_codec.lower() in LOSSLESS_CODECS) else 320
        elif effective_cutoff_hz >= 18200.0:
            est_bitrate = 256
        elif effective_cutoff_hz >= 16400.0:
            est_bitrate = 192
        elif effective_cutoff_hz >= 14500.0:
            est_bitrate = 128
        else:
            est_bitrate = 96

        codec_l = (claimed_codec or "").lower().lstrip(".")
        claimed_high_quality = (codec_l in LOSSLESS_CODECS) or ((claimed_bitrate_kbps or 0) >= 256)
        is_fake_upscale = bool(
            not is_synthetic
            and claimed_high_quality
            and (effective_cutoff_hz < 16400.0 or brickwall_drop_db >= 24.0)
        )

        # 6. Construcción de advertencias y puntuación
        warnings: list[str] = []
        score = 100.0

        if is_synthetic:
            score = 5.0
            verdict = "SYNTHETIC_REJECTED"
            warnings.append(
                f"Tono sintético / onda senoidal detectada (Peak Ratio={peak_ratio:.2f}, Flatness={spectral_flatness:.5f})."
            )
        elif is_fake_upscale:
            score -= 45.0
            verdict = "FAKE_TRANSCODE"
            warnings.append(
                f"Fake Upscale detectado: declarado como '{codec_l.upper() or 'HIGH'}' pero presenta corte brickwall en "
                f"{effective_cutoff_hz / 1000.0:.1f} kHz (equivalente a ~{est_bitrate} kbps)."
            )
        else:
            verdict = "STUDIO_MASTER" if codec_l in LOSSLESS_CODECS or est_bitrate >= 320 else "CLEAN_LOSSY"

        if clip_ratio > 0.002 or clipped_count > 250:
            penalty = min(30.0, 10.0 + clip_ratio * 2500.0)
            score -= penalty
            if verdict in ("STUDIO_MASTER", "CLEAN_LOSSY"):
                verdict = "CLIPPED_MASTER"
            warnings.append(
                f"Clipping digital detectado: {clipped_count} muestras saturadas ({clip_ratio * 100:.2f}%)."
            )

        if not mono_ok:
            score -= 25.0
            if verdict in ("STUDIO_MASTER", "CLEAN_LOSSY"):
                verdict = "PHASE_ISSUE"
            warnings.append(
                f"Riesgo de cancelación en subwoofers mono: correlación de fase sub-120Hz = {sub_phase_corr:.2f}."
            )

        score = float(np.clip(round(score, 1), 0.0, 100.0))

        return ForensicAuditReport(
            verdict=verdict,
            authenticity_score=score,
            effective_cutoff_hz=round(effective_cutoff_hz, 1),
            estimated_true_bitrate_kbps=est_bitrate,
            is_fake_upscale=is_fake_upscale,
            is_synthetic_tone=is_synthetic,
            spectral_flatness=round(spectral_flatness, 6),
            peak_ratio=round(peak_ratio, 4),
            clipped_samples_count=clipped_count,
            clipping_ratio=round(clip_ratio, 5),
            sub_bass_phase_correlation=round(sub_phase_corr, 3),
            mono_compatible_sub=mono_ok,
            warnings=warnings,
        )

    @classmethod
    def analyze_file(
        cls,
        file_path: Path | str,
        claimed_codec: str | None = None,
        claimed_bitrate_kbps: int | None = None,
        ffmpeg_path: str = "ffmpeg",
    ) -> ForensicAuditReport:
        """Decodifica una ventana estéreo a 44.1 kHz (Nyquist 22.05 kHz) y ejecuta la auditoría forense completa."""
        p = Path(file_path)
        codec = claimed_codec or p.suffix.lstrip(".").lower()
        sr = 44100
        cmd = [
            ffmpeg_path, "-v", "quiet",
            "-ss", "20", "-t", "25",
            "-i", str(p),
            "-vn", "-f", "f32le", "-ac", "2", "-ar", str(sr), "-"
        ]
        raw = subprocess.run(cmd, capture_output=True).stdout
        if len(raw) < sr * 4 * 2 * 3:
            cmd = [
                ffmpeg_path, "-v", "quiet",
                "-t", "20",
                "-i", str(p),
                "-vn", "-f", "f32le", "-ac", "2", "-ar", str(sr), "-"
            ]
            raw = subprocess.run(cmd, capture_output=True).stdout

        if not raw:
            return cls.analyze_signal(np.array([], dtype=np.float32), sr=sr, claimed_codec=codec)

        samples = np.frombuffer(raw, dtype=np.float32)
        n_stereo = len(samples) // 2
        stereo = samples[: n_stereo * 2].reshape(n_stereo, 2)
        return cls.analyze_signal(
            stereo,
            sr=sr,
            claimed_codec=codec,
            claimed_bitrate_kbps=claimed_bitrate_kbps,
        )
