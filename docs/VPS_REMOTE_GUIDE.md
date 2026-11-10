# ☁️ Guía: HERA (`hera-node-rs v0.6.0`) en VPS Remoto y Cloudflare Edge + Copiloto Local

Esta guía explica cómo desplegar el motor nativo en Rust **`hera-node-rs v0.6.0` (`~10 MB RAM`, cero Python)** en un **Micro-VPS 24/7** o en **Cloudflare Workers (`$0.00/mes`)** y conectarlo a **HERA Desktop** o a tu terminal local (`Antigravity`, `Claude Desktop`, `Cursor`, `Codex`) mediante el protocolo **MCP**.

---

## 💡 1. La Arquitectura `v0.6.0`: Estudio Local + Nodo de Enjambre 24/7

![Concepto y Arquitectura del Enjambre P2P Simbiótico de HERA v0.6.0](../assets/hera_swarm_concept.jpg)

```text
┌──────────────────────────────────────────────────────────────────────┐
│  TU LAPTOP DE CABINA / PC LOCAL (Windows, macOS o Linux)             │
│                                                                      │
│  🎛️ HERA Desktop v0.6.0 (Tauri v2 Dark Booth Studio · ~8 MB)         │
│     ├── Biblioteca + 4 Hot Cues + DualDeckEngine (#![forbid(unsafe)])│
│     ├── Modo Mini Always-on-Top (360x408 / 560x52) para VirtualDJ    │
│     └── Espacio /agent (Ctrl+J · 44 Tools MCP Loopback Bearer)       │
└──────────────────────────────────┬───────────────────────────────────┘
                                   │
               Conexión MCP (stdio SSH / HTTP POST /mcp)
               + Cascada P2P Zero-Config de 5 Etapas
                                   │
                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│  NODO REMOTO 24/7 (Micro-VPS ~10 MB RAM o Cloudflare Worker Free)    │
│                                                                      │
│  🦀 hera-node-rs v0.6.0 (100% Pure Rust · Axum + Symphonia + rustfft)│
│     ├── Auditoría Forense Zero-Trust (< 80 ms, 8 codecs en streaming)│
│     ├── Optimizador Global 2-Opt TSP + Exportadores VDJ/RB/Traktor   │
│     ├── 21 Herramientas MCP (stdio, HTTP POST /mcp y Code Mode)      │
│     └── Edge Guardian P2P Swarm (Water-Filling 9 GB / 85k req cap,   │
│         bloques Merkle BEP 52 de 4 MB y Créditos Turbo 1 : 1.5)      │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 🚀 2. Paso a Paso: Cómo Ponerlo en Marcha

### Opción A: Nodo `Edge Guardian` en Cloudflare Workers + R2 (`$0.00/mes`)

Compila `hera-node-rs` a `wasm32-unknown-unknown` y despliégalo en tu cuenta gratuita de Cloudflare:

```bash
cd hera-rs
cargo check --target wasm32-unknown-unknown --features cloudflare --no-default-features
npx wrangler deploy
```

- **Protección Matemática de Costo `$0.00`:** El algoritmo **Constrained Water-Filling** limita automáticamente tu Worker a `9,000 MB` de R2 (sobre `10 GB` gratis) y `85,000 req/día` (sobre `100,000 req/día` gratis).
- **Créditos Turbo `1 : 1.5`:** Mientras tu nodo supera los desafíos `Proof-of-Custody` de 64 bytes, ganas `1.5 MB` de prioridad de descarga por cada `1.0 MB` servido al enjambre.

---

### Opción B: Super-Nodo 24/7 en un Micro-VPS Linux (`~10 MB RAM`)

En cualquier VPS Linux (Ubuntu / Debian en Hetzner, DigitalOcean, Vultr o Railway), instala el binario estático en Rust con el instalador de 1 línea:

```bash
curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/deploy/install-node.sh | bash
```

O compílalo directamente desde el workspace Cargo:

```bash
git clone https://github.com/davidcastilloc/hera.git
cd hera
cargo build --release -p hera-node-rs

# Iniciar servidor HTTP Axum + Enjambre P2P en el puerto 8080
./target/release/hera-node --port 8080
```

Verifica el estado del nodo y las métricas del enjambre:

```bash
curl -s http://127.0.0.1:8080/health | jq .
curl -s http://127.0.0.1:8080/api/v1/swarm/strength | jq .
```

---

## 🔌 3. Conectar tu Agente Local (`agy`, Claude, Cursor) al VPS por MCP

Como `hera-node` incluye un servidor **MCP JSON-RPC 2.0 nativo sobre `stdio`** (`--mcp`), puedes conectarte desde tu laptop al VPS mediante SSH sin exponer puertos adicionales:

```json
{
  "mcpServers": {
    "hera-remote": {
      "command": "ssh",
      "args": [
        "dj@IP_DE_TU_VPS",
        "/usr/local/bin/hera-node",
        "--mcp"
      ]
    }
  }
}
```

También puedes consumir el endpoint MCP HTTP o **Code Mode** directamente:
- `GET  /mcp/tools` — Catálogo completo de las 21 herramientas MCP.
- `POST /mcp` — Ejecución JSON-RPC 2.0 estándar.
- `POST /mcp/code-mode` — Ejecución multi-paso determinista en un solo turno (`analyze -> 2-Opt TSP -> export`).

---

## 🎧 4. Flujo Completo desde Cabina o Terminal

1. **Pides el set o puente armónico:** Desde la pestaña **`Agente` (`Ctrl+J`)** de HERA Desktop o desde tu terminal con `agy`:
   > *«Ármame un set de 90 minutos de Afro & Tech House entre 124 y 126 BPM empezando en 8A, verifica los masters con Symphonia y expórtalo para VirtualDJ y Rekordbox XML.»*
2. **Auditoría y Curaduría en Rust (`< 80 ms` por pista):**
   - `hera-node-rs` verifica cada pista en cuarentena (`Symphonia + rustfft`), clava los **4 Hot Cues** (`Intro`, `Breakdown`, `Drop`, `Outro`) y sella el activo (`hera_seal` + `audio_sha256`).
   - El motor `synergy.rs` ejecuta **`2-Opt TSP`** sobre las 4 capas acústicas + `SimHash u64`.
3. **Sincronización a tu Cabina:**
   - Descarga los bloques verificados de `4 MB` mediante `Swarm Striping` (`POST /api/v1/swarm/parallel-manifest`) o cárgala al instante en **VirtualDJ** usando el botón **`+ VDJ`** del **`Modo Mini` (`Ctrl+Shift+M`)**.
