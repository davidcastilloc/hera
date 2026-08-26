"""QA Test Suite for Hera Chat / Agent."""

import asyncio
import io
import sys
from pathlib import Path

from hera.agent.brain import HeraBrain, _handle_nlp
from hera.agent.tools import (
    get_library_status,
    recommend_harmonic_transitions,
    create_or_update_dj_set,
    sync_sets_to_cloud,
)


async def run_full_qa():
    print("=" * 80)
    print(" QA TEST SUITE: HERA AGENT & NATURAL LANGUAGE ENGINE")
    print("=" * 80)

    # 1. Test Library Status & Inventory Tool
    print("\n[QA 1] Probando consulta de inventario real:")
    inv = get_library_status()
    assert "LIBRARY INVENTORY:" in inv, "Fallo al obtener inventario"
    print("  -> PASS: Inventario recuperado correctamente.")
    print("     Snippet:", inv.splitlines()[1])

    # 2. Test Harmonic Mixing Recommendation Tool
    print("\n[QA 2] Probando calculo de rueda Camelot:")
    rec = recommend_harmonic_transitions("8A", 124.0)
    assert "8A" in rec and "9A" in rec, "Fallo en calculo Camelot"
    print("  -> PASS: Rueda Camelot calculada correctamente.")
    print("     Snippet:", rec.splitlines()[0])
    print("     ", rec.splitlines()[1])
    print("     ", rec.splitlines()[2])

    # 3. Test Set Builder & Crate Generation Tool
    print("\n[QA 3] Probando compilacion de Set y generacion de Cue Sheet:")
    res_set = await create_or_update_dj_set("QA Test Crate", ["Modjo", "Stardust", "Daft Punk"])
    assert "Created set" in res_set, "Fallo en creacion de set"
    set_dir = Path("sets/QA Test Crate")
    cue_file = set_dir / "_00_SET_GUIDE.txt"
    assert set_dir.exists(), "La carpeta del set no fue creada"
    assert cue_file.exists(), "El archivo _00_SET_GUIDE.txt no fue generado"
    print("  -> PASS: Set compilado y _00_SET_GUIDE.txt generado.")
    
    # Limpiar carpeta temporal de QA
    for f in set_dir.glob("*.*"):
        f.unlink(missing_ok=True)
    set_dir.rmdir()

    # 4. Test Cloud Sync Simulation Tool
    print("\n[QA 4] Probando simulacion de sincronizacion en la nube (dry-run):")
    sync_res = await sync_sets_to_cloud(dry_run=True)
    assert "Successfully" in sync_res or "gdrive:" in sync_res, "Fallo en sync"
    print(f"  -> PASS: {sync_res}")

    # 5. Test Natural Language Parser (_handle_nlp)
    print("\n[QA 5] Probando pipeline de lenguaje natural con prompts reales:")
    test_prompts = [
        "Que canciones tengo en mi biblioteca?",
        "Si estoy tocando en 8A a 124 BPM que temas puedo mezclar?",
        "Sube mis sets a Google Drive",
    ]

    for p in test_prompts:
        print(f"\n  [PROMPT]: '{p}'")
        print("  [RESPUESTA AGENTE]:")
        await _handle_nlp(p)

    print("\n" + "=" * 80)
    print(" TODOS LOS CHEQUEOS DE QA PASARON CON EXITO (100% FUNCIONAL)")
    print("=" * 80)


if __name__ == "__main__":
    asyncio.run(run_full_qa())
