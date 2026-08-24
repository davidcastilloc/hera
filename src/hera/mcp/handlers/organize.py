"""Handler para la tool organize_track."""

from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.organizer import TrackOrganizer
from hera.domain.repositories import TrackRepository


async def handle_organize_track(
    track_id: str,
    template: str | None,
    collision_policy: str | None,
    db: Database,
    config: HeraConfig,
) -> dict:
    conn = await db.connect()
    track_repo = TrackRepository(conn)

    organizer = TrackOrganizer(track_repo, config.library_dir)
    res = await organizer.organize_track(
        track_id=track_id,
        template=template or config.organize_template,
        collision_policy=collision_policy or config.collision_policy,
    )
    return res.model_dump()
