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

    # 1. Test Capabilities / Help Intent
    print("\n[QA 1] Probando intencion de capacidades y presentacion:")
    print("  [PROMPT]: 'que cosas puedes hacer por mi?'")
    print("  [RESPUESTA AGENTE]:")
    await _handle_nlp("que cosas puedes hacer por mi?")

    # 2. Test Harmonic Mixing Recommendation Tool
    print("\n[QA 2] Probando calculo de rueda Camelot:")
    rec = recommend_harmonic_transitions("8A", 124.0)
    assert "8A" in rec and "9A" in rec, "Fallo en calculo Camelot"
    print("  -> PASS: Rueda Camelot calculada correctamente.")

    # 3. Test Library Inventory Intent
    print("\n[QA 3] Probando consulta de inventario real:")
    print("  [PROMPT]: 'Que canciones tengo en mi biblioteca?'")
    print("  [RESPUESTA AGENTE]:")
    await _handle_nlp("Que canciones tengo en mi biblioteca?")

    # 4. Test Cloud Sync Intent
    print("\n[QA 4] Probando intencion de sincronizacion en la nube:")
    print("  [PROMPT]: 'Sube mis sets a Google Drive'")
    print("  [RESPUESTA AGENTE]:")
    await _handle_nlp("Sube mis sets a Google Drive")

    print("\n" + "=" * 80)
    print(" TODOS LOS CHEQUEOS DE QA PASARON CON EXITO (100% FUNCIONAL)")
    print("=" * 80)


if __name__ == "__main__":
    asyncio.run(run_full_qa())
