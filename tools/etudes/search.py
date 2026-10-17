"""Find etude positions by search, then check them with bots.

A position is good when the greedy bot (depth 1) loses and the thinking bots
(depth 2 and 3) win: there is a solution and it is not the first idea. The
search only proposes; every found position is read and named by a person.

  ADMIN_KEY=... python3 tools/etudes/search.py THEME [N] [KEEPER_DEPTH]
"""
import json
import os
import random
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from etudes import P, T  # noqa: E402

PEOPLE = ["hunchback", "piper", "bagpiper", "falconer", "granny-with-a-sheep"]
GNOMES = ["forest-gnome", "miner", "gnome-axeman", "gnome-with-a-pot"]
WITCHES = ["old-witch", "witch-with-a-key", "yaga-with-a-bird"]
MAGES = ["old-wizard", "shaman", "wizard-with-a-raven"]
UNDEAD = ["old-vampire", "dark-vampire", "red-vampire", "the-creature", "captain-salazar"]
WATER = ["steam-mermaid", "old-mermaid"]
ALL = PEOPLE + GNOMES + WITCHES + MAGES + UNDEAD + WATER

# theme -> (player pool, keeper pool, player count, keeper count, terrain kinds)
THEMES = {
    "melee": (PEOPLE + GNOMES, GNOMES + UNDEAD, (2, 3), (2, 3), []),
    "range": (["falconer", "old-witch", "witch-with-a-key"] + PEOPLE[:2], GNOMES + UNDEAD[:3], (2, 3), (2, 3), ["hill", "ravine", "cover"]),
    "healer": (["granny-with-a-sheep", "shaman", "gnome-with-a-pot"] + PEOPLE[:1] + GNOMES[:3], UNDEAD + GNOMES, (2, 3), (3, 3), []),
    "curse": (["old-witch", "bagpiper", "piper", "old-wizard"] + GNOMES[:3], GNOMES[2:] + UNDEAD[2:], (2, 3), (2, 2), []),
    "ring": (GNOMES + ["hunchback"], MAGES + WITCHES, (2, 3), (2, 3), ["cover"]),
    "terrain": (PEOPLE + GNOMES, UNDEAD + GNOMES, (2, 3), (2, 3), ["mire", "ravine", "pit", "cover", "hill"]),
    "altar": (ALL, ALL, (2, 3), (2, 3), ["altar", "spring", "embers"]),
    "breakthrough": (PEOPLE + GNOMES + WITCHES, GNOMES + UNDEAD, (2, 3), (2, 3), ["ravine", "cover", "mire"]),
}
RULES = {
    "breakthrough": {"secondSideCoin": 1, "openingAttacks": 1, "walkSpendsTurn": False, "retaliation": False,
                     "actsPerTurn": 255, "escalationFrom": 0, "idleToll": 0, "maxRounds": 12,
                     "keeperHoldsOnTime": False, "longShotPower": 25, "pointBlankPower": 50, "breakthrough": True},
}
MAX_ROUND = 12


def candidate(theme, rng, k):
    pp, kp, pc, kc, grounds = THEMES[theme]
    pn, kn = rng.randint(*pc), rng.randint(*kc)
    pl = rng.sample(pp, min(pn, len(pp)))
    kl = rng.sample(kp, min(kn, len(kp)))
    pcells = rng.sample([(x, y) for x in range(3) for y in range(3, 6)], len(pl))
    kcells = rng.sample([(x, y) for x in range(3) for y in range(0, 3)], len(kl))
    terrain = []
    used = set(pcells) | set(kcells)
    for g in rng.sample(grounds, rng.randint(0, min(2, len(grounds)))) if grounds else []:
        for _ in range(rng.randint(1, 3)):
            c = (rng.randint(0, 2), rng.randint(0, 5))
            if c not in used and all(t["cell"] != {"x": c[0], "y": c[1]} for t in terrain):
                terrain.append(T(c[0], c[1], g))
    setup = dict(
        playerBoard=[P(c, x, y) for c, (x, y) in zip(pl, pcells)], playerHand=[],
        keeperBoard=[P(c, x, y) for c, (x, y) in zip(kl, kcells)], keeperHand=[],
    )
    if terrain:
        setup["terrain"] = terrain
    if theme in RULES:
        setup["rules"] = RULES[theme]
    return f"{theme}-{k}", setup


def run(theme, n, depth, seed=1):
    rng = random.Random(seed)
    cands = [candidate(theme, rng, k) for k in range(n)]
    Path("/tmp/search_etudes.json").write_text(json.dumps(
        [{"slug": s, "botDepth": depth, "setup": st} for s, st in cands]))
    key = os.environ["ADMIN_KEY"]
    cards = subprocess.run(["curl", "-s", "-H", f"Authorization: Bearer {key}",
                            "http://localhost:3000/api/v1/admin/battles/cards"], capture_output=True, text=True).stdout
    Path("/tmp/etude_cards.json").write_text(cards)
    env = dict(os.environ, ETUDE_JSON="1", ETUDE_SECS="20")
    out = subprocess.run([str(ROOT / "src-tauri/server/target/release/examples/etude_check"),
                          "/tmp/etude_cards.json", "/tmp/search_etudes.json"], capture_output=True, text=True, env=env).stdout
    found = []
    by = dict(cands)
    for line in out.splitlines():
        r = json.loads(line)
        if r["d1"]["o"] == "L" and r["d2"]["o"] == "W" and r["d3"]["o"] == "W" and r["d3"]["rounds"] <= MAX_ROUND and r["d2"]["rounds"] <= MAX_ROUND:
            found.append((r["d3"]["rounds"], r["d3"]["acts"], r["slug"], by[r["slug"]]))
    found.sort(key=lambda f: (-f[0], -f[1]))
    return len(cands), found


if __name__ == "__main__":
    theme = sys.argv[1]
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 200
    depth = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    total, found = run(theme, n, depth)
    print(f"{theme}: {len(found)} of {total} fit (greedy loses, thinking wins)")
    for rounds, acts, slug, setup in found[:6]:
        pb = ", ".join(f"{p['card']}@{p['x']},{p['y']}" for p in setup["playerBoard"])
        kb = ", ".join(f"{p['card']}@{p['x']},{p['y']}" for p in setup["keeperBoard"])
        tr = ", ".join(f"{t['ground']}@{t['cell']['x']},{t['cell']['y']}" for t in setup.get("terrain", []))
        print(f"  {slug:14} {rounds}кр {acts}д | игрок: {pb} | хранитель: {kb}" + (f" | земля: {tr}" if tr else ""))
