"""Make the folder that carries the card set to a server: deploy/card-set/.

Local only (needs the pictures in ~/Downloads/рамки and the local server for
the motion set). Everything the loader needs ends up in the folder, so the
loader itself uses nothing but the standard library.

  .venv-img/bin/python tools/card_set/export_package.py [http://localhost:3000/api/v1]
"""
import json
import re
import shutil
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import build  # noqa: E402
import build_images  # noqa: E402
import cards as C  # noqa: E402
import races  # noqa: E402
from PIL import Image  # noqa: E402

OUT = ROOT / "deploy" / "card-set"
UPLOADS = ROOT / "src-tauri" / "server" / "uploads"


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:3000/api/v1"
    races.check()  # a broken ring would ship a race that wins by itself
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "images").mkdir(parents=True)
    (OUT / "motion-images").mkdir()
    tmp = OUT / "_tmp"
    build_images.build(tmp)
    pos = json.loads((tmp / "positions.json").read_text())

    cards = []
    for c in C.CARDS:
        src = tmp / f"{c['slug']}.png"
        Image.open(src).convert("RGB").save(OUT / "images" / f"{c['slug']}.webp", "WEBP", quality=94, method=6)
        fo = build.dress(c, "{{IMAGE}}", pos[c["slug"]], str(src))
        body = build.card_body(c, "{{RACE}}", "{{WORK}}", "{{ART}}", fo, c["power"], c["health"])
        cards.append({"slug": c["slug"], "image": f"images/{c['slug']}.webp", "work": c["work"],
                      "race": c["race"], "body": body})
    shutil.rmtree(tmp)

    # motions the cards wear, with the images that live in the uploads folder
    used = set()
    for c in C.CARDS:
        used.update(c["motion"].values())
    with urllib.request.urlopen(f"{base}/battles/motions") as r:
        motions = [m for m in json.load(r)["motions"] if m["id"] in used]
    assert len(motions) == len(used), "a card wears a motion that is not in the local set"
    for m in motions:
        for g in m["gestures"]:
            img = g.get("image") or ""
            if img.startswith("/static/"):
                f = UPLOADS / img[len("/static/"):]
                shutil.copy(f, OUT / "motion-images" / f.name)
                g["image"] = "{{MOTION:" + f.name + "}}"
    package = {
        "races": [{"slug": s, "nameEn": en, "nameRu": ru, "nameUk": uk, "note": note}
                  for s, en, ru, uk, note, _ in C.RACES],
        "ring": C.RACE_RING, "cards": cards, "motions": motions,
    }
    (OUT / "cards.json").write_text(json.dumps(package, ensure_ascii=False, indent=1))
    shutil.copy(HERE / "load.py", OUT / "load.py")
    shutil.copy(HERE / "README.md", OUT / "README.md")
    print("cards", len(cards), "motions", len(motions), "->", OUT)


if __name__ == "__main__":
    main()
