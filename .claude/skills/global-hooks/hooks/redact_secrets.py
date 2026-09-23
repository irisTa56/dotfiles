#!/usr/bin/env python3
"""PostToolUse hook: replace the secrets gitleaks finds in a tool's output.

Reads the hook event on stdin. Every string in `tool_response` is scanned in
one gitleaks run, and each secret found is replaced wherever it appears, so the
output keeps the tool's own shape, which Claude Code requires of a replacement.

When the scanner cannot run, the output is withheld rather than passed on, and
the hook exits 2 so Claude is told why.
"""

import json
import os
import subprocess
import sys

HOOKS_DIR = os.path.dirname(os.path.abspath(__file__))
CONFIG = os.path.join(HOOKS_DIR, "gitleaks.toml")
# Tests point this at a stand-in to exercise the failure path.
GITLEAKS = os.environ.get("REDACT_SECRETS_GITLEAKS", "gitleaks")
MARK = "[REDACTED by gitleaks: {}]"
WITHHELD = "[output withheld: the gitleaks secret scan could not run]"
# Read returns these as base64 payloads, not text a secret could be read from.
BINARY_READ_TYPES = {"image", "pdf"}
# Discriminators the output schema checks: withholding them too would make
# Claude Code reject the replacement and pass the original output on.
STRUCTURAL_KEYS = {"type", "cell_type"}


class ScanError(Exception):
    pass


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)


def rewrite(value, fn):
    if isinstance(value, str):
        return fn(value)
    if isinstance(value, dict):
        return {key: item if key in STRUCTURAL_KEYS else rewrite(item, fn)
                for key, item in value.items()}
    if isinstance(value, list):
        return [rewrite(item, fn) for item in value]
    return value


def scan(text):
    try:
        run = subprocess.run(
            [GITLEAKS, "stdin", "--config", CONFIG, "--no-banner",
             "--exit-code", "0", "--report-format", "json",
             "--report-path", "-", "--log-level", "error"],
            input=text, capture_output=True, text=True, timeout=20)
    except (OSError, subprocess.TimeoutExpired) as err:
        raise ScanError(f"{GITLEAKS}: {err}") from err
    if run.returncode != 0:
        raise ScanError(f"{GITLEAKS} exited {run.returncode}: {run.stderr.strip()}")
    try:
        return json.loads(run.stdout)
    except json.JSONDecodeError as err:
        raise ScanError(f"{GITLEAKS} printed no JSON report: {err}") from err


def respond(output):
    json.dump({"hookSpecificOutput": {
        "hookEventName": "PostToolUse", "updatedToolOutput": output}}, sys.stdout)


def main():
    event = json.load(sys.stdin)
    response = event.get("tool_response")
    if not isinstance(response, (dict, list, str)):
        return 0
    if isinstance(response, dict) and response.get("type") in BINARY_READ_TYPES:
        return 0
    text = "\n".join(s for s in strings(response) if s)
    if not text:
        return 0
    try:
        findings = scan(text)
    except ScanError as err:
        respond(rewrite(response, lambda s: WITHHELD if s else s))
        print(f"redact_secrets: withheld this tool's output: {err}", file=sys.stderr)
        return 2
    # Longest first, so a secret that contains another is replaced whole.
    secrets = sorted({(f["Secret"], f["RuleID"]) for f in findings if f.get("Secret")},
                     key=lambda pair: -len(pair[0]))
    if not secrets:
        return 0

    def redact(s):
        for secret, rule in secrets:
            s = s.replace(secret, MARK.format(rule))
        return s

    respond(rewrite(response, redact))
    return 0


if __name__ == "__main__":
    sys.exit(main())
