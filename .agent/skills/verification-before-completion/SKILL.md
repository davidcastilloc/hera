---
name: verification-before-completion
description: Use when about to claim work is complete, fixed, or passing, before committing or creating PRs - requires running verification commands and confirming output before making any success claims; evidence before assertions always
---

# Verification Before Completion

## Overview

**Core principle:** Evidence before claims, always.

**Violating the letter of this rule is violating the spirit of this rule.**

## The Iron Law (Receipt-Driven Development — RDD)

```
NO COMPLETION CLAIMS WITHOUT FRESH TERMINAL RECEIPTS (LINTER + TYPECHECK + TESTS IN GREEN)
```

If you haven't run the verification command in this turn and attached the real terminal output receipt, you cannot claim it passes.

## Anti-Hallucination & Multi-OS Integrity Gate

Before claiming completion on any task:
1. **Zero Fake Tests:** Verify tests exercise real production logic (forbidden to write mocks that only test mocks).
2. **Zero Empty Stubs:** Verify no `TODO`, `pass`, or `unimplemented!()` placeholders were introduced as fake implementations.
3. **Zero Deleted Tests:** Verify no existing tests were deleted or weakened to make the test suite pass.
4. **Multi-OS Path Hygiene:** Verify `git diff` contains zero OS-specific absolute paths (`C:\Users\...`, `/Users/...`, `/home/...`).
5. **Atomic Handoff Ready:** Verify `git config user.name` returns `"david"`, the task checkbox `[x]` is updated in `docs/superpowers/plans/`, and `docs/superpowers/DECISIONS.md` reflects any critical architectural findings.

## The Gate Function

```
BEFORE claiming any status or expressing satisfaction:

1. IDENTIFY: What command proves this claim? (Build + Linter + Typecheck + Test Suite)
2. RUN: Execute the FULL command (fresh, complete)
3. READ: Full output, check exit code (must be 0), count failures (must be 0)
4. VERIFY: Does output confirm the claim without fake mocks, stubs, or deleted tests?
   - If NO: State actual status with evidence
   - If YES: State claim WITH real terminal output receipt
5. ONLY THEN: Make the claim and commit atomically (`multi-instance-handoff`)

Skip any step = lying, not verifying
```

## Common Failures

| Claim | Requires | Not Sufficient |
|-------|----------|----------------|
| Tests pass | Test command output: 0 failures | Previous run, "should pass" |
| Linter clean | Linter output: 0 errors | Partial check, extrapolation |
| Build succeeds | Build command: exit 0 | Linter passing, logs look good |
| Bug fixed | Test original symptom: passes | Code changed, assumed fixed |
| Regression test works | Red-green cycle verified | Test passes once |
| Agent completed | VCS diff shows changes | Agent reports "success" |
| Requirements met | Line-by-line checklist | Tests passing |

## Red Flags - STOP

- Using "should", "probably", "seems to"
- Expressing satisfaction before verification ("Great!", "Perfect!", "Done!", etc.)
- About to commit/push/PR without verification
- Trusting agent success reports
- Relying on partial verification
- Thinking "just this once"
- Tired and wanting work over
- **ANY wording implying success without having run verification**

## Rationalization Prevention

| Excuse | Reality |
|--------|---------|
| "Should work now" | RUN the verification |
| "I'm confident" | Confidence ≠ evidence |
| "Just this once" | No exceptions |
| "Linter passed" | Linter ≠ compiler |
| "Agent said success" | Verify independently |
| "I'm tired" | Exhaustion ≠ excuse |
| "Partial check is enough" | Partial proves nothing |
| "Different words so rule doesn't apply" | Spirit over letter |

## Key Patterns

**Tests:**
```
✅ [Run test command] [See: 34/34 pass] "All tests pass"
❌ "Should pass now" / "Looks correct"
```

**Regression tests (TDD Red-Green):**
```
✅ Write → Run (pass) → Revert fix → Run (MUST FAIL) → Restore → Run (pass)
❌ "I've written a regression test" (without red-green verification)
```

**Build:**
```
✅ [Run build] [See: exit 0] "Build passes"
❌ "Linter passed" (linter doesn't check compilation)
```

**Requirements:**
```
✅ Re-read plan → Create checklist → Verify each → Report gaps or completion
❌ "Tests pass, phase complete"
```

**Agent delegation:**
```
✅ Agent reports success → Check VCS diff → Verify changes → Report actual state
❌ Trust agent report
```

## When To Apply

**ALWAYS before:**
- ANY variation of success/completion claims
- ANY expression of satisfaction
- ANY positive statement about work state
- Committing, PR creation, task completion
- Moving to next task
- Delegating to agents

**Rule applies to:**
- Exact phrases
- Paraphrases and synonyms
- Implications of success
- ANY communication suggesting completion/correctness
