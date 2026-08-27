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
        # Set 5 - Side B: French Touch & Deep Disco
        "Together So Much Love To Give",
        "Alan Braxe Intro",
        "Cassius 1999",
        "Etienne de Crecy Am I Wrong",
        "Daft Punk Harder Better Faster Stronger",
        "Demon You Are My High",
        "Junior Jack E Samba",
        
        # Set 6 - Side B: Electro & Dirty Club Anthems
        "Bodyrox Yeah Yeah",
        "The Egg Walking Away Tocadisco",
        "Freeform Five No More Conversations Mylo",
        "Mason Exceeder",
        "Switch A Bit Patchy",
        "Deep Dish Say Hello",
        
        # Set 7 - Side B: Trance & Progressive Euphoria
        "Tiesto Adagio for Strings",
        "Paul van Dyk For An Angel",
        "Energy 52 Cafe Del Mar",
        "Motorcycle As The Rush Comes",
        "Robert Miles Children",
        "Safri Duo Played A Live",
        "ATB 9 PM Till I Come",
        
        # Set 8 - Side B: Soulful House & Vocal Divas
        "Shapeshifters Lola's Theme",
        "David Morales Needin U",
        "Praise Cats Shined On Me",
        "Barbara Tucker Beautiful People",
        "CeCe Peniston Finally",
        "Juliet Avalon Jacques Lu Cont",
    ]
    
    print("--- BUSCANDO Y ENCOLANDO DESCARGAS SIDE B EN SOULSEEK P2P ---")
    res = await search_and_acquire_tracks(queries)
    print(res)

if __name__ == "__main__":
    asyncio.run(main())
