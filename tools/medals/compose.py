"""Put stat medallions into the corners of a finished card picture.

The medallions (cost, power, health, mana) come from tools/medals/*.png and are
baked into the card image: they replace the empty sockets drawn on the picture.
The numbers stay live — the card prints them over the panel centres this script
reports, in percent of the card.

  compose.py CARD.png OUT.png cost:tl health:tr power:br [--hue cost=0,power=40]

Corners: tl tr bl br. `--hue name=deg` turns the coloured parts of one medallion
(silver and black stay) so one set serves every race palette.
"""
import colorsys
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).parent
HEIGHT = 400          # medallion height on a 1536-high card
PANEL = (0.5, 0.43)   # centre of the dark panel inside a medallion, as fractions


def turn_hue(img, degrees, sat=1.0):
    if not degrees and sat == 1.0:
        return img
    a = np.asarray(img.convert("RGBA"), np.float32) / 255
    r, g, b, al = a[..., 0], a[..., 1], a[..., 2], a[..., 3]
    mx, mn = a[..., :3].max(-1), a[..., :3].min(-1)
    v, s = mx, np.where(mx > 0, (mx - mn) / np.maximum(mx, 1e-6), 0)
    d = np.maximum(mx - mn, 1e-6)
    h = np.where(mx == r, ((g - b) / d) % 6, np.where(mx == g, (b - r) / d + 2, (r - g) / d + 4)) / 6
    h = (h + degrees / 360) % 1
    # only the coloured parts turn; silver, black and smoke keep their tint
    keep = np.clip((s - 0.15) / 0.25, 0, 1)
    k = (h * 6).astype(int) % 6
    f = h * 6 - np.floor(h * 6)
    s = s * sat
    p, q, t = v * (1 - s), v * (1 - f * s), v * (1 - (1 - f) * s)
    rgb = np.stack([np.choose(k, [v, q, p, p, t, v]), np.choose(k, [t, v, v, q, p, p]), np.choose(k, [p, p, t, v, v, q])], -1)
    out = a[..., :3] * (1 - keep[..., None]) + rgb * keep[..., None]
    return Image.fromarray((np.dstack([out, al]) * 255).astype(np.uint8), "RGBA")


def compose(card_path, out_path, items, hues=None, margin=4, sats=None):
    card = Image.open(card_path).convert("RGBA")
    scale = card.height / 1536
    H = round(HEIGHT * scale)
    result = {}
    for name, corner in items:
        m = turn_hue(Image.open(HERE / f"{name}.png"), (hues or {}).get(name, 0), (sats or {}).get(name, 1.0))
        w = round(m.width * H / m.height)
        m = m.resize((w, H), Image.LANCZOS)
        x = margin if corner[1] == "l" else card.width - margin - w
        y = 0 if corner[0] == "t" else card.height - H
        card.alpha_composite(m, (x, y))
        result[name] = {
            "x": round((x + PANEL[0] * w) / card.width * 100, 2),
            "y": round((y + PANEL[1] * H) / card.height * 100, 2),
        }
    card.convert("RGB").save(out_path)
    return result


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    hues = {}
    if "--hue" in sys.argv:
        for pair in sys.argv[sys.argv.index("--hue") + 1].split(","):
            k, v = pair.split("=")
            hues[k] = float(v)
    items = [a.split(":") for a in args[2:] if ":" in a]
    print(json.dumps(compose(args[0], args[1], items, hues)))
