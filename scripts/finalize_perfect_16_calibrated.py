"""Calibración final de duraciones (73.0 - 78.5 min) con 0% duplicados garantizado."""

import asyncio
import shutil
from pathlib import Path
from hera.agent.tools import create_or_update_dj_set
from mutagen.flac import FLAC
from mutagen.mp3 import MP3

sets_dir = Path("sets")
if sets_dir.exists():
    shutil.rmtree(sets_dir)
sets_dir.mkdir(exist_ok=True)

PERFECT_CALIBRATED_16 = {
    # VOL 1 (French Touch & Disco)
    "Set 1 - French Touch & Vocal House (Side A)": [
        "Modjo", "Stardust", "Daft Punk - One More Time", "Bob Sinclar - Love Generation",
        "Spiller - Groovejet", "Superfunk - Lucky Star", "Room 5 - Make Luv",
        "Alan Braxe - Intro", "Michael Gray - The Weekend", "Roger Sanchez - Another Chance",
        "Axwell - Feel The Vibe", "Junior Jack - E Samba", "Cassius - Cassius 1999",
        "Daft Punk - Harder Better", "Bob Sinclar - World, Hold On"
    ],
    "Set 5 - French Touch & Deep Disco (Side B)": [
        "The Supermen Lovers - Starlight", "Shakedown - At Night", "Etienne de Crecy - Am I Wrong",
        "Demon vs Heartbreaker - You Are My High", "Junior Jack - Stupidisco", "Martin Solveig - Rocking Music",
        "Benjamin Diamond - In Your Arms", "Alex Gopher - The Child", "Cassius - Feeling For You",
        "Bob Sinclar - I Feel For You", "Dimitri From Paris - Sacre Francais", "Mousse T - Horny 98",
        "Daft Punk - Crescendolls", "The Shapeshifters - Back To Basics", "Armand Van Helden - My My My"
    ],

    # VOL 2 (Electro House & Peak-Time) - Subiendo a ~76m con temas exclusivos
    "Set 2 - Electro House & Peak-Time (Side A)": [
        "Benny Benassi - Satisfaction", "Eric Prydz - Call On Me", "Fedde Le Grand - Put Your Hands Up 4 Detroit",
        "Mylo - Drop The Pressure", "Deep Dish - Flashdance", "Tomcraft - Loneliness",
        "Cassius - The Sound of Violence", "Deep Dish - Say Hello", "The Shapeshifters - Lola's Theme",
        "Mylo - In My Arms", "David Guetta - The World Is Mine", "Yves LaRock - Rise Up",
        "Robin S - Show Me Love", "Buy Now", "Vandalism", "Switch - A Bit Patchy",
        "Camille Jones vs Fedde Le Grand - The Creeps", "Freaks - The Creeps"
    ],
    "Set 6 - Electro & Dirty Club Anthems (Side B)": [
        "Mason vs Princess Superstar - Perfect Exceeder", "Bodyrox ft. Luciana - Yeah Yeah",
        "Outwork ft. Mr. Gee - Elektro", "Dave Spoon - At Night", "TV Rock ft. Seany B - Flaunt It",
        "Spektrum - Kinda New", "Enur ft. Natasja - Calabria 2007", "Uniting Nations - Out Of Touch",
        "Sunset Strippers - Falling Stars", "Hi_Tack - Say Say Say", "Supermode - Tell Me Why",
        "Studio B - I See Girls", "Fedde Le Grand - Let Me Think About It"
    ],

    # VOL 3 (Sensation White & Eurodance Classics)
    "Set 3 - Eurodance & Trance Anthems (Side A)": [
        "Darude - Sandstorm", "Barthezz - On The Move", "Sylver - Turn The Tide",
        "Gigi D'Agostino - L'Amour Toujours", "Alice Deejay - Better Off Alone", "Robert Miles - Children",
        "Tiesto - Adagio for Strings", "Veracocha - Carte Blanche", "ATB - 9 PM (Till I Come)",
        "Safri Duo - Played-A-Live", "Push - Strange World", "System F - Out Of The Blue",
        "Marco V - Simulated", "Cygnus X - Superstring", "Lasgo - Something"
    ],
    "Set 7 - Trance & Progressive Euphoria (Side B)": [
        "Motorcycle - As The Rush Comes", "Paul van Dyk - For An Angel", "Rank 1 - Airwave",
        "Armin van Buuren feat. Jan Vayne - Serenity", "Push - Universal Nation",
        "Energy 52 - Cafe Del Mar", "Delerium - Silence", "Tiesto - Lethal Industry",
        "Paul Oakenfold - Southern Sun", "Cosmic Gate - Fire Wire", "Svenson and Gielen - The Beauty of Silence",
        "Mauro Picotto - Komodo", "Gouryella - Gouryella", "4 Strings - Take Me Away"
    ],

    # VOL 4 (House Divas & Vocal Legends) - Subiendo a ~75m
    "Set 4 - House Divas & Club Classics (Side A)": [
        "Crystal Waters - Gypsy Woman", "CeCe Peniston - Finally", "Moloko - Sing It Back",
        "DB Boulevard - Point Of View", "Joey Negro - Make a Move On Me", "David Morales - Needin' U",
        "Rui Da Silva - Touch Me", "Sandy B - Make The World Go Round", "Barbara Tucker - Beautiful People",
        "Ultra Nat - Free", "The Bucketheads - The Bomb", "Tori Amos - Professional Widow",
        "Nightcrawlers - Push The Feeling On", "Everything But The Girl - Missing", "Corona - The Rhythm of the Night",
        "Alison Limerick - Where Love Lives", "Livin Joy - Dreamer"
    ],
    # Set 8: Ajustado de 86m a ~75m
    "Set 8 - Soulful House & Vocal Legends (Side B)": [
        "Robin S - Luv 4 Luv", "Ultra Nate - Found A Cure", "Masters At Work ft. India - To Be In Love",
        "Juliet Roberts - Caught In The Middle", "Black Box - Ride On Time", "Rosie Gaines - Closer Than Close",
        "Wamdue Project - King of My Castle", "Soulsearcher - Cant Get Enough", "Pete Heller - Big Love",
        "Heller and Farley Project - Ultra Flava", "Duke - So In Love With You", "Gusto - Discos Revenge",
        "Praise Cats - Shined On Me"
    ],

    # VOL 5 (Progressive House & Stadium Melodic) - Ajustado de 81m a ~76m
    "Set 9 - Progressive House & Stadium Melodic (Side A)": [
        "Deadmau5 - I Remember", "Eric Prydz - Pjanoo", "Swedish House Mafia - One (Original Mix)",
        "Axwell, Ingrosso - Leave The World Behind", "Tim Berg (Avicii) - Seek Bromance",
        "Adrian Lux - Teenage Crime", "Guru Josh Project - Infinity 2008", "Kaskade - Move For Me",
        "Afrojack ft. Eva Simons - Take Over Control", "Sebastian Ingrosso - Kidsos",
        "John Dahlback - Pyramid", "Reflekt - Need To Feel Loved", "Deadmau5 - Ghosts 'n' Stuff"
    ],
    "Set 13 - Progressive House & Deep Anthems (Side B)": [
        "Deadmau5 - Strobe", "Eric Prydz vs Floyd - Proper Education", "Axwell - I Found U",
        "David Guetta - Love Is Gone", "Sander van Doorn - Riff", "Steve Angello - Knas",
        "Swedish House Mafia - Miami 2 Ibiza", "Paul Kalkbrenner - Sky and Sand",
        "Klaas - Our Own Way", "Fedde Le Grand - 3 Minutes To Explain", "Dirty South & Axwell - Open Your Heart",
        "Kaskade - Dynasty", "Dirty South - Walking Alone"
    ],

    # VOL 6 (French Electro & Bloghouse Anthems)
    "Set 10 - French Electro & Bloghouse (Side A)": [
        "Justice - D.A.N.C.E. (Extended)", "The Bloody Beetroots ft. Steve Aoki - Warp 1.9",
        "Kid Cudi vs Crookers - Day 'n' Nite", "Boys Noize - & Down", "Simian Mobile Disco - Hustler",
        "MSTRKRFT - Heartbreaker", "Chuckie - Let The Bass Kick", "Afrojack - Polkadots",
        "Major Lazer - Pon De Floor", "Sidney Samson - Riverside", "Bingo Players - Cry (Just a Little)",
        "Fake Blood - Mars", "Wolfgang Gartner - Illmerica", "Dada Life - Kick Out The Epic",
        "Justice - Genesis", "Boys Noize - Oi Oi Oi", "Uffie - Pop The Glock"
    ],
    "Set 14 - Dirty Dutch & Club Bilde (Side B)": [
        "Justice vs Simian - We Are Your Friends", "Yeah Yeah Yeahs - Heads Will Roll",
        "Duck Sauce - Barbra Streisand", "Martin Solveig & Dragonette - Hello", "Mr. Oizo - Positif",
        "Kavinsky - Nightcall", "Duck Sauce - aNYway", "Digitalism - Pogo", "Vitalic - Poison Lips",
        "The Chemical Brothers - Hey Boy Hey Girl", "Congorock - Babylon",
        "Laidback Luke & Steve Aoki - Turbulence", "SebastiAn - Motor", "Steve Angello & Laidback Luke - Show Me Love"
    ],

    # VOL 7 (Trance Renaissance & Sensation Stadium)
    "Set 11 - Sensation White & ASOT Trance (Side A)": [
        "Armin van Buuren ft. Sharon den Adel - In and Out of Love", "Dash Berlin - Till The Sky Falls Down",
        "Dash Berlin ft. Emma Hewitt - Waiting", "Above & Beyond - Alone Tonight",
        "Gareth Emery - Sanctuary", "John O'Callaghan - Big Sky", "OceanLab - Satellite",
        "Tiesto - Elements of Life", "Marcel Woods - Advanced", "Sander van Doorn - By Any Demand",
        "Ferry Corsten - Beautiful"
    ],
    "Set 15 - Trance Renaissance & Euphoria (Side B)": [
        "Armin van Buuren - Going Wrong", "Above & Beyond - Can't Sleep", "OceanLab - On A Good Day",
        "Super8 & Tab - Helsinki Scorchin'", "Stoneface & Terminal - Supernature", "Aly & Fila - Lost Language",
        "Filo & Peri - Anthem", "Leon Bolier - Ocean Drive Boulevard", "Rank 1 - L.E.D. There Be Light",
        "Sander van Doorn - Renegade", "Tiesto ft. Maxi Jazz - Dance4Life"
    ],

    # VOL 8 (Commercial Vocal Dance & Global Club Hits)
    "Set 12 - Commercial Vocal Dance & Global Bangers (Side A)": [
        "David Guetta ft. Kelly Rowland - When Love Takes Over", "David Guetta ft. Akon - Sexy Bitch",
        "David Guetta ft. Kid Cudi - Memories", "Calvin Harris - I'm Not Alone", "Inna - Hot",
        "Inna - Deja Vu", "Edward Maya - Stereo Love", "Cascada - Evacuate The Dancefloor",
        "September - Cry For You", "Basshunter - Now You're Gone", "Stromae - Alors On Danse",
        "Pitbull - I Know You Want Me (Calle Ocho)", "Yolanda Be Cool vs DCUP - We No Speak Americano",
        "R.I.O - Shine On"
    ],
    "Set 16 - European Club Hits & Vocal Anthems (Side B)": [
        "David Guetta vs The Egg - Love Don't Let Me Go", "The Black Eyed Peas - I Gotta Feeling",
        "Calvin Harris - Flashback", "Calvin Harris - Acceptable in the 80s", "Inna - Sun Is Up",
        "Edward Maya - This Is My Life", "Laurent Wolf - Wash My World", "Antoine Clamaran - Gold",
        "Pakito - Living On Video", "Junior Caldera - Can't Fight This Feeling", "Remady - No Superstar",
        "Klaas - What Is Love 2K9", "Agnes - Release Me", "Oceana - Cry Cry", "R.I.O - When The Sun Comes Down"
    ]
}

async def build_and_audit():
    for name, track_list in PERFECT_CALIBRATED_16.items():
        await create_or_update_dj_set(name, track_list)

    track_locations = {}
    total = 0.0
    print("\n" + "="*92)
    print(f"{'DISCO / SET':<58} | {'TRKS':<4} | {'DURACIÓN':<8} | {'CD-R (80m)'}")
    print("="*92)
    
    for s in sorted(sets_dir.glob("Set*"), key=lambda x: int(x.name.split(" - ")[0].replace("Set ", ""))):
        files = [f for f in s.glob("*.*") if not f.name.startswith("_") and f.suffix.lower() in [".flac", ".mp3"]]
        dur = sum(FLAC(str(f)).info.length / 60.0 if f.suffix.lower() == ".flac" else MP3(str(f)).info.length / 60.0 for f in files)
        total += dur
        
        for f in files:
            clean = f.stem.split(". ", 1)[-1].split(" [")[0].strip()
            if clean not in track_locations:
                track_locations[clean] = []
            track_locations[clean].append(s.name)
            
        print(f"{s.name:<58} | {len(files):2d}   | {dur:5.2f}m  | {dur/80*100:4.1f}%")

    print("="*92)
    h = int(total // 60)
    m = int(total % 60)
    print(f"TOTAL: {h} HORAS Y {m:02d} MINUTOS ({total:.1f} minutos)")
    print("="*92)
    
    dups = {k: v for k, v in track_locations.items() if len(v) > 1}
    print(f"\nAUDITORÍA DE DUPLICADOS: {len(dups)} encontrados")
    if dups:
        for k, v in dups.items():
            print(f"  [DUPLICADO] {k} -> {v}")
    else:
        print(">>> RESULTADO FINAL: 100% TEMAS ÚNICOS (0% DUPLICADOS EN TODA LA COLECCIÓN) <<<")

if __name__ == "__main__":
    asyncio.run(build_and_audit())
