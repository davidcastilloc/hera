## 🎯 Summary of Changes

<!-- Describe what this PR solves or builds and why. -->

- **Related Issue:** Fixes #<!-- issue number if applicable -->
- **ODD Change Scope:**
  - [ ] **Micro-change / Bugfix (`< 30` LOC):** Direct test + minimal fix + terminal verification receipt.
  - [ ] **Feature / Architectural Change (`>= 30` LOC):** Spec in `docs/superpowers/specs/` + Plan in `docs/superpowers/plans/` + `docs/superpowers/DECISIONS.md` updated (if applicable).

---

## 🛡️ HERA Community & Open-Core Checklist

- [ ] **Cross-Platform Portability (Windows / macOS / Linux):** Uses `pathlib.Path` and repository-relative paths only (zero hardcoded `C:\`, `/Users/`, or `/home/` paths; zero OS symlinks).
- [ ] **Zero Synthetic Placeholders & Quarantine First:** Audio validation rules and `quarantine/` -> `library/` boundaries are preserved.
- [ ] **Open-Core Boundary Respect:** Zero private SaaS keys, pricing logic, or admin secrets introduced into the public `hera` repository.
- [ ] **Anti-Hallucination & Stack Respect:** Zero fake mock-only tests, zero `TODO`/`pass`/`unimplemented!()` stubs, zero deleted tests, and zero unapproved dependencies added to `pyproject.toml` or `hera-rs/Cargo.toml`.

---

## 🧾 Receipt-Driven Verification (Mandatory Terminal Output)

Paste the real terminal output (`exit code 0`) from running the checks below:

```text
# 1. uv run ruff check .
# 2. uv run pytest
# 3. cargo test --manifest-path hera-rs/Cargo.toml (if Rust files were touched)

<PASTE TERMINAL OUTPUT HERE>
```
