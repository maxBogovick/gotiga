"""Find the empty round sockets drawn in the corners of a finished card."""
import numpy as np
from PIL import Image
from scipy import ndimage

WIN = (0.30, 0.24)   # corner window, as fractions of width and height


def find(path):
    im = Image.open(path).convert("RGB")
    a = np.asarray(im, np.int32)
    h, w = a.shape[:2]
    ww, wh = int(w * WIN[0]), int(h * WIN[1])
    out = {}
    for name, (x0, y0) in {"tl": (0, 0), "tr": (w - ww, 0), "bl": (0, h - wh), "br": (w - ww, h - wh)}.items():
        win = a[y0:y0 + wh, x0:x0 + ww]
        dark = win.sum(-1) < 130
        lab, n = ndimage.label(dark)
        best = None
        for i in range(1, n + 1):
            ys, xs = np.where(lab == i)
            bw, bh = xs.max() - xs.min() + 1, ys.max() - ys.min() + 1
            fill = len(xs) / (bw * bh)
            if len(xs) > 0.012 * w * w and 0.75 < bw / bh < 1.33 and fill > 0.65:
                if best is None or len(xs) > best[0]:
                    best = (len(xs), x0 + xs.mean(), y0 + ys.mean())
        out[name] = None if best is None else (round(best[1] / w * 100, 1), round(best[2] / h * 100, 1))
    return out


if __name__ == "__main__":
    import sys
    for f in sys.argv[1:]:
        print(f.split("/")[-1], find(f))
