# HERA v0.7.0 — Scope Reducido para Este Release: Motor BPM (Completado) + Tareas 2, 4, 6, 7, 8 y 9

> **Alcance Oficial de este Release (Ajustado por el Usuario):**
> - **Completada:** `Task 1` (Motor BPM de 5 capas: Salsa, BPM Variable y Tecnomerengue).
> - **Activas en este Release:** `Task 2` (Detector CD rayado/saltos), `Task 4` (Reglas de curación: Crates Audio+Video, regla `< 10m` Mixes, Parejas Ganadoras, Exclusiones), `Task 6` (Contratos IPC, SQLite 06, escáner en 2 fases con botón Cancelar), `Task 7` (System Tray Tooltip en vivo + herramientas feedback en `/agent`), `Task 8` (Interfaz Studio: BETA badge, OnboardingWizardModal, vistas Mixes `<10m` vs `>10m`, badges) y `Task 9` (Verificación E2E obligatoria para probar).
> - **Archivadas para el Próximo Release:** `Task 3` (8 Hot Cues anti-zona-vacía `>= 50%` y merger de Cues VDJ/Rekordbox) y `Task 5` (Identificador acústico tipo Shazam).

---

## Tareas Activas de este Release

- [x] **Task 1: Motor BPM de 5 Capas (`hera-rs/src/bpm_engine/`)**  
  Implementado en `2dc50e2` y `fa05a6c`. 5/5 tests unitarios pasando (`bpm_engine_qa.rs`).

- [ ] **Task 2: Detector Forense de Saltos / CD Rayado (*«21. Eres tú»*) y Decodificador Cancelable (`<= 600s`)**
  - **Archivos:** `hera-rs/src/forensics_glitch.rs` (`<= 349` LOC), `hera-rs/src/forensics.rs`, `hera-rs/src/lib.rs`.
  - **Entrega:** Detección de paquetes corruptos, micro-cortes de energía y bucles de buffer repetido (efecto CD rayado) sin romper el Golden Master de `forensics_and_4_hot_cues`, más decodificación segura cancelable para audio y video (`.mp4`, `.mov`, `.mkv`).

- [ ] **Task 4: Reglas de Curación en `synergy.rs` (Crates Audio+Video, Regla `< 10m` Mixes, Parejas Ganadoras y Exclusiones)**
  - **Archivos:** `hera-rs/src/dj_learning/curation_rules.rs` (`<= 349` LOC), `hera-rs/src/synergy.rs`.
  - **Entrega:** Soporte mixto Audio + Video, filtro duro `< 10 min` (`<= 600s`) que deriva mixes a un área lógica de Mixes sin mover archivos en disco, Parejas Ganadoras (*«Perdóname mi vida → Conquístame»*) con bonus en 2-Opt usando `entry_bpm`/`exit_bpm`, y Exclusiones (*«Clavelito ya no»*).

- [ ] **Task 6: Contratos Hexagonales (`hera-ipc`), Migración SQLite `06`, Descubridor de Material Trabajable y Escáner Cancelable**
  - **Archivos:** `hera-desktop/src-ipc/src/dj_types.rs` (`<= 349` LOC), `hera-desktop/src-tauri/src/migrations/06_create_dj_learning_tables.sql`, `hera-desktop/src-tauri/src/libs/track.rs`, `hera-desktop/src-tauri/src/libs/database.rs`, `hera-desktop/src-tauri/src/plugins/db.rs`, `hera-desktop/src-tauri/src/plugins/config.rs`.
  - **Entrega:** Columnas y tablas SQLite para Mixes `> 10m`, saltos de CD, BPM variable, Parejas Ganadoras y pistas retiradas; escáner en 2 fases con botón **Cancelar** (`AtomicBool` y `cancel_library_scan`) y descarte previo de audios `< 30s` / carpetas basura.

- [ ] **Task 7: System Tray Tooltip Nativo en Vivo (`TrayIconBuilder`) y Herramientas en `/agent`**
  - **Archivos:** `hera-desktop/src-tauri/src/main.rs`, `hera-desktop/src-tauri/src/plugins/hera_dj/tray_and_learning.rs` (`<= 349` LOC), `hera-desktop/src-tauri/src/plugins/hera_agent/`.
  - **Entrega:** Inicialización de `TrayIconBuilder` con tooltip en vivo (`HERA BETA · Sonando: ...`), menú rápido y herramientas de feedback en `/agent` para guardar parejas y descartar canciones directamente desde el chat.

- [ ] **Task 8: Interfaz Frontend Studio (`hera-desktop/src/`)**
  - **Archivos:** `hera-desktop/src/components/OnboardingWizardModal.tsx`, `hera-desktop/src/components/Navigation.tsx`, `hera-desktop/src/components/HeraDjToolbar.tsx`, `hera-desktop/src/components/TrackRow.tsx`, `hera-desktop/src/routes/dna.tsx`, `hera-desktop/src/routes/agent.tsx`.
  - **Entrega:** Asistente inicial de bienvenida (`OnboardingWizardModal`) en 3 pasos con aviso de fase BETA, insignia permanente `BETA`, botón para cancelar escaneo en la barra de estado, selector `Pistas (< 10m)` vs `Mixes (> 10m)`, etiquetas `[VIDEO]`, `⚠️ Saltos / CD rayado` y `BPM Variable`.

- [ ] **Task 9: Verificación Integral E2E y Pruebas con tu Biblioteca Real**
  - **Archivos:** `hera-desktop/src/__tests__/hera-studio-qa.test-e2e.tsx`, `docs/superpowers/DECISIONS.md`.
  - **Entrega:** Pruebas E2E automatizadas, suite completa (`cargo check`, `cargo test`, `bun run check && bun run test && bun run build`), y validación real con tus canciones.

---

## 📦 Tareas Archivadas para el Próximo Release

Estas tareas quedan formalmente archivadas para el siguiente ciclo y no bloquean el release actual:

### [ARCHIVADO] Task 3: 8 Hot Cues Anti-Zona-Vacía (`>= 50%`) y Merger de Cues VDJ/Rekordbox
- **Motivo de Archivo:** El usuario solicitó priorizar estabilidad del detector de saltos, crates de video, área de mixes, escáner cancelable, tray y onboarding. Los 4 Hot Cues actuales siguen funcionando mientras se madura la compuerta `>= 50%`.
- **Diseño Preservado:** Se encuentra completamente especificado en `docs/superpowers/specs/2026-11-11-pc-qa-curation-bpm-cues-shazam-design.md` (Sección 3.2).

### [ARCHIVADO] Task 5: Identificador Acústico Tipo Shazam (Constelación de Picos + ADN Completo)
- **Motivo de Archivo:** Se posterga la indexación de huellas espectrales y consulta híbrida local/swarm/AcoustID para un release dedicado a metadata enrichment.
- **Diseño Preservado:** Se encuentra completamente especificado en `docs/superpowers/specs/2026-11-11-pc-qa-curation-bpm-cues-shazam-design.md` (Sección 3.5).
