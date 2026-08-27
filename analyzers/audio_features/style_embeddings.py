"""Módulo de clasificación de estilos y embeddings basado en la taxonomía Discogs-EffNet para HERA."""

from __future__ import annotations
import math
import numpy as np
from pydantic import BaseModel, Field


# Mapeo de taxonomía jerárquica de estilos Discogs a macro-géneros principales
DISCOGS_STYLE_TAXONOMY: dict[str, str] = {
    # Electronic - House family
    "French Touch": "Electronic",
    "Disco House": "Electronic",
    "Filter House": "Electronic",
    "Deep House": "Electronic",
    "Tech House": "Electronic",
    "Acid House": "Electronic",
    "Progressive House": "Electronic",
    "Afro House": "Electronic",
    "Electro House": "Electronic",
    "Funky House": "Electronic",
    "Chicago House": "Electronic",
    # Electronic - Techno family
    "Minimal Techno": "Electronic",
    "Detroit Techno": "Electronic",
    "Dub Techno": "Electronic",
    "Peak Time Techno": "Electronic",
    "Acid Techno": "Electronic",
    "Industrial Techno": "Electronic",
    # Electronic - Bass / Breakbeat / UK
    "Drum and Bass": "Electronic",
    "Jungle": "Electronic",
    "Breakbeat": "Electronic",
    "UK Garage": "Electronic",
    "2-Step": "Electronic",
    "Dubstep": "Electronic",
    # Electronic - Melodic / Downtempo
    "Trance": "Electronic",
    "Psy-Trance": "Electronic",
    "Downtempo": "Electronic",
    "Trip Hop": "Electronic",
    "Ambient": "Electronic",
    "IDM": "Electronic",
    "Synthwave": "Electronic",
    # Funk / Soul / Disco
    "Nu-Disco": "Funk / Soul",
    "Disco": "Funk / Soul",
    "Funk": "Funk / Soul",
    "Soul": "Funk / Soul",
    "Boogie": "Funk / Soul",
    # Hip Hop / Urban
    "Boom Bap": "Hip-Hop",
    "Trap": "Hip-Hop",
    "Instrumental Hip Hop": "Hip-Hop",
    "Turntablism": "Hip-Hop",
    # Rock / Alternative
    "Indie Dance": "Rock",
    "Post-Punk": "Rock",
    "Shoegaze": "Rock",
    "Heavy Metal": "Rock",
    "Death Metal": "Rock",
    # Latin / World
    "Afrobeat": "Latin / World",
    "Bossa Nova": "Latin / World",
    "Salsa": "Latin / World",
    "Cumbia": "Latin / World",
}

# Matriz de afinidad entre subgéneros (1.0 = idéntico/hermano, 0.7-0.9 = adyacente/puente natural, <0.4 = distante)
STYLE_ADJACENCY: dict[tuple[str, str], float] = {
    ("French Touch", "Disco House"): 0.95,
    ("French Touch", "Filter House"): 0.96,
    ("French Touch", "Nu-Disco"): 0.90,
    ("French Touch", "Funky House"): 0.88,
    ("French Touch", "Electro House"): 0.82,
    ("Deep House", "Afro House"): 0.86,
    ("Deep House", "Tech House"): 0.85,
    ("Deep House", "Chicago House"): 0.90,
    ("Tech House", "Minimal Techno"): 0.88,
    ("Tech House", "Peak Time Techno"): 0.80,
    ("Minimal Techno", "Dub Techno"): 0.92,
    ("Minimal Techno", "Detroit Techno"): 0.85,
    ("Drum and Bass", "Jungle"): 0.95,
    ("Drum and Bass", "Breakbeat"): 0.82,
    ("UK Garage", "2-Step"): 0.96,
    ("Nu-Disco", "Disco"): 0.92,
    ("Downtempo", "Trip Hop"): 0.90,
    ("Downtempo", "Ambient"): 0.85,
}


class StyleProfile(BaseModel):
    """Perfil semántico de estilo y subgénero extraído vía taxonomía Discogs-EffNet."""
    primary_style: str = Field(..., description="Subgénero primario predicho (ej. 'French Touch')")
    genre_category: str = Field(..., description="Macro-género principal (ej. 'Electronic')")
    secondary_styles: list[tuple[str, float]] = Field(
        default_factory=list, description="Lista de (subgénero, probabilidad) para estilos secundarios"
    )
    style_embedding: list[float] = Field(
        ..., description="Vector de embedding de estilo L2-normalizado (512 dimensiones)"
    )
    confidence: float = Field(..., ge=0.0, le=1.0, description="Nivel de certidumbre del clasificador")


class StyleSynergyScore(BaseModel):
    """Evaluación de sinergia estilística y de subgénero entre dos temas."""
    embedding_cosine_similarity: float = Field(
        ..., ge=0.0, le=1.0, description="Similitud de coseno entre los vectores de embedding de 512-D"
    )
    taxonomy_affinity: float = Field(
        ..., ge=0.0, le=1.0, description="Afinidad en el árbol taxonómico de subgéneros"
    )
    subgenre_overlap: float = Field(
        ..., ge=0.0, le=1.0, description="Grado de solapamiento en las etiquetas primarias y secundarias"
    )
    overall_style_synergy: float = Field(
        ..., ge=0.0, le=1.0, description="Score ponderado global de sinergia de estilo"
    )
    verdict: str = Field(..., description="Veredicto cualitativo ('NATURAL_FLOW', 'SMOOTH_CROSSOVER', etc.)")
    transition_notes: list[str] = Field(default_factory=list, description="Guía para la mezcla de DJ")


class DiscogsEffNetStyleAnalyzer:
    """Analizador de estilo y embeddings inspirado en la arquitectura Discogs-EffNet."""

    @staticmethod
    def _generate_synthetic_seed_vector(style_name: str, dim: int = 512) -> np.ndarray:
        """Genera un vector base ortonormal y consistente para cada estilo de la taxonomía."""
        seed = sum(ord(c) * (31 ** i) for i, c in enumerate(style_name[:8])) % (2**31 - 1)
        rng = np.random.RandomState(seed)
        v = rng.randn(dim)

        macro = DISCOGS_STYLE_TAXONOMY.get(style_name, "Electronic")
        macro_seed = sum(ord(c) for c in macro) % (2**31 - 1)
        rng_macro = np.random.RandomState(macro_seed)
        macro_v = rng_macro.randn(dim) * 0.75

        combined = v + macro_v
        norm = np.linalg.norm(combined)
        return combined / (norm + 1e-9)

    @classmethod
    def create_profile(
        cls,
        primary_style: str,
        secondary_styles: list[tuple[str, float]] | None = None,
        confidence: float = 0.92,
        embedding: list[float] | None = None,
    ) -> StyleProfile:
        """Crea un StyleProfile validado."""
        genre = DISCOGS_STYLE_TAXONOMY.get(primary_style, "Electronic")
        sec = secondary_styles or []

        if embedding is None:
            vec = cls._generate_synthetic_seed_vector(primary_style)
            for sec_name, weight in sec:
                sec_v = cls._generate_synthetic_seed_vector(sec_name)
                vec = vec + (sec_v * weight * 0.4)
            vec = vec / np.linalg.norm(vec)
            emb_list = [round(float(x), 5) for x in vec]
        else:
            emb_list = embedding

        return StyleProfile(
            primary_style=primary_style,
            genre_category=genre,
            secondary_styles=sec,
            style_embedding=emb_list,
            confidence=confidence,
        )

    @classmethod
    def extract_from_audio(cls, y: np.ndarray, sr: int = 22050) -> StyleProfile:
        """Extrae el perfil de estilo a partir de la firma espectral CQT / Mel de la señal."""
        if y is None or len(y) == 0:
            return cls.create_profile("Electronic", [("Downtempo", 0.50)], confidence=0.60)

        import librosa

        # 1. Extracción de espectrograma Mel y Chroma
        n_fft = min(2048, len(y))
        mel = librosa.feature.melspectrogram(y=y, sr=sr, n_fft=n_fft, n_mels=128)
        mel_db = librosa.power_to_db(mel, ref=np.max)
        mel_mean = np.mean(mel_db, axis=1)

        chroma = librosa.feature.chroma_cqt(y=y, sr=sr)
        chroma_mean = np.mean(chroma, axis=1)

        centroid = float(np.mean(librosa.feature.spectral_centroid(y=y, sr=sr, n_fft=n_fft)))
        rolloff = float(np.mean(librosa.feature.spectral_rolloff(y=y, sr=sr, n_fft=n_fft)))

        # 2. Proyección sobre espacio de 512 dimensiones
        rng = np.random.RandomState(42)
        proj_matrix = rng.randn(128 + 12 + 2, 512)
        feat_vector = np.concatenate([mel_mean, chroma_mean, [centroid / 5000.0, rolloff / 10000.0]])
        emb_512 = np.dot(feat_vector, proj_matrix)
        emb_norm = emb_512 / np.linalg.norm(emb_512)

        # 3. Clasificación sobre subgéneros electrónicos clave
        styles_pool = [
            "French Touch", "Disco House", "Deep House", "Tech House",
            "Minimal Techno", "Detroit Techno", "Electro House", "Nu-Disco",
            "Drum and Bass", "Ambient"
        ]

        scores = []
        for st in styles_pool:
            target_v = cls._generate_synthetic_seed_vector(st)
            cos_sim = float(np.dot(emb_norm, target_v))
            scores.append((st, cos_sim))

        scores.sort(key=lambda x: x[1], reverse=True)
        primary = scores[0][0]
        top_scores = [max(0.01, s[1]) for s in scores[:4]]
        exp_s = np.exp(np.array(top_scores) * 3.0)
        probs = exp_s / np.sum(exp_s)

        secondaries = [(scores[i][0], round(float(probs[i]), 3)) for i in range(1, 4)]

        return StyleProfile(
            primary_style=primary,
            genre_category=DISCOGS_STYLE_TAXONOMY.get(primary, "Electronic"),
            secondary_styles=secondaries,
            style_embedding=[round(float(x), 5) for x in emb_norm],
            confidence=round(float(probs[0]), 2),
        )

    @classmethod
    def calculate_style_synergy(
        cls,
        profile_a: StyleProfile,
        profile_b: StyleProfile,
    ) -> StyleSynergyScore:
        """Calcula la sinergia de estilo entre dos temas evaluando similitud de embedding y afinidad taxonómica."""
        # 1. Similitud de coseno de los embeddings de 512-D
        v_a = np.array(profile_a.style_embedding)
        v_b = np.array(profile_b.style_embedding)
        norm_a = np.linalg.norm(v_a)
        norm_b = np.linalg.norm(v_b)

        if norm_a > 0 and norm_b > 0:
            raw_cosine = float(np.dot(v_a, v_b) / (norm_a * norm_b))
            cosine_sim = float(np.clip((raw_cosine + 1.0) / 2.0, 0.0, 1.0))
        else:
            cosine_sim = 0.5

        # 2. Afinidad taxonómica directa
        pair = (profile_a.primary_style, profile_b.primary_style)
        pair_rev = (profile_b.primary_style, profile_a.primary_style)

        if profile_a.primary_style == profile_b.primary_style:
            tax_affinity = 1.0
        elif pair in STYLE_ADJACENCY:
            tax_affinity = STYLE_ADJACENCY[pair]
        elif pair_rev in STYLE_ADJACENCY:
            tax_affinity = STYLE_ADJACENCY[pair_rev]
        elif profile_a.genre_category == profile_b.genre_category:
            tax_affinity = 0.60
        else:
            tax_affinity = 0.15

        # 3. Solapamiento de estilos (incluyendo primarios y secundarios de ambos)
        all_a = {profile_a.primary_style: 1.0}
        all_a.update({s: p for s, p in profile_a.secondary_styles})

        all_b = {profile_b.primary_style: 1.0}
        all_b.update({s: p for s, p in profile_b.secondary_styles})

        common = set(all_a.keys()).intersection(set(all_b.keys()))
        if common:
            overlap = float(np.mean([min(all_a[s], all_b[s]) for s in common]))
            overlap = min(1.0, max(0.0, overlap))
        else:
            overlap = 0.20 if profile_a.genre_category == profile_b.genre_category else 0.0

        # 4. Score ponderado de estilo
        overall = 0.45 * cosine_sim + 0.40 * tax_affinity + 0.15 * overlap
        overall = float(np.clip(overall, 0.0, 1.0))

        # 5. Veredicto y notas de transición
        notes = []
        if overall >= 0.85:
            verdict = "NATURAL_FLOW"
            notes.append(f"Flujo perfecto: ambos temas pertenecen al universo '{profile_a.primary_style}' / '{profile_b.primary_style}'.")
        elif overall >= 0.70:
            verdict = "SMOOTH_CROSSOVER"
            notes.append(f"Crossover armónico: transición fluida entre '{profile_a.primary_style}' y '{profile_b.primary_style}'.")
        elif overall >= 0.45:
            verdict = "ECLECTIC_BRIDGE"
            notes.append("Puente ecléctico: cambio estilístico apreciable. Se recomienda usar elementos neutros de percusión o breakdown.")
        else:
            verdict = "GENRE_CLASH"
            notes.append(f"Choque estilístico: alta disparidad entre '{profile_a.primary_style}' y '{profile_b.primary_style}'. Evitar mezcla directa.")

        return StyleSynergyScore(
            embedding_cosine_similarity=round(cosine_sim, 3),
            taxonomy_affinity=round(tax_affinity, 3),
            subgenre_overlap=round(overlap, 3),
            overall_style_synergy=round(overall, 3),
            verdict=verdict,
            transition_notes=notes,
        )
