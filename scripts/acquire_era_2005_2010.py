"""Script de adquisicion masiva para la Era 2005-2010 (Sets 9 al 16)."""

import asyncio
from pathlib import Path
from hera.domain.config import HeraConfig
from hera.infra.lifecycle import SlskdLifecycle
from hera.agent.tools import search_and_acquire_tracks
from providers.ytdlp.client import YtdlpProvider

TRACKS_TO_ACQUIRE = [
    # Twin-Pack 5 (Vol 5) - Progressive House & Melodic
    "Deadmau5 Kaskade I Remember",
    "Eric Prydz Pjanoo Club Mix",
    "Swedish House Mafia One Original Mix",
    "Leave The World Behind Axwell Ingrosso Angello",
    "Tim Berg Seek Bromance Vocal Edit",
    "Adrian Lux Teenage Crime Axwell Remode",
    "Guru Josh Project Infinity 2008 Klaas",
    "Kaskade Move For Me Extended",
    "Afrojack Eva Simons Take Over Control",
    "Sebastian Ingrosso Kidsos",
    "John Dahlback Pyramid Dirty South",
    "Reflekt Need To Feel Loved Adam K Soha",
    "Deadmau5 Rob Swire Ghosts n Stuff",
    "Dirty South Walking Alone",
    "Deadmau5 Strobe Club Edit",
    "Eric Prydz Proper Education Club Mix",
    "Axwell I Found U Classic Mix",
    "TV Rock Flaunt It Original",
    "David Guetta Love Is Gone Joachim Garraud",
    "Sander van Doorn Riff",
    "Steve Angello Knas",
    "Swedish House Mafia Miami 2 Ibiza",
    "Paul Kalkbrenner Sky and Sand",
    "Klaas Our Own Way",
    "Fedde Le Grand 3 Minutes To Explain",
    "Dirty South Open Your Heart Axwell",
    "Kaskade Dynasty Arena Mix",

    # Twin-Pack 6 (Vol 6) - French Electro, Bloghouse & Dirty Dutch
    "Justice D.A.N.C.E. Extended",
    "The Bloody Beetroots Steve Aoki Warp 1.9",
    "Kid Cudi Crookers Day n Nite",
    "Boys Noize and Down",
    "Simian Mobile Disco Hustler",
    "MSTRKRFT Heartbreaker Wolfgang Gartner",
    "Chuckie Let The Bass Kick",
    "Afrojack Polkadots",
    "Major Lazer Pon De Floor",
    "Sidney Samson Riverside",
    "Bingo Players Cry Just a Little",
    "Fake Blood Mars Original",
    "Wolfgang Gartner Illmerica",
    "Dada Life Kick Out The Epic Motherfucker",
    "Justice Genesis",
    "Justice Simian We Are Your Friends",
    "Yeah Yeah Yeahs Heads Will Roll A-Trak",
    "Duck Sauce Barbra Streisand",
    "Martin Solveig Dragonette Hello",
    "Mr Oizo Positif",
    "Kavinsky Nightcall Original",
    "Duck Sauce aNYway",
    "Digitalism Pogo",
    "Vitalic Poison Lips",
    "Chemical Brothers Hey Boy Hey Girl Soulwax",
    "Congorock Babylon Original",
    "Laidback Luke Steve Aoki Turbulence",
    "Boys Noize Oi Oi Oi",
    "Uffie Pop The Glock",
    "Sebastian Motor Original",

    # Twin-Pack 7 (Vol 7) - Sensation White & ASOT Trance Renaissance
    "Armin van Buuren Sharon den Adel In and Out of Love",
    "Dash Berlin Till The Sky Falls Down",
    "Dash Berlin Emma Hewitt Waiting Extended",
    "Above and Beyond Alone Tonight Club Mix",
    "Gareth Emery Lucy Saunders Sanctuary",
    "John O Callaghan Audrey Gallagher Big Sky",
    "OceanLab Above and Beyond Satellite",
    "Tiesto Elements of Life Original",
    "Ferry Corsten Beautiful Extended",
    "Paul van Dyk Rea Garvey Let Go",
    "Cosmic Gate Exploration of Space Back 2 The Future",
    "W and W Mustang Original Mix",
    "Above and Beyond Richard Bedford Sun and Moon",
    "Armin van Buuren Chris Jones Going Wrong",
    "Above and Beyond Cant Sleep Original",
    "OceanLab On A Good Day",
    "Super8 and Tab Helsinki Scorchin",
    "Stoneface and Terminal Supernature",
    "Aly and Fila Lost Language",
    "Filo and Peri Eric Lumiere Anthem",
    "Leon Bolier Ocean Drive Boulevard",
    "Rank 1 L.E.D. There Be Light",
    "Sander van Doorn Renegade Trance Energy",
    "Tiesto Maxi Jazz Dance4Life",
    "ATB Ecstasy Club Mix",
    "Armin van Buuren Jaren Unforgivable",
    "Ferry Corsten Radio Crash Extended",
    "John O Callaghan Find Yourself Cosmic Gate",

    # Twin-Pack 8 (Vol 8) - Commercial Vocal Dance & Global Bangers
    "David Guetta Kelly Rowland When Love Takes Over",
    "David Guetta Akon Sexy Bitch Extended",
    "David Guetta Kid Cudi Memories",
    "Calvin Harris Im Not Alone Extended",
    "Inna Hot Play and Win Extended",
    "Inna Deja Vu Play and Win",
    "Edward Maya Vika Jigulina Stereo Love",
    "Cascada Evacuate The Dancefloor",
    "September Cry For You Extended",
    "Basshunter Now Youre Gone Club Mix",
    "Stromae Alors On Danse Extended",
    "Pitbull I Know You Want Me Calle Ocho",
    "Yolanda Be Cool DCUP We No Speak Americano",
    "RIO Shine On Original",
    "Cascada Everytime We Touch Club Mix",
    "David Guetta The Egg Love Dont Let Me Go Walking Away",
    "Black Eyed Peas I Gotta Feeling David Guetta FMIF",
    "Calvin Harris Flashback Original Extended",
    "Calvin Harris Acceptable in the 80s",
    "Inna Sun Is Up Play and Win",
    "Edward Maya This Is My Life Extended",
    "Laurent Wolf Wash My World Club Mix",
    "Antoine Clamaran Gold Extended",
    "Pakito Living On Video Original 12",
    "Junior Caldera Cant Fight This Feeling",
    "Remady No Superstar Full Vocal",
    "Klaas What Is Love 2K9 Club Mix",
    "Agnes Release Me Extended",
    "Oceana Cry Cry DJ Fisun",
    "RIO When The Sun Comes Down Club Mix"
]

async def main():
    cfg = HeraConfig.load('config/hera.toml')
    lifecycle = SlskdLifecycle(cfg)
    lifecycle.ensure_running_sync()

    print(f'=== Iniciando adquisición de {len(TRACKS_TO_ACQUIRE)} himnos (2005-2010) ===')
    
    batch_size = 10
    quarantine = Path('quarantine')
    quarantine.mkdir(exist_ok=True)
    
    for i in range(0, len(TRACKS_TO_ACQUIRE), batch_size):
        batch = TRACKS_TO_ACQUIRE[i:i+batch_size]
        print(f'\n--- Procesando Lote {i//batch_size + 1} / {len(TRACKS_TO_ACQUIRE)//batch_size + 1} ---')
        try:
            res = await search_and_acquire_tracks(batch)
            print(res)
        except Exception as e:
            print(f'Error en lote Soulseek: {e}')
            
        await asyncio.sleep(2.0)

if __name__ == '__main__':
    asyncio.run(main())
