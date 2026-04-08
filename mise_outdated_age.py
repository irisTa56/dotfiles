#!/usr/bin/env python3
"""Show outdated mise tools with release dates and age."""

import json
import subprocess
import sys
from datetime import datetime, timezone


def fmt_date(iso_str):
    if not iso_str:
        return "-"
    try:
        dt = datetime.fromisoformat(iso_str.replace("Z", "+00:00"))
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        days = (datetime.now(timezone.utc) - dt).days
        label = "today" if days < 1 else "1d ago" if days == 1 else f"{days}d ago"
        return f"{dt:%Y-%m-%d} ({label})"
    except Exception:
        return iso_str[:10]


def get_release_dates(tool, current, latest):
    try:
        r = subprocess.run(
            ["mise", "ls-remote", tool, "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if r.returncode == 0:
            vmap = {v["version"]: v.get("created_at") for v in json.loads(r.stdout)}
            return fmt_date(vmap.get(current)), fmt_date(vmap.get(latest))
    except Exception:
        pass
    return "-", "-"


def main():
    r = subprocess.run(["mise", "outdated", "--json"], capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(r.stderr.strip() or f"mise outdated failed (exit {r.returncode})")
    outdated = json.loads(r.stdout)
    if not outdated:
        print("All tools are up to date.")
        return

    rows = []
    for name, info in outdated.items():
        cur, lat = info.get("current", "?"), info.get("latest", "?")
        cur_d, lat_d = get_release_dates(name, cur, lat)
        rows.append((name, info.get("requested", "?"), cur, cur_d, lat, lat_d))

    hdrs = ("Tool", "Requested", "Current", "Released", "Latest", "Released")
    widths = [max(len(h), *(len(r[i]) for r in rows)) for i, h in enumerate(hdrs)]
    fmt = "  ".join(f"{{:<{w}}}" for w in widths)
    print(fmt.format(*hdrs))
    print(fmt.format(*("-" * w for w in widths)))
    for r in rows:
        print(fmt.format(*r))


if __name__ == "__main__":
    main()
