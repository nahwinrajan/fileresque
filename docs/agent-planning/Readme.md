# Agent Planning Docs

This directory contains per-task planning documents created by agents during development.

## Naming Convention

```
<sequence>-<phase>-<subtask>.md
```

Examples:
- `01-p0-project-scaffold.md`
- `09-p2-apfs-parser.md`
- `17-p4-recovery-engine.md`

## File Template

See `docs/structure.md` for the full planning doc template.

## Index

| File | Task ID | Owner | Status |
|------|---------|-------|--------|
| decisions.md | — | 🟠 TPM | Ongoing |
| (tasks created by agents as work begins) | | | |

## Lifecycle

1. Agent reads task from `docs/features/feature-breakdown-phases.md`
2. Agent creates planning doc BEFORE writing any code
3. `[DEV]` and `[QA]` collaborate on the test plan section
4. `[DEV]` implements
5. `[QA]` appends sign-off section
6. `[SEC]` appends security sign-off (if applicable)
7. Status set to `Done`