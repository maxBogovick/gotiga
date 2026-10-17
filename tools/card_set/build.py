"""Create the card set on a server through the admin API.

  ADMIN_KEY=... build.py --base http://localhost:3000/api/v1 [--local] [--only slug,...]

Order: pictures with medallions → races → cards → race damage table.
Reads the work photos (public, read-only) from --works (default: the site).
With --local, works missing from the local database are added (same id as on
the site), because a card is linked to its work by id.

Nothing here deletes anything: wipe the old cards and races first, by hand.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_images  # noqa: E402
import cards as C  # noqa: E402
import layout  # noqa: E402

INK_LIGHT, INK_DARK = "#f4ead8", "#0d0703"
SILVER = {"cost": "#e9e6f2", "health": "#f4e8e8", "power": "#f6ece0", "mana": "#e6eef6"}


def http(method, url, key=None, body=None, files=None):
    cmd = ["curl", "-s", "-m", "120", "-X", method, url, "-w", "\n%{http_code}"]
    if key:
        cmd += ["-H", f"Authorization: Bearer {key}"]
    if files:
        cmd += ["-F", f"file=@{files}"]
    elif body is not None:
        cmd += ["-H", "Content-Type: application/json", "-d", json.dumps(body, ensure_ascii=False)]
    out = subprocess.run(cmd, capture_output=True, text=True).stdout
    text, code = out.rsplit("\n", 1)
    try:
        data = json.loads(text) if text.strip() else None
    except ValueError:
        data = text
    return int(code), data


def ink_at(path, x, y):
    """Light ink on a dark spot of the picture, dark ink on a light one."""
    from PIL import Image
    import numpy as np
    im = Image.open(path).convert("RGB")
    w, h = im.size
    box = im.crop((int(w * (x - 18) / 100), int(h * (y - 2.5) / 100), int(w * (x + 18) / 100), int(h * (y + 2.5) / 100)))
    a = np.asarray(box, np.float32) / 255
    lin = np.where(a <= 0.04045, a / 12.92, ((a + 0.055) / 1.055) ** 2.4)
    lum = float((0.2126 * lin[..., 0] + 0.7152 * lin[..., 1] + 0.0722 * lin[..., 2]).mean())
    return INK_LIGHT if lum < 0.18 else INK_DARK


def dress(card, url, pos, picture):
    ny, ey = layout.of(card)
    m = pos.get("mana")
    marks = {
        "title": {"x": 50.3, "y": ny, "width": 54, "size": 0.75, "ink": ink_at(picture, 50, ny), "shown": True},
        "effect": {"x": 50, "y": ey, "width": 46, "size": 0.95, "ink": ink_at(picture, 50, ey), "shown": True},
        "lore": {"shown": False},
        "cost": {"shown": True, "glyph": False},
        "health": {"shown": True, "glyph": False},
        "power": {"shown": True, "glyph": False},
    }
    if m:
        marks["mana"] = {"x": m["x"], "y": m["y"], "size": 1.8, "bold": True, "ink": SILVER["mana"],
                         "shown": True, "glyph": False}
    d = {
        "frameImage": url, "frameMode": "freeform", "artBaked": True, "frameScaleX": 1, "frameScaleY": 1,
        "layout": "corners", "insetTop": 0, "insetRight": 0, "insetBottom": 0, "insetLeft": 0,
        "costShape": "none", "powerShape": "none", "healthShape": "none",
        "costInk": SILVER["cost"], "powerInk": SILVER["power"], "healthInk": SILVER["health"],
        "costX": pos["cost"]["x"], "costY": pos["cost"]["y"],
        "powerX": pos["power"]["x"], "powerY": pos["power"]["y"],
        "healthX": pos["health"]["x"], "healthY": pos["health"]["y"],
        "costSize": 1.8, "powerSize": 1.8, "healthSize": 1.8,
        "costWeight": 700, "powerWeight": 700, "healthWeight": 700,
        "freeMarks": marks,
    }
    return d


def card_body(c, race_id, fig, art_url, fo, power, health):
    e = c["effect"]
    return {
        "slug": c["slug"], "status": "published", "tier": c["tier"], "raceId": race_id,
        "titleEn": c["en"], "titleRu": c["ru"], "titleUk": c["uk"],
        "effectEn": e[0], "effectRu": e[1], "effectUk": e[2],
        "cost": c["cost"], "power": power, "health": health, "mana": c["mana"],
        "armor": c["armor"], "ward": c["ward"], "attackChannel": c["channel"], "reach": c["reach"],
        "step": 1, "speed": 3, "mend": 0, "traits": [], "abilities": c["abilities"],
        "priceDust": c["price_dust"], "priceFeed": None, "artUrl": art_url,
        "frameOverride": json.dumps(fo, ensure_ascii=False),
        "motionWear": json.dumps(c["motion"]), "figurineId": fig,
        "shoots": bool(c.get("shoots")),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--works", default="https://ritunia.com/api/v1")
    ap.add_argument("--local", action="store_true")
    ap.add_argument("--only", default="")
    ap.add_argument("--out", default=str(Path(tempfile.gettempdir()) / "card_set"))
    a = ap.parse_args()
    key = os.environ["ADMIN_KEY"]
    only = set(filter(None, a.only.split(",")))
    cards = [c for c in C.CARDS if not only or c["slug"] in only]

    pos = build_images.build(a.out, only or None)
    pos = json.loads((Path(a.out) / "positions.json").read_text())

    _, works = http("GET", f"{a.works}/figurines?perPage=200")
    by_slug = {w["slug"]: w for w in works["items"]}
    if a.local:
        for c in cards:
            w = by_slug[c["work"]]
            sql = ("insert into figurines (id,name,short_text,slug,status,is_visible) values "
                   f"('{w['id']}', $q${w['name']}$q$, $q${w.get('shortText') or ''}$q$, '{w['slug']}', "
                   f"'{w['status']}', true) on conflict (id) do nothing")
            subprocess.run(["docker", "exec", "server-db-1", "psql", "-U", "gotiga", "-d", "gotiga", "-c", sql],
                           capture_output=True)

    # races
    _, existing = http("GET", f"{a.base}/battles/races")
    race_id = {r["slug"]: r["id"] for r in (existing or [])}
    for slug, en, ru, uk, note, _hue in C.RACES:
        if slug in race_id:
            continue
        code, r = http("POST", f"{a.base}/admin/battles/races", key, {
            "slug": slug, "nameEn": en, "nameRu": ru, "nameUk": uk, "noteRu": note, "noteEn": note, "noteUk": note})
        print("race", slug, code)
        race_id[slug] = r["id"]

    for c in cards:
        picture = Path(a.out) / f"{c['slug']}.png"
        code, up = http("POST", f"{a.base}/admin/battles/frames/art", key, files=str(picture))
        if code >= 300:
            print("UPLOAD FAILED", c["slug"], code, up)
            continue
        w = by_slug[c["work"]]
        fo = dress(c, up["url"], pos[c["slug"]], str(picture))
        power, health = c["power"], c["health"]
        for _ in range(8):
            body = card_body(c, race_id[c["race"]], w["id"], w["faceImageLargeUrl"], fo, power, health)
            code, res = http("POST", f"{a.base}/admin/battles/cards", key, body)
            if code < 300:
                pts, bal = res.get("budgetPoints") or 0, res.get("balanceIndex") or 0
                note = ""
                # Price follows the card: 1.0 on the curve is points = 4 * cost + 2.
                if not 0.85 <= bal <= 1.15:
                    cost = max(1, min(9, round((pts - 2) / 4)))
                    if cost != body["cost"]:
                        body["cost"] = cost
                        code, res = http("PUT", f"{a.base}/admin/battles/cards/{res['id']}", key, body)
                        note = f"cost {c['cost']} -> {cost}, balance now {res.get('balanceIndex') if code < 300 else code}"
                print("card", c["slug"], "ok", f"power {power} health {health}", "points", pts, "balance", bal, note)
                break
            text = json.dumps(res, ensure_ascii=False)
            if "overTierBudget" in text:
                if health > 4:
                    health -= 1
                else:
                    power -= 1
                continue
            print("card", c["slug"], "FAILED", code, text[:300])
            break

    # race damage ring (needs every race id)
    _, rs = http("GET", f"{a.base}/battles/races")
    ids = {r["slug"]: r["id"] for r in rs}
    for slug, en, ru, uk, note, _hue in C.RACES:
        target = C.RACE_RING[slug]
        body = {"slug": slug, "nameEn": en, "nameRu": ru, "nameUk": uk, "noteEn": note, "noteRu": note,
                "noteUk": note, "raceDamage": {"attack": 1, "against": [ids[target]], "defence": 0, "from": []}}
        code, res = http("PUT", f"{a.base}/admin/battles/races/{ids[slug]}", key, body)
        print("race damage", slug, "->", target, code)


if __name__ == "__main__":
    main()
