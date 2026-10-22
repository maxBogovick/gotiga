"""Create or update the etudes on a server through the admin API.

  ADMIN_KEY=... python3 tools/etudes/build.py --base http://localhost:3000/api/v1 [--hide-others]

By slug: an etude that exists is updated, a new one is created; the order on the
map follows `sort_order` in etudes.py. `--hide-others` turns every other etude
on this server into a draft (they are kept, only not shown on the map).
"""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from etudes import ETUDES as _STUDIES  # noqa: E402
from meetings import MEETINGS  # noqa: E402

sys.path.insert(0, str(HERE.parent / "i18n"))
import texts as T  # noqa: E402  (es/de/it from tools/i18n/content)

ETUDES = _STUDIES + MEETINGS  # training etudes, then campaign meetings


def http(method, url, key, body=None):
    cmd = ["curl", "-s", "-m", "60", "-X", method, url, "-H", f"Authorization: Bearer {key}", "-w", "\n%{http_code}"]
    if body is not None:
        cmd += ["-H", "Content-Type: application/json", "-d", json.dumps(body, ensure_ascii=False)]
    out = subprocess.run(cmd, capture_output=True, text=True).stdout
    text, code = out.rsplit("\n", 1)
    try:
        data = json.loads(text) if text.strip() else None
    except ValueError:
        data = text
    return int(code), data


def body_of(e, status="published"):
    t, n = e["title"], e["note"]
    return {"slug": e["slug"], "titleEn": t[0], "titleRu": t[1], "titleUk": t[2],
            "noteEn": n[0], "noteRu": n[1], "noteUk": n[2], "setup": e["setup"], "botDepth": e["bot_depth"],
            "rewardDust": e["reward_dust"], "rewardFinishDust": e["reward_finish_dust"],
            "playerSide": e.get("player_side", "scripted"), "status": status,
            **T.challenge_fields(e["slug"])}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--hide-others", action="store_true")
    a = ap.parse_args()
    base, key = a.base.rstrip("/"), os.environ["ADMIN_KEY"]
    _, now = http("GET", f"{base}/admin/battles/challenges", key)
    now = now if isinstance(now, list) else now.get("challenges", [])
    by_slug = {c["slug"]: c for c in now}
    ids = []
    for e in sorted(ETUDES, key=lambda e: e["sort_order"]):
        body = body_of(e)
        if e["slug"] in by_slug:
            T.carry(body, by_slug[e["slug"]], ("title", "note"))
            code, res = http("PUT", f"{base}/admin/battles/challenges/{by_slug[e['slug']]['id']}", key, body)
            verb = "updated"
        else:
            code, res = http("POST", f"{base}/admin/battles/challenges", key, body)
            verb = "created"
        print(f"{e['slug']:26} {verb if code < 300 else 'FAILED'} {code}", "" if code < 300 else str(res)[:300])
        if code < 300:
            ids.append(res["id"])
    mine = {e["slug"] for e in ETUDES}
    if a.hide_others:
        for c in now:
            if c["slug"] not in mine and c["status"] != "draft":
                b = {k: c[k] for k in ("slug", "titleEn", "titleRu", "titleUk", "noteEn", "noteRu", "noteUk", "setup",
                                       "botDepth", "rewardDust", "rewardFinishDust", "playerSide")}
                T.carry(b, c, ("title", "note"))
                b["status"] = "draft"
                code, _ = http("PUT", f"{base}/admin/battles/challenges/{c['id']}", key, b)
                print(f"{c['slug']:26} hidden (draft) {code}")
    if ids:
        code, _ = http("POST", f"{base}/admin/battles/challenges/reorder", key, {"ids": ids})
        print("order", code)


if __name__ == "__main__":
    main()
