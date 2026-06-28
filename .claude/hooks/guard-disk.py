#!/usr/bin/env python3
import json, sys, re

data = json.load(sys.stdin)
cmd = data.get("tool_input", {}).get("command", "")

# Block: running the app, raw-disk access, destructive disk ops.
# Allowed (not matched): cargo build/test, bun build, make test/smoke/lint.
PATTERNS = [
    r"/dev/r?disk",                 # raw disk devices  /dev/disk*  /dev/rdisk*
    r"\bdiskutil\b",
    r"\bdd\b",
    r"\bmkfs\b", r"\bnewfs\b", r"\basr\b", r"\bhdiutil\b",
    r"\btauri\s+(dev|build)\b",     # launch / bundle the Tauri app (hits the backend)
    r"\bmake\s+(dev|build)\b",
    r"\bcargo\s+run\b",
    r"target/(debug|release)/fileresque",  # executing the built binary
    r"\bsudo\b",                    # no privilege escalation for agents
]

for p in PATTERNS:
    if re.search(p, cmd):
        print(
            f"BLOCKED by guard-disk hook (/{p}/): agents must not run the app, "
            f"escalate privileges, or touch raw disks. Run this yourself in a terminal.",
            file=sys.stderr,
        )
        sys.exit(2)   # exit 2 = block the tool call, feed reason back to the agent
sys.exit(0)
