"""Contact sheets for choosing which work a card picture belongs to.

  .venv-img/bin/python tools/card_set/match_work.py PICTURE... [--works https://ritunia.com/api/v1]
Writes $OUT/new_cards.png (the pictures) and $OUT/works_N.png (every work of the
site with its slug). Look at them side by side and name the slug for each picture.
Works that already have a card in cards.py are marked with a star.
"""
import json
import os
import sys
import urllib.request
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import cards as C  # noqa: E402

F = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", 15)


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 card-set"})
    return urllib.request.urlopen(req).read()


def sheet(files, labels, out, cols, tw, th):
    rows = (len(files) + cols - 1) // cols
    im = Image.new("RGB", (cols * tw, rows * (th + 34)), (24, 22, 20))
    d = ImageDraw.Draw(im)
    for i, (f, label) in enumerate(zip(files, labels)):
        t = Image.open(f).convert("RGB")
        t.thumbnail((tw - 6, th))
        x, y = (i % cols) * tw + 3, (i // cols) * (th + 34)
        im.paste(t, (x, y))
        d.text((x, y + th + 2), f"{i + 1}. {label}"[:34], font=F, fill=(235, 225, 205))
    im.save(out)
    print("written", out)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    works = "https://ritunia.com/api/v1"
    if "--works" in sys.argv:
        works = sys.argv[sys.argv.index("--works") + 1]
        args = [a for a in args if a != works]
    out = Path(os.environ.get("OUT", "/tmp/match_work"))
    (out / "thumbs").mkdir(parents=True, exist_ok=True)
    sheet(args, [Path(a).name for a in args], out / "new_cards.png", min(6, len(args)), 220, 320)
    items = json.loads(fetch(f"{works}/figurines?perPage=500"))["items"]
    taken = {c["work"] for c in C.CARDS}
    files, labels = [], []
    for w in sorted(items, key=lambda w: w["slug"]):
        if not w.get("faceImageUrl"):
            continue
        p = out / "thumbs" / f"{w['slug']}.jpg"
        if not p.exists():
            p.write_bytes(fetch(w["faceImageUrl"]))
        files.append(p)
        labels.append(("★ " if w["slug"] in taken else "") + w["slug"])
    for n in range(0, len(files), 43):
        sheet(files[n:n + 43], labels[n:n + 43], out / f"works_{n // 43}.png", 7, 200, 200)


if __name__ == "__main__":
    main()
