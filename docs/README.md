# 📚 Hera Knowledge Base & Documentation Index

This directory contains the canonical documentation, specifications, and architecture decisions for **Hera**.

---

## 📑 Canonical Documents

| Document | Target Audience | Summary |
|---|---|---|
| [`../README.md`](../README.md) | All Users & Developers | Project overview, core features, architecture, Quickstart, CLI reference, and **Wishlist Board**. |
| [`../CONTRIBUTING.md`](../CONTRIBUTING.md) | Contributors & Maintainers | Development setup, architecture layers, coding standards, testing, and PR workflow. |
| [`Hera_Especificacion_Tecnica_PRD_v0.1.docx`](Hera_Especificacion_Tecnica_PRD_v0.1.docx) | Product, Backend, DevOps | Full technical PRD: vision, providers, pipeline, data schema, roadmap, and MVP acceptance criteria. |
| [`Hera_Guia_para_Agentes_AI.md`](Hera_Guia_para_Agentes_AI.md) | AI Agents & MCP Developers | Operational rules, tool contracts, execution sequences, state machines, and guardrails. |

---

## 🏛️ Architectural Guardrails

1. **Local-First Single-User Architecture:** Hera runs locally with Python 3.11+, Pydantic v2, SQLite, file storage, and MCP over `stdio`. 
2. **Cross-Platform Portability:** Runs identically on Linux (x86_64, ARM64), macOS, and Windows.
3. **No Heavy Cloud Infrastructure Required:** PostgreSQL, Redis, Kafka, Kubernetes, multi-tenancy, and mandatory paid cloud AI APIs are not required for core operations.
4. **Cloud Agility via `rclone`:** External cloud storage synchronization (Google Drive, Cloudflare R2, AWS S3, Dropbox) is decoupled and orchestrated via `rclone`.
