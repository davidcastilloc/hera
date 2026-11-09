# HERA — Human-First Redesign & Universal AI Connectivity Onboarding Specification

**Date:** 2026-11-09  
**Status:** Approved  
**Topic:** DJ Cabina-First UI/UX Redesign (`✨ Vista Simple` vs `🎛️ Vista Pro`) + 5 Universal AI Connectivity Modes

---

## 1. Problem Statement & User Findings

The initial technical proposals for HERA Studio displayed advanced engineering parameters (Camelot harmonic wheel keys, 2-Opt TSP optimization, FFT spectral cutoff in kHz, SimHash u64 POPCNT, stdio subprocesses). While accurate at the DSP engine level, this overwhelms non-technical DJs.

A DJ in a live booth or preparing a set needs:
1. Clear music matching without music theory intimidation ("Mezcla fluida", "Sube el ritmo suave", "Sin choque de voces").
2. 4 Hot Cues in plain language ("1: Entrada", "2: Calma", "3: Explosión / Coro", "4: Salida limpia").
3. Effortless AI onboarding ("Usar la IA de mi PC", "IA Gratuita de HERA", "Modo Fiesta Sin Internet") where the DJ is always "El Jefe".
4. Dual-mode UI: `✨ Vista Simple` (default for 90% of DJs) + `🎛️ Vista Pro` (accessible via toggle in the top bar for DJs who want Camelot keys and kHz forensics).

---

## 2. Core Architecture & Design Decisions

1. **State Management for View Mode:**
   - Store `viewMode: 'simple' | 'pro'` in persistent configuration / Zustand store (`useHeraStore` or localStorage).
   - Default to `'simple'`.
2. **Harmonic Language Translation Layer:**
   - In `✨ Vista Simple`:
     - Keys like `8A -> 9A` display as `💚 Mezcla Suave (98%)`.
     - BPM deltas display as `Sube suave (+1.5 BPM)`.
     - 4 Hot Cues display as `🟢 1: Entrada`, `🔵 2: Calma`, `🔴 3: Explosión`, `🟡 4: Salida`.
     - Forensics display as `✨ Estudio (Puro)`.
   - In `🎛️ Vista Pro`:
     - Full Camelot codes (`8A · Am`), raw BPM (`125.5 BPM`), exact frequencies (`21.8 kHz FLAC`), and TSP metrics.
3. **5 Universal AI Connectivity Modes Drawer:**
   - Smooth slide-out contextual drawer without layout shift.
   - High-level friendly cards with live detection badges.
   - Commander Mode Selector ("👑 Tú eres El Jefe", "⚡ Piloto Automático", "👀 Solo Consejos").
   - Immutable Swarm Protection badge (`🤝 Comunidad HERA: Siempre Activo`).
4. **DNA & Style Profiler:**
   - Human Vibe breakdown ("🌌 Melódico y Envolvente", "🔥 Energía de Hora Pico", "☀️ Cantado y Alegre").
   - Conversational summary card: *"Así te entiende tu Copiloto IA ahora mismo..."*.
5. **Crate & Library Management:**
   - Direct 1-click export to VirtualDJ, Rekordbox, Traktor, and universal M3U8.
