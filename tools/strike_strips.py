#!/usr/bin/env python3
"""House-drawn melee strips: axe, sword, mace.

Six frames in a row, 256 px each, canvas 1536×256 — the same width the
motion sheet uses, under the 1600 px upload squeeze so joints stay sharp.
Ink on transparent, not on paper: these ship as stock motion art, not as
a sheet to slice. Re-run whenever the silhouettes change.

    .venv-tools/bin/python tools/strike_strips.py
"""

from __future__ import annotations

import math
import os

from PIL import Image, ImageDraw

FRAME = 256
N = 6
W, H = FRAME * N, FRAME
INK = (52, 37, 28, 235)
INK_MID = (111, 59, 36, 210)
INK_FAINT = (52, 37, 28, 90)
EDGE = (42, 28, 22, 255)

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                   "..", "static", "battles", "motion")


def canvas() -> Image.Image:
    return Image.new("RGBA", (W, H), (0, 0, 0, 0))


def rotate(pts: list[tuple[float, float]], cx: float, cy: float, deg: float) -> list[tuple[float, float]]:
    a = math.radians(deg)
    c, s = math.cos(a), math.sin(a)
    out = []
    for x, y in pts:
        dx, dy = x - cx, y - cy
        out.append((cx + dx * c - dy * s, cy + dx * s + dy * c))
    return out


def shift(pts: list[tuple[float, float]], dx: float, dy: float) -> list[tuple[float, float]]:
    return [(x + dx, y + dy) for x, y in pts]


def draw_poly(d: ImageDraw.ImageDraw, pts: list[tuple[float, float]], fill, outline=EDGE, width=2):
    if len(pts) < 3:
        return
    d.polygon(pts, fill=fill, outline=outline)
    if width > 1:
        d.line(pts + [pts[0]], fill=outline, width=width, joint="curve")


def sword_shape() -> list[tuple[float, float]]:
    # Blade pointing right, hilt at left. Local coords around (0, 0).
    return [
        (-78, -7), (-78, 7), (-52, 7), (-48, 22), (-36, 22), (-40, 7),
        (88, 5), (102, 0), (88, -5),
        (-40, -7), (-36, -22), (-48, -22), (-52, -7),
    ]


def axe_shape() -> list[tuple[float, float]]:
    # Haft vertical-ish, bit to the right. Local around (0, 0).
    haft = [
        (-8, -70), (8, -70), (10, 78), (0, 90), (-10, 78),
    ]
    bit = [
        (6, -58), (78, -48), (92, -18), (78, 8), (18, -8), (8, -18),
    ]
    return haft, bit


def mace_shape() -> tuple[list[tuple[float, float]], list[tuple[float, float]]]:
    haft = [(-7, -20), (7, -20), (9, 88), (0, 98), (-9, 88)]
    # Flanged head: a hex plus spikes.
    spikes = []
    for i in range(6):
        a = math.radians(i * 60 - 90)
        spikes.append((math.cos(a) * 36, -36 + math.sin(a) * 36))
        b = math.radians(i * 60 - 60)
        spikes.append((math.cos(b) * 22, -36 + math.sin(b) * 22))
    return haft, spikes


def blot(d: ImageDraw.ImageDraw, cx: float, cy: float, r: float, alpha: int):
    col = (111, 59, 36, alpha)
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=col)


def paint_sword(img: Image.Image):
    d = ImageDraw.Draw(img)
    shape = sword_shape()
    # Frames: enter from top-right, slash across, leave bottom-left.
    poses = [
        (210, 36, -28, 0.55),
        (168, 72, -18, 0.85),
        (128, 118, -8, 1.0),
        (96, 148, 6, 1.0),
        (64, 186, 18, 0.7),
        (28, 226, 28, 0.35),
    ]
    for i, (cx, cy, ang, scale) in enumerate(poses):
        ox = i * FRAME
        pts = [(x * scale, y * scale) for x, y in shape]
        pts = rotate(pts, 0, 0, ang)
        pts = shift(pts, ox + cx, cy)
        fill = INK if scale > 0.6 else INK_FAINT
        draw_poly(d, pts, fill)
        if i in (2, 3):
            # A thin cut across the portrait — the stroke itself.
            x0, y0 = ox + 40, 48
            x1, y1 = ox + 216, 210
            d.line([(x0, y0), (x1, y1)], fill=INK_MID, width=3)


def paint_axe(img: Image.Image):
    d = ImageDraw.Draw(img)
    poses = [
        (188, 40, -55, 0.7),
        (164, 70, -30, 0.9),
        (132, 118, -8, 1.0),
        (120, 150, 8, 1.0),
        (128, 188, 22, 0.75),
        (140, 222, 34, 0.4),
    ]
    for i, (cx, cy, ang, scale) in enumerate(poses):
        ox = i * FRAME
        haft, bit = axe_shape()
        def place(pts):
            p = [(x * scale, y * scale) for x, y in pts]
            p = rotate(p, 0, 0, ang)
            return shift(p, ox + cx, cy)
        draw_poly(d, place(haft), INK if scale > 0.5 else INK_FAINT, width=3)
        draw_poly(d, place(bit), INK_MID if scale > 0.5 else INK_FAINT, width=2)
        if i in (2, 3):
            blot(d, ox + 128, 150, 10 + i * 2, 70)


def paint_mace(img: Image.Image):
    d = ImageDraw.Draw(img)
    poses = [
        (128, 28, -12, 0.75),
        (128, 64, -6, 0.9),
        (128, 110, 0, 1.0),
        (128, 148, 4, 1.0),
        (128, 168, 8, 0.85),
        (128, 196, 12, 0.45),
    ]
    for i, (cx, cy, ang, scale) in enumerate(poses):
        ox = i * FRAME
        haft, head = mace_shape()
        def place(pts):
            p = [(x * scale, y * scale) for x, y in pts]
            p = rotate(p, 0, 0, ang)
            return shift(p, ox + cx, cy)
        draw_poly(d, place(haft), INK if scale > 0.5 else INK_FAINT, width=3)
        draw_poly(d, place(head), INK_MID if scale > 0.5 else INK_FAINT, width=2)
        if i >= 3:
            blot(d, ox + 128, 168, 8 + (i - 3) * 7, 50 + (i - 3) * 20)


def save(name: str, paint):
    img = canvas()
    paint(img)
    os.makedirs(os.path.abspath(OUT), exist_ok=True)
    path = os.path.join(os.path.abspath(OUT), f"{name}.png")
    img.save(path, "PNG")
    print(path, img.size)


def main():
    save("sword", paint_sword)
    save("axe", paint_axe)
    save("mace", paint_mace)


if __name__ == "__main__":
    main()
