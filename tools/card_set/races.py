"""Check the race ring and pick a medallion colour for a race.

  races.py check                 # the ring is one closed cycle over all races
  races.py preview 20 150 300    # cost medallion at these hues, next to the existing races
Output of `preview` goes to $OUT (default /tmp/race_hues.png).
"""
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "medals"))
import cards as C  # noqa: E402


def check(races=C.RACES, ring=C.RACE_RING):
    """Every race strong against exactly one other and weak against exactly one,
    all in a single loop: then no race wins by itself. Raises AssertionError."""
    slugs = [r[0] for r in races]
    assert set(ring) == set(slugs), f"ring keys differ from races: {set(ring) ^ set(slugs)}"
    assert sorted(ring.values()) == sorted(slugs), "every race must be the target of exactly one other"
    assert all(k != v for k, v in ring.items()), "a race cannot be strong against itself"
    seen, cur = [], slugs[0]
    while cur not in seen:
        seen.append(cur)
        cur = ring[cur]
    assert len(seen) == len(slugs), f"the ring is split into several loops: {seen}"
    return seen


def hue_gaps(races=C.RACES):
    """Distance in degrees between the cost-medallion hues of the races, plus the
    fixed colours of the other medallions (health red 0, power amber 38, mana steel 215)."""
    fixed = {"health": 0, "power": 38, "mana": 215}
    out = {r[0]: r[5] for r in races}
    out.update({f"({k})": v for k, v in fixed.items()})
    return out


def preview(hues, out):
    from PIL import Image, ImageDraw, ImageFont
    from compose import turn_hue
    base = Image.open(HERE.parent / "medals" / "cost.png")
    items = [(r[2], r[5]) for r in C.RACES] + [(f"новая {h}°", h) for h in hues]
    h = 330
    w = round(base.width * h / base.height)
    font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", 20)
    sheet = Image.new("RGB", (len(items) * (w + 16) + 16, h + 50), (30, 26, 24))
    d = ImageDraw.Draw(sheet)
    for i, (label, hue) in enumerate(items):
        m = turn_hue(base, hue - 270).resize((w, h), Image.LANCZOS)
        x = 16 + i * (w + 16)
        sheet.paste(m, (x, 10), m)
        d.text((x, h + 18), f"{label} {hue}°"[:22], font=font, fill=(235, 225, 205))
    sheet.save(out)
    print("written", out)


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "check"
    if cmd == "check":
        print("ring is one loop:", " → ".join(check() + [C.RACES[0][0]]))
        for k, v in sorted(hue_gaps().items(), key=lambda kv: kv[1]):
            print(f"  hue {v:3}°  {k}")
    elif cmd == "preview":
        preview([float(a) for a in sys.argv[2:]], os.environ.get("OUT", "/tmp/race_hues.png"))
