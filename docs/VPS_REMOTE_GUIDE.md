# ☁️ Guía: HERA en VPS Remoto + Curadora AI en tu Terminal Local

Esta guía explica en lenguaje claro y sin complicaciones cómo funciona la arquitectura **Cerebro (Local) + Músculo (VPS 24/7)** y cómo configurarla paso a paso para que cualquier DJ, productor o amante de la música pueda tener su propio super-nodo y asistente virtual.

---

## 💡 1. La Idea Central (En pocas palabras)

Míralo como un restaurante:
* **Tú (El DJ):** Pides lo que quieres desde tu laptop o terminal (*Ármame un set de 90 min de French Touch a 124 BPM*).
* **Tu Asistente Virtual / Antigravity (El Mesero):** Entiende de música, reglas armónicas (Camelot Wheel) y toma tu comanda.
* **El Servidor VPS en la Nube (La Cocina Industrial):** Una máquina encendida 24/7 con internet ultrarrápido (1 Gbps) que busca en P2P, descarga a cuarentena, analiza con FFmpeg (BPM/Key/LUFS), organiza los archivos y crea tus playlists sin saturar tu PC local.

`	ext
┌────────────────────────────────────────────────────────┐
│  TU LAPTOP / PC LOCAL (Windows, Mac o Linux)           │
│                                                        │
│  Tú escribes en la terminal:                           │
│  Curadora, ármame un set de Deep House a 122 BPM     │
│                                                        │
│  🧠 Agente / CLI (Antigravity 'agy', Claude, Cursor)   │
│     └── Skill 'curadora-hera' (Cerebro Musical)        │
└───────────────────────────┬────────────────────────────┘
                            │
              (Conexión remota segura vía MCP)
              (Túnel SSH o endpoint privado)
                            │
                            ▼
┌────────────────────────────────────────────────────────┐
│  SERVIDOR VPS EN LA NUBE (Ubuntu / Debian / Linux)     │
│                                                        │
│  💪 HERA Super-Node (El Músculo)                       │
│     ├── Descarga en segundo plano por P2P (slskd)      │
│     ├── Cuarentena e integridad acústica (FFmpeg)      │
│     ├── Análisis acústico DSP (BPM, Camelot, LUFS)     │
│     ├── Organización limpia con tags ID3 / FLAC        │
│     └── Sincronización a Google Drive / USB de DJ      │
└────────────────────────────────────────────────────────┘
`

---

## 🚀 2. Paso a Paso: Cómo ponerlo en marcha

### Paso 1: Levantar el Servidor en tu VPS (El Músculo)
En tu servidor remoto (DigitalOcean, Hetzner, AWS, Linode o cualquier Linux), ejecuta el instalador de 1-clic:

`ash
curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/install.sh | bash
`

Esto dejará listo:
* El servicio P2P (slskd) corriendo en segundo plano con auto-arranque.
* Las carpetas /library, /sets, /quarantine y /exports.
* Los motores de audio (fmpeg, pcalc, librosa).

Para comprobar que todo está en orden en el VPS:
`ash
hera status
`

---

### Paso 2: Conectar tu CLI Local al VPS (El Cable)
En tu computadora personal (donde tienes instalado gy / Antigravity, Claude Desktop o Cursor), agrega la configuración del servidor MCP de HERA para que se conecte a tu VPS mediante SSH.

Edita tu archivo de configuración de MCP (ejemplo: ~/.gemini/antigravity/mcp_config.json o la configuración de Claude Desktop):

`json
{
  mcpServers: {
    hera: {
      command: ssh,
      args: [
        usuario@IP_DE_TU_VPS,
        /home/usuario/hera/.venv/bin/hera,
        serve
      ]
    }
  }
}
`

> *(Opcional para pruebas locales en Windows con WSL: puedes cambiar ssh por wsl y args: [-d, Ubuntu, -e, /home/usuario/hera/.venv/bin/hera, serve]).*

---

### Paso 3: Instalar la Curadora Virtual (El Cerebro)
Asegúrate de tener el skill de la curadora en tu carpeta global de skills:

Ubicación: ~/.gemini/config/skills/curadora-hera/SKILL.md

Este skill le enseña a tu agente:
1. Reglas estrictas de calidad (FLAC Lossless > MP3 320k > Cero audio falso).
2. Teoría armónica de la Rueda Camelot (progresiones de tonalidad y tempo).
3. Cómo ejecutar búsquedas, descargas a cuarentena y exportación de crates de DJ de forma autónoma.

---

### Paso 4: ¡Dar órdenes en lenguaje natural!
Abre tu terminal en cualquier carpeta y empieza a hablarle a tu Curadora:

`	ext
> Curadora, ármame un set de 90 minutos de French Touch a 124 BPM, prioridad FLAC. Autorización: purchased_copy
`

**Lo que pasará automáticamente:**
1. Tu agente local envía las instrucciones al VPS.
2. El VPS busca las mejores versiones de estudio en Soulseek.
3. Las descarga en la VPS, las pasa por cuarentena y FFmpeg para verificar que sean auténticas.
4. Calcula su BPM, Key Camelot y energía.
5. Inyecta tags ID3v2.4 / Vorbis limpios.
6. Construye la playlist .m3u8, el archivo XML para Rekordbox y la guía visual del set.
7. Te entrega un reporte final en tu terminal.

---

## 🎧 3. ¿Cómo llevo la música a mi USB para tocar en el club?

Tienes dos opciones muy sencillas:

1. **Auto-Sync a la Nube (Google Drive, S3, R2):**
   Configura clone en tu VPS ejecutando clone config. Una vez configurado, HERA puede subir automáticamente tus sets terminados a tu carpeta de Google Drive.
2. **Descarga directa por SCP / SFTP:**
   Desde tu PC local descargas el crate con un solo comando:
   `ash
   scp -r usuario@IP_DE_TU_VPS:~/hera/sets/MiNuevoSet ./MisSets/
   `

---

## 🛡️ Ventajas de esta arquitectura
* **Cero calentamiento de tu laptop:** Los gigabytes de descarga y procesos DSP ocurren en la nube.
* **Nodo 24/7:** Sigues compartiendo y colaborando con la comunidad DJ sin necesidad de dejar tu PC encendida.
* **Control total desde cualquier lugar:** Puedes pedir música desde tu laptop de viaje, tu PC de escritorio o tu terminal móvil.
