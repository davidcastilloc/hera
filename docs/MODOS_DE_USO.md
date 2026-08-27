# 🎧 Guía Completa de Modos de Uso de HERA
### *«Elige tu forma favorita de crear música: desde 1-clic visual hasta super-agentes autónomos»*

HERA está diseñada para adaptarse a tu estilo de trabajo. No necesitas ser un experto en sistemas para usarla. Aquí tienes los **4 modos de uso**, explicados paso a paso con ejemplos reales para cada uno.

---

## 🗺️ Tabla Comparativa de Métodos

| Método | Nivel de Dificultad | ¿Qué necesitas? | ¿Cómo se siente? |
| :--- | :---: | :--- | :--- |
| **1. Modo Visual (Web UI)** | ⭐ (Ultra Fácil) | Navegador Web (http://localhost:8501) | Como usar Spotify / Rekordbox moderno |
| **2. Modo Terminal Directo (hera chat)** | ⭐ (Sin Configuración) | Tu terminal habitual | Como chatear con un copiloto DJ por WhatsApp |
| **3. Modo Curadora en Antigravity CLI (gy)** | ⭐⭐ (Poder Autónomo) | Antigravity CLI (gy) + MCP | Un agente de IA que programa y ejecuta sets completos |
| **4. Modo Super-Nodo 24/7 (VPS Remoto)** | ⭐⭐⭐ (Servidor en la Nube) | VPS Linux / DigitalOcean / Hetzner | Tu propio estudio de audio trabajando 24/7 en la nube |

---

## 🎨 Modo 1: Interfaz Gráfica Web (hera ui)
*Ideal para: DJs visuales, productores y usuarios que prefieren usar el ratón y ver portadas y formas de onda.*

### 🚀 Cómo iniciarlo:
Abre tu terminal y escribe:
`ash
hera ui
`
Se abrirá automáticamente en tu navegador en http://localhost:8501.

### 🖥️ Lo que puedes hacer:
1. **Buscador Visual:** Escribe el nombre de un artista o canción y filtra con selectores por formato (FLAC Lossless, MP3 320k) y versión (Extended, Original, Dub).
2. **Visor de Camelot Wheel:** Un círculo interactivo que te muestra qué canciones de tu biblioteca combinan armónicamente con la que estás escuchando.
3. **Reproductor y Forma de Onda:** Escucha preescuchas de tus tracks con análisis de volumen LUFS en tiempo real.
4. **Exportar a USB:** Un botón de 1-clic para transferir el crate ordenado directamente a tu pendrive formateado para CDJs/Pioneer.

---

## 💬 Modo 2: Chat Directo en Terminal (hera chat o hera agent)
*Ideal para: El usuario que no quiere abrir páginas web ni configurar servidores y solo quiere pedir música rápido.*

### 🚀 Cómo iniciarlo:
`ash
hera chat
`

### 🗣️ Ejemplo de conversación real:
`	ext
🎧 HERA: ¡Hola! Soy tu asistente de curaduría acústica. ¿Qué música necesitas hoy?

TÚ > Búscame 5 tracks de French Touch de los 2000s en calidad FLAC.

🎧 HERA: 
Buscando en catálogo local y red autorizada...
He encontrado 5 candidatos con calidad de estudio verificada:
 1. Modjo - Lady (Hear Me Tonight) [126 BPM | 8A | FLAC 16-bit]
 2. Stardust - Music Sounds Better With You [124 BPM | 7A | FLAC 16-bit]
 3. Cassius - Cassius 1999 [125 BPM | 8A | FLAC 16-bit]
 4. Alan Braxe & Fred Falke - Intro [125 BPM | 9A | FLAC 24-bit]
 5. Demon vs Heartbreaker - You Are My High [120 BPM | 6A | FLAC 16-bit]

¿Deseas que los descargue a tu biblioteca local (~/hera/library)?

TÚ > Sí, descárgalos y organízalos.

🎧 HERA: 
[OK] Descargados a cuarentena.
[OK] Validación de integridad acústica con FFmpeg completada (Cero errores).
[OK] Tags ID3v2.4 inyectados.
[OK] Archivos guardados en: ~/hera/library/French Touch/
`

---

## 🧠 Modo 3: Curadora Virtual en Antigravity CLI (gy)
*Ideal para: Desarrolladores, usuarios avanzados y DJs que usan agentes inteligentes para resolver tareas complejas.*

Gracias a la integración del **Skill Global (curadora-hera)** y el protocolo **MCP**, puedes abrir tu terminal en cualquier carpeta de tu ordenador y hablar con tu asistente.

### 🚀 Cómo usarlo:
Abre gy (o Claude Desktop / Cursor):
`ash
agy
`

### 🗣️ Ejemplos de órdenes de alto nivel:

#### Ejemplo A: Construir un Set Armónico Completo
> *Curadora, constrúyeme un set de 60 minutos de Deep House melódico para un sunset entre 120 y 123 BPM. Quiero progresión armónica Camelot suave y exportación en M3U8 y Rekordbox XML. Autorización: purchased_copy.*

#### Ejemplo B: Consultoría Armónica y Transición
> *Tengo puesto un track en 8A (A menor) a 124 BPM y la pista está muy llena. ¿Qué track de mi biblioteca me recomiendas para subir la energía al máximo?*
> 
> *Respuesta del Agente:* *Te recomiendo saltar a **10A (B menor)** o cambiar de modo a **8B (C mayor)** para una sensación de euforia brillante. Aquí tienes 3 opciones en tu biblioteca...*

#### Ejemplo C: Auditoría de Calidad de Audio
> *Analízame los archivos de audio en la carpeta ~/Descargas/NuevosTracks y dime si alguno es un MP3 falso re-escalado a FLAC.*

---

## ☁️ Modo 4: Super-Nodo 24/7 en la Nube (VPS Remoto)
*Ideal para: Quien quiere descargas ultrarrápidas (1 Gbps), tener su nodo Soulseek compartiendo 24/7 y no gastar batería ni disco de su laptop.*

En este modo, el servidor HERA corre en una VPS barata (Ubuntu / Debian) y tú lo manejas por control remoto desde tu casa.

### 🚀 Cómo ponerlo en marcha:
1. **En tu VPS:**
   `ash
   curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/install.sh | bash
   `
2. **En tu PC local (Configuración MCP por SSH):**
   `json
   {
     mcpServers: {
       hera: {
         command: ssh,
         args: [usuario@IP_DE_TU_VPS, /home/usuario/hera/.venv/bin/hera, serve]
       }
     }
   }
   `
3. **Descarga tu música:** Cuando el VPS termina de armar tus sets, se sincronizan automáticamente con tu Google Drive o los bajas con:
   `ash
   scp -r usuario@IP_DE_TU_VPS:~/hera/sets/MiSet ./MisSets/
   `

---

## 🎯 ¿Cuál debo elegir?

* **Si quieres empezar en 1 minuto:** Ejecuta hera chat en tu terminal.
* **Si te gusta la interfaz gráfica:** Ejecuta hera ui y usa el navegador.
* **Si usas Antigravity / AI Agents a diario:** Usa la Curadora Virtual con gy.
* **Si eres un DJ profesional con catálogo grande:** Levanta un VPS 24/7.
