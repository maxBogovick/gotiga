"""Cut the four stat medallions out of the two generated sheets.

Each sheet is 1536x1024 RGBA, two medallions side by side on transparency.
Writes tools/medals/<name>.png cropped to the medallion (smoke included).
Run: .venv-img/bin/python tools/medals/extract.py <sheet-6> <sheet-7>
"""
import sys
from pathlib import Path
from PIL import Image
import numpy as np

HERE = Path(__file__).parent
NAMES = [("cost", "power"), ("health", "mana")]
for sheet, names in zip(sys.argv[1:3], NAMES):
    im = Image.open(sheet).convert("RGBA")
    a = np.array(im)[..., 3]
    mid = im.width // 2
    for half, name in zip((0, mid), names):
        sub = a[:, half:half + mid]
        ys, xs = np.where(sub > 12)
        box = (half + xs.min(), ys.min(), half + xs.max() + 1, ys.max() + 1)
        out = im.crop(box)
        out.save(HERE / f"{name}.png")
        print(name, out.size)
