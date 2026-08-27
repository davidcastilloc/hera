"""Script definitivo de deduplicación y calibración para 16 Sets únicos."""

import asyncio
import shutil
from pathlib import Path
from hera.agent.tools import create_or_update_dj_set
from mutagen.flac import FLAC
from mutagen.mp3 import MP3

# Borrar carpeta sets por completo para evitar carpetas con nombres anteriores
sets_dir = Path("sets")
if sets_dir.exists():
    shutil.rmtree(sets_dir)
sets_dir.mkdir(exist_ok=True)

CLEAN_16_SETS = {
    # ================= ERA 1: 1991 - 2005 =================
    # VOL 1: French Touch & Disco House
    "Set 1 - French Touch & Vocal House (Side A)": [
        "Lady (Hear Me Tonight)", "Music Sounds Better With You", "One More Time",
        "Love Generation", "Groovejet", "Lucky Star", "Make Luv", "Intro",
        "The Weekend", "Another Chance", "Feel The Vibe", "E Samba", "Cassius 1999",
        "Harder Better Faster Stronger", "World, Hold On"
    ],
    "Set 5 - French Touch & Deep Disco (Side B)": [
        "Starlight", "At Night (Kid Creme", "Am I Wrong", "You Are My High",
        "Stupidisco", "Rocking Music", "In Your Arms", "The Child",
        "Feeling For You", "I Feel For You", "Sacre Francais", "Horny 98",
        "Crescendolls", "Back To Basics", "My My My"
    ],

    # VOL 2: Electro House & Peak-Time
    "Set 2 - Electro House & Peak-Time (Side A)": [
        "Satisfaction", "Call on Me", "Put Your Hands Up 4 Detroit", "Drop The Pressure",
        "Flashdance", "Loneliness", "The Sound of Violence", "Say Hello",
        "Lola's Theme", "In My Arms", "The World Is Mine", "Rise Up",
        "Show Me Love", "Buy Now", "Never Say Never", "A Bit Patchy"
    ],
    "Set 6 - Electro & Dirty Club Anthems (Side B)": [
        "Perfect Exceeder", "Yeah Yeah", "Let Me Think About It", "Camille Jones",
        "Destination Calabria", "Elektro", "Dave Spoon", "Flaunt It",
        "Kinda New", "Calabria 2007", "Out Of Touch", "Falling Stars",
        "Say Say Say", "Tell Me Why", "Steve Bug", "I See Girls"
    ],

    # VOL 3: Sensation White & Eurodance Classics
    "Set 3 - Eurodance & Trance Anthems (Side A)": [
        "Sandstorm", "On The Move", "Turn The Tide", "L'Amour Toujours",
        "Better Off Alone", "Children", "Adagio for Strings", "Carte Blanche",
        "9 PM (Till I Come)", "Played-A-Live", "Strange World", "Out Of The Blue",
        "Simulated", "Superstring", "Lasgo"
    ],
    "Set 7 - Trance & Progressive Euphoria (Side B)": [
        "As The Rush Comes", "For An Angel", "Airwave", "Serenity",
        "Universal Nation", "Cafe Del Mar", "Silence (Tiesto", "Lethal Industry",
        "Southern Sun", "Fire Wire", "Beauty of Silence", "Komodo",
        "Gouryella", "Take Me Away (Into The Night)"
    ],

    # VOL 4: House Divas & Vocal Legends
    "Set 4 - House Divas & Club Classics (Side A)": [
        "Gypsy Woman", "Finally", "Sing It Back", "Point Of View",
        "Make a Move On Me", "Needin' U", "Touch Me", "Make The World Go Round",
        "Beautiful People", "Free (Mood II Swing", "The Bomb",
        "Professional Widow", "Push The Feeling On", "Missing (Todd Terry", "Rhythm of the Night"
    ],
    "Set 8 - Soulful House & Vocal Legends (Side B)": [
        "Luv 4 Luv", "Found A Cure", "To Be In Love", "Caught In The Middle",
        "Where Love Lives", "Dreamer", "Ride On Time", "Closer Than Close",
        "King of My Castle", "Cant Get Enough", "Big Love", "Ultra Flava",
        "So In Love With You", "Discos Revenge", "Shined On Me"
    ],

    # ================= ERA 2: 2005 - 2010 =================
    # VOL 5: Progressive House & Stadium Melodic
    "Set 9 - Progressive House & Stadium Melodic (Side A)": [
        "I Remember", "Pjanoo", "One", "Leave The World Behind",
        "Seek Bromance", "Teenage Crime", "Infinity 2008", "Move For Me",
        "Take Over Control", "Kidsos", "Pyramid", "Need To Feel Loved",
        "Ghosts 'n' Stuff", "Walking Alone"
    ],
    "Set 13 - Progressive House & Deep Anthems (Side B)": [
        "Strobe", "Proper Education", "I Found U", "David Guetta Love Is Gone",
        "Riff", "Knas", "Miami 2 Ibiza", "Sky and Sand",
        "Our Own Way", "3 Minutes To Explain", "Open Your Heart", "Dynasty"
    ],

    # VOL 6: French Electro & Bloghouse Anthems
    "Set 10 - French Electro & Bloghouse (Side A)": [
        "D.A.N.C.E.", "Warp 1.9", "Day 'n' Nite", "& Down",
        "Hustler", "Heartbreaker", "Let The Bass Kick", "Polkadots",
        "Pon De Floor", "Riverside", "Cry (Just a Little)", "Mars",
        "Illmerica", "Kick Out The Epic", "Genesis"
    ],
    "Set 14 - Dirty Dutch & Club Bilde (Side B)": [
        "We Are Your Friends", "Heads Will Roll", "Barbra Streisand", "Hello",
        "Positif", "Nightcall", "aNYway", "Pogo",
        "Poison Lips", "Hey Boy Hey Girl", "Babylon", "Turbulence",
        "Oi Oi Oi", "Pop The Glock", "Motor"
    ],

    # VOL 7: Trance Renaissance & Sensation Stadium
    "Set 11 - Sensation White & ASOT Trance (Side A)": [
        "In and Out of Love", "Till The Sky Falls Down", "Waiting",
        "Alone Tonight", "Sanctuary", "Big Sky", "Satellite",
        "Elements of Life", "Advanced", "By Any Demand", "Beautiful"
    ],
    "Set 15 - Trance Renaissance & Euphoria (Side B)": [
        "Going Wrong", "Can't Sleep", "On A Good Day", "Helsinki Scorchin",
        "Supernature", "Lost Language", "Anthem", "Ocean Drive Boulevard",
        "L.E.D. There Be Light", "Renegade", "Dance4Life"
    ],

    # VOL 8: Commercial Vocal Dance & Global Club Hits
    "Set 12 - Commercial Vocal Dance & Global Bangers (Side A)": [
        "When Love Takes Over", "Sexy Bitch", "Memories", "I'm Not Alone",
        "Hot", "Deja Vu", "Stereo Love", "Evacuate The Dancefloor",
        "Cry For You", "Now You're Gone", "Alors On Danse", "I Know You Want Me",
        "We No Speak Americano", "Shine On"
    ],
    "Set 16 - European Club Hits & Vocal Anthems (Side B)": [
        "Love Don't Let Me Go", "I Gotta Feeling", "Flashback", "Acceptable in the 80s",
        "Sun Is Up", "This Is My Life", "Wash My World", "Gold",
        "Living On Video", "Can't Fight This Feeling", "No Superstar", "What Is Love 2K9",
        "Release Me", "Cry Cry", "When The Sun Comes Down"
    ]
}

async def build_zero_duplicates_certified():
    for name, track_list in CLEAN_16_SETS.items():
        await create_or_update_dj_set(name, track_list)

    track_locations = {}
    total = 0.0
    print("\n" + "="*90)
    print(f"{'DISCO / SET':<56} | {'TRKS':<4} | {'DURACIÓN':<8} | {'CD-R (80m)'}")
    print("="*90)
    
    for s in sorted(sets_dir.glob("Set*"), key=lambda x: int(x.name.split(" - ")[0].replace("Set ", ""))):
        files = [f for f in s.glob("*.*") if not f.name.startswith("_") and f.suffix.lower() in [".flac", ".mp3"]]
        dur = sum(FLAC(str(f)).info.length / 60.0 if f.suffix.lower() == ".flac" else MP3(str(f)).info.length / 60.0 for f in files)
        total += dur
        
        for f in files:
            clean = f.stem.split(". ", 1)[-1].split(" [")[0].strip()
            if clean not in track_locations:
                track_locations[clean] = []
            track_locations[clean].append(s.name)
            
        print(f"{s.name:<56} | {len(files):2d}   | {dur:5.2f}m  | {dur/80*100:4.1f}%")

    print("="*90)
    dups = {k: v for k, v in track_locations.items() if len(v) > 1}
    print(f"TOTAL DUPLICADOS ENCONTRADOS: {len(dups)}")
    if dups:
        for k, v in dups.items():
            print(f"  [ALERTA DUPLICADO] {k} -> {v}")
    else:
        print(">>> CERTIFICACIÓN APROBADA: 100% TEMAS ÚNICOS (0% DUPLICADOS EN TODA LA BIBLIOTECA) <<<")

if __name__ == "__main__":
    asyncio.run(build_zero_duplicates_certified())
