import sys
import asyncio
from hera.agent.tools import search_and_acquire_tracks

if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

async def main():
    queries = [
        # Trance & Eurodance fillers (Sets 3 y 7)
        "Rank 1 Airwave",
        "System F Out Of The Blue",
        "Gouryella Gouryella",
        "Delerium Silence Tiesto",
        "Paul Oakenfold Southern Sun",
        "Lasgo Something",
        "Ian Van Dahl Castles In The Sky",
        "Special D Come With Me",
        "Scooter The Logical Song",
        "Milk Inc Walk On Water",
        
        # Electro House / Dirty Club fillers (Sets 2 y 6)
        "The Chemical Brothers Hey Boy Hey Girl",
        "Fatboy Slim Right Here Right Now",
        "Underworld Born Slippy",
        "Prodigy Smack My Bitch Up",
        "Justice vs Simian Never Be Alone",
        "MSTRKRFT Easy Love",
        
        # Vocal House / Disco fillers (Sets 4, 5 y 8)
        "Shakedown At Night",
        "Solu Music Fade",
        "Moony Dove",
        "Kylie Minogue Can't Get You Out Of My Head",
        "Sophie Ellis-Bextor Murder On The Dancefloor",
        "Milk & Sugar Let The Sun Shine",
    ]
    
    print("--- BUSCANDO Y DESCARGANDO PISTAS PARA ALCANZAR 60+ MINUTOS EN TODOS LOS SETS ---")
    res = await search_and_acquire_tracks(queries)
    print(res)

if __name__ == "__main__":
    asyncio.run(main())
