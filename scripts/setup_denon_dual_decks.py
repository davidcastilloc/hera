"""Organización y generación de Guías de Mezcla Cruzada (Ping-Pong) para la Denon DN-D4500."""

import os
import shutil
from pathlib import Path
from mutagen.flac import FLAC
from mutagen.mp3 import MP3

sets_dir = Path("sets")

# Mapeo de nombres antiguos a la estructura profesional de Volúmenes Gemelos para Denon
PAIRS = [
    # VOL 1
    ("Set 1 - French Touch & Vocal House (Side A)", "VOL 01 - DECK A - French Touch & Vocal House"),
    ("Set 5 - French Touch & Deep Disco (Side B)",   "VOL 01 - DECK B - French Touch & Deep Disco"),
    # VOL 2
    ("Set 2 - Electro House & Peak-Time (Side A)",   "VOL 02 - DECK A - Electro House & Peak-Time"),
    ("Set 6 - Electro & Dirty Club Anthems (Side B)","VOL 02 - DECK B - Electro & Dirty Club Anthems"),
    # VOL 3
    ("Set 3 - Eurodance & Trance Anthems (Side A)",  "VOL 03 - DECK A - Eurodance & Trance Anthems"),
    ("Set 7 - Trance & Progressive Euphoria (Side B)","VOL 03 - DECK B - Trance & Progressive Euphoria"),
    # VOL 4
    ("Set 4 - House Divas & Club Classics (Side A)", "VOL 04 - DECK A - House Divas & Club Classics"),
    ("Set 8 - Soulful House & Vocal Legends (Side B)","VOL 04 - DECK B - Soulful House & Vocal Legends"),
    # VOL 5
    ("Set 9 - Progressive House & Stadium Melodic (Side A)", "VOL 05 - DECK A - Progressive House & Stadium Melodic"),
    ("Set 13 - Progressive House & Deep Anthems (Side B)",   "VOL 05 - DECK B - Progressive House & Deep Anthems"),
    # VOL 6
    ("Set 10 - French Electro & Bloghouse (Side A)", "VOL 06 - DECK A - French Electro & Bloghouse"),
    ("Set 14 - Dirty Dutch & Club Bilde (Side B)",   "VOL 06 - DECK B - Dirty Dutch & Club Bilde"),
    # VOL 7
    ("Set 11 - Sensation White & ASOT Trance (Side A)", "VOL 07 - DECK A - Sensation White & ASOT Trance"),
    ("Set 15 - Trance Renaissance & Euphoria (Side B)",  "VOL 07 - DECK B - Trance Renaissance & Euphoria"),
    # VOL 8
    ("Set 12 - Commercial Vocal Dance & Global Bangers (Side A)", "VOL 08 - DECK A - Commercial Vocal Dance & Global Bangers"),
    ("Set 16 - European Club Hits & Vocal Anthems (Side B)",      "VOL 08 - DECK B - European Club Hits & Vocal Anthems"),
]

# Renombrar carpetas
for old_name, new_name in PAIRS:
    old_p = sets_dir / old_name
    new_p = sets_dir / new_name
    if old_p.exists() and old_p != new_p:
        if new_p.exists():
            shutil.rmtree(new_p)
        old_p.rename(new_p)
        print(f"Renombrado: {old_name} -> {new_name}")

print("\n=== Generando Guías de Mezcla Ping-Pong (Deck A vs Deck B) ===")

def get_track_info(f_path):
    dur = 0.0
    bpm = ""
    key = ""
    try:
        if f_path.suffix.lower() == ".flac":
            fl = FLAC(str(f_path))
            dur = fl.info.length / 60.0
            bpm = fl.get("BPM", [""])[0]
            key = fl.get("INITIALKEY", [""])[0]
        else:
            mp3 = MP3(str(f_path))
            dur = mp3.info.length / 60.0
            id3 = mp3.tags
            if id3:
                bpm = str(id3.get("TBPM", ""))
                key = str(id3.get("TKEY", ""))
    except Exception:
        pass
    
    # Extraer titulo limpio del nombre de archivo
    name = f_path.stem
    if ". " in name:
        name = name.split(". ", 1)[1]
    if " [" in name:
        name = name.split(" [")[0]
    return name, dur, bpm, key

for vol_num in range(1, 9):
    vol_str = f"VOL {vol_num:02d}"
    deck_a_folder = list(sets_dir.glob(f"{vol_str} - DECK A*"))[0]
    deck_b_folder = list(sets_dir.glob(f"{vol_str} - DECK B*"))[0]
    
    tracks_a = sorted([f for f in deck_a_folder.glob("*.*") if not f.name.startswith("_") and f.suffix.lower() in [".flac", ".mp3"]])
    tracks_b = sorted([f for f in deck_b_folder.glob("*.*") if not f.name.startswith("_") and f.suffix.lower() in [".flac", ".mp3"]])
    
    info_a = [get_track_info(t) for t in tracks_a]
    info_b = [get_track_info(t) for t in tracks_b]
    
    chart = [
        "=" * 98,
        "??? DENON DN-D4500 — GUÍA DE MEZCLA EN VIVO & PING-PONG (DUAL DECK CHART)",
        f"?? {vol_str}: {deck_a_folder.name.split(' - ')[-1]} (DECK A) vs {deck_b_folder.name.split(' - ')[-1]} (DECK B)",
        "=" * 98,
        f"{'BANDEJA 1 / DRIVE 1 (DECK A - Izquierda)':<48} | {'BANDEJA 2 / DRIVE 2 (DECK B - Derecha)':<48}",
        "-" * 98,
    ]
    
    max_len = max(len(info_a), len(info_b))
    for i in range(max_len):
        str_a = ""
        str_b = ""
        if i < len(info_a):
            t_name, t_dur, t_bpm, t_key = info_a[i]
            k_str = f"[{t_key}]" if t_key else ""
            str_a = f"{i+1:02d}. {t_name[:28]:<28} {t_bpm[:3]:>3}BPM {k_str:>4} ({t_dur:4.1f}m)"
        if i < len(info_b):
            t_name, t_dur, t_bpm, t_key = info_b[i]
            k_str = f"[{t_key}]" if t_key else ""
            str_b = f"{i+1:02d}. {t_name[:28]:<28} {t_bpm[:3]:>3}BPM {k_str:>4} ({t_dur:4.1f}m)"
            
        chart.append(f"{str_a:<48} | {str_b:<48}")
        
    chart.extend([
        "=" * 98,
        "?? INSTRUCCIONES PARA PINCHAR CON TU DENON DN-D4500:",
        "1. Carga el CD DECK A en la Bandeja 1 (Izquierda) y el CD DECK B en la Bandeja 2 (Derecha).",
        "2. Modo Manual (DJ Mixing): Haz ping-pong alternando Pista A1 -> Pista B1 -> Pista A2 -> Pista B2.",
        "3. Modo Automático (Relay Play): Presiona el botón 'RELAY' en la consola. La Denon alternará de",
        "   forma automática entre la Bandeja 1 y la Bandeja 2 cada vez que termine una canción (2.5h continuo).",
        "=" * 98,
    ])
    
    # Guardar la guía dual en ambas carpetas del volumen para fácil acceso
    chart_text = "\n".join(chart) + "\n"
    (deck_a_folder / "_00_DENON_DUAL_DECK_CHART.txt").write_text(chart_text, encoding="utf-8")
    (deck_b_folder / "_00_DENON_DUAL_DECK_CHART.txt").write_text(chart_text, encoding="utf-8")
    print(f"Guía Dual generada para {vol_str} (Deck A: {len(tracks_a)} tracks / Deck B: {len(tracks_b)} tracks)")

print("\n=== Estructuración Dual Deck Completada con Éxito ===")
