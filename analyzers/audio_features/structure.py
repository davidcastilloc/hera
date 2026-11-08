"""Motor de segmentación estructural acústica y detección de Hot Cues reales para HERA.

Reemplaza los offsets estáticos por detección real sobre la señal de audio:
1. Fase exacta del Primer Downbeat (Compás 1.1.1) mediante plegado de fase de sub-graves.
2. Entrada de Frase / Voz (Cue 2) en fronteras cuantizadas de 4/8 compases.
3. Detección de Drop / Clímax (Cue 3) mediante contraste Breakdown-to-Drop en sub-bajo + brillo.
4. Punto óptimo de Salida de Mezcla / Outro Loop (Cue 4) cuantizado al beatgrid.
5. Banderas de actividad vocal en Intro/Outro para evitar choques vocales (Vocal Clash Avoidance).
"""

from __future__ import annotations
import numpy as np
from pydantic import BaseModel, Field


class HotCuePoint(BaseModel):
    """Punto de Hot Cue cuantizado al beatgrid y alineado a estructura musical."""
    num: int = Field(..., ge=1, le=8, description="Número de Hot Cue (1..8)")
    name: str = Field(..., description="Etiqueta visible en cabina (Rekordbox / Universal DJ XML)")
    position_sec: float = Field(..., ge=0.0, description="Posición exacta en segundos")
    bar_index: int = Field(..., ge=0, description="Índice de compás (0-based) desde el primer downbeat")
    confidence: float = Field(default=0.90, ge=0.0, le=1.0, description="Confianza de la frontera estructural")
    cue_type: str = Field(..., description="Tipo semántico: downbeat | phrase_in | drop | mix_out")


class StructuralCueProfile(BaseModel):
    """Perfil estructural completo de una pista musical para mezcla DJ."""
    first_downbeat_sec: float = Field(..., ge=0.0, description="Segundo exacto del primer golpe 1.1.1")
    phrase_in_sec: float = Field(..., ge=0.0, description="Entrada de frase / voz (Cue 2)")
    drop_climax_sec: float = Field(..., ge=0.0, description="Drop principal o clímax energético (Cue 3)")
    mix_out_sec: float = Field(..., ge=0.0, description="Inicio del outro / salida de mezcla (Cue 4)")
    bar_duration_sec: float = Field(..., gt=0.0, description="Duración de un compás de 4/4 en segundos")
    vocal_intro_active: bool = Field(default=False, description="Presencia vocal detectada en los primeros 8 compases")
    vocal_outro_active: bool = Field(default=False, description="Presencia vocal detectada en los últimos 8 compases")
    drop_intensity_ratio: float = Field(default=1.25, ge=1.0, description="Relación de impacto energético en el Drop")
    hot_cues: list[HotCuePoint] = Field(default_factory=list, description="Lista ordenada de los 4 Hot Cues estructurales")


class StructuralCueAnalyzer:
    """Analizador DSP de estructura musical por compases, frases y contraste energético."""

    @staticmethod
    def build_fallback_profile(bpm: float = 124.0, duration_sec: float = 210.0) -> StructuralCueProfile:
        """Construye un perfil estructural cuantizado por compases cuando no hay señal PCM disponible."""
        safe_bpm = max(60.0, min(200.0, float(bpm or 124.0)))
        sec_per_beat = 60.0 / safe_bpm
        bar_sec = 4.0 * sec_per_beat
        dur = max(30.0, float(duration_sec or 210.0))

        cue1 = 0.20
        cue2 = min(dur * 0.25, cue1 + 8.0 * bar_sec)
        cue3 = min(dur * 0.45, cue1 + 16.0 * bar_sec)
        cue4 = max(dur * 0.65, dur - 16.0 * bar_sec)

        cues = [
            HotCuePoint(num=1, name="1. INTRO BEAT", position_sec=round(cue1, 3), bar_index=0, confidence=0.75, cue_type="downbeat"),
            HotCuePoint(num=2, name="2. ENTRADA (8 COMP)", position_sec=round(cue2, 3), bar_index=8, confidence=0.75, cue_type="phrase_in"),
            HotCuePoint(num=3, name="3. DROP / CORO (16C)", position_sec=round(cue3, 3), bar_index=16, confidence=0.75, cue_type="drop"),
            HotCuePoint(
                num=4,
                name="4. SALIDA MIX (-16C)",
                position_sec=round(cue4, 3),
                bar_index=max(24, int(round((cue4 - cue1) / bar_sec))),
                confidence=0.75,
                cue_type="mix_out",
            ),
        ]
        return StructuralCueProfile(
            first_downbeat_sec=round(cue1, 3),
            phrase_in_sec=round(cue2, 3),
            drop_climax_sec=round(cue3, 3),
            mix_out_sec=round(cue4, 3),
            bar_duration_sec=round(bar_sec, 4),
            vocal_intro_active=False,
            vocal_outro_active=False,
            drop_intensity_ratio=1.20,
            hot_cues=cues,
        )

    @classmethod
    def extract_from_audio(
        cls,
        y: np.ndarray,
        sr: int = 22050,
        bpm: float = 124.0,
        total_duration_sec: float | None = None,
        analysis_offset_sec: float = 0.0,
    ) -> StructuralCueProfile:
        """Detecta la fase real del downbeat, entradas de frase, drop principal y mix-out sobre la señal de audio."""
        safe_bpm = max(60.0, min(200.0, float(bpm or 124.0)))
        sec_per_beat = 60.0 / safe_bpm
        bar_sec = 4.0 * sec_per_beat

        if y is None or len(y) < sr * 4:
            est_dur = float(total_duration_sec) if (total_duration_sec and total_duration_sec > 0) else 210.0
            return cls.build_fallback_profile(safe_bpm, est_dur)

        sig_dur = len(y) / float(sr)
        full_dur = float(total_duration_sec) if (total_duration_sec and total_duration_sec >= sig_dur) else (analysis_offset_sec + sig_dur)

        n_fft = 1024
        hop = 256
        fps = sr / float(hop)
        n_frames = (len(y) - n_fft) // hop
        if n_frames < 16:
            return cls.build_fallback_profile(safe_bpm, full_dur)

        strides = (y.strides[0] * hop, y.strides[0])
        frames = np.lib.stride_tricks.as_strided(y, shape=(n_frames, n_fft), strides=strides)
        win = np.hanning(n_fft).astype(np.float32)
        spec = np.abs(np.fft.rfft(frames * win, axis=1))
        freqs = np.fft.rfftfreq(n_fft, 1.0 / sr)

        # Bandas estructurales: Sub-Bass (20-165 Hz), Mid/Vocal (300-3400 Hz), High/Brilliance (3400-9000 Hz)
        sub_mask = (freqs >= 20.0) & (freqs <= 165.0)
        mid_mask = (freqs >= 300.0) & (freqs <= 3400.0)
        high_mask = (freqs > 3400.0) & (freqs <= min(9000.0, sr * 0.48))

        sub_env = np.mean(spec[:, sub_mask] ** 2, axis=1) if np.any(sub_mask) else np.mean(spec ** 2, axis=1)
        mid_env = np.mean(spec[:, mid_mask] ** 2, axis=1) if np.any(mid_mask) else np.mean(spec ** 2, axis=1)
        high_env = np.mean(spec[:, high_mask] ** 2, axis=1) if np.any(high_mask) else np.mean(spec ** 2, axis=1)

        # 1. Fase del Beatgrid y Primer Downbeat (Cue 1)
        sub_flux = np.maximum(0.0, np.diff(sub_env, prepend=0.0))
        period_frames = max(4, int(round(sec_per_beat * fps)))
        n_beats_fold = len(sub_flux) // period_frames
        if n_beats_fold >= 2:
            folded = sub_flux[: n_beats_fold * period_frames].reshape(n_beats_fold, period_frames)
            phase_frame = int(np.argmax(np.mean(folded, axis=0)))
            beat_phase_sec = phase_frame / fps
        else:
            beat_phase_sec = 0.0

        if analysis_offset_sec <= 0.5:
            # Buscar el primer impacto rítmico por encima del umbral de ruido inicial
            search_frames = min(len(sub_env), int(min(15.0, sig_dur * 0.30) * fps))
            p90 = float(np.percentile(sub_env[:search_frames], 90)) if search_frames > 0 else 0.0
            thresh = max(1e-7, 0.22 * p90)
            active_idx = np.where(sub_env[:search_frames] >= thresh)[0]
            raw_start_sec = float(active_idx[0] / fps) if len(active_idx) > 0 else beat_phase_sec
            # Alinear al beat de fase más cercano
            k_beats = max(0, int(round((raw_start_sec - beat_phase_sec) / sec_per_beat)))
            cue1_sec = max(0.0, beat_phase_sec + k_beats * sec_per_beat)
        else:
            # Si el buffer comienza con offset (p.ej. -ss 15), proyectar la fase exacta al compás 0
            abs_phase = (analysis_offset_sec + beat_phase_sec) % sec_per_beat
            cue1_sec = abs_phase if abs_phase >= 0.05 else (abs_phase + sec_per_beat)

        # 2. Energía por compás sobre la ventana analizada
        total_bars = max(8, int((full_dur - cue1_sec) // bar_sec))
        bar_sub = np.zeros(total_bars, dtype=np.float64)
        bar_mid = np.zeros(total_bars, dtype=np.float64)
        bar_high = np.zeros(total_bars, dtype=np.float64)
        bar_valid = np.zeros(total_bars, dtype=bool)

        for b_idx in range(total_bars):
            b_start_abs = cue1_sec + b_idx * bar_sec
            b_end_abs = b_start_abs + bar_sec
            rel_start = b_start_abs - analysis_offset_sec
            rel_end = b_end_abs - analysis_offset_sec
            if rel_end <= 0.0 or rel_start >= sig_dur:
                continue
            f0 = max(0, int(rel_start * fps))
            f1 = min(len(sub_env), int(rel_end * fps))
            if f1 > f0 + 1:
                bar_sub[b_idx] = float(np.mean(sub_env[f0:f1]))
                bar_mid[b_idx] = float(np.mean(mid_env[f0:f1]))
                bar_high[b_idx] = float(np.mean(high_env[f0:f1]))
                bar_valid[b_idx] = True

        # Normalizar curvas de compás para análisis de novedad estructural
        valid_indices = np.where(bar_valid)[0]
        if len(valid_indices) < 6:
            return cls.build_fallback_profile(safe_bpm, full_dur)

        sub_norm = bar_sub / (float(np.percentile(bar_sub[valid_indices], 90)) + 1e-9)
        mid_norm = bar_mid / (float(np.percentile(bar_mid[valid_indices], 90)) + 1e-9)
        high_norm = bar_high / (float(np.percentile(bar_high[valid_indices], 90)) + 1e-9)
        composite_energy = 0.50 * sub_norm + 0.30 * mid_norm + 0.20 * high_norm

        # 3. Detección de Cue 3 primero: Drop / Clímax dentro de los compases con señal decodificada real
        valid_min_bar = int(valid_indices[0])
        valid_max_bar = int(valid_indices[-1])
        valid_span = max(8, valid_max_bar - valid_min_bar + 1)

        raw_min_drop = max(valid_min_bar + 4, valid_min_bar + int(valid_span * 0.25))
        min_drop_bar = max(4, (raw_min_drop // 2) * 2)
        max_drop_bar = max(min_drop_bar + 4, min(valid_max_bar - 2, valid_min_bar + int(valid_span * 0.85)))
        phrase_grid = [b for b in range(min_drop_bar, max_drop_bar + 1, 2)]
        if not phrase_grid:
            phrase_grid = [min(valid_max_bar - 2, max(valid_min_bar + 4, 16))]

        best_drop_bar = phrase_grid[0]
        best_drop_score = -1e9
        best_drop_ratio = 1.15

        for b in phrase_grid:
            pre_win = composite_energy[max(valid_min_bar, b - 4) : b]
            post_win = composite_energy[b : min(valid_max_bar + 1, b + 4)]
            pre_sub = sub_norm[max(valid_min_bar, b - 4) : b]
            post_sub = sub_norm[b : min(valid_max_bar + 1, b + 4)]
            if len(pre_win) == 0 or len(post_win) == 0:
                continue
            pre_e = float(np.mean(pre_win))
            post_e = float(np.mean(post_win))
            sub_jump = float(np.mean(post_sub) - np.mean(pre_sub))
            # Premiar tanto la magnitud absoluta del Drop como el contraste tras un Breakdown
            contrast_score = (post_e - 0.65 * pre_e) + 0.60 * sub_jump
            if contrast_score > best_drop_score:
                best_drop_score = contrast_score
                best_drop_bar = b
                best_drop_ratio = float(np.clip((post_e + 0.05) / (pre_e + 0.05), 1.05, 4.0))

        cue3_sec = cue1_sec + best_drop_bar * bar_sec

        # 4. Detección de Cue 2: Entrada de Frase / Voz (estrictamente antes del Breakdown/Drop)
        candidate_phrase_bars = [b for b in (4, 8, 12, 16) if b <= best_drop_bar - 4]
        if not candidate_phrase_bars:
            candidate_phrase_bars = [max(2, best_drop_bar // 2)]

        best_phrase_bar = candidate_phrase_bars[0]
        best_phrase_gain = -1e9
        for b in candidate_phrase_bars:
            if b >= valid_min_bar + 2 and b + 2 <= valid_max_bar:
                pre = float(np.mean(composite_energy[max(valid_min_bar, b - 4) : b]))
                post = float(np.mean(composite_energy[b : min(valid_max_bar + 1, b + 4)]))
                vocal_jump = float(
                    np.mean(mid_norm[b : min(valid_max_bar + 1, b + 4)])
                    - np.mean(mid_norm[max(valid_min_bar, b - 4) : b])
                )
                gain = (post - pre) + 0.45 * vocal_jump
            else:
                # Si el compás 8 cae en los primeros 15s (antes del offset), priorizar 8 compases estándar
                gain = 0.25 if b == 8 else 0.10
            if gain > best_phrase_gain:
                best_phrase_gain = gain
                best_phrase_bar = b

        cue2_sec = cue1_sec + best_phrase_bar * bar_sec

        # 5. Detección de Cue 4: Mix-Out / Outro Loop (si tenemos señal hasta el final, buscar caída; si no, -16 compases cuantizados)
        if total_bars <= valid_max_bar + 4:
            min_out_bar = max(best_drop_bar + 6, int(total_bars * 0.62))
            max_out_bar = max(min_out_bar + 2, total_bars - 4)
            out_candidates = list(range(min_out_bar, min(total_bars - 1, max_out_bar), 4))
            if not out_candidates:
                out_candidates = [max(best_drop_bar + 4, total_bars - 16)]

            best_out_bar = out_candidates[-1]
            best_out_drop = -1e9
            for b in out_candidates:
                pre_m = float(np.mean(mid_norm[max(0, b - 4) : b]))
                post_m = float(np.mean(mid_norm[b : min(total_bars, b + 4)]))
                step_down = pre_m - post_m
                if step_down > best_out_drop:
                    best_out_drop = step_down
                    best_out_bar = b
        else:
            best_out_bar = max(best_drop_bar + 8, ((total_bars - 16) // 4) * 4)

        cue4_sec = min(full_dur - 2.0 * bar_sec, cue1_sec + best_out_bar * bar_sec)
        cue4_sec = max(cue3_sec + 4.0 * bar_sec, cue4_sec)

        # 6. Actividad Vocal en Intro (primeros 8 compases válidos) y Outro (últimos 8 compases válidos)
        intro_valid = valid_indices[: min(8, len(valid_indices))]
        outro_valid = valid_indices[max(0, len(valid_indices) - 8) :]
        median_mid = float(np.median(mid_norm[valid_indices]))
        vocal_intro = bool(float(np.mean(mid_norm[intro_valid])) > max(0.52, median_mid * 0.92))
        vocal_outro = bool(float(np.mean(mid_norm[outro_valid])) > max(0.52, median_mid * 0.92))

        cues = [
            HotCuePoint(
                num=1,
                name="1. INTRO BEAT",
                position_sec=round(float(cue1_sec), 3),
                bar_index=0,
                confidence=0.95,
                cue_type="downbeat",
            ),
            HotCuePoint(
                num=2,
                name=f"2. ENTRADA ({best_phrase_bar} COMP)",
                position_sec=round(float(cue2_sec), 3),
                bar_index=int(best_phrase_bar),
                confidence=0.91,
                cue_type="phrase_in",
            ),
            HotCuePoint(
                num=3,
                name=f"3. DROP / CORO ({best_drop_bar}C)",
                position_sec=round(float(cue3_sec), 3),
                bar_index=int(best_drop_bar),
                confidence=0.93,
                cue_type="drop",
            ),
            HotCuePoint(
                num=4,
                name=f"4. SALIDA MIX ({best_out_bar}C)",
                position_sec=round(float(cue4_sec), 3),
                bar_index=int(best_out_bar),
                confidence=0.90,
                cue_type="mix_out",
            ),
        ]

        return StructuralCueProfile(
            first_downbeat_sec=round(float(cue1_sec), 3),
            phrase_in_sec=round(float(cue2_sec), 3),
            drop_climax_sec=round(float(cue3_sec), 3),
            mix_out_sec=round(float(cue4_sec), 3),
            bar_duration_sec=round(float(bar_sec), 4),
            vocal_intro_active=vocal_intro,
            vocal_outro_active=vocal_outro,
            drop_intensity_ratio=round(best_drop_ratio, 2),
            hot_cues=cues,
        )
