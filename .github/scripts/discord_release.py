#!/usr/bin/env python3
"""Posts an NGM Desktop release to Discord through webhooks (the NGM Addon's
.github/scripts/discord_release.py, for the app).

Reads (environment):
  TAG                        the release's tag, e.g. v0.2.5
  DISCORD_RELEASE_WEBHOOK    every release, no ping (repository secret; never printed)
  DISCORD_ANNOUNCE_WEBHOOK   feature releases only (x.y.0), with a role ping
  DISCORD_ANNOUNCE_ROLE_ID   the role(s) to ping there (repository variable;
                             several role IDs separated by commas ping each)
Posts the version, its CHANGELOG.md section ("## 0.2.5 · 18 November 2026")
and the download link. With --dry-run it prints instead of sending.
"""
import json
import os
import re
import sys
import urllib.request

REPO = "keasbeexd/ngm-desktop"
MAX_NOTES = 3500


def changelog_section(text: str, version: str) -> str:
    out, inside = [], False
    for line in text.splitlines():
        if line.startswith("## "):
            if inside:
                break
            inside = line[3:].split("·")[0].strip() == version
            continue
        if inside:
            out.append(line)
    notes = "\n".join(out).strip()
    if len(notes) > MAX_NOTES:
        notes = notes[:MAX_NOTES].rsplit("\n", 1)[0] + "\n…"
    return notes


def is_feature_release(version: str) -> bool:
    parts = version.split(".")
    return len(parts) == 3 and parts[2] == "0"


def payload(version: str, notes: str, tag: str, ping_roles: list[str] | None = None) -> dict:
    release = f"https://github.com/{REPO}/releases/tag/{tag}"
    body = {
        "username": "NGM Desktop",
        "allowed_mentions": {"parse": [], "roles": ping_roles or []},
        "embeds": [{
            "title": f"NGM Desktop {version} is out",
            "url": release,
            "description": notes or "See the release notes on GitHub.",
            "color": 0xC79C3E,
            "fields": [{
                "name": "Get it",
                "value": f"[Download](https://github.com/{REPO}/releases/latest) for Windows, macOS and Linux · [What's new](https://ngmpeon.app/changelog#desktop)\n"
                         "Already installed? It updates itself (Keep NGM Desktop up to date).",
            }],
        }],
    }
    if ping_roles:
        body["content"] = " ".join(f"<@&{r}>" for r in ping_roles)
    return body


def send(url: str, body: dict, label: str) -> None:
    if not url.startswith("https://discord.com/api/webhooks/"):
        raise SystemExit(f"::error::{label} isn't a Discord webhook URL")
    req = urllib.request.Request(url, data=json.dumps(body).encode(), method="POST",
                                 headers={"Content-Type": "application/json", "User-Agent": "ngm-desktop-release"})
    with urllib.request.urlopen(req, timeout=15) as r:
        print(f"Posted to {label} (HTTP {r.status}).")


def main() -> int:
    dry = "--dry-run" in sys.argv
    tag = os.environ.get("TAG", "")
    if not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        print(f"Not a version tag ({tag!r}); nothing to announce.")
        return 0
    version = tag[1:]
    with open("CHANGELOG.md", encoding="utf-8") as f:
        notes = changelog_section(f.read(), version)
    # one role ID, or several separated by commas: each is pinged
    roles = [r.strip() for r in os.environ.get("DISCORD_ANNOUNCE_ROLE_ID", "").split(",") if r.strip()]
    if any(not re.fullmatch(r"\d{17,20}", r) for r in roles):
        print("::warning::DISCORD_ANNOUNCE_ROLE_ID isn't a role ID (or comma-separated role IDs); posting without a ping")
        roles = []
    roles = list(dict.fromkeys(roles))[:5]
    posts = [("DISCORD_RELEASE_WEBHOOK", payload(version, notes, tag))]
    if is_feature_release(version):
        posts.append(("DISCORD_ANNOUNCE_WEBHOOK", payload(version, notes, tag, roles or None)))
    for env_name, body in posts:
        if dry:
            print(f"--- {env_name} ---")
            print(json.dumps(body, indent=2, ensure_ascii=False))
            continue
        url = os.environ.get(env_name, "")
        if not url:
            print(f"No {env_name} secret: skipping that post.")
            continue
        send(url, body, env_name)
    return 0


if __name__ == "__main__":
    sys.exit(main())
