"""Where the name and the effect go on each picture, in percent of the card.

Measured by `plates.py`, then checked by eye on an overlay; the three pictures
with dark plates (plates.py looks for parchment) are written by hand.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cards as C  # noqa: E402
import plates  # noqa: E402

SRC = Path.home() / "Downloads" / "рамки"
# file stem -> (name y, effect y)
HAND = {
    "vampir-funny": (71.6, 84.5),
    "vampir_2": (71.5, 84.5),
    "witch_2": (71.8, 84.5),
}


def of(card):
    stem = Path(card["file"]).stem
    if stem in HAND:
        return HAND[stem]
    p = plates.find(SRC / card["file"])
    top, bottom = p["body"]["top"], p["body"]["bottom"]
    name_y = p["band_y"] if p["band_y"] and top - 9 < p["band_y"] < top - 1 else top - 4.6
    return round(float(name_y), 1), round(float((top + bottom) / 2 + 0.6), 1)


if __name__ == "__main__":
    for c in C.CARDS:
        print(c["slug"], of(c))
