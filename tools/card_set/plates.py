"""Find the two plates drawn on a finished card: name band and text plate.

Returns percent of the card: where the name goes (centre of the band) and where
the effect text goes (centre of the text plate), both horizontally centred.
"""
import numpy as np
from PIL import Image
from scipy import ndimage


def find(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    a = np.asarray(im.resize((w // 4, h // 4)), np.int32)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    tan = (r > 105) & (r < 240) & (r >= g) & (g >= b) & (r - b > 22) & (r - b < 100) & (g > 80)
    hh, ww = tan.shape
    mask = np.zeros_like(tan)
    mask[int(hh * 0.55):, int(ww * 0.12):int(ww * 0.88)] = tan[int(hh * 0.55):, int(ww * 0.12):int(ww * 0.88)]
    mask = ndimage.binary_opening(mask, iterations=2)
    lab, n = ndimage.label(mask)
    if not n:
        return None
    sizes = ndimage.sum(mask, lab, range(1, n + 1))
    body = int(np.argmax(sizes)) + 1
    ys, xs = np.where(lab == body)
    top, bottom = ys.min() / hh * 100, ys.max() / hh * 100
    left, right = xs.min() / ww * 100, xs.max() / ww * 100
    # name band: the next big tan component above the body, else just above it
    band = None
    for i in np.argsort(sizes)[::-1]:
        j = int(i) + 1
        if j == body or sizes[i] < 0.02 * sizes[body - 1]:
            continue
        yy, xx = np.where(lab == j)
        if yy.max() / hh * 100 < top + 1:
            band = (yy.min() / hh * 100 + yy.max() / hh * 100) / 2
            break
    return {
        "body": {"top": round(top, 1), "bottom": round(bottom, 1), "left": round(left, 1), "right": round(right, 1)},
        "band_y": round(band, 1) if band else None,
    }


if __name__ == "__main__":
    import sys
    for f in sys.argv[1:]:
        print(f.split("/")[-1][:20], find(f))
