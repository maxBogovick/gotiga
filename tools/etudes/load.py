#!/usr/bin/env python3
"""Put the etudes on a server. Standard library only (needs curl).

  python3 load.py --base https://ritunia.com/api/v1 --dry-run      # reads only
  python3 load.py --base https://ritunia.com/api/v1 --verify       # reads only: is it all there
  ADMIN_KEY=... python3 load.py --base https://ritunia.com/api/v1  # creates the missing etudes
  ADMIN_KEY=... python3 load.py --base ... --update                # also rewrites the ones that exist

The cards the etudes use must already be on the server (load the card set
first). Nothing is deleted. The order on the map follows `sortOrder`.
"""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def http(method, url, key=None, body=None):
    cmd = ["curl", "-s", "-m", "90", "-X", method, url, "-w", "\n%{http_code}"]
    if key:
        cmd += ["-H", f"Authorization: Bearer {key}"]
    if body is not None:
        cmd += ["-H", "Content-Type: application/json", "-d", json.dumps(body, ensure_ascii=False)]
    out = subprocess.run(cmd, capture_output=True, text=True).stdout
    text, code = out.rsplit("\n", 1)
    try:
        data = json.loads(text) if text.strip() else None
    except ValueError:
        data = text
    return int(code), data


def used_cards(setup):
    names = set()
    for k in ("playerBoard", "keeperBoard"):
        names.update(p["card"] for p in setup.get(k, []))
    for k in ("playerHand", "keeperHand"):
        names.update(setup.get(k, []))
    return names


def known_cards(base):
    code, cards = http("GET", f"{base}/battles/cards")
    have = {c["slug"] for c in (cards if isinstance(cards, list) else [])}
    code, gear = http("GET", f"{base}/battles/etude-cards")  # walls, towers: for etudes only
    if code == 200 and isinstance(gear, list):
        have |= {c["slug"] for c in gear}
    return have


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--verify", action="store_true")
    ap.add_argument("--update", action="store_true")
    a = ap.parse_args()
    base = a.base.rstrip("/")
    pkg = json.loads((HERE / "etudes.json").read_text())
    etudes = sorted(pkg["etudes"], key=lambda e: e["sortOrder"])

    code, now = http("GET", f"{base}/battles/challenges")
    now = now if isinstance(now, list) else []
    have = {c["slug"] for c in now}
    if a.verify:
        lost = [e["slug"] for e in etudes if e["slug"] not in have]
        print(f"etudes from the set on the server: {len(etudes) - len(lost)} of {len(etudes)}")
        if lost:
            print("  missing:", ", ".join(lost))
        print("ALL IN PLACE" if not lost else "NOT COMPLETE")
        sys.exit(0 if not lost else 1)

    cards = known_cards(base)
    bad = {e["slug"]: sorted(used_cards(e["body"]["setup"]) - cards) for e in etudes}
    bad = {k: v for k, v in bad.items() if v}
    if bad:
        print("cards missing on the server (load the card set first):")
        for k, v in bad.items():
            print(f"  {k}: {', '.join(v)}")
        sys.exit("stopped: nothing written")
    todo = [e for e in etudes if e["slug"] not in have]
    print(f"cards: all found. on the server: {len(have)} etudes. to create: {len(todo)}"
          + (f", to update: {len(etudes) - len(todo)}" if a.update else ""))
    if a.dry_run:
        print("dry run: nothing written")
        return
    key = os.environ.get("ADMIN_KEY", "")
    if not key:
        sys.exit("ADMIN_KEY is not set")

    _, admin = http("GET", f"{base}/admin/battles/challenges", key)
    admin = admin if isinstance(admin, list) else admin.get("challenges", [])
    id_of = {c["slug"]: c["id"] for c in admin}
    ids = []
    for e in etudes:
        if e["slug"] in id_of:
            if not a.update:
                ids.append(id_of[e["slug"]])
                continue
            code, res = http("PUT", f"{base}/admin/battles/challenges/{id_of[e['slug']]}", key, e["body"])
            verb = "updated"
        else:
            code, res = http("POST", f"{base}/admin/battles/challenges", key, e["body"])
            verb = "created"
        print(f"{e['slug']:26} {verb if code < 300 else 'FAILED'} {code}", "" if code < 300 else str(res)[:250])
        if code < 300:
            ids.append(res["id"])
    # the set goes first on the map, in its own order; others keep their place after it
    rest = [c["id"] for c in admin if c["id"] not in ids]
    code, _ = http("POST", f"{base}/admin/battles/challenges/reorder", key, {"ids": ids + rest})
    print("order", code)
    print("done")


if __name__ == "__main__":
    main()
