"""Find etude positions that are ABOUT a mechanic.

A position is accepted when the thinking bots win it, the greedy bot does not,
and the thinking bots do NOT win it once the mechanic is switched off
(`ablation.py`). Proposes only: every position is read and named by a person.

  ADMIN_KEY=... python3 tools/etudes/search2.py MECHANIC [N] [KEEPER_DEPTH]
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
import ablation  # noqa: E402
from etudes import P, T  # noqa: E402
from search import PEOPLE, GNOMES, WITCHES, MAGES, UNDEAD, WATER, ALL  # noqa: E402

CELLS_P = [(x, y) for x in range(3) for y in range(3, 6)]
CELLS_K = [(x, y) for x in range(3) for y in range(0, 3)]
BRUISERS = ["gnome-axeman", "the-creature", "miner", "dark-vampire", "forest-gnome", "hunchback", "red-vampire"]

# mechanic -> how to draw a candidate. Every generator returns (setup, ablation spec)
def gen(mech, rng):
    def pick(pool, n):
        return rng.sample(pool, min(n, len(pool)))
    def place(cards, cells):
        return [P(c, x, y) for c, (x, y) in zip(cards, rng.sample(cells, len(cards)))]
    pl, kl = [], []
    terrain, rules, hands = [], None, ([], [])
    ablate = {}
    if mech == "curse":
        casters = pick(["old-witch", "bagpiper"], rng.randint(1, 2))
        pl = casters + pick(PEOPLE + GNOMES, rng.randint(0, 1))
        kl = pick(BRUISERS + ["captain-salazar"], rng.randint(2, 3))
        ablate = {"no_abilities": casters}
    elif mech == "bless":
        casters = ["piper"] if rng.random() < .5 else ["old-wizard"]
        pl = casters + pick(BRUISERS, rng.randint(1, 2))
        kl = pick(BRUISERS, rng.randint(2, 3))
        ablate = {"no_abilities": casters}
    elif mech == "heal":
        healers = pick(["granny-with-a-sheep", "shaman", "gnome-with-a-pot"], 1)
        pl = healers + pick(BRUISERS, rng.randint(1, 2))
        kl = pick(BRUISERS + WITCHES[:2], rng.randint(2, 3))
        ablate = {"no_abilities": healers}
    elif mech == "silence":
        c = ["old-mermaid"]
        pl = c + pick(BRUISERS + MAGES[:1], rng.randint(1, 2))
        kl = pick(MAGES + WITCHES + ["captain-salazar"], rng.randint(2, 3))
        ablate = {"no_abilities": c}
    elif mech == "hold":
        c = ["steam-mermaid"]
        pl = c + pick(BRUISERS, rng.randint(1, 2))
        kl = pick(BRUISERS + ["wizard-with-a-raven"], rng.randint(2, 3))
        ablate = {"no_abilities": c}
    elif mech == "chain":
        c = ["wizard-with-a-raven"]
        pl = c + pick(BRUISERS, rng.randint(1, 2))
        kl = pick(PEOPLE + GNOMES, rng.randint(3, 4))
        ablate = {"no_abilities": c}
    elif mech == "ring":
        pairs = [(GNOMES, MAGES), (MAGES, UNDEAD), (UNDEAD, PEOPLE), (PEOPLE, WITCHES), (WITCHES, WATER), (WATER, GNOMES)]
        a, b = rng.choice(pairs)
        pl, kl = pick(a, rng.randint(2, 3)), pick(b, rng.randint(2, 3))
        ablate = {"no_race": True}
    elif mech in ("ravine", "mire", "pit", "cover", "hill"):
        pl = pick(PEOPLE + GNOMES + WITCHES[:1], rng.randint(2, 3))
        kl = pick(BRUISERS + UNDEAD[:2], rng.randint(2, 3))
        used = set()
        for _ in range(rng.randint(2, 4)):
            c = (rng.randint(0, 2), rng.randint(1, 4))
            if c not in used:
                used.add(c)
                terrain.append(T(c[0], c[1], mech))
        ablate = {"no_ground": [mech]}
    elif mech == "altar":
        pl = pick(ALL, rng.randint(2, 3))
        kl = pick(ALL, rng.randint(2, 3))
        hands = (pick(ALL, 2), pick(ALL, 2))
        terrain = [T(rng.randint(0, 2), 3, "altar"), T(rng.randint(0, 2), 2, "altar")]
        ablate = {"no_ground": ["altar"]}
    elif mech == "breakthrough":
        pl = pick(PEOPLE + GNOMES + WITCHES, rng.randint(2, 3))
        kl = pick(BRUISERS + UNDEAD[:2] + ["shaman", "granny-with-a-sheep"], rng.randint(3, 4))
        for _ in range(rng.randint(0, 3)):
            terrain.append(T(rng.randint(0, 2), rng.randint(1, 4), rng.choice(["ravine", "mire", "cover"])))
        rules = ablation.DEFAULT_RULES | {"breakthrough": True}
        ablate = {"rules": {"breakthrough": False}}
    else:
        raise SystemExit(f"unknown mechanic {mech}")
    pl, kl = list(dict.fromkeys(pl)), list(dict.fromkeys(kl))
    pcells = [c for c in CELLS_P if not any(t["cell"] == {"x": c[0], "y": c[1]} for t in terrain)]
    kcells = [c for c in CELLS_K if not any(t["cell"] == {"x": c[0], "y": c[1]} for t in terrain)]
    setup = dict(playerBoard=place(pl, pcells), playerHand=hands[0], keeperBoard=place(kl, kcells), keeperHand=hands[1])
    if terrain:
        setup["terrain"] = terrain
    if rules:
        setup["rules"] = rules
    return setup, ablate


SECONDS = 240      # time limit for one search
HARD_CAP = 250     # candidates per search; more needs --force


def run(mech, n, depth, seed=1):
    rng = random.Random(seed)
    entries, specs = [], {}
    for k in range(n):
        setup, ab = gen(mech, rng)
        slug = f"{mech}-{k}"
        real = {"slug": slug, "botDepth": depth, "setup": setup}
        entries += [real, ablation.variant(real, ab)]
        specs[slug] = ab
    Path("/tmp/search2.json").write_text(json.dumps(entries))
    key = os.environ["ADMIN_KEY"]
    cards = subprocess.run(["curl", "-s", "-H", f"Authorization: Bearer {key}",
                            "http://localhost:3000/api/v1/admin/battles/cards"], capture_output=True, text=True).stdout
    Path("/tmp/etude_cards.json").write_text(cards)
    env = dict(os.environ, ETUDE_JSON="1", ETUDE_SECS="20")
    try:
        out = subprocess.run([str(ROOT / "src-tauri/server/target/release/examples/etude_check"), "/tmp/etude_cards.json", "/tmp/search2.json"],
                             capture_output=True, text=True, env=env, timeout=SECONDS).stdout
    except subprocess.TimeoutExpired as e:
        print(f"stopped by the time limit ({SECONDS} s): the candidates seen so far are used")
        out = (e.stdout or b"").decode() if isinstance(e.stdout, bytes) else (e.stdout or "")
    res = {}
    for line in out.splitlines():
        try:
            r = json.loads(line)
            res[r["slug"]] = r
        except ValueError:
            pass
    by = {e["slug"]: e for e in entries}
    found = []
    for slug in specs:
        a, b = res.get(slug), res.get(slug + "#without")
        if a and b and a["d3"]["rounds"] <= 12 and ablation.passes(a, b, specs[slug]):
            found.append((a["d3"]["rounds"], a["d3"]["acts"], slug, by[slug]["setup"], specs[slug]))
    found.sort(key=lambda f: (-f[0], -f[1]))
    return n, found


def show(setup):
    pb = ", ".join(f"{p['card']}@{p['x']},{p['y']}" for p in setup["playerBoard"])
    kb = ", ".join(f"{p['card']}@{p['x']},{p['y']}" for p in setup["keeperBoard"])
    ph = ", ".join(setup.get("playerHand", [])); kh = ", ".join(setup.get("keeperHand", []))
    tr = ", ".join(f"{t['ground']}@{t['cell']['x']},{t['cell']['y']}" for t in setup.get("terrain", []))
    return f"игрок: {pb}" + (f" +рука {ph}" if ph else "") + f" | хранитель: {kb}" + (f" +рука {kh}" if kh else "") + (f" | земля: {tr}" if tr else "")


if __name__ == "__main__":
    mech = sys.argv[1]
    n = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2] != "--force" else 120
    if n > HARD_CAP and "--force" not in sys.argv:
        sys.exit(f"refused: {n} candidates is heavy (500 took minutes of full CPU). Up to {HARD_CAP}; more needs --force.")
    depth = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    total, found = run(mech, n, depth)
    print(f"{mech}: {len(found)} of {total} are about the mechanic")
    for rounds, acts, slug, setup, ab in found[:5]:
        print(f"  {slug:14} {rounds}кр {acts}д | {show(setup)}")
