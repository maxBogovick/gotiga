"""Change numbers of cards already on a server: tune.py BASE slug key=value ...

  ADMIN_KEY=... tune.py http://localhost:3000/api/v1 the-creature power=5 health=8
Reads the card, changes the named fields, saves it through the same endpoint the
admin table uses, so points and price index are recomputed by the server.
"""
import json
import os
import subprocess
import sys


def call(method, url, body=None):
    cmd = ["curl", "-s", "-m", "60", "-X", method, url, "-H", f"Authorization: Bearer {os.environ['ADMIN_KEY']}",
           "-w", "\n%{http_code}"]
    if body is not None:
        cmd += ["-H", "Content-Type: application/json", "-d", json.dumps(body, ensure_ascii=False)]
    out = subprocess.run(cmd, capture_output=True, text=True).stdout
    text, code = out.rsplit("\n", 1)
    return int(code), json.loads(text) if text.strip().startswith(("{", "[")) else text


def tune(base, slug, **fields):
    _, listing = call("GET", f"{base}/admin/battles/cards")
    listing = listing if isinstance(listing, list) else listing.get("cards", listing)
    card = next(c for c in listing if c["slug"] == slug)
    _, full = call("GET", f"{base}/admin/battles/cards/{card['id']}")
    body = dict(full)
    for k, v in fields.items():
        body[k] = v
    if isinstance(body.get("frameOverride"), (dict, list)):
        body["frameOverride"] = json.dumps(body["frameOverride"], ensure_ascii=False)
    if isinstance(body.get("motionWear"), (dict, list)):
        body["motionWear"] = json.dumps(body["motionWear"])
    code, res = call("PUT", f"{base}/admin/battles/cards/{card['id']}", body)
    return code, res


if __name__ == "__main__":
    base, slug = sys.argv[1], sys.argv[2]
    kv = {a.split("=")[0]: json.loads(a.split("=")[1]) for a in sys.argv[3:]}
    code, res = tune(base, slug, **kv)
    if code < 300:
        print(slug, "ok", "points", res.get("budgetPoints"), "balance", res.get("balanceIndex"),
              "power", res.get("power"), "health", res.get("health"), "cost", res.get("cost"))
    else:
        print(slug, "FAILED", code, str(res)[:300])
