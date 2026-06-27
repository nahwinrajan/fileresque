# FileResque — Claude Project Instructions

> This file is read automatically by Claude on every interaction in this project.  
> For full context, read `CONTEXT.md`. For task list, read `docs/features/feature-breakdown-phases.md`.

---

## What This Project Is

**FileResque** — a native desktop file recovery application.  
Stack: **Tauri 2** (Rust backend) · **Svelte + TypeScript** frontend  
Platforms: **macOS** (primary) · Windows (secondary)  
Network: **Zero.** This app never makes a network call. Ever.

---

## Agent System

Five agents operate on this project. Every agent has a definition in `.claude/agents/`.  
**Always identify which agent you are acting as** before responding to a task.

| Agent | Colour | Model | When to invoke |
|-------|--------|-------|----------------|
| TPM | 🟠 | claude-sonnet-4-6 | Coordination, decisions, scope, `[TPM_QUERY]` resolution |
| Developer | 🔵 | claude-sonnet-4-6 | All Rust and Svelte/TS implementation |
| QA | 🟢 | claude-haiku-4-5-20251001 | Verification, test authorship, sign-off gate |
| Designer | 🟣 | claude-sonnet-4-6 | UI/UX, design system, component specs |
| Security | 🔴 | claude-sonnet-4-6 | Unsafe audits, entitlements, security gates (veto power) |

**No task is DONE without 🟢 QA sign-off.**  
**No security-gated task ships without 🔴 Security approval.**

---

## Before Doing Anything

1. **Identify your agent role** from the task context or the user's instruction.
2. **Check if a planning doc exists** for this task in `docs/agent-planning/`. If not, create it using the template in `docs/structure.md` before writing any code.
3. **Check `docs/agent-planning/decisions.md`** for prior decisions that affect your task.
4. **If ambiguity exists**, emit a `[TPM_QUERY]` — do not make unilateral scope or architecture decisions.

---

## Absolute Rules (No Exceptions)

- **No network calls** in any production code path
- **No `unwrap()` / `expect()`** without `// JUSTIFIED:` comment outside tests
- **All `unsafe` blocks** must have `// SAFETY:` comment
- **Cognitive complexity ≤ 15** per Rust function (clippy enforced)
- **Coverage ≥ 80%** Rust · **≥ 70%** frontend (CI enforced)
- **Filenames from raw disk** are always sanitised before use as filesystem paths
- **🟢 QA signs off** before any task status → Done
- **🔴 Security approves** before any security-gated task merges

---

## Key File Locations

| What | Where |
|------|-------|
| Full AI context | `CONTEXT.md` |
| Human setup guide | `README.md` |
| All tasks + phases | `docs/features/feature-breakdown-phases.md` |
| Technical research | `docs/research/research-analysis-report.md` |
| Architecture | `docs/architecture/system-architecture.md` |
| Design system | `docs/design/design-brief.md` |
| Decision log | `docs/agent-planning/decisions.md` |
| Per-task planning | `docs/agent-planning/<seq>-<phase>-<subtask>.md` |
| Agent definitions | `.claude/agents/*.md` |

---

## Raising a Query

```
[TPM_QUERY]
From: <🟣 Designer / 🔵 Developer / 🟢 QA / 🔴 Security>
Phase: P<N>
Task: <task_id>
Question: <specific, answerable question>
Options: [A] ... [B] ... [C] ...
Blocking: yes | no
```

Post to the task's planning doc and surface to 🟠 TPM.

---

## Task Completion Checklist

Copy into every planning doc before marking Done:

- [ ] `cargo clippy` — 0 warnings
- [ ] `cargo fmt` — clean
- [ ] Cognitive complexity ≤ 15 on all new functions
- [ ] Unit tests written (table-driven); coverage ≥ 80%
- [ ] No `unwrap()`/`expect()` without `// JUSTIFIED:`
- [ ] All `unsafe` blocks have `// SAFETY:`
- [ ] Planning doc updated with `## Implementation Notes`
- [ ] 🟢 QA sign-off appended to planning doc
- [ ] 🔴 Security sign-off appended (if security gate applies)