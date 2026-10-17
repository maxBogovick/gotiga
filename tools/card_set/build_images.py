"""Compose the medallions into every card picture of the set.

  build_images.py OUT_DIR [slug ...]     (pictures are read from ~/Downloads/рамки)

Writes OUT_DIR/<slug>.png and OUT_DIR/positions.json: where each number goes, in
percent of the card (the panel centre of its medallion).
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "medals"))
import cards as C  # noqa: E402
from compose import compose  # noqa: E402

SRC = Path.home() / "Downloads" / "рамки"
HUE_COST = {r[0]: r[5] for r in C.RACES}


def build(out, only=None):
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    pos = {}
    for c in C.CARDS:
        if only and c["slug"] not in only:
            continue
        items = [("cost", "tl"), ("health", "tr"), ("power", "br")]
        if c.get("mana_medal", True) and c["slug"] != "captain-salazar":
            items.append(("mana", "bl"))
        hues = {"cost": HUE_COST[c["race"]] - 270, "health": C.HEALTH_HUE, "power": C.POWER_HUE, "mana": -55}
        sats = {"mana": 0.5}
        pos[c["slug"]] = compose(SRC / c["file"], out / f"{c['slug']}.png", items, hues, sats=sats)
    path = out / "positions.json"
    old = json.loads(path.read_text()) if path.exists() else {}
    old.update(pos)
    path.write_text(json.dumps(old, indent=1))
    return pos


if __name__ == "__main__":
    build(sys.argv[1], set(sys.argv[2:]) or None)
