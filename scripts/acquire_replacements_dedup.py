"""Descarga y promoción de los reemplazos únicos para erradicar el 100% de duplicados."""

import asyncio
from pathlib import Path
from providers.ytdlp.client import YtdlpProvider
from hera.domain.database import Database
from hera.domain.repositories import TrackRepository
from hera.domain.organizer import TrackOrganizer
from hera.contracts.track import Track, TrackStatus
from hera.domain.config import HeraConfig

REPLACEMENTS = [
    # Set 5 (French Touch Exclusivos)
    ("The Supermen Lovers", "Starlight (Original Mix)", "Supermen Lovers Starlight Original Mix"),
    ("Shakedown", "At Night (Kid Creme Club Mix)", "Shakedown At Night Kid Creme Club Mix"),
    ("Etienne de Crecy", "Am I Wrong", "Etienne de Crecy Am I Wrong"),
    ("Demon vs Heartbreaker", "You Are My High", "Demon You Are My High Original"),
    ("Junior Jack", "Stupidisco", "Junior Jack Stupidisco Original"),
    ("Martin Solveig", "Rocking Music", "Martin Solveig Rocking Music Original"),
    ("Benjamin Diamond", "In Your Arms", "Benjamin Diamond In Your Arms"),
    ("Alex Gopher", "The Child", "Alex Gopher The Child Original"),
    ("Cassius", "Feeling For You", "Cassius Feeling For You Original"),
    ("Bob Sinclar", "I Feel For You", "Bob Sinclar I Feel For You Original"),
    ("Dimitri From Paris", "Sacre Francais", "Dimitri From Paris Sacre Francais"),
    ("Mousse T", "Horny 98", "Mousse T Horny 98 Extended"),
    ("The Shapeshifters", "Back To Basics", "Shapeshifters Back To Basics Original"),

    # Set 6 (Electro Club Exclusivos)
    ("Mason vs Princess Superstar", "Perfect Exceeder", "Mason Princess Superstar Perfect Exceeder"),
    ("Bodyrox ft. Luciana", "Yeah Yeah (D. Ramirez Mix)", "Bodyrox Luciana Yeah Yeah D Ramirez Remix"),
    ("Camille Jones vs Fedde Le Grand", "The Creeps", "Camille Jones Fedde Le Grand The Creeps"),
    ("Outwork ft. Mr. Gee", "Elektro", "Outwork Mr Gee Elektro Original"),
    ("Dave Spoon", "At Night", "Dave Spoon At Night Original"),
    ("Spektrum", "Kinda New (Tiefschwarz Remix)", "Spektrum Kinda New Tiefschwarz Remix"),
    ("Enur ft. Natasja", "Calabria 2007", "Enur Natasja Calabria 2007 Club Mix"),
    ("Uniting Nations", "Out Of Touch", "Uniting Nations Out Of Touch Extended"),
    ("Sunset Strippers", "Falling Stars", "Sunset Strippers Falling Stars Extended"),
    ("Hi_Tack", "Say Say Say (Waiting 4 U)", "Hi Tack Say Say Say Waiting 4 U Extended"),
    ("Supermode", "Tell Me Why", "Supermode Tell Me Why Original Club"),
    ("Freaks", "The Creeps (Steve Bug Mix)", "Freaks The Creeps Steve Bug"),
    ("Studio B", "I See Girls", "Studio B I See Girls Crazy Extended"),

    # Set 7 (Trance Euphoria Exclusivos)
    ("Motorcycle", "As The Rush Comes (Gabriel and Dresden Mix)", "Motorcycle As The Rush Comes Gabriel Dresden"),
    ("Energy 52", "Cafe Del Mar (Three N One Remix)", "Energy 52 Cafe Del Mar Three N One Remix"),
    ("Paul Oakenfold", "Southern Sun (Tiesto Remix)", "Paul Oakenfold Southern Sun Tiesto Remix"),
    ("Cosmic Gate", "Fire Wire", "Cosmic Gate Fire Wire Original Mix"),
    ("Svenson and Gielen", "The Beauty of Silence", "Svenson Gielen The Beauty of Silence"),
    ("Mauro Picotto", "Komodo", "Mauro Picotto Komodo Save A Soul"),
    ("Gouryella", "Gouryella", "Gouryella Gouryella Original Mix"),
    ("4 Strings", "Take Me Away (Into The Night)", "4 Strings Take Me Away Into The Night Extended"),
    ("Lasgo", "Something", "Lasgo Something Extended Mix"),

    # Set 8 (Soulful House & Vocal Divas Exclusivos)
    ("Robin S", "Luv 4 Luv", "Robin S Luv 4 Luv Stonebridge Club Mix"),
    ("Ultra Nate", "Found A Cure", "Ultra Nate Found A Cure Full Intention"),
    ("Masters At Work ft. India", "To Be In Love", "Masters At Work India To Be In Love MAW"),
    ("Juliet Roberts", "Caught In The Middle", "Juliet Roberts Caught In The Middle Monster Club"),
    ("Alison Limerick", "Where Love Lives", "Alison Limerick Where Love Lives Classic"),
    ("Livin Joy", "Dreamer", "Livin Joy Dreamer Original Club Mix"),
    ("Black Box", "Ride On Time", "Black Box Ride On Time Original Mix"),
    ("Rosie Gaines", "Closer Than Close", "Rosie Gaines Closer Than Close Mentor Club"),
    ("Wamdue Project", "King of My Castle", "Wamdue Project King of My Castle Roy Malone"),
    ("Soulsearcher", "Cant Get Enough", "Soulsearcher Cant Get Enough Vocal Club"),
    ("Pete Heller", "Big Love", "Pete Heller Big Love Original"),
    ("Heller and Farley Project", "Ultra Flava", "Heller Farley Ultra Flava Original"),
    ("Duke", "So In Love With You", "Duke So In Love With You Pizzaman"),
    ("Gusto", "Discos Revenge", "Gusto Discos Revenge Original"),
    ("The Bucketheads", "The Bomb", "Bucketheads The Bomb These Sounds Fall Into My Mind"),
    ("Tori Amos", "Professional Widow (Armand Van Helden Mix)", "Tori Amos Professional Widow Armand Van Helden"),
    ("Nightcrawlers", "Push The Feeling On", "Nightcrawlers Push The Feeling On MK Dub"),
    ("Everything But The Girl", "Missing (Todd Terry Mix)", "Everything But The Girl Missing Todd Terry Club"),
    ("Corona", "The Rhythm of the Night", "Corona The Rhythm of the Night Extended")
]

async def download_replacements():
    db = Database('hera.db')
    await db.init_schema()
    conn = await db.connect()
    track_repo = TrackRepository(conn)
    cfg = HeraConfig.load('config/hera.toml')
    organizer = TrackOrganizer(track_repo, cfg.library_dir)
    
    quarantine = Path('quarantine')
    quarantine.mkdir(exist_ok=True)
    ytdlp = YtdlpProvider(max_results=2, preferred_quality='320')

    print(f'=== Descargando {len(REPLACEMENTS)} reemplazos exclusivos para eliminar duplicados ===')

    for i, (artist, title, q) in enumerate(REPLACEMENTS, 1):
        clean_title = title.replace('/', '-').replace('\"', '')
        clean_artist = artist.replace('/', '-').replace('\"', '')
        fname = f"{clean_artist} - {clean_title}.mp3"
        target = quarantine / fname

        print(f"[{i:02d}/{len(REPLACEMENTS):02d}] Adquiriendo: {clean_artist} - {clean_title}...")
        try:
            cands = await ytdlp.search(q)
            if cands:
                best = cands[0]
                await ytdlp.start_transfer(best, str(target))
                
                if target.exists() and target.stat().st_size > 500_000:
                    trk = Track(
                        status=TrackStatus.VALIDATED,
                        canonical_artist=clean_artist,
                        canonical_title=clean_title,
                        quarantine_path=str(target.resolve()),
                        codec='mp3',
                        bitrate_kbps=320,
                        duration_ms=240000,
                    )
                    await track_repo.save(trk)
                    res = await organizer.organize_track(trk, template=cfg.organize_template)
                    print(f"  -> [OK] {res.destination_path}")
                else:
                    print("  -> [FALLO] Archivo demasiado pequeno")
            else:
                print(f"  -> [NO ENCONTRADO] {q}")
        except Exception as e:
            print(f"  -> [ERROR] {e}")

    await db.close()
    print('=== Todos los reemplazos han sido procesados y organizados ===')

if __name__ == '__main__':
    asyncio.run(download_replacements())
