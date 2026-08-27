"""Motor de scoring y ranking explicable para candidatos musicales."""

from hera.contracts.candidate import Candidate, ScoreComponents
from hera.contracts.preference import PreferenceProfile


class RankingEngine:
    """Calcula y explica scores para candidatos."""

    def __init__(self, version: str = "scoring/1.0"):
        self.version = version

    def compute_score(
        self,
        candidate: Candidate,
        profile: PreferenceProfile | None = None,
    ) -> tuple[float, ScoreComponents, list[str]]:
        """Calcula el score total (0-100), sus componentes y las razones explicables."""
        reasons: list[str] = []

        # 1. Identidad (0.0 - 1.0)
        ident = candidate.score_components.identity
        if ident >= 0.90:
            reasons.append("Alta coincidencia exacta en artista y título")

        # 2. Calidad técnica (0.0 - 1.0)
        fmt = (candidate.format or "").upper()
        is_lossless = fmt in {"FLAC", "ALAC", "WAV", "AIFF"}
        tech = 1.00 if is_lossless else (0.75 if (candidate.bitrate_kbps or 0) >= 320 else 0.50)
        if is_lossless:
            reasons.append(f"Master Lossless Original ({fmt}) - Máxima Fidelidad Acústica")
        elif (candidate.bitrate_kbps or 0) >= 320:
            reasons.append(f"MP3 de alta tasa de bits ({candidate.bitrate_kbps} kbps)")

        # 3. Fuente (0.0 - 1.0) - Soulseek y P2P prioritarios por catálogo FLAC nativo
        source_weights = {
            "slskd": 0.95,     # Soulseek P2P prioritario para coleccionismo FLAC
            "prowlarr": 0.90,  # BitTorrent / Trackers Lossless
            "lidarr": 0.85,
            "archive": 0.70,
            "ytdlp": 0.50,     # yt-dlp solo como fallback cuando no hay FLAC P2P
        }
        source = source_weights.get(candidate.provider.lower(), 0.60)

        # 4. Disponibilidad (0.0 - 1.0)
        avail_weights = {
            "local": 1.0,
            "archive": 0.95,
            "ytdlp": 0.90,
            "bandcamp": 0.85,
            "lidarr": 0.80,
            "prowlarr": 0.80,
            "slskd": 0.75,
        }
        avail = avail_weights.get(candidate.provider.lower(), 0.80)

        # 5. Preferencias (0.0 - 1.0)
        pref = 0.70
        if profile:
            if fmt in profile.preferred_formats:
                pref += 0.20
                reasons.append(f"Formato preferido por el DJ ({fmt})")
            if candidate.version and any(ex.lower() in candidate.version.lower() for ex in profile.excluded_versions):
                pref -= 0.50
                reasons.append(f"Versión no deseada detectada: {candidate.version}")
        pref = max(0.0, min(1.0, pref))

        # 6. Metadata (0.0 - 1.0)
        meta = 0.80 if candidate.version or candidate.duration_ms else 0.50

        # 7. Riesgo / Integridad (0.0 - 1.0)
        risk = 0.90
        # Penalizaciones por anomalías
        if candidate.file_size_bytes and candidate.file_size_bytes < 100_000:
            risk -= 0.40
            reasons.append("Penalización: tamaño de archivo sospechosamente pequeño")

        risk = max(0.0, min(1.0, risk))

        # Fórmula ponderada canónica
        total = (
            30 * ident
            + 25 * tech
            + 15 * source
            + 10 * avail
            + 10 * pref
            + 5 * meta
            + 5 * risk
        )
        total_score = round(max(0.0, min(100.0, total)), 1)

        components = ScoreComponents(
            identity=round(ident, 2),
            technical=round(tech, 2),
            source=round(source, 2),
            availability=round(avail, 2),
            preference=round(pref, 2),
            metadata=round(meta, 2),
            risk=round(risk, 2),
        )

        return total_score, components, reasons
