"""Organizador transaccional de archivos y aplicación de plantillas de biblioteca."""

from pathlib import Path
import shutil
from pydantic import BaseModel
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.track import Track, TrackStatus
from hera.domain.repositories import TrackRepository
from hera.policy.path_validator import sanitize_filename, validate_path_safety


class OrganizeResult(BaseModel):
    track_id: str
    status: str
    source_path: str
    destination_path: str
    collision_detected: bool = False
    action_taken: str = "moved"


class TrackOrganizer:
    def __init__(self, track_repo: TrackRepository, library_base_dir: Path | str):
        self.track_repo = track_repo
        self.library_base_dir = Path(library_base_dir).resolve()

    def render_path(self, track: Track, template: str, ext: str) -> Path:
        """Renderiza una ruta destino a partir de la plantilla y los metadatos del track."""
        clean_artist = sanitize_filename(track.canonical_artist or "Unknown Artist")
        clean_title = sanitize_filename(track.canonical_title or "Unknown Title")
        clean_version = f" [{sanitize_filename(track.version)}]" if track.version else ""
        clean_year = "2024"
        clean_release = sanitize_filename(track.canonical_title)
        clean_track_no = "01"
        clean_ext = ext.lstrip(".")

        rendered = template
        rendered = rendered.replace("{Artist}", clean_artist)
        rendered = rendered.replace("{Year}", clean_year)
        rendered = rendered.replace("{Release}", clean_release)
        rendered = rendered.replace("{TrackNo}", clean_track_no)
        rendered = rendered.replace("{Title}", clean_title)
        rendered = rendered.replace(" [{Version}]", clean_version)
        rendered = rendered.replace("{Version}", sanitize_filename(track.version or ""))
        rendered = rendered.replace("{ext}", clean_ext)

        return self.library_base_dir / rendered

    async def organize_track(
        self,
        track_id: str | Track,
        template: str = "{Artist}/{Year} - {Release}/{TrackNo} - {Title} [{Version}].{ext}",
        collision_policy: str = "review",
    ) -> OrganizeResult:
        """Promueve un track desde cuarentena hacia la biblioteca organizada."""
        if isinstance(track_id, Track):
            track = track_id
        else:
            track = await self.track_repo.get_by_id(track_id)
        if not track:
            raise HeraException(HeraErrorCode.INVALID_MEDIA, f"Track {track_id} no encontrado")

        source_path_str = track.quarantine_path or track.library_path
        if not source_path_str:
            raise HeraException(HeraErrorCode.INVALID_MEDIA, f"Track {track_id} no tiene ruta física asignada")

        src = Path(source_path_str)
        if not src.exists():
            raise HeraException(HeraErrorCode.INVALID_MEDIA, f"El archivo origen {src} no existe")

        ext = src.suffix or ".flac"
        dst = self.render_path(track, template, ext)

        if not validate_path_safety(self.library_base_dir, dst):
            raise HeraException(
                HeraErrorCode.POLICY_DENIED, "La ruta calculada viola los límites del directorio de biblioteca"
            )

        collision = dst.exists()
        action = "moved"

        if collision:
            if collision_policy == "review":
                return OrganizeResult(
                    track_id=track_id,
                    status="collision_needs_review",
                    source_path=str(src),
                    destination_path=str(dst),
                    collision_detected=True,
                    action_taken="held_in_quarantine",
                )
            elif collision_policy == "suffix":
                dst = dst.with_name(f"{dst.stem}_{track.id[:6]}{dst.suffix}")
            elif collision_policy == "skip":
                return OrganizeResult(
                    track_id=track_id,
                    status="skipped",
                    source_path=str(src),
                    destination_path=str(dst),
                    collision_detected=True,
                    action_taken="skipped",
                )

        # Mover archivo atómicamente
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(src, dst)

        track.library_path = str(dst.resolve())
        track.quarantine_path = None
        track.status = TrackStatus.ORGANIZED
        await self.track_repo.save(track)

        return OrganizeResult(
            track_id=track.id,
            status="organized",
            source_path=str(src),
            destination_path=str(dst),
            collision_detected=collision,
            action_taken=action,
        )
