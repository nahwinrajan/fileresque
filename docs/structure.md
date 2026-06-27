# Documentation Structure

> Reference for all agents and human contributors. Every document in `docs/` has a defined owner, audience, and lifecycle.

---

## Directory Map

```
docs/
├── structure.md                          # This file — doc map and ownership
│
├── research/
│   └── research-analysis-report.md      # [TPM] authored; read-only after P0
│
├── features/
│   └── feature-breakdown-phases.md      # [TPM] authored; updated by [TPM] only
│
├── architecture/
│   └── system-architecture.md           # [DEV] + [TPM]; updated when arch changes
│
├── design/
│   └── design-brief.md                  # [DES] authored in chat first (Phase 0)
│                                         # Contains: brand, tokens, component inventory
│
└── agent-planning/
    ├── decisions.md                      # [TPM] — append-only decision log
    │
    ├── 01-p0-project-scaffold.md
    ├── 02-p0-ci-pipeline.md
    ├── 03-p0-design-system.md
    ├── 04-p0-entitlements.md
    ├── 05-p1-disk-enum-macos.md
    ├── 06-p1-disk-enum-windows.md
    ├── 07-p1-disk-list-ui.md
    ├── 08-p1-permission-onboarding.md
    ├── 09-p2-apfs-parser.md
    ├── 10-p2-hfsplus-parser.md
    ├── 11-p2-ntfs-parser.md
    ├── 12-p2-scan-progress.md
    ├── 13-p2-scan-results-ui.md
    ├── 14-p3-block-checker.md
    ├── 15-p3-probability-ui.md
    ├── 16-p4-destination-preflight.md
    ├── 17-p4-recovery-engine.md
    ├── 18-p4-recovery-ui.md
    ├── 19-p4-audit-log.md
    ├── 20-p5-codesign.md
    ├── 21-p5-windows-installer.md
    ├── 22-p5-error-recovery.md
    ├── 23-p5-accessibility.md
    └── 24-p5-security-audit.md
```

---

## Agent Planning Doc Template

Every task in `docs/features/feature-breakdown-phases.md` that carries a `PLANNING:` directive must have a corresponding doc. Use this template:

```markdown
# <seq>-<phase>-<subtask>
**Task ID:** P<N>-T<NN>
**Phase:** <phase name>
**Owner:** [AGENT]
**Status:** Planning | In Progress | QA Review | Done

---

## Overview
[One paragraph describing what this task achieves and why]

## Scope
[Bullet list of what is included]

## Out of Scope
[Bullet list of what is explicitly excluded]

## Dependencies
- Blocked by: [task IDs]

---

## [DEV / DES] Plan
[Implementation plan, module structure, function signatures — written BEFORE coding begins]

## Edge Cases
[List of edge cases this task must handle, referencing EC-XX from research doc where applicable]

## Test Plan
[Table of test cases — written by [DEV] and reviewed by [QA] before coding]

| Case | Input | Expected | Branch covered |
|------|-------|----------|----------------|
| happy_path | ... | ... | main flow |
| branch_X | ... | ... | condition Y |

---

## Implementation Notes
[Written by [DEV] AFTER implementation — what was actually built, any deviations from plan]

## Open Questions / TPM Queries
[Any [TPM_QUERY] blocks raised during this task]

---

## QA Sign-off
[Written by [QA] — see qa.md for format]

## Security Sign-off
[Written by [SEC] — only for tasks with security gate; see security.md for format]
```

---

## decisions.md Format

```markdown
# Decision Log

## [DECISION-001] — <Title>
**Date:** YYYY-MM-DD
**Decided by:** [TPM]
**Context:** [What situation prompted this decision]
**Options considered:**
- A: ...
- B: ...
**Decision:** [A or B or other]
**Rationale:** [1-3 sentences]
**Consequences:** [What this means for other tasks]
---
```

---

## Ownership Summary

| Document | Owner | Audience | Mutable by |
|----------|-------|----------|-----------|
| `research-analysis-report.md` | TPM | All agents | TPM only (append new findings) |
| `feature-breakdown-phases.md` | TPM | All agents | TPM only |
| `system-architecture.md` | DEV + TPM | DEV, QA, SEC | DEV + TPM after decision |
| `design-brief.md` | DES | DEV, QA | DES only |
| `decisions.md` | TPM | All agents | TPM only (append-only) |
| `<seq>-<phase>-<subtask>.md` | Task owner agent | Collab agents | Task owner + QA + SEC |
| `README.md` | TPM | Humans | Any agent updating install steps |
| `CONTEXT.md` | TPM | AI agents | TPM after major arch change |
| `structure.md` | TPM | All agents | TPM only |