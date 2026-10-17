"""Draw where the name and the effect text will stand on card pictures.

  .venv-img/bin/python tools/card_set/check_layout.py [slug ...]   # default: all
Writes $OUT (default /tmp/layout_check.png): the picture with medallions, a red
frame where the name goes, a green frame where the effect text goes. If a frame
misses its plate, add the picture to HAND in layout.py.
"""
import os
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_images  # noqa: E402
import cards as C  # noqa: E402
import layout  # noqa: E402


def main():
    only = set(sys.argv[1:])
    tmp = Path(tempfile.mkdtemp())
    build_images.build(tmp, only or None)
    cs = [c for c in C.CARDS if not only or c["slug"] in only]
    cols, W = min(6, len(cs)), 240
    rows = (len(cs) + cols - 1) // cols
    H = 350
    sheet = Image.new("RGB", (cols * W, rows * H), (24, 22, 20))
    d = ImageDraw.Draw(sheet)
    f = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", 13)
    for i, c in enumerate(cs):
        t = Image.open(tmp / f"{c['slug']}.png").convert("RGB")
        t = t.resize((W - 4, int((W - 4) * t.height / t.width)))
        x, y = (i % cols) * W + 2, (i // cols) * H
        sheet.paste(t, (x, y))
        ny, ey = layout.of(c)
        d.rectangle((x + t.width * 0.27, y + t.height * (ny - 2.5) / 100, x + t.width * 0.73, y + t.height * (ny + 2.5) / 100), outline=(255, 60, 60))
        d.rectangle((x + t.width * 0.27, y + t.height * (ey - 6) / 100, x + t.width * 0.73, y + t.height * (ey + 6) / 100), outline=(60, 255, 60))
        d.text((x, y + t.height + 2), c["slug"], font=f, fill=(235, 225, 205))
    out = os.environ.get("OUT", "/tmp/layout_check.png")
    sheet.save(out)
    print("written", out)


if __name__ == "__main__":
    main()
