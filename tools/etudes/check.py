"""Run the etude checker (bots) on tools/etudes/etudes.py.

  ADMIN_KEY=... python3 tools/etudes/check.py [slug ...]
"""
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from etudes import ETUDES  # noqa: E402
import ablation  # noqa: E402


def run_json(entries, secs=60):
    Path("/tmp/etudes_run.json").write_text(json.dumps(entries))
    env = dict(os.environ, ETUDE_JSON="1", ETUDE_SECS=str(secs))
    exe = ROOT / "src-tauri/server/target/release/examples/etude_check"
    try:
        out = subprocess.run([str(exe), "/tmp/etude_cards.json", "/tmp/etudes_run.json"],
                             capture_output=True, text=True, env=env, timeout=240).stdout
    except subprocess.TimeoutExpired as e:
        sys.exit("stopped by the 240 s limit: check fewer etudes at a time")
    return {r["slug"]: r for r in map(json.loads, out.splitlines())}


def check_mechanics():
    """Real etude vs the same etude without its mechanic."""
    entries, pairs = [], []
    mech = {}
    for e in ETUDES:
        if not e.get("mechanic"):
            continue
        mech[e["slug"]] = e["mechanic"]
        real = {"slug": e["slug"], "botDepth": e["bot_depth"], "setup": e["setup"]}
        entries += [real, ablation.variant(real, e["mechanic"])]
        pairs.append(e["slug"])
    res = run_json(entries)
    word = {"W": "выиграл", "L": "проиграл", "D": "ничья", "T": "время"}
    print(f"{'этюд':26} {'с механикой (ум.)':20} {'без неё (ум.)':20} итог")
    for slug in pairs:
        a, b = res[slug], res[slug + "#without"]
        ok = ablation.passes(a, b, mech[slug])
        print(f"{slug:26} {word[a['d3']['o']]:20} {word[b['d3']['o']]:20} {'✓ про механику' if ok else '✗ ' + ('линия её не использует' if ablation.passes(a, b) else 'механика не нужна')}")


def main():
    only = sys.argv[1:]
    if only == ["--mechanics"]:
        subprocess.run(["true"])
        cards = subprocess.run(["curl", "-s", "-H", f"Authorization: Bearer {os.environ['ADMIN_KEY']}",
                               "http://localhost:3000/api/v1/admin/battles/cards"], capture_output=True, text=True).stdout
        Path("/tmp/etude_cards.json").write_text(cards)
        check_mechanics()
        return
    key = os.environ["ADMIN_KEY"]
    cards = subprocess.run(["curl", "-s", "-H", f"Authorization: Bearer {key}",
                            "http://localhost:3000/api/v1/admin/battles/cards"], capture_output=True, text=True).stdout
    Path("/tmp/etude_cards.json").write_text(cards)
    data = [{"slug": e["slug"], "botDepth": e["bot_depth"], "setup": e["setup"]} for e in ETUDES]
    Path("/tmp/etudes.json").write_text(json.dumps(data))
    exe = ROOT / "src-tauri/server/target/release/examples/etude_check"
    subprocess.run([str(exe), "/tmp/etude_cards.json", "/tmp/etudes.json", *only])


if __name__ == "__main__":
    main()
