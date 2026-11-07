"""Reproductor Web Dual-Deck estilo CDJ/Mixer y Explorador de Biblioteca para Hera DJ Studio."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import base64
import mimetypes
import os
from pathlib import Path
import re
import socket
import threading
import urllib.parse

import streamlit as st
import streamlit.components.v1 as components

from hera.domain.config import HeraConfig

BPM_CAMELOT_RE = re.compile(r"\[(\d+(?:\.\d+)?)\s*BPM\s*-\s*([0-9]{1,2}[AB])\]", re.IGNORECASE)


class _AudioRangeHandler(BaseHTTPRequestHandler):
    """Servidor HTTP ligero con soporte CORS y Byte-Range para streaming de audio local."""

    def log_message(self, format, *args):
        pass  # Silenciar logs de cada chunk HTTP

    def _send_cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Range, Content-Type")
        self.send_header("Accept-Ranges", "bytes")

    def do_OPTIONS(self):
        self.send_response(200)
        self._send_cors()
        self.end_headers()

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path != "/stream":
            self.send_error(404, "Not found")
            return

        qs = urllib.parse.parse_qs(parsed.query)
        raw_path = qs.get("path", [None])[0]
        if not raw_path:
            self.send_error(400, "Missing path")
            return

        file_path = Path(urllib.parse.unquote(raw_path))
        if not file_path.exists() or not file_path.is_file():
            self.send_error(404, "File not found")
            return

        file_size = file_path.stat().st_size
        ext = file_path.suffix.lower()
        mime_map = {
            ".mp3": "audio/mpeg",
            ".flac": "audio/flac",
            ".wav": "audio/wav",
            ".m4a": "audio/mp4",
            ".aac": "audio/aac",
            ".ogg": "audio/ogg",
            ".aif": "audio/aiff",
            ".aiff": "audio/aiff",
        }
        content_type = mime_map.get(ext) or mimetypes.guess_type(str(file_path))[0] or "application/octet-stream"

        range_header = self.headers.get("Range")
        start = 0
        end = file_size - 1
        status_code = 200

        if range_header and range_header.startswith("bytes="):
            try:
                ranges = range_header.replace("bytes=", "").split(",")[0].split("-")
                if ranges[0]:
                    start = int(ranges[0])
                if len(ranges) > 1 and ranges[1]:
                    end = min(int(ranges[1]), file_size - 1)
                status_code = 206
            except Exception:
                start = 0
                end = file_size - 1

        length = max(0, end - start + 1)

        try:
            self.send_response(status_code)
            self._send_cors()
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(length))
            if status_code == 206:
                self.send_header("Content-Range", f"bytes {start}-{end}/{file_size}")
            self.end_headers()

            with open(file_path, "rb") as f:
                f.seek(start)
                remaining = length
                chunk_size = 64 * 1024
                while remaining > 0:
                    data = f.read(min(chunk_size, remaining))
                    if not data:
                        break
                    self.wfile.write(data)
                    remaining -= len(data)
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            pass


@st.cache_resource
def ensure_audio_stream_server(preferred_port: int = 8502) -> int:
    """Inicia un servidor de streaming de audio local en segundo plano y retorna el puerto activo."""
    for port in range(preferred_port, preferred_port + 15):
        try:
            server = ThreadingHTTPServer(("127.0.0.1", port), _AudioRangeHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            return port
        except OSError:
            continue
    return preferred_port


def get_compatible_camelot_keys(camelot_key: str | None) -> list[str]:
    """Retorna las claves armónicas compatibles en la Rueda de Camelot (misma, +-1 y relativa A/B)."""
    if not camelot_key:
        return []
    m = re.match(r"^(\d{1,2})([AB])$", camelot_key.strip().upper())
    if not m:
        return []
    num = int(m.group(1))
    letter = m.group(2)
    if not (1 <= num <= 12):
        return []

    prev_num = 12 if num == 1 else num - 1
    next_num = 1 if num == 12 else num + 1
    other_letter = "B" if letter == "A" else "A"

    return [
        f"{num}{letter}",
        f"{next_num}{letter}",
        f"{prev_num}{letter}",
        f"{num}{other_letter}",
    ]


@st.cache_data(ttl=120, show_spinner=False)
def scan_dj_library(roots: tuple[str, ...], max_tracks: int = 2500) -> list[dict]:
    """Escanea los directorios musicales y extrae metadatos rápidos (título, carpeta, BPM, Camelot)."""
    valid_exts = {".mp3", ".flac", ".wav", ".m4a", ".aiff", ".aif", ".ogg", ".mp4", ".webm", ".mkv", ".mov"}
    skip_prefixes = ("00_crates_de_combate", "00_sets_estrategicos", "curated_library", ".git", "__pycache__")
    seen_paths: set[str] = set()
    tracks: list[dict] = []

    for r_str in roots:
        root_path = Path(r_str)
        if not root_path.exists() or not root_path.is_dir():
            continue
        root_abs = os.path.abspath(str(root_path))

        for dirpath, dirs, filenames in os.walk(root_abs):
            dirs[:] = [d for d in dirs if not d.lower().startswith(skip_prefixes)]
            for fname in filenames:
                if fname == ".gitkeep":
                    continue
                dot = fname.rfind(".")
                if dot == -1:
                    continue
                ext = fname[dot:].lower()
                if ext not in valid_exts:
                    continue

                full_p = os.path.join(dirpath, fname)
                norm_p = os.path.normcase(full_p)
                if norm_p in seen_paths:
                    continue
                seen_paths.add(norm_p)
                if len(tracks) >= max_tracks:
                    break

                size_mb = 0.0

                stem = fname[:dot]
                bpm: float | None = None
                camelot: str | None = None

                match = BPM_CAMELOT_RE.search(stem)
                if match:
                    try:
                        bpm = float(match.group(1))
                    except ValueError:
                        bpm = None
                    camelot = match.group(2).upper()
                    clean_title = BPM_CAMELOT_RE.sub("", stem).strip(" -_[]")
                else:
                    clean_title = stem

                rel_dir = os.path.relpath(dirpath, root_abs)
                if rel_dir == ".":
                    genre_group = root_path.name
                    sub_folder = root_path.name
                else:
                    parts = rel_dir.split(os.sep)
                    genre_group = parts[0]
                    sub_folder = " / ".join(parts[1:]) if len(parts) > 1 else root_path.name

                tracks.append(
                    {
                        "title": clean_title or stem,
                        "raw_name": fname,
                        "path": full_p,
                        "genre": genre_group.replace("_", " "),
                        "subfolder": sub_folder.replace("_", " "),
                        "bpm": bpm,
                        "camelot": camelot,
                        "format": ext.lstrip(".").upper(),
                        "size_mb": size_mb,
                    }
                )
            if len(tracks) >= max_tracks:
                break

    tracks.sort(key=lambda x: (x["genre"].lower(), x["title"].lower()))
    return tracks


def _build_stream_url(track_path: str | None, port: int) -> str:
    if not track_path:
        return ""
    encoded = urllib.parse.quote(track_path, safe="")
    return f"http://127.0.0.1:{port}/stream?path={encoded}"


def render_dual_deck_console(deck_a: dict | None, deck_b: dict | None, port: int) -> None:
    """Renderiza la consola DJ Dual-Deck con Web Audio API, EQ de 3 bandas, Pitch, Crossfader y Espectro FFT."""
    url_a = _build_stream_url(deck_a["path"] if deck_a else None, port)
    url_b = _build_stream_url(deck_b["path"] if deck_b else None, port)

    title_a = (deck_a["title"] if deck_a else "Sin track cargado — Selecciona en la Biblioteca").replace('"', "&quot;")
    genre_a = (f"{deck_a['genre']} • {deck_a['subfolder']}" if deck_a else "Deck A Listo").replace('"', "&quot;")
    bpm_a = deck_a["bpm"] if (deck_a and deck_a.get("bpm")) else 125.0
    cam_a = (deck_a["camelot"] if (deck_a and deck_a.get("camelot")) else "--")
    fmt_a = (f"{deck_a['format']} • {deck_a['size_mb']} MB" if deck_a else "---")

    title_b = (deck_b["title"] if deck_b else "Sin track cargado — Selecciona en la Biblioteca").replace('"', "&quot;")
    genre_b = (f"{deck_b['genre']} • {deck_b['subfolder']}" if deck_b else "Deck B Listo").replace('"', "&quot;")
    bpm_b = deck_b["bpm"] if (deck_b and deck_b.get("bpm")) else 125.0
    cam_b = (deck_b["camelot"] if (deck_b and deck_b.get("camelot")) else "--")
    fmt_b = (f"{deck_b['format']} • {deck_b['size_mb']} MB" if deck_b else "---")

    html_code = f"""
    <!DOCTYPE html>
    <html>
    <head>
    <meta charset="utf-8">
    <style>
        * {{
            box-sizing: border-box;
            user-select: none;
            font-family: 'Inter', 'Segoe UI', system-ui, -apple-system, sans-serif;
        }}
        body {{
            margin: 0;
            padding: 4px;
            background: transparent;
            color: #e6f1ff;
        }}
        .console-wrap {{
            background: radial-gradient(circle at 50% 0%, #17223b 0%, #0b0f19 75%);
            border: 1px solid #233554;
            border-radius: 16px;
            padding: 16px;
            box-shadow: 0 12px 35px rgba(0, 0, 0, 0.65), inset 0 0 25px rgba(0, 242, 254, 0.04);
            display: grid;
            grid-template-columns: 1fr 230px 1fr;
            gap: 14px;
            align-items: stretch;
        }}
        .deck {{
            background: rgba(13, 18, 30, 0.88);
            border-radius: 14px;
            padding: 14px;
            position: relative;
            display: flex;
            flex-direction: column;
            justify-content: space-between;
        }}
        .deck-a {{
            border: 1px solid rgba(0, 242, 254, 0.35);
            box-shadow: 0 0 20px rgba(0, 242, 254, 0.08);
        }}
        .deck-b {{
            border: 1px solid rgba(255, 0, 127, 0.35);
            box-shadow: 0 0 20px rgba(255, 0, 127, 0.08);
        }}
        .deck-header {{
            display: flex;
            justify-content: space-between;
            align-items: flex-start;
            gap: 10px;
            margin-bottom: 10px;
        }}
        .deck-tag {{
            font-size: 0.72rem;
            font-weight: 800;
            letter-spacing: 1.5px;
            padding: 3px 9px;
            border-radius: 6px;
            text-transform: uppercase;
        }}
        .tag-a {{
            background: rgba(0, 242, 254, 0.15);
            color: #00f2fe;
            border: 1px solid #00f2fe;
        }}
        .tag-b {{
            background: rgba(255, 0, 127, 0.15);
            color: #ff4da6;
            border: 1px solid #ff007f;
        }}
        .track-info {{
            flex: 1;
            min-width: 0;
        }}
        .track-title {{
            font-size: 0.98rem;
            font-weight: 700;
            color: #ffffff;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            margin-bottom: 3px;
        }}
        .track-sub {{
            font-size: 0.75rem;
            color: #8892b0;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }}
        .badges {{
            display: flex;
            gap: 6px;
            align-items: center;
            flex-shrink: 0;
        }}
        .badge-bpm {{
            background: #141d30;
            border: 1px solid #2b3d5f;
            color: #64ffda;
            padding: 4px 8px;
            border-radius: 7px;
            font-size: 0.8rem;
            font-weight: 800;
            font-variant-numeric: tabular-nums;
        }}
        .badge-cam {{
            background: linear-gradient(135deg, #7928ca, #ff0080);
            color: #fff;
            padding: 4px 9px;
            border-radius: 7px;
            font-size: 0.8rem;
            font-weight: 800;
        }}
        /* Jog + Spectrum Row */
        .platter-row {{
            display: flex;
            align-items: center;
            gap: 14px;
            margin: 6px 0 10px 0;
        }}
        .jog-wheel {{
            width: 112px;
            height: 112px;
            border-radius: 50%;
            flex-shrink: 0;
            position: relative;
            display: flex;
            align-items: center;
            justify-content: center;
            background:
                radial-gradient(circle, #101626 26%, transparent 27%),
                repeating-radial-gradient(circle, #182036 0px, #0d121f 3px, #182036 6px);
            box-shadow: 0 6px 18px rgba(0,0,0,0.6);
            transition: box-shadow 0.2s ease;
        }}
        .jog-a {{
            border: 3px solid #00f2fe;
            box-shadow: 0 0 16px rgba(0, 242, 254, 0.35);
        }}
        .jog-b {{
            border: 3px solid #ff007f;
            box-shadow: 0 0 16px rgba(255, 0, 127, 0.35);
        }}
        .jog-marker {{
            position: absolute;
            top: 7px;
            left: 50%;
            transform: translateX(-50%);
            width: 6px;
            height: 14px;
            border-radius: 3px;
        }}
        .marker-a {{ background: #00f2fe; box-shadow: 0 0 8px #00f2fe; }}
        .marker-b {{ background: #ff007f; box-shadow: 0 0 8px #ff007f; }}
        .jog-center {{
            width: 44px;
            height: 44px;
            border-radius: 50%;
            background: #0b0e17;
            border: 2px solid #2b3a55;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            font-size: 0.65rem;
            font-weight: 800;
            color: #ccd6f6;
            z-index: 2;
        }}
        @keyframes spinDisk {{
            from {{ transform: rotate(0deg); }}
            to {{ transform: rotate(360deg); }}
        }}
        .spinning {{
            animation: spinDisk 2.2s linear infinite;
        }}
        .viz-box {{
            flex: 1;
            display: flex;
            flex-direction: column;
            gap: 6px;
        }}
        canvas.spec-canvas {{
            width: 100%;
            height: 76px;
            background: #080b12;
            border: 1px solid #1d2840;
            border-radius: 8px;
        }}
        .time-row {{
            display: flex;
            justify-content: space-between;
            font-size: 0.74rem;
            color: #8892b0;
            font-variant-numeric: tabular-nums;
        }}
        /* Scrub bar */
        .seek-slider {{
            width: 100%;
            accent-color: #00f2fe;
            cursor: pointer;
            height: 6px;
            margin: 4px 0 10px 0;
        }}
        .seek-b {{
            accent-color: #ff007f;
        }}
        /* Controls & Pitch */
        .controls-row {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 8px;
            margin-bottom: 10px;
        }}
        .btn-group {{
            display: flex;
            gap: 6px;
        }}
        .dj-btn {{
            background: #172238;
            color: #e6f1ff;
            border: 1px solid #293c61;
            border-radius: 8px;
            padding: 7px 13px;
            font-size: 0.78rem;
            font-weight: 700;
            cursor: pointer;
            transition: all 0.15s ease;
        }}
        .dj-btn:hover {{
            border-color: #64ffda;
            color: #64ffda;
        }}
        .btn-play-a.active {{
            background: linear-gradient(135deg, #00f2fe, #0088ff);
            color: #050811;
            border-color: #00f2fe;
            box-shadow: 0 0 14px rgba(0, 242, 254, 0.5);
        }}
        .btn-play-b.active {{
            background: linear-gradient(135deg, #ff007f, #9900ff);
            color: #ffffff;
            border-color: #ff007f;
            box-shadow: 0 0 14px rgba(255, 0, 127, 0.5);
        }}
        .pitch-box {{
            display: flex;
            align-items: center;
            gap: 6px;
            background: #0d1322;
            padding: 4px 8px;
            border-radius: 8px;
            border: 1px solid #1d2942;
            font-size: 0.72rem;
        }}
        .pitch-box input[type=range] {{
            width: 85px;
            cursor: pointer;
        }}
        /* 3-Band EQ Row */
        .eq-row {{
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 8px;
            background: #0a0e18;
            padding: 8px 10px;
            border-radius: 10px;
            border: 1px solid #182238;
        }}
        .eq-band {{
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 3px;
        }}
        .eq-band label {{
            font-size: 0.68rem;
            font-weight: 700;
            color: #8892b0;
            letter-spacing: 0.8px;
        }}
        .eq-band input[type=range] {{
            width: 100%;
            cursor: pointer;
        }}
        /* Center Mixer */
        .mixer-col {{
            background: #0c111d;
            border: 1px solid #22304d;
            border-radius: 14px;
            padding: 12px;
            display: flex;
            flex-direction: column;
            justify-content: space-between;
            align-items: center;
        }}
        .mixer-title {{
            font-size: 0.78rem;
            font-weight: 800;
            letter-spacing: 2px;
            color: #64ffda;
            text-transform: uppercase;
            margin-bottom: 4px;
        }}
        .faders-wrap {{
            display: flex;
            justify-content: space-around;
            align-items: center;
            width: 100%;
            margin: 6px 0;
            gap: 10px;
        }}
        .ch-fader {{
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 4px;
            background: #111829;
            padding: 8px 10px;
            border-radius: 10px;
            border: 1px solid #1e2b45;
            flex: 1;
        }}
        .ch-fader span {{
            font-size: 0.7rem;
            font-weight: 700;
            color: #ccd6f6;
        }}
        .ch-fader input[type=range] {{
            width: 100%;
            cursor: pointer;
        }}
        .xfader-box {{
            width: 100%;
            background: #111829;
            border: 1px solid #243454;
            border-radius: 10px;
            padding: 10px;
            text-align: center;
        }}
        .xfader-labels {{
            display: flex;
            justify-content: space-between;
            font-size: 0.7rem;
            font-weight: 800;
            margin-bottom: 4px;
        }}
        .xfader-box input[type=range] {{
            width: 100%;
            cursor: pointer;
            accent-color: #64ffda;
        }}
        .sync-btn {{
            width: 100%;
            background: linear-gradient(90deg, rgba(0,242,254,0.18), rgba(255,0,127,0.18));
            border: 1px solid #64ffda;
            color: #64ffda;
            font-weight: 800;
            font-size: 0.74rem;
            padding: 7px;
            border-radius: 8px;
            cursor: pointer;
            letter-spacing: 1px;
        }}
        .sync-btn:hover {{
            background: #64ffda;
            color: #080c14;
        }}
    </style>
    </head>
    <body>
    <div class="console-wrap">
        <!-- DECK A -->
        <div class="deck deck-a">
            <div>
                <div class="deck-header">
                    <span class="deck-tag tag-a">DECK A</span>
                    <div class="track-info">
                        <div class="track-title" title="{title_a}">{title_a}</div>
                        <div class="track-sub">{genre_a} • {fmt_a}</div>
                    </div>
                    <div class="badges">
                        <span class="badge-bpm" id="bpmDisplayA">{bpm_a:.1f} BPM</span>
                        <span class="badge-cam">{cam_a}</span>
                    </div>
                </div>

                <div class="platter-row">
                    <div class="jog-wheel jog-a" id="jogA">
                        <div class="jog-marker marker-a"></div>
                        <div class="jog-center">
                            <span>CDJ-A</span>
                            <span id="rateLabelA" style="color:#00f2fe;">0.0%</span>
                        </div>
                    </div>
                    <div class="viz-box">
                        <canvas id="canvasA" class="spec-canvas" width="320" height="76"></canvas>
                        <div class="time-row">
                            <span id="timeElapsedA">00:00</span>
                            <span id="timeRemainA">-00:00</span>
                        </div>
                    </div>
                </div>

                <input type="range" id="seekA" class="seek-slider" min="0" max="1000" value="0">
            </div>

            <div>
                <div class="controls-row">
                    <div class="btn-group">
                        <button class="dj-btn btn-play-a" id="playBtnA" onclick="togglePlay('A')">▶ PLAY</button>
                        <button class="dj-btn" onclick="cueTrack('A')">⏮ CUE</button>
                    </div>
                    <div class="pitch-box">
                        <span>TEMPO</span>
                        <input type="range" id="pitchA" min="0.84" max="1.16" step="0.002" value="1.0" oninput="updatePitch('A', this.value)">
                        <button class="dj-btn" style="padding:3px 6px;font-size:0.65rem;" onclick="resetPitch('A')">0%</button>
                    </div>
                </div>

                <div class="eq-row">
                    <div class="eq-band">
                        <label>LOW <span id="lowValA">0dB</span></label>
                        <input type="range" min="-24" max="12" step="0.5" value="0" oninput="setEQ('A', 'low', this.value)">
                    </div>
                    <div class="eq-band">
                        <label>MID <span id="midValA">0dB</span></label>
                        <input type="range" min="-24" max="12" step="0.5" value="0" oninput="setEQ('A', 'mid', this.value)">
                    </div>
                    <div class="eq-band">
                        <label>HIGH <span id="highValA">0dB</span></label>
                        <input type="range" min="-24" max="12" step="0.5" value="0" oninput="setEQ('A', 'high', this.value)">
                    </div>
                </div>
            </div>
            <audio id="audioA" crossorigin="anonymous" src="{url_a}" preload="metadata"></audio>
        </div>

        <!-- CENTER MIXER -->
        <div class="mixer-col">
            <div style="text-align:center;width:100%;">
                <div class="mixer-title">🎛️ HERA MIXER</div>
                <div style="font-size:0.68rem;color:#8892b0;">DSP 3-Band Isolator</div>
            </div>

            <button class="sync-btn" onclick="syncDecks()">⚡ BEAT SYNC (B ➔ A)</button>

            <div class="faders-wrap">
                <div class="ch-fader">
                    <span style="color:#00f2fe;">VOL A</span>
                    <input type="range" id="volA" min="0" max="1" step="0.01" value="0.9" oninput="updateVolumes()">
                    <span id="volLabelA">90%</span>
                </div>
                <div class="ch-fader">
                    <span style="color:#ff4da6;">VOL B</span>
                    <input type="range" id="volB" min="0" max="1" step="0.01" value="0.9" oninput="updateVolumes()">
                    <span id="volLabelB">90%</span>
                </div>
            </div>

            <div class="xfader-box">
                <div class="xfader-labels">
                    <span style="color:#00f2fe;">◀ DECK A</span>
                    <span style="color:#ccd6f6;" id="xfaderCenter" ondblclick="resetXfader()">CROSSFADER</span>
                    <span style="color:#ff4da6;">DECK B ▶</span>
                </div>
                <input type="range" id="xfader" min="-1" max="1" step="0.01" value="0" oninput="updateVolumes()">
                <div style="margin-top:4px;">
                    <button class="dj-btn" style="padding:3px 8px;font-size:0.65rem;" onclick="resetXfader()">CENTRAR A/B</button>
                </div>
            </div>
        </div>

        <!-- DECK B -->
        <div class="deck deck-b">
            <div>
                <div class="deck-header">
                    <span class="deck-tag tag-b">DECK B</span>
                    <div class="track-info">
                        <div class="track-title" title="{title_b}">{title_b}</div>
                        <div class="track-sub">{genre_b} • {fmt_b}</div>
                    </div>
                    <div class="badges">
                        <span class="badge-bpm" id="bpmDisplayB">{bpm_b:.1f} BPM</span>
                        <span class="badge-cam">{cam_b}</span>
                    </div>
                </div>

                <div class="platter-row">
                    <div class="jog-wheel jog-b" id="jogB">
                        <div class="jog-marker marker-b"></div>
                        <div class="jog-center">
                            <span>CDJ-B</span>
                            <span id="rateLabelB" style="color:#ff4da6;">0.0%</span>
                        </div>
                    </div>
                    <div class="viz-box">
                        <canvas id="canvasB" class="spec-canvas" width="320" height="76"></canvas>
                        <div class="time-row">
                            <span id="timeElapsedB">00:00</span>
                            <span id="timeRemainB">-00:00</span>
                        </div>
                    </div>
                </div>

                <input type="range" id="seekB" class="seek-slider seek-b" min="0" max="1000" value="0">
            </div>

            <div>
                <div class="controls-row">
                    <div class="btn-group">
                        <button class="dj-btn btn-play-b" id="playBtnB" onclick="togglePlay('B')">▶ PLAY</button>
                        <button class="dj-btn" onclick="cueTrack('B')">⏮ CUE</button>
                    </div>
                    <div class="pitch-box">
                        <span>TEMPO</span>
                        <input type="range" id="pitchB" min="0.84" max="1.16" step="0.002" value="1.0" oninput="updatePitch('B', this.value)">
                        <button class="dj-btn" style="padding:3px 6px;font-size:0.65rem;" onclick="resetPitch('B')">0%</button>
                    </div>
                </div>

                <div class="eq-row">
                    <div class="eq-band">
                        <label>LOW <span id="lowValB">0dB</span></label>
                        <input type="range" min="-24" max="12" step="0.5" value="0" oninput="setEQ('B', 'low', this.value)">
                    </div>
                    <div class="eq-band">
                        <label>MID <span id="midValB">0dB</span></label>
                        <input type="range" min="-24" max="12" step="0.5" value="0" oninput="setEQ('B', 'mid', this.value)">
                    </div>
                    <div class="eq-band">
                        <label>HIGH <span id="highValB">0dB</span></label>
                        <input type="range" min="-24" max="12" step="0.5" value="0" oninput="setEQ('B', 'high', this.value)">
                    </div>
                </div>
            </div>
            <audio id="audioB" crossorigin="anonymous" src="{url_b}" preload="metadata"></audio>
        </div>
    </div>

    <script>
        const baseBpmA = {bpm_a:.2f};
        const baseBpmB = {bpm_b:.2f};
        let audioCtx = null;

        const decks = {{
            A: {{
                audio: document.getElementById('audioA'),
                playBtn: document.getElementById('playBtnA'),
                jog: document.getElementById('jogA'),
                seek: document.getElementById('seekA'),
                elapsed: document.getElementById('timeElapsedA'),
                remain: document.getElementById('timeRemainA'),
                canvas: document.getElementById('canvasA'),
                bpmDisplay: document.getElementById('bpmDisplayA'),
                rateLabel: document.getElementById('rateLabelA'),
                pitchSlider: document.getElementById('pitchA'),
                baseBpm: baseBpmA,
                color: '#00f2fe',
                source: null, low: null, mid: null, high: null, gain: null, analyser: null
            }},
            B: {{
                audio: document.getElementById('audioB'),
                playBtn: document.getElementById('playBtnB'),
                jog: document.getElementById('jogB'),
                seek: document.getElementById('seekB'),
                elapsed: document.getElementById('timeElapsedB'),
                remain: document.getElementById('timeRemainB'),
                canvas: document.getElementById('canvasB'),
                bpmDisplay: document.getElementById('bpmDisplayB'),
                rateLabel: document.getElementById('rateLabelB'),
                pitchSlider: document.getElementById('pitchB'),
                baseBpm: baseBpmB,
                color: '#ff007f',
                source: null, low: null, mid: null, high: null, gain: null, analyser: null
            }}
        }};

        function initAudioGraph() {{
            if (!audioCtx) {{
                const AudioContext = window.AudioContext || window.webkitAudioContext;
                audioCtx = new AudioContext();
            }}
            if (audioCtx.state === 'suspended') {{
                audioCtx.resume();
            }}
            ['A', 'B'].forEach(id => {{
                const d = decks[id];
                if (!d.source && d.audio.getAttribute('src')) {{
                    try {{
                        d.source = audioCtx.createMediaElementSource(d.audio);
                        d.low = audioCtx.createBiquadFilter();
                        d.low.type = 'lowshelf';
                        d.low.frequency.value = 320;

                        d.mid = audioCtx.createBiquadFilter();
                        d.mid.type = 'peaking';
                        d.mid.frequency.value = 1000;
                        d.mid.Q.value = 0.8;

                        d.high = audioCtx.createBiquadFilter();
                        d.high.type = 'highshelf';
                        d.high.frequency.value = 3200;

                        d.gain = audioCtx.createGain();
                        d.analyser = audioCtx.createAnalyser();
                        d.analyser.fftSize = 128;

                        d.source.connect(d.low);
                        d.low.connect(d.mid);
                        d.mid.connect(d.high);
                        d.high.connect(d.gain);
                        d.gain.connect(d.analyser);
                        d.analyser.connect(audioCtx.destination);
                    }} catch (e) {{
                        console.warn('Audio graph init note:', e);
                    }}
                }}
            }});
            updateVolumes();
        }}

        function fmtTime(sec) {{
            if (!sec || isNaN(sec) || sec < 0) return '00:00';
            const m = Math.floor(sec / 60);
            const s = Math.floor(sec % 60);
            return String(m).padStart(2, '0') + ':' + String(s).padStart(2, '0');
        }}

        function togglePlay(id) {{
            initAudioGraph();
            const d = decks[id];
            if (!d.audio.getAttribute('src')) return;
            if (d.audio.paused) {{
                d.audio.play();
                d.playBtn.textContent = '⏸ PAUSE';
                d.playBtn.classList.add('active');
                d.jog.classList.add('spinning');
            }} else {{
                d.audio.pause();
                d.playBtn.textContent = '▶ PLAY';
                d.playBtn.classList.remove('active');
                d.jog.classList.remove('spinning');
            }}
        }}

        function cueTrack(id) {{
            const d = decks[id];
            d.audio.pause();
            d.audio.currentTime = 0;
            d.playBtn.textContent = '▶ PLAY';
            d.playBtn.classList.remove('active');
            d.jog.classList.remove('spinning');
        }}

        function updatePitch(id, val) {{
            const d = decks[id];
            const rate = parseFloat(val);
            d.audio.preservesPitch = false;
            d.audio.playbackRate = rate;
            const pct = ((rate - 1.0) * 100).toFixed(1);
            d.rateLabel.textContent = (pct >= 0 ? '+' : '') + pct + '%';
            d.bpmDisplay.textContent = (d.baseBpm * rate).toFixed(1) + ' BPM';
        }}

        function resetPitch(id) {{
            const d = decks[id];
            d.pitchSlider.value = 1.0;
            updatePitch(id, 1.0);
        }}

        function syncDecks() {{
            const targetBpm = decks.A.baseBpm * decks.A.audio.playbackRate;
            if (decks.B.baseBpm > 0) {{
                const ratio = Math.max(0.84, Math.min(1.16, targetBpm / decks.B.baseBpm));
                decks.B.pitchSlider.value = ratio;
                updatePitch('B', ratio);
            }}
        }}

        function setEQ(id, band, val) {{
            initAudioGraph();
            const d = decks[id];
            const db = parseFloat(val);
            if (d[band]) {{
                d[band].gain.value = db;
            }}
            document.getElementById(band + 'Val' + id).textContent = (db > 0 ? '+' : '') + db + 'dB';
        }}

        function updateVolumes() {{
            const vA = parseFloat(document.getElementById('volA').value);
            const vB = parseFloat(document.getElementById('volB').value);
            const xf = parseFloat(document.getElementById('xfader').value);

            document.getElementById('volLabelA').textContent = Math.round(vA * 100) + '%';
            document.getElementById('volLabelB').textContent = Math.round(vB * 100) + '%';

            const weightA = xf <= 0 ? 1.0 : (1.0 - xf);
            const weightB = xf >= 0 ? 1.0 : (1.0 + xf);

            const gainA = vA * weightA;
            const gainB = vB * weightB;

            if (decks.A.gain) decks.A.gain.gain.value = gainA;
            else decks.A.audio.volume = gainA;

            if (decks.B.gain) decks.B.gain.gain.value = gainB;
            else decks.B.audio.volume = gainB;
        }}

        function resetXfader() {{
            document.getElementById('xfader').value = 0;
            updateVolumes();
        }}

        ['A', 'B'].forEach(id => {{
            const d = decks[id];
            d.audio.addEventListener('timeupdate', () => {{
                const dur = d.audio.duration || 0;
                const cur = d.audio.currentTime || 0;
                if (dur > 0) {{
                    d.seek.value = Math.floor((cur / dur) * 1000);
                    d.elapsed.textContent = fmtTime(cur);
                    d.remain.textContent = '-' + fmtTime(Math.max(0, dur - cur));
                }}
            }});
            d.seek.addEventListener('input', () => {{
                const dur = d.audio.duration || 0;
                if (dur > 0) {{
                    d.audio.currentTime = (d.seek.value / 1000) * dur;
                }}
            }});
            d.audio.addEventListener('ended', () => {{
                d.playBtn.textContent = '▶ PLAY';
                d.playBtn.classList.remove('active');
                d.jog.classList.remove('spinning');
            }});
        }});

        function drawSpectrum() {{
            requestAnimationFrame(drawSpectrum);
            ['A', 'B'].forEach(id => {{
                const d = decks[id];
                const ctx = d.canvas.getContext('2d');
                const w = d.canvas.width;
                const h = d.canvas.height;
                ctx.clearRect(0, 0, w, h);

                // Grid lines
                ctx.strokeStyle = 'rgba(255,255,255,0.04)';
                ctx.beginPath();
                ctx.moveTo(0, h / 2);
                ctx.lineTo(w, h / 2);
                ctx.stroke();

                if (!d.analyser) return;
                const bufferLength = d.analyser.frequencyBinCount;
                const dataArray = new Uint8Array(bufferLength);
                d.analyser.getByteFrequencyData(dataArray);

                const barWidth = (w / bufferLength) * 1.45;
                let x = 0;
                for (let i = 0; i < bufferLength; i++) {{
                    const v = dataArray[i] / 255.0;
                    const barHeight = Math.max(2, v * (h - 6));
                    const grad = ctx.createLinearGradient(0, h, 0, 0);
                    if (id === 'A') {{
                        grad.addColorStop(0, '#0055ff');
                        grad.addColorStop(1, '#00f2fe');
                    }} else {{
                        grad.addColorStop(0, '#7928ca');
                        grad.addColorStop(1, '#ff007f');
                    }}
                    ctx.fillStyle = grad;
                    ctx.fillRect(x, h - barHeight, barWidth - 2, barHeight);
                    x += barWidth;
                }}
            }});
        }}
        drawSpectrum();
    </script>
    </body>
    </html>
    """
    components.html(html_code, height=395, scrolling=False)


def render_dj_studio_tab(cfg: HeraConfig) -> None:
    """Renderiza la pestaña completa del Reproductor Dual-Deck y Explorador de Biblioteca."""
    port = ensure_audio_stream_server(8502)

    roots = [
        str(Path(cfg.library_dir).resolve()),
        str((Path(cfg.data_dir) / "sets").resolve()),
    ]
    for folder in cfg.providers.local_folders:
        p = Path(folder)
        if not p.is_absolute():
            p = (Path(cfg.data_dir) / p).resolve()
        if p.exists():
            roots.append(str(p))

    tracks = scan_dj_library(tuple(roots))

    if not tracks:
        st.info("📦 No se encontraron pistas de audio en la biblioteca ni en las carpetas locales configuradas.")
        return

    # Inicializar Deck A y Deck B por defecto con los primeros 2 tracks
    if "deck_a_track" not in st.session_state or st.session_state.deck_a_track is None:
        st.session_state.deck_a_track = tracks[0]
    if "deck_b_track" not in st.session_state or st.session_state.deck_b_track is None:
        st.session_state.deck_b_track = tracks[1] if len(tracks) > 1 else tracks[0]

    # 1. Renderizar la Consola Dual-Deck arriba
    render_dual_deck_console(st.session_state.deck_a_track, st.session_state.deck_b_track, port)

    # 2. Filtros de Exploración + Asistente Armónico Camelot
    st.markdown("#### 🗂️ Explorador de Repertorio & Asistente Armónico Camelot")

    genres = ["Todos los Géneros"] + sorted({t["genre"] for t in tracks})
    camelot_keys = ["Todas las Claves"] + sorted({t["camelot"] for t in tracks if t["camelot"]})

    col_search, col_genre, col_cam, col_harm = st.columns([2.2, 1.5, 1.1, 1.4])
    with col_search:
        search_q = st.text_input("🔍 Buscar por artista, canción o carpeta:", placeholder="Ej. Afro House, Aguanile, Feid, 126...")
    with col_genre:
        sel_genre = st.selectbox("📁 Género / Categoría:", options=genres)
    with col_cam:
        sel_cam = st.selectbox("🎼 Clave Camelot:", options=camelot_keys)
    with col_harm:
        st.write("")
        st.write("")
        harmonic_match_only = st.checkbox("🎯 Armónicos con Deck A", value=False, help="Filtra canciones compatibles en Camelot y BPM con lo que suena en el Deck A")

    filtered = tracks
    if sel_genre != "Todos los Géneros":
        filtered = [t for t in filtered if t["genre"] == sel_genre]
    if sel_cam != "Todas las Claves":
        filtered = [t for t in filtered if t["camelot"] == sel_cam]
    if search_q.strip():
        q_tokens = search_q.lower().split()
        filtered = [
            t for t in filtered
            if all(tok in f"{t['title']} {t['genre']} {t['subfolder']} {t['bpm'] or ''} {t['camelot'] or ''}".lower() for tok in q_tokens)
        ]

    if harmonic_match_only and st.session_state.deck_a_track:
        da = st.session_state.deck_a_track
        compat_keys = set(get_compatible_camelot_keys(da.get("camelot")))
        da_bpm = da.get("bpm")
        if compat_keys:
            filtered = [
                t for t in filtered
                if t.get("camelot") in compat_keys
                and (not da_bpm or not t.get("bpm") or abs(t["bpm"] - da_bpm) <= 8.0)
                and t["path"] != da["path"]
            ]

    st.caption(f"Mostrando **{min(len(filtered), 25)}** de **{len(filtered):,}** pistas encontradas (Total en repertorio: **{len(tracks):,}** tracks)")

    # 3. Lista interactiva de pistas con carga en 1 clic a Deck A / Deck B
    for idx, trk in enumerate(filtered[:25]):
        c_info, c_meta, c_btn_a, c_btn_b = st.columns([4.2, 1.8, 1.0, 1.0])
        with c_info:
            st.markdown(
                f"**{trk['title']}**  \n"
                f"<span style='color:#8892b0;font-size:0.8rem;'>📁 {trk['genre']} › {trk['subfolder']}</span>",
                unsafe_allow_html=True,
            )
        with c_meta:
            bpm_str = f"{trk['bpm']:.0f} BPM" if trk.get("bpm") else "-- BPM"
            cam_str = trk.get("camelot") or "--"
            st.markdown(
                f"<span style='background:#141d30;color:#64ffda;padding:3px 7px;border-radius:6px;font-size:0.78rem;font-weight:700;'>{bpm_str}</span> "
                f"<span style='background:#3b1d5e;color:#ff80bf;padding:3px 7px;border-radius:6px;font-size:0.78rem;font-weight:700;'>{cam_str}</span> "
                f"<span style='color:#8892b0;font-size:0.75rem;'>{trk['format']}</span>",
                unsafe_allow_html=True,
            )
        with c_btn_a:
            if st.button("🟦 Deck A", key=f"load_a_{idx}_{trk['path'][-18:]}", use_container_width=True):
                st.session_state.deck_a_track = trk
                st.rerun()
        with c_btn_b:
            if st.button("🟪 Deck B", key=f"load_b_{idx}_{trk['path'][-18:]}", use_container_width=True):
                st.session_state.deck_b_track = trk
                st.rerun()
