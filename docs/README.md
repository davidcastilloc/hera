# 📚 HERA Knowledge Base & Documentation Index (`v0.6.0`)

This directory contains the canonical documentation, product visual guides, P2P Swarm architecture, and engineering specifications for **HERA (`v0.6.0` — 100% Pure Rust + Tauri v2)**.

---

## 📑 Canonical Documents

| Document | Target Audience | Summary |
|---|---|---|
| [`../README.md`](../README.md) | All Users, DJs & Developers | Project overview, new `v0.6.0` visual banner, 5-screen product tour, Symbiotic P2P Swarm concept (*«Por el bien del enjambre»*), and quickstart. |
| [`../hera-desktop/README.md`](../hera-desktop/README.md) | Desktop Users & Frontend/Tauri Devs | **HERA Desktop (`v0.6.0` Dark Booth Studio)** guide: `Biblioteca`, `Agente` (`Ctrl+J`), `Transición`, `Mi Estilo`, and `Modo Mini` (`Ctrl+Shift+M`). |
| [`../hera-rs/README.md`](../hera-rs/README.md) | Rust, DSP & Edge Engineers | **`hera-node-rs v0.6.0`** reference: `Symphonia` + `rustfft` forensics, 4 Hot Cues, `DualDeckEngine`, 2-Opt TSP, 21-tool MCP, and WASM Edge worker. |
| [`MODOS_DE_USO.md`](MODOS_DE_USO.md) | DJs, Producers & Operators | Visual step-by-step guide to all **5 usage modes** in `v0.6.0` (Dark Booth Studio, Modo Mini, `/agent` Workspace, MCP `stdio`/`HTTP`, and 24/7 Edge/VPS Swarm Node). |
| [`FEDERATED_R2_SWARM.md`](FEDERATED_R2_SWARM.md) | Community DJs & Self-Hosters | **Federated R2 Edge Swarm (`Edge Guardian`)**: Constrained Water-Filling (`9.0 GB` / `85k req/day` safe caps), `1:1.5` Turbo Credits, `4 MB` BEP 52 Merkle blocks (`8.4x`), and 64B `Proof-of-Custody`. |
| [`VPS_REMOTE_GUIDE.md`](VPS_REMOTE_GUIDE.md) | Cloud & Self-Hosted DJs | Running `hera-node-rs` (`~10 MB RAM`) on a 24/7 Micro-VPS or Cloudflare Worker and connecting it to HERA Desktop or MCP clients. |
| [`Hera_Guia_para_Agentes_AI.md`](Hera_Guia_para_Agentes_AI.md) | AI Agents & MCP Developers | Operational rules, 21/44 MCP tool contracts, Zero-Trust forensic pipeline, and Swarm-First invariants. |
| [`superpowers/DECISIONS.md`](superpowers/DECISIONS.md) | Maintainers & AI Subagents | Architectural Decision Records (`ADR-001` through `ADR-026`) and root-cause verification receipts. |

---

## 🏛️ Architectural Guardrails (`v0.6.0`)

1. **100% Pure-Rust & Zero Python:** HERA runs entirely on native Rust (`hera-node-rs` + `hera-ipc` + `Tauri v2`) with SQLite (`sqlx`), `Symphonia` + `rustfft` DSP, and MCP over `stdio` / `HTTP` / `Code Mode` (`~8 MB` desktop installer, `~10 MB` RAM).
2. **Swarm-First Symbiosis (*«Por el bien del enjambre»*):** 100% of HERA's local DSP, 4 Hot Cues, 2-Opt TSP crate curation, DJ exporters, and `Cloudflare Workers AI Free` (`10k Neurons/day`) are `$0.00` free; in exchange, all free/local/beta nodes participate in the P2P Swarm by default (`share_on_p2p = true`).
3. **Quarantine-First Zero-Trust Audio:** Every track is audited with `Symphonia` + `rustfft` (`VERIFIED_STUDIO_MASTER`) and sealed with `hera_seal` + `audio_sha256` before entering the library or the P2P swarm.
4. **Hexagonal Contract Safety (`hera-ipc`):** All DTOs (`ts-rs`), Tauri v2 ACL Triad permissions across all 8 plugins, and documentation visual assets are continuously verified via `cargo test -p hera-ipc`.
