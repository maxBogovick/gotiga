#!/usr/bin/env python3
"""Put the card set on a server. Standard library only (needs curl).

  python3 load.py --base https://ritunia.com/api/v1 --dry-run     # reads only
  python3 load.py --base https://ritunia.com/api/v1 --verify      # reads only: is it all there
  python3 load.py --sql check|after|rollback                      # prints SQL for the server's database
  ADMIN_KEY=... python3 load.py --base https://ritunia.com/api/v1  # writes

Safe to repeat: races and cards that already exist (same slug) are skipped, the
motion set is merged by id. Nothing is ever deleted. Stops before writing
anything if a work is missing on the server or the server is too old.
"""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def http(method, url, key=None, body=None, file=None):
    cmd = ["curl", "-s", "-m", "180", "-X", method, url, "-w", "\n%{http_code}"]
    if key:
        cmd += ["-H", f"Authorization: Bearer {key}"]
    if file:
        cmd += ["-F", f"file=@{file}"]
    elif body is not None:
        cmd += ["-H", "Content-Type: application/json", "-d", json.dumps(body, ensure_ascii=False)]
    out = subprocess.run(cmd, capture_output=True, text=True).stdout
    text, code = out.rsplit("\n", 1)
    try:
        data = json.loads(text) if text.strip() else None
    except ValueError:
        data = text
    return int(code), data


def fill(value, mapping):
    if isinstance(value, str):
        for k, v in mapping.items():
            value = value.replace(k, v)
        return value
    if isinstance(value, dict):
        return {k: fill(v, mapping) for k, v in value.items()}
    if isinstance(value, list):
        return [fill(v, mapping) for v in value]
    return value


def sql(which, pkg):
    cards = ",".join("'" + c["slug"] + "'" for c in pkg["cards"])
    races = ",".join("'" + r["slug"] + "'" for r in pkg["races"])
    if which == "check":
        return """-- before loading: the server must be on the new code
select version, success from _sqlx_migrations order by version desc limit 3;
select column_name from information_schema.columns
where table_name = 'battle_cards' and column_name in ('title_uk', 'shoots', 'motion_wear', 'edition_size');
select column_name from information_schema.columns
where table_name = 'battle_races' and column_name in ('name_uk', 'race_damage');
select slug, name_ru from battle_races;
select status, count(*) from battle_cards group by status;"""
    if which == "after":
        return f"""-- after loading
select slug, status, tier, cost, power, health from battle_cards
where slug in ({cards}) order by tier, slug;
select r.slug, count(c.id) as cards, r.race_damage is not null as has_damage
from battle_races r left join battle_cards c on c.race_id = r.id group by r.slug, r.race_damage;
select count(*) from battle_cards where figurine_id is null and status = 'published';"""
    return f"""-- rollback: removes only what this set loaded
begin;
delete from battle_cards where slug in ({cards});
delete from battle_races where slug in ({races});
commit;"""


def verify(base, pkg):
    _, cards = http("GET", f"{base}/battles/cards")
    _, races = http("GET", f"{base}/battles/races")
    have = {c["slug"]: c for c in cards}
    want = [c["slug"] for c in pkg["cards"]]
    lost = [s for s in want if s not in have]
    print(f"cards from the set on the server: {len(want) - len(lost)} of {len(want)}")
    if lost:
        print("  missing:", ", ".join(lost))
    by_race = {r["slug"]: 0 for r in pkg["races"]}
    for s in want:
        if s in have:
            by_race[next(c["race"] for c in pkg["cards"] if c["slug"] == s)] += 1
    server_races = {r["slug"]: r for r in races}
    for slug, n in by_race.items():
        r = server_races.get(slug)
        print(f"  {slug:8} cards {n}  race on server: {'yes' if r else 'NO'}")
    _, mo = http("GET", f"{base}/battles/motions")
    ids = {m["id"] for m in mo["motions"]}
    need = {m["id"] for m in pkg["motions"]}
    print(f"motions: {len(need & ids)} of {len(need)} present")
    bad = [s for s in want if s in have and not have[s].get("artUrl") and not have[s].get("frameOverride")]
    print("cards without picture:", bad or "none")
    ok = not lost and need <= ids and all(server_races.get(s) for s in by_race)
    print("ALL IN PLACE" if ok else "NOT COMPLETE")
    return ok


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base")
    ap.add_argument("--verify", action="store_true")
    ap.add_argument("--sql", choices=["check", "after", "rollback"])
    ap.add_argument("--works", default=None, help="where the works are read from (default: --base)")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--reset-race-damage", action="store_true",
                    help="also rewrite the damage table of races that already existed")
    a = ap.parse_args()
    pkg = json.loads((HERE / "cards.json").read_text())
    if a.sql:
        print(sql(a.sql, pkg))
        return
    if not a.base:
        sys.exit("--base is required")
    base = a.base.rstrip("/")
    if a.verify:
        sys.exit(0 if verify(base, pkg) else 1)
    key = os.environ.get("ADMIN_KEY", "")
    if not a.dry_run and not key:
        sys.exit("ADMIN_KEY is not set")

    # 1. the works every card is linked to
    code, works = http("GET", f"{a.works or base}/figurines?perPage=500")
    by_slug = {w["slug"]: w for w in works["items"]}
    missing = [c["work"] for c in pkg["cards"] if c["work"] not in by_slug]
    if missing:
        sys.exit(f"works not found on the server: {missing}")
    print(f"works: all {len(pkg['cards'])} found")

    # 2. what is already there
    _, races = http("GET", f"{base}/battles/races")
    race_id = {r["slug"]: r["id"] for r in races}
    _, cards_now = http("GET", f"{base}/battles/cards")
    have = {c["slug"] for c in cards_now}
    _, mo = http("GET", f"{base}/battles/motions")
    print(f"already on the server: races {sorted(race_id)}, cards {len(have)}, motions {len(mo['motions'])}")
    todo = [c for c in pkg["cards"] if c["slug"] not in have]
    print(f"to create: {len(todo)} cards, races {[r['slug'] for r in pkg['races'] if r['slug'] not in race_id]}, "
          f"motions {len(pkg['motions'])}")
    if a.dry_run:
        print("dry run: nothing written")
        return

    # 3. motion images and the motion set (merged by id); only what is missing
    have_ids = {m["id"] for m in mo["motions"]}
    new_motions = [m for m in pkg["motions"] if m["id"] not in have_ids]
    if new_motions:
        mapping = {}
        wanted = json.dumps(new_motions)
        for f in sorted((HERE / "motion-images").iterdir()):
            if "{{MOTION:" + f.name + "}}" not in wanted:
                continue
            code, up = http("POST", f"{base}/admin/battles/frames/art", key, file=str(f))
            if code >= 300:
                sys.exit(f"motion image failed: {f.name} {code} {up}")
            mapping["{{MOTION:" + f.name + "}}"] = up["url"]
        merged = mo["motions"] + [fill(m, mapping) for m in new_motions]
        code, res = http("POST", f"{base}/admin/battles/motions", key, {"motions": merged})
        print(f"motions: added {len(new_motions)}, saved {code}")
        if code >= 300:
            sys.exit(str(res))
    else:
        print("motions: all present")

    # 4. races
    created_races = set()
    for r in pkg["races"]:
        if r["slug"] in race_id:
            continue
        code, res = http("POST", f"{base}/admin/battles/races", key, {
            "slug": r["slug"], "nameEn": r["nameEn"], "nameRu": r["nameRu"], "nameUk": r["nameUk"],
            "noteEn": r["note"], "noteRu": r["note"], "noteUk": r["note"], **r.get("texts", {})})
        if code >= 300:
            sys.exit(f"race {r['slug']} failed: {code} {res}")
        race_id[r["slug"]] = res["id"]
        created_races.add(r["slug"])
        print("race", r["slug"], code)

    # 5. cards
    for c in todo:
        code, up = http("POST", f"{base}/admin/battles/frames/art", key, file=str(HERE / c["image"]))
        if code >= 300:
            print("IMAGE FAILED", c["slug"], code, up)
            continue
        w = by_slug[c["work"]]
        body = fill(c["body"], {"{{IMAGE}}": up["url"], "{{RACE}}": race_id[c["race"]], "{{WORK}}": w["id"],
                                "{{ART}}": w.get("faceImageLargeUrl") or ""})
        code, res = http("POST", f"{base}/admin/battles/cards", key, body)
        print("card", c["slug"], code, "" if code < 300 else str(res)[:200])

    # 6. race damage: a ring, every race strong against one other
    _, rs = http("GET", f"{base}/battles/races")
    ids = {r["slug"]: r["id"] for r in rs}
    now = {r["slug"]: r for r in rs}
    for r in pkg["races"]:
        # Only races made in this run: a table edited by hand on the server stays.
        if r["slug"] not in created_races and not a.reset_race_damage:
            continue
        target = pkg["ring"][r["slug"]]
        body = {
            "slug": r["slug"], "nameEn": r["nameEn"], "nameRu": r["nameRu"], "nameUk": r["nameUk"],
            "noteEn": r["note"], "noteRu": r["note"], "noteUk": r["note"],
            "raceDamage": {"attack": 1, "against": [ids[target]], "defence": 0, "from": []},
            **r.get("texts", {})}
        # a PUT carries the whole race: keep es/de/it the server already has
        for k in ("name", "note"):
            for lang in ("Es", "De", "It"):
                if k + lang not in body and (now.get(r["slug"], {}).get(k + lang) or "").strip():
                    body[k + lang] = now[r["slug"]][k + lang]
        code, res = http("PUT", f"{base}/admin/battles/races/{ids[r['slug']]}", key, body)
        print("race damage", r["slug"], "->", target, code)
    print("race damage: kept as it is on the server" if not created_races and not a.reset_race_damage else "")
    print("done")


if __name__ == "__main__":
    main()
