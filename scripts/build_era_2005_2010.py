"""Ensamblaje y calibración de los Sets 9 al 16 (Era 2005-2010) para CD-R de 80 min."""

import asyncio
import shutil
from pathlib import Path
from hera.agent.tools import create_or_update_dj_set
from mutagen.flac import FLAC
from mutagen.mp3 import MP3

SETS_2005_2010 = {
    # Twin-Pack 5 (Vol 5) — Progressive House & Stadium Melodic
    "Set 9 - Progressive House & Stadium Melodic (Side A)": [
        "I Remember", "Pjanoo", "One", "Leave The World Behind",
        "Seek Bromance", "Teenage Crime", "Infinity 2008", "Move For Me",
        "Take Over Control", "Kidsos", "Pyramid", "Need To Feel Loved",
        "Ghosts 'n' Stuff", "Walking Alone", "Show Me Love"
    ],
    "Set 13 - Progressive House & Deep Anthems (Side B)": [
        "Strobe", "Proper Education", "I Found U", "Flaunt It",
        "Love Is Gone", "Riff", "Knas", "Miami 2 Ibiza",
        "Sky and Sand", "Our Own Way", "3 Minutes To Explain",
        "Open Your Heart", "Dynasty"
    ],

    # Twin-Pack 6 (Vol 6) — French Electro, Bloghouse & Dirty Dutch
    "Set 10 - French Electro & Bloghouse (Side A)": [
        "D.A.N.C.E.", "Warp 1.9", "Day 'n' Nite", "& Down",
        "Hustler", "Heartbreaker", "Let The Bass Kick", "Polkadots",
        "Pon De Floor", "Riverside", "Cry (Just a Little)", "Mars",
        "Illmerica", "Kick Out The Epic", "Genesis"
    ],
    "Set 14 - Dirty Dutch & Club Bilde (Side B)": [
        "We Are Your Friends", "Heads Will Roll", "Barbra Streisand",
        "Hello", "Positif", "Nightcall", "aNYway",
        "Pogo", "Poison Lips", "Hey Boy Hey Girl", "Babylon",
        "Turbulence", "Oi Oi Oi", "Pop The Glock", "Motor"
    ],

    # Twin-Pack 7 (Vol 7) — Sensation White & ASOT Trance Renaissance
    "Set 11 - Sensation White & ASOT Trance (Side A)": [
        "In and Out of Love", "Till The Sky Falls Down", "Waiting",
        "Alone Tonight", "Sanctuary", "Big Sky", "Satellite",
        "Elements of Life", "Advanced", "By Any Demand", "Beautiful",
        "Let Go", "Exploration of Space", "Mustang", "Sun & Moon"
    ],
    "Set 15 - Trance Renaissance & Euphoria (Side B)": [
        "Going Wrong", "Can't Sleep", "On A Good Day", "Helsinki Scorchin",
        "Supernature", "Lost Language", "Anthem", "Ocean Drive Boulevard",
        "L.E.D. There Be Light", "Renegade", "Dance4Life", "Ecstasy",
        "Unforgivable", "Radio Crash", "Find Yourself"
    ],

    # Twin-Pack 8 (Vol 8) — Commercial Vocal Dance & Global Bangers
    "Set 12 - Commercial Vocal Dance & Global Bangers (Side A)": [
        "When Love Takes Over", "Sexy Bitch", "Memories", "I'm Not Alone",
        "Hot", "Deja Vu", "Stereo Love", "Evacuate The Dancefloor",
        "Cry For You", "Now You're Gone", "Alors On Danse", "I Know You Want Me",
        "We No Speak Americano", "Shine On", "Everytime We Touch"
    ],
    "Set 16 - European Club Hits & Vocal Anthems (Side B)": [
        "Love Don't Let Me Go", "I Gotta Feeling", "Flashback",
        "Acceptable in the 80s", "Sun Is Up", "This Is My Life",
        "Wash My World", "Gold", "Living On Video", "Can't Fight This Feeling",
        "No Superstar", "What Is Love 2K9", "Release Me", "Cry Cry", "When The Sun Comes Down"
    ]
}

async def build_all_2005_2010():
    print("=== Ensamblando Sets 9 al 16 (Era 2005-2010) ===")
    for name, track_list in SETS_2005_2010.items():
        s_folder = Path("sets") / name
        if s_folder.exists():
            shutil.rmtree(s_folder)
        await create_or_update_dj_set(name, track_list)
        
        files = [f for f in s_folder.glob("*.*") if not f.name.startswith("_") and f.suffix.lower() in [".flac", ".mp3"]]
        dur = sum(FLAC(str(f)).info.length / 60.0 if f.suffix.lower() == ".flac" else MP3(str(f)).info.length / 60.0 for f in files)
        print(f"{name:<58} | {len(files):2d} tracks | {dur:5.2f}m ({dur/80*100:4.1f}% CD)")

if __name__ == "__main__":
    asyncio.run(build_all_2005_2010())
