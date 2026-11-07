"""Pruebas de verificación de las 3 mejoras de clase mundial en ingeniería de audio para HERA:
1. Segmentación Estructural y Hot Cues Acústicos Reales (Downbeat 1.1.1, Frase, Drop Post-Breakdown, Mix-Out) + Exportación Rekordbox/Ableton.
2. Auditoría Forense Zero-Trust (Detección de Fake Lossless 128k->FLAC, Tonos Sintéticos, Clipping y Fase Mono de Subgraves <120Hz).
3. Optimizador Global 2-Opt TSP + Puentes Polirrítmicos 3:4 (AltBpm) + Aviso Anti-Choque Vocal + Telemetría Real en SQLite.
"""

from pathlib import Path
import xml.etree.ElementTree as ET
import numpy as np
import pytest

from analyzers.audio_features.structure import StructuralCueAnalyzer, StructuralCueProfile
from analyzers.audio_features.synergy_engine import UnifiedSynergyEngine, TrackAcousticFingerprint
from analyzers.audio_features.transients import TransientProfile
from analyzers.audio_features.style_embeddings import DiscogsEffNetStyleAnalyzer
from analyzers.ffmpeg.forensics import AudioForensicsAnalyzer
from hera.application.use_cases.build_crate import BuildHarmonicCrateUseCase
from hera.contracts.crate import Crate, CrateTrack
from hera.contracts.track import Track, TrackStatus
from hera.domain.database import Database
from hera.domain.export import CrateExporter
from hera.domain.repositories import TrackRepository


def _synthesize_structured_club_track(sr: int = 22050, bpm: float = 128.0) -> tuple[np.ndarray, float, float]:
    """Sintetiza un tema electrónico de 32 compases con:
    - Compases 0..4: Intro percusivo suave
    - Compases 4..12: Entrada de frase vocal/armónica (800-1600 Hz)
    - Compases 12..16: Breakdown (silencio de sub-bajo)
    - Compases 16..26: DROP PRINCIPAL (explosión de sub-bajo 55Hz + brillo 5kHz)
    - Compases 26..32: Outro de mezcla (cae la voz, queda solo el beat)
    Retorna (y, expected_phrase_sec, expected_drop_sec).
    """
    sec_per_beat = 60.0 / bpm
    bar_sec = 4.0 * sec_per_beat
    total_bars = 32
    dur = total_bars * bar_sec
    n_samples = int(sr * dur)
    t = np.arange(n_samples, dtype=np.float32) / sr
    y = np.zeros(n_samples, dtype=np.float32)

    # Pulso de kick en cada beat
    impulse_len = int(0.08 * sr)
    t_imp = np.arange(impulse_len, dtype=np.float32) / sr
    kick_wave = np.sin(2.0 * np.pi * 58.0 * t_imp) * np.exp(-t_imp * 35.0)

    for b_idx in range(total_bars):
        b_start = int(b_idx * bar_sec * sr)
        b_end = min(n_samples, int((b_idx + 1) * bar_sec * sr))
        t_bar = t[b_start:b_end]

        # Kicks por compás: suaves en intro (0..4), ausentes en breakdown (12..16), potentes en Drop (16..26)
        kick_gain = 0.25
        if 4 <= b_idx < 12:
            kick_gain = 0.45
        elif 12 <= b_idx < 16:
            kick_gain = 0.02  # Breakdown sin bombo
        elif 16 <= b_idx < 26:
            kick_gain = 0.95  # Drop explosivo
        else:
            kick_gain = 0.50  # Outro

        for beat in range(4):
            k_pos = int((b_idx * 4 + beat) * sec_per_beat * sr)
            if k_pos + impulse_len < n_samples:
                y[k_pos : k_pos + impulse_len] += kick_gain * kick_wave

        # Componente vocal / armónico medio (compases 4..12 y 16..26)
        if (4 <= b_idx < 12) or (16 <= b_idx < 26):
            y[b_start:b_end] += 0.30 * np.sin(2.0 * np.pi * 880.0 * t_bar)

        # Brillo de alta frecuencia en el Drop (compases 16..26)
        if 16 <= b_idx < 26:
            y[b_start:b_end] += 0.25 * np.sin(2.0 * np.pi * 4800.0 * t_bar)

    return y, 4.0 * bar_sec, 16.0 * bar_sec


def test_structural_cue_analyzer_detects_real_drop_and_phrases():
    """Demuestra que StructuralCueAnalyzer detecta el Drop real tras el Breakdown en el compás 16 y el Mix-Out."""
    sr = 22050
    bpm = 128.0
    y, expected_phrase_sec, expected_drop_sec = _synthesize_structured_club_track(sr=sr, bpm=bpm)

    profile = StructuralCueAnalyzer.extract_from_audio(y, sr=sr, bpm=bpm)

    assert isinstance(profile, StructuralCueProfile)
    assert len(profile.hot_cues) == 4
    bar_sec = 4.0 * (60.0 / bpm)

    # Cue 2 debe detectar la entrada de frase en el compás 4 (+-1 compás)
    assert abs(profile.phrase_in_sec - expected_phrase_sec) <= bar_sec * 1.1
    # Cue 3 debe detectar exactamente el Drop tras el Breakdown en el compás 16 (+-1 compás)
    assert abs(profile.drop_climax_sec - expected_drop_sec) <= bar_sec * 1.1
    assert profile.drop_intensity_ratio >= 1.40
    # Cue 4 debe ubicarse en el tercio final (compás >= 24)
    assert profile.mix_out_sec >= 22.0 * bar_sec


def test_audio_forensics_detects_fake_flac_synthetic_and_phase_cancellation():
    """Demuestra que AudioForensicsAnalyzer detecta:
    1. Onda senoidal sintética (SYNTHETIC_REJECTED)
    2. Fake FLAC transcodificado desde MP3 128 kbps con corte brickwall a 14.8 kHz (FAKE_TRANSCODE)
    3. Master auténtico de rango completo hasta 20.5 kHz (STUDIO_MASTER)
    4. Cancelación de fase en sub-graves <120Hz (mono_compatible_sub = False)
    """
    sr = 44100
    dur = 4.0
    n = int(sr * dur)
    t = np.arange(n, dtype=np.float32) / sr

    # 1. Tono senoidal sintético puro (440 Hz)
    sine_wave = 0.5 * np.sin(2.0 * np.pi * 440.0 * t)
    rep_sine = AudioForensicsAnalyzer.analyze_signal(sine_wave, sr=sr, claimed_codec="flac")
    assert rep_sine.is_synthetic_tone is True
    assert rep_sine.verdict == "SYNTHETIC_REJECTED"
    assert rep_sine.authenticity_score <= 10.0

    # 2. Señal musical de banda ancha vs Fake FLAC recortado a 14.8 kHz
    rng = np.random.RandomState(123)
    broadband = rng.randn(n).astype(np.float32) * 0.18
    # Añadir cuerpo musical rico sin saturar
    broadband += 0.20 * np.sin(2.0 * np.pi * 65.0 * t) + 0.15 * np.sin(2.0 * np.pi * 440.0 * t)
    broadband = np.clip(broadband, -0.85, 0.85)

    rep_master = AudioForensicsAnalyzer.analyze_signal(broadband, sr=sr, claimed_codec="flac", claimed_bitrate_kbps=1411)
    assert rep_master.is_fake_upscale is False
    assert rep_master.verdict == "STUDIO_MASTER"
    assert rep_master.effective_cutoff_hz >= 19500.0
    assert rep_master.authenticity_score >= 95.0

    # Simular MP3 de 128 kbps aplicando filtro brickwall en dominio FFT a 14.8 kHz y guardándolo como 'flac'
    F = np.fft.rfft(broadband)
    freqs = np.fft.rfftfreq(n, 1.0 / sr)
    F[freqs > 14800.0] *= 1e-4  # Caída abrupta de -80 dB sobre 14.8 kHz
    fake_flac_sig = np.fft.irfft(F, n=n).astype(np.float32)

    rep_fake = AudioForensicsAnalyzer.analyze_signal(fake_flac_sig, sr=sr, claimed_codec="flac", claimed_bitrate_kbps=1411)
    assert rep_fake.is_fake_upscale is True
    assert rep_fake.verdict == "FAKE_TRANSCODE"
    assert rep_fake.effective_cutoff_hz <= 16000.0
    assert rep_fake.estimated_true_bitrate_kbps <= 128
    assert rep_fake.authenticity_score <= 60.0

    # 3. Prueba de cancelación de fase en sub-graves (<120 Hz) en estéreo (L = +sub, R = -sub)
    sub_wave = 0.4 * np.sin(2.0 * np.pi * 60.0 * t)
    left_ch = broadband + sub_wave
    right_ch = broadband - sub_wave
    stereo_out_of_phase = np.column_stack([left_ch, right_ch])

    rep_phase = AudioForensicsAnalyzer.analyze_signal(stereo_out_of_phase, sr=sr, claimed_codec="wav")
    assert rep_phase.mono_compatible_sub is False
    assert rep_phase.sub_bass_phase_correlation < -0.50


@pytest.mark.asyncio
async def test_polyrhythmic_bridge_2opt_tsp_and_real_hot_cue_export(tmp_path: Path):
    """Demuestra el puente polirrítmico 3:4 (93 BPM Reggaetón -> 124 BPM Guaracha/Club),
    la mejora de sinergia global con 2-Opt TSP y la inyección de Hot Cues acústicos en Rekordbox XML y Ableton Live.
    """
    # 1. Puente Polirrítmico 3:4 (93 BPM con AltBpm=124 <-> 124 BPM)
    fp_urbano = TrackAcousticFingerprint(
        track_id="trk_urb_93",
        title="Ferxxo 100",
        artist="Feid",
        bpm=93.0,
        alt_bpm=124.0,
        musical_key="A minor",
        camelot="8A",
        energy=0.80,
        transients=TransientProfile(crest_factor=7.5, transient_density=3.8, attack_sharpness=0.80, spectral_transient_ratio=1.9),
        style=DiscogsEffNetStyleAnalyzer.create_profile("Dembow", [("Guaracha", 0.85)]),
        vocal_outro_active=True,
    )
    fp_club = TrackAcousticFingerprint(
        track_id="trk_club_124",
        title="Beso (Club Mix)",
        artist="Hugel",
        bpm=124.0,
        alt_bpm=93.0,
        musical_key="A minor",
        camelot="8A",
        energy=0.86,
        transients=TransientProfile(crest_factor=7.8, transient_density=4.0, attack_sharpness=0.82, spectral_transient_ratio=1.8),
        style=DiscogsEffNetStyleAnalyzer.create_profile("Guaracha", [("Afro House", 0.85)]),
        vocal_intro_active=True,
    )

    syn = UnifiedSynergyEngine.evaluate_track_pair(fp_urbano, fp_club)
    assert syn.polyrhythmic_bridge is True
    assert syn.tempo_synergy >= 0.90
    assert syn.vocal_clash_warning is True
    assert "Puente Polirrítmico 3:4" in syn.executive_summary
    assert "Aviso Vocal" in syn.executive_summary

    # 2. Exportación de Hot Cues Acústicos Reales a Rekordbox XML y Ableton Live
    sr = 22050
    bpm = 128.0
    y, _, expected_drop_sec = _synthesize_structured_club_track(sr=sr, bpm=bpm)
    struct_prof = StructuralCueAnalyzer.extract_from_audio(y, sr=sr, bpm=bpm)

    db_file = tmp_path / "hera_test.db"
    db = Database(db_file)
    await db.init_schema()
    conn = await db.connect()
    track_repo = TrackRepository(conn)

    audio_f = tmp_path / "daft_punk_structured.flac"
    audio_f.write_bytes(b"fLaC" + b"\x00" * 4096)

    track = Track(
        id="trk_struct_01",
        status=TrackStatus.ORGANIZED,
        canonical_title="Aerodynamic",
        canonical_artist="Daft Punk",
        duration_ms=int(32 * 4 * (60.0 / bpm) * 1000),
        bpm=bpm,
        camelot="8A",
        library_path=str(audio_f),
        provenance_json={
            "hera_dsp_telemetry": {
                "alt_bpm": 96.0,
                "structure": struct_prof.model_dump(),
            }
        },
    )
    await track_repo.save(track)

    crate = Crate(
        name="Acoustic Cues Showcase",
        brief="Test Real Cues",
        duration_target_minutes=10,
        tracks=[CrateTrack(track_id=track.id, position=1)],
    )
    exporter = CrateExporter(track_repo, tmp_path / "exports")

    rb_xml = await exporter.export_rekordbox_xml(crate, [track])
    rb_tree = ET.parse(rb_xml)
    pos_marks = rb_tree.getroot().findall(".//POSITION_MARK")
    assert len(pos_marks) == 4
    drop_mark_sec = float(pos_marks[2].attrib["Start"])
    assert abs(drop_mark_sec - struct_prof.drop_climax_sec) < 0.01

    abl_idx = await exporter.export_ableton_curated(crate, [track])
    import json
    abl_data = json.loads(abl_idx.read_text(encoding="utf-8"))
    assert len(abl_data["tracks"][0]["hot_cues"]) == 4
    assert abs(abl_data["tracks"][0]["hot_cues"][2]["position_sec"] - struct_prof.drop_climax_sec) < 0.01

    # Verificar que el Cue Sheet incluye los Hot Cues acústicos y métricas globales
    uc = BuildHarmonicCrateUseCase()
    sheet = uc.generate_cue_sheet("Acoustic Cues Showcase", [track])
    assert "[Hot Cues Acústicos]" in sheet
    assert f"C3:{struct_prof.drop_climax_sec:.1f}s" in sheet

    await db.close()
