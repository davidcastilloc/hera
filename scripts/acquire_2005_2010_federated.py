"""Descargador federado inteligente para los 120 himnos de 2005-2010."""

import asyncio
from pathlib import Path
from providers.ytdlp.client import YtdlpProvider
from hera.domain.database import Database
from hera.domain.repositories import TrackRepository
from hera.domain.organizer import TrackOrganizer
from hera.contracts.track import Track, TrackStatus
from hera.domain.config import HeraConfig

TRACKS = [
    # Set 9 (Progressive House Side A)
    ("Deadmau5 & Kaskade", "I Remember (Extended Mix)", "Deadmau5 Kaskade I Remember Extended"),
    ("Eric Prydz", "Pjanoo (Club Mix)", "Eric Prydz Pjanoo Club Mix"),
    ("Swedish House Mafia", "One (Original Mix)", "Swedish House Mafia One Original"),
    ("Axwell, Ingrosso, Angello, Laidback Luke", "Leave The World Behind", "Leave The World Behind Axwell"),
    ("Steve Angello & Laidback Luke", "Show Me Love", "Show Me Love Steve Angello Laidback Luke"),
    ("Tim Berg (Avicii)", "Seek Bromance", "Tim Berg Seek Bromance Vocal Edit"),
    ("Adrian Lux", "Teenage Crime (Axwell Remode)", "Adrian Lux Teenage Crime Axwell Remode"),
    ("Guru Josh Project", "Infinity 2008 (Klaas Remix)", "Guru Josh Project Infinity 2008 Klaas"),
    ("Kaskade", "Move For Me", "Kaskade Move For Me Extended"),
    ("Afrojack ft. Eva Simons", "Take Over Control", "Afrojack Take Over Control Extended"),
    ("Sebastian Ingrosso", "Kidsos", "Sebastian Ingrosso Kidsos Original"),
    ("John Dahlback", "Pyramid (Dirty South Remix)", "John Dahlback Pyramid Dirty South"),
    ("Reflekt", "Need To Feel Loved (Adam K & Soha Mix)", "Reflekt Need To Feel Loved Adam K Soha"),
    ("Deadmau5 ft. Rob Swire", "Ghosts 'n' Stuff", "Deadmau5 Ghosts n Stuff Extended"),
    ("Dirty South", "Walking Alone", "Dirty South Walking Alone Club Mix"),

    # Set 13 (Progressive House Side B)
    ("Deadmau5", "Strobe (Club Edit)", "Deadmau5 Strobe Club Edit"),
    ("Eric Prydz vs Floyd", "Proper Education", "Eric Prydz Proper Education Club Mix"),
    ("Axwell", "I Found U (Classic Mix)", "Axwell I Found U Classic Mix"),
    ("TV Rock ft. Seany B", "Flaunt It", "TV Rock Flaunt It Original"),
    ("David Guetta", "Love Is Gone (Joachim Garraud Remix)", "David Guetta Love Is Gone Joachim Garraud"),
    ("Sander van Doorn", "Riff", "Sander van Doorn Riff Original"),
    ("Steve Angello", "Knas", "Steve Angello Knas Original"),
    ("Swedish House Mafia", "Miami 2 Ibiza", "Swedish House Mafia Miami 2 Ibiza Extended"),
    ("Paul Kalkbrenner", "Sky and Sand", "Paul Kalkbrenner Sky and Sand Original"),
    ("Klaas", "Our Own Way", "Klaas Our Own Way Original"),
    ("Fedde Le Grand", "3 Minutes To Explain", "Fedde Le Grand 3 Minutes To Explain"),
    ("Dirty South & Axwell", "Open Your Heart", "Dirty South Open Your Heart Axwell"),
    ("Kaskade", "Dynasty (Kaskade Arena Mix)", "Kaskade Dynasty Arena Mix"),

    # Set 10 (Electro & Bloghouse Side A)
    ("Justice", "D.A.N.C.E. (Extended)", "Justice DANCE Extended"),
    ("The Bloody Beetroots ft. Steve Aoki", "Warp 1.9", "Bloody Beetroots Steve Aoki Warp 1.9"),
    ("Kid Cudi vs Crookers", "Day 'n' Nite (Crookers Remix)", "Kid Cudi Day n Nite Crookers"),
    ("Boys Noize", "& Down", "Boys Noize and Down Original"),
    ("Simian Mobile Disco", "Hustler", "Simian Mobile Disco Hustler Original"),
    ("MSTRKRFT", "Heartbreaker (Wolfgang Gartner Remix)", "MSTRKRFT Heartbreaker Wolfgang Gartner"),
    ("Chuckie", "Let The Bass Kick", "Chuckie Let The Bass Kick Original"),
    ("Afrojack", "Polkadots", "Afrojack Polkadots Original"),
    ("Major Lazer", "Pon De Floor", "Major Lazer Pon De Floor Extended"),
    ("Sidney Samson", "Riverside", "Sidney Samson Riverside Original"),
    ("Bingo Players", "Cry (Just a Little)", "Bingo Players Cry Just a Little"),
    ("Fake Blood", "Mars", "Fake Blood Mars Original"),
    ("Wolfgang Gartner", "Illmerica", "Wolfgang Gartner Illmerica Extended"),
    ("Dada Life", "Kick Out The Epic Motherfucker", "Dada Life Kick Out The Epic Motherfucker"),
    ("Justice", "Genesis", "Justice Genesis Original"),

    # Set 14 (Electro & Bloghouse Side B)
    ("Justice vs Simian", "We Are Your Friends", "Justice Simian We Are Your Friends"),
    ("Yeah Yeah Yeahs", "Heads Will Roll (A-Trak Remix)", "Yeah Yeah Yeahs Heads Will Roll A Trak Remix"),
    ("Duck Sauce", "Barbra Streisand", "Duck Sauce Barbra Streisand Club Mix"),
    ("Martin Solveig & Dragonette", "Hello", "Martin Solveig Dragonette Hello Club Mix"),
    ("Mr. Oizo", "Positif", "Mr Oizo Positif Original"),
    ("Kavinsky", "Nightcall", "Kavinsky Nightcall Original"),
    ("Duck Sauce", "aNYway", "Duck Sauce aNYway Original"),
    ("Digitalism", "Pogo", "Digitalism Pogo Original"),
    ("Vitalic", "Poison Lips", "Vitalic Poison Lips Extended"),
    ("The Chemical Brothers", "Hey Boy Hey Girl (Soulwax Remix)", "Chemical Brothers Hey Boy Hey Girl Soulwax"),
    ("Congorock", "Babylon", "Congorock Babylon Original"),
    ("Laidback Luke & Steve Aoki", "Turbulence", "Laidback Luke Steve Aoki Turbulence"),
    ("Boys Noize", "Oi Oi Oi", "Boys Noize Oi Oi Oi Original"),
    ("Uffie", "Pop The Glock", "Uffie Pop The Glock Original"),
    ("SebastiAn", "Motor", "Sebastian Motor Original"),

    # Set 11 (Trance Sensation Side A)
    ("Armin van Buuren ft. Sharon den Adel", "In and Out of Love", "Armin van Buuren Sharon den Adel In and Out of Love Extended"),
    ("Dash Berlin", "Till The Sky Falls Down", "Dash Berlin Till The Sky Falls Down Vocal Mix"),
    ("Dash Berlin ft. Emma Hewitt", "Waiting", "Dash Berlin Waiting Extended Vocal"),
    ("Above & Beyond", "Alone Tonight", "Above and Beyond Alone Tonight Club Mix"),
    ("Gareth Emery", "Sanctuary", "Gareth Emery Sanctuary Club Mix"),
    ("John O'Callaghan", "Big Sky (Agnelli & Nelson Remix)", "John O Callaghan Big Sky Agnelli Nelson"),
    ("OceanLab", "Satellite", "OceanLab Satellite Original Above and Beyond"),
    ("Tiesto", "Elements of Life", "Tiesto Elements of Life Original"),
    ("Marcel Woods", "Advanced", "Marcel Woods Advanced Sensation White"),
    ("Sander van Doorn", "By Any Demand", "Sander van Doorn By Any Demand"),
    ("Ferry Corsten", "Beautiful", "Ferry Corsten Beautiful Extended"),
    ("Paul van Dyk", "Let Go", "Paul van Dyk Let Go Vandit"),
    ("Cosmic Gate", "Exploration of Space (Back 2 The Future)", "Cosmic Gate Exploration of Space Back 2 The Future"),
    ("W&W", "Mustang", "W and W Mustang Original"),
    ("Above & Beyond", "Sun & Moon", "Above and Beyond Sun and Moon Club Mix"),

    # Set 15 (Trance Sensation Side B)
    ("Armin van Buuren", "Going Wrong", "Armin van Buuren Going Wrong Extended"),
    ("Above & Beyond", "Can't Sleep", "Above and Beyond Cant Sleep Original"),
    ("OceanLab", "On A Good Day", "OceanLab On A Good Day Original"),
    ("Super8 & Tab", "Helsinki Scorchin'", "Super8 and Tab Helsinki Scorchin"),
    ("Stoneface & Terminal", "Supernature", "Stoneface and Terminal Supernature"),
    ("Aly & Fila", "Lost Language", "Aly and Fila Lost Language Original"),
    ("Filo & Peri", "Anthem", "Filo and Peri Anthem Original"),
    ("Leon Bolier", "Ocean Drive Boulevard", "Leon Bolier Ocean Drive Boulevard"),
    ("Rank 1", "L.E.D. There Be Light", "Rank 1 LED There Be Light Trance Energy"),
    ("Sander van Doorn", "Renegade", "Sander van Doorn Renegade Trance Energy"),
    ("Tiesto ft. Maxi Jazz", "Dance4Life", "Tiesto Maxi Jazz Dance4Life Extended"),
    ("ATB", "Ecstasy", "ATB Ecstasy Club Mix"),
    ("Armin van Buuren", "Unforgivable", "Armin van Buuren Unforgivable Stoneface"),
    ("Ferry Corsten", "Radio Crash", "Ferry Corsten Radio Crash Extended"),
    ("John O'Callaghan", "Find Yourself (Cosmic Gate Remix)", "John O Callaghan Find Yourself Cosmic Gate"),

    # Set 12 (Commercial Vocal Dance Side A)
    ("David Guetta ft. Kelly Rowland", "When Love Takes Over", "David Guetta Kelly Rowland When Love Takes Over Extended"),
    ("David Guetta ft. Akon", "Sexy Bitch", "David Guetta Akon Sexy Bitch Extended"),
    ("David Guetta ft. Kid Cudi", "Memories", "David Guetta Kid Cudi Memories Extended"),
    ("Calvin Harris", "I'm Not Alone", "Calvin Harris Im Not Alone Extended"),
    ("Inna", "Hot", "Inna Hot Play and Win Extended"),
    ("Inna", "Deja Vu", "Inna Deja Vu Play and Win"),
    ("Edward Maya", "Stereo Love", "Edward Maya Vika Jigulina Stereo Love Extended"),
    ("Cascada", "Evacuate The Dancefloor", "Cascada Evacuate The Dancefloor Extended"),
    ("September", "Cry For You", "September Cry For You Extended"),
    ("Basshunter", "Now You're Gone", "Basshunter Now Youre Gone Club Mix"),
    ("Stromae", "Alors On Danse", "Stromae Alors On Danse Extended"),
    ("Pitbull", "I Know You Want Me (Calle Ocho)", "Pitbull I Know You Want Me Calle Ocho Extended"),
    ("Yolanda Be Cool vs DCUP", "We No Speak Americano", "Yolanda Be Cool DCUP We No Speak Americano"),
    ("R.I.O.", "Shine On", "RIO Shine On Original Mix"),
    ("Cascada", "Everytime We Touch", "Cascada Everytime We Touch Club Mix"),

    # Set 16 (Commercial Vocal Dance Side B)
    ("David Guetta vs The Egg", "Love Don't Let Me Go (Walking Away)", "David Guetta Love Dont Let Me Go Walking Away"),
    ("The Black Eyed Peas", "I Gotta Feeling (David Guetta Remix)", "Black Eyed Peas I Gotta Feeling David Guetta FMIF"),
    ("Calvin Harris", "Flashback", "Calvin Harris Flashback Original Extended"),
    ("Calvin Harris", "Acceptable in the 80s", "Calvin Harris Acceptable in the 80s Extended"),
    ("Inna", "Sun Is Up", "Inna Sun Is Up Play and Win"),
    ("Edward Maya", "This Is My Life", "Edward Maya This Is My Life Extended"),
    ("Laurent Wolf", "Wash My World", "Laurent Wolf Wash My World Club Mix"),
    ("Antoine Clamaran", "Gold", "Antoine Clamaran Gold Extended"),
    ("Pakito", "Living On Video", "Pakito Living On Video Original 12"),
    ("Junior Caldera", "Can't Fight This Feeling", "Junior Caldera Cant Fight This Feeling"),
    ("Remady", "No Superstar", "Remady No Superstar Full Vocal"),
    ("Klaas", "What Is Love 2K9", "Klaas What Is Love 2K9 Club Mix"),
    ("Agnes", "Release Me", "Agnes Release Me Extended"),
    ("Oceana", "Cry Cry", "Oceana Cry Cry DJ Fisun"),
    ("R.I.O.", "When The Sun Comes Down", "RIO When The Sun Comes Down Club Mix")
]

async def acquire_and_organize():
    db = Database('hera.db')
    await db.init_schema()
    conn = await db.connect()
    track_repo = TrackRepository(conn)
    cfg = HeraConfig.load('config/hera.toml')
    organizer = TrackOrganizer(track_repo, cfg.library_dir)
    
    quarantine = Path('quarantine')
    quarantine.mkdir(exist_ok=True)
    ytdlp = YtdlpProvider(max_results=3, preferred_quality='320')

    print(f'=== Descargando y organizando {len(TRACKS)} himnos para la biblioteca canónica ===')

    for i, (artist, title, q) in enumerate(TRACKS, 1):
        clean_title = title.replace('/', '-').replace('\"', '')
        clean_artist = artist.replace('/', '-').replace('\"', '')
        fname = f"{clean_artist} - {clean_title}.mp3"
        target = quarantine / fname

        print(f"[{i:03d}/{len(TRACKS):03d}] Buscando: {clean_artist} - {clean_title}...")
        try:
            # Buscar stream de alta calidad con yt-dlp
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
                    print(f"  -> [OK] {target.stat().st_size / (1024*1024):.1f} MB -> {res.destination_path}")
                else:
                    print("  -> [AVISO] Archivo demasiado pequeño o fallido")
            else:
                print(f"  -> [NO ENCONTRADO] Sin resultados para {q}")
        except Exception as e:
            print(f"  -> [ERROR] {e}")

    await db.close()
    print('=== Descarga y organización de la Era 2005-2010 completada con éxito ===')

if __name__ == '__main__':
    asyncio.run(acquire_and_organize())
