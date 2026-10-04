#!/usr/bin/env python3
"""House-drawn magic strips: six kinds of spell besides lightning.

    fireball       — flight: a burning ball with a tail of flame
    fireburst      — target: flame bursts, the card chars, smoke rises
    ravens         — flight: three ravens, wings beating
    ravens-strike  — target: they swoop, strike, scatter; feathers fall
    ice-lances     — flight: a volley of three icicles
    frost          — target: shards hit, frost spreads, cracks, crumbles
    soul-vortex    — target: a dark whirl opens over the card and closes
    soul-stream    — beam: a pale stream runs from the target to the caster
    soul-glow      — striker: the caster takes it in
    poison-glob    — flight: a wobbling drop of poison
    poison-cloud   — target: a cloud spreads, drips fall, it thins away
    ghost-arm      — beam: a spectral arm reaches from the caster
    ghost-hand     — target: a bony hand closes on the card; it cracks

Cell strips (flight, target, striker) are drawn on a canvas in the TRUE
proportions of the board cell, 3 wide and 4 tall (256×341), and saved
squashed to 256×256 frames: the stage stretches every frame back to the
cell, so a ring drawn round here lands on the card round. Beam strips are
512×160 frames, the same as lightning (`spell_strips.py`), and borrow its
`Light`: one way to draw light, so the spells look like one hand drew them.

Things that fly and are not round point to the right: angle 0 on the stage
is "toward a target to the right". Ravens are drawn from behind, wings
spread, because their flight does not turn — a raven turned toward a target
on the left would fly upside down.

Every scatter comes from a seeded generator: the PNG is the same on every
run, and a replayed match shows the same thing twice.

    .venv-tools/bin/python tools/magic_strips.py
"""

from __future__ import annotations

import math
import os
import random

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

import vfx
from spell_strips import BH, BW, INK, MID, OUT, S, Light, Tone, haloed, join, rgba

TW, TH = 256, 341  # a cell in true proportions; saved as 256×256
CX, CY = 128, 170


def blank(w: int, h: int) -> Image.Image:
    return Image.new("RGBA", (w * S, h * S), (0, 0, 0, 0))


class Frame:
    """One frame: a soft glow under ink (with the paper halo the action seals
    wear, so ink reads on a dark photograph), light over both, and a top layer
    over everything."""

    def __init__(self, tone: Tone | None = None, w: int = TW, h: int = TH):
        self.w, self.h = w, h
        self.glow = blank(w, h)
        self.gd = ImageDraw.Draw(self.glow)
        self.ink = blank(w, h)
        self.d = ImageDraw.Draw(self.ink)
        self.top = blank(w, h)
        self.td = ImageDraw.Draw(self.top)
        self.light = Light(w, h, tone) if tone else None

    @staticmethod
    def p(x: float, y: float) -> tuple[float, float]:
        return (x * S, y * S)

    def disc(self, d: ImageDraw.ImageDraw, x: float, y: float, r: float, col):
        d.ellipse([(x - r) * S, (y - r) * S, (x + r) * S, (y + r) * S], fill=col)

    def poly(self, d: ImageDraw.ImageDraw, pts, fill, outline=None, width: float = 0):
        d.polygon([self.p(*q) for q in pts], fill=fill)
        if outline and width:
            d.line([self.p(*q) for q in pts + [pts[0]]], fill=outline, width=max(1, int(width * S)), joint="curve")

    def line(self, d: ImageDraw.ImageDraw, pts, col, width: float):
        d.line([self.p(*q) for q in pts], fill=col, width=max(1, int(width * S)), joint="curve")

    def render(self, out_w: int | None, out_h: int | None, *, glow_blur: float = 7, halo: int = 3,
               alpha: float = 1.0) -> Image.Image:
        """Composited at `out_w`×`out_h`; with None, at drawing scale — to be
        laid into a field canvas (`vfx`) of the same size."""
        img = self.glow.filter(ImageFilter.GaussianBlur(glow_blur * S)) if glow_blur else self.glow
        if halo:
            img = haloed(img, self.ink, halo * S)
        else:
            img.alpha_composite(self.ink)
        if self.light:
            img.alpha_composite(self.light.flatten(self.w * S, self.h * S))
        img.alpha_composite(self.top)
        if alpha < 1:
            a = img.getchannel("A").point(lambda v: int(v * alpha))
            img.putalpha(a)
        return img if out_w is None else img.resize((out_w, out_h), Image.LANCZOS)


def cell(f: Frame, **kw) -> Image.Image:
    return f.render(256, 256, **kw)


# ── fire ────────────────────────────────────────────────────────────────
#
# Drawn as a field, not as shapes (`vfx.py`): a density per pixel stirred by
# turbulence and coloured through a heat ramp, dark red at the ragged edge,
# white at the heart. Shapes — a disc, petals of flame — read as a cartoon.

FIRE_RAMP = [
    (0.00, (60, 8, 2, 0)), (0.10, (90, 15, 5, 0)), (0.20, (130, 22, 6, 140)),
    (0.32, (185, 45, 8, 215)), (0.46, (232, 95, 18, 240)), (0.62, (255, 150, 40, 250)),
    (0.78, (255, 205, 105, 255)), (0.92, (255, 240, 190, 255)), (1.10, (255, 253, 240, 255)),
]
SMOKE = (36, 28, 26)
EMBER_HOT = (255, 196, 96)


def streaks(img: Image.Image, segs, color, width: float, glow: float = 4.0):
    """Short bright lines — embers and drops in motion — with their own glow."""
    layer = vfx.canvas(img.width // vfx.SS, img.height // vfx.SS)
    d = ImageDraw.Draw(layer)
    for (x0, y0, x1, y1, a) in segs:
        d.line([(x0 * vfx.SS, y0 * vfx.SS), (x1 * vfx.SS, y1 * vfx.SS)],
               fill=(*color, int(255 * a)), width=max(1, int(width * vfx.SS)))
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(glow * vfx.SS)))
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(glow * vfx.SS * 0.4)))
    img.alpha_composite(layer)


def to_cell(img: Image.Image) -> Image.Image:
    return img.resize((256, 256), Image.LANCZOS)


def fireball() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    hx, hy, R = 176, 170, 21
    dx, dy = X - hx, Y - hy
    behind = np.clip(-dx, 0, None)
    width = R * 0.85 + behind * 0.16
    frames = []
    for i in range(6):
        # The turbulence is sampled stretched along the flight and slid back
        # a little every frame: the flame streams off the ball.
        n = vfx.warped(X * 0.55 + i * 16, Y, 14, 3, 9)
        # The ball's edge is stirred too: a clean disc reads as a sun.
        rough = vfx.warped(X + i * 7, Y - i * 5, 7, 4, 6)
        core = np.clip(1 - np.sqrt(dx ** 2 + dy ** 2) / R * (0.85 + 0.35 * rough), 0, 1)
        trail = np.exp(-(dy / width) ** 2) * np.exp(-behind / 95) * vfx.smooth(6, -4, dx)
        d = np.maximum(core ** 0.5 * 1.15, trail * (0.15 + 1.25 * n))
        # Nothing reaches the frame's left edge: it would be cut straight.
        edge = vfx.smooth(0, 40, X)
        d = d * edge
        smoke = (np.exp(-(dy / (width * 1.25)) ** 2) * vfx.smooth(30, 90, behind)
                 * np.exp(-behind / 170)
                 * vfx.smooth(0.45, 0.75, vfx.warped(X * 0.7 + i * 10, Y, 18, 9, 10))) * edge
        img = vfx.canvas(TW, TH)
        vfx.over(img, vfx.tint(smoke * 0.75, SMOKE))
        fire = vfx.ramp(d, FIRE_RAMP)
        vfx.over(img, vfx.bloom(fire, 0.5, 11, 0.9))
        vfx.over(img, fire)
        rnd = random.Random(70 + i)
        segs = []
        for _ in range(11):
            b = rnd.uniform(25, 150)
            y = hy + rnd.uniform(-1, 1) * (R * 0.85 + b * 0.16) * 1.1
            x = hx - b
            ln = rnd.uniform(4, 10)
            segs.append((x, y, x - ln, y + rnd.uniform(-1.5, 1.5), rnd.uniform(0.5, 1)))
        streaks(img, segs, EMBER_HOT, 1.1, 3)
        frames.append(to_cell(img))
    return join(frames)


FIREBURST_FRAMES = 8


def fireburst() -> Image.Image:
    """The blast, the flame standing up off it, smoke climbing, and the card
    burnt underneath with its edge still glowing."""
    X, Y = vfx.grid(TW, TH)
    cx, cy = 128, 182
    r0 = np.sqrt((X - cx) ** 2 + (Y - cy) ** 2)
    # radius, heat, rise, smoke, char, char glow, shock ring radius, ring
    plan = [
        (34, 1.45, 0, 0.0, 0.0, 0.0, 42, 1.0),
        (62, 1.2, 6, 0.15, 0.35, 0.5, 80, 0.6),
        (78, 1.0, 16, 0.35, 0.75, 1.0, 104, 0.25),
        (74, 0.8, 34, 0.6, 1.0, 1.0, 0, 0),
        (62, 0.6, 52, 0.85, 1.0, 0.85, 0, 0),
        (46, 0.42, 72, 0.85, 1.0, 0.7, 0, 0),
        # The last frame stays on the card until the motion ends: the burn
        # fades out rather than vanishing in one step.
        (30, 0.25, 92, 0.6, 0.75, 0.5, 0, 0),
        (0, 0.0, 112, 0.3, 0.4, 0.3, 0, 0),
    ]
    cn = vfx.fbm(X, Y, 14, 55, 5)
    veins_n = vfx.fbm(X, Y, 8, 66, 4)
    rc = np.sqrt((X - cx) ** 2 + (Y - cy - 8) ** 2)
    v = (1 - rc / 54) + (cn - 0.5) * 0.9
    frames = []
    for i, (R, heat, rise, smoke_a, char_a, glow_a, ring_r, ring_a) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        # The card, burnt: a brown scorch round a black patch whose ragged
        # edge and veins still glow.
        if char_a:
            halo = vfx.smooth(-0.25, 0.0, v) * (1 - vfx.smooth(0, 0.08, v))
            vfx.over(img, vfx.tint(halo * 0.35 * char_a, (95, 48, 20)))
            vfx.over(img, vfx.tint(vfx.smooth(0, 0.08, v) * 0.88 * char_a, (26, 16, 11)))
            rim = np.exp(-((v - 0.02) / 0.05) ** 2)
            veins = vfx.smooth(0.62, 0.7, veins_n) * vfx.smooth(0.1, 0.3, v) * 0.6
            vfx.over(img, vfx.ramp((rim * 0.75 + veins) * glow_a, FIRE_RAMP))
        ecy = cy - rise * 0.45
        n = vfx.warped(X, Y + i * 9, 16, 21, 12)
        if smoke_a:
            scy = ecy - R * 0.55 - rise * 0.5
            rs = np.sqrt(((X - cx) / 1.15) ** 2 + (Y - scy) ** 2)
            sm = (np.clip(1 - rs / (40 + rise * 0.75 + R * 0.3), 0, 1) ** 0.7
                  * vfx.smooth(0.35, 0.75, vfx.warped(X, Y + i * 12, 20, 41, 14)) * smoke_a)
            vfx.over(img, vfx.tint(sm * 0.82, SMOKE))
        if R:
            dy = (Y - ecy) / (1 + rise / 90)
            base = np.clip(1 - np.sqrt((X - cx) ** 2 + dy ** 2) / R, 0, 1)
            d = base ** 0.65 * heat * (0.45 + 0.95 * n)
            if 2 <= i <= 6:
                streak = vfx.fbm(X * 1.6, Y * 0.4 + i * 14, 10, 33, 4)
                column = np.clip(1 - np.abs(X - cx) / (R * 0.95), 0, 1) ** 0.7
                height = np.clip(1 - (ecy - Y) / (R * 1.4 + rise * 0.8), 0, 1) * (Y < ecy + R * 0.2)
                d = np.maximum(d, column * height * vfx.smooth(0.35, 0.8, streak) * heat * 0.95)
            fire = vfx.ramp(d, FIRE_RAMP)
            vfx.over(img, vfx.bloom(fire, 0.45, 14, 1.0))
            vfx.over(img, fire)
        if ring_a:
            # The shock: a ragged wave of heat, not a drawn circle.
            wobble = vfx.fbm(X, Y, 9, 57 + i, 4)
            ring = np.exp(-((r0 * (0.92 + 0.16 * wobble) - ring_r) / 6) ** 2) * ring_a
            ring = ring * vfx.smooth(0.3, 0.7, vfx.fbm(X, Y, 6, 58 + i, 3))
            vfx.over(img, vfx.ramp(ring * 0.7, FIRE_RAMP))
        if i >= 3:
            rnd = random.Random(90)
            segs = []
            for k in range(14):
                x = cx + rnd.uniform(-60, 60)
                y = cy - rnd.uniform(0, 40) - (i - 3) * rnd.uniform(16, 30)
                ln = rnd.uniform(3, 8)
                segs.append((x, y, x + rnd.uniform(-1.5, 1.5), y + ln, max(0.0, 1 - (i - 3) * 0.2)))
            streaks(img, segs, EMBER_HOT, 1.1, 3)
        frames.append(to_cell(img))
    return join(frames)


def framed(X: np.ndarray, Y: np.ndarray, w: float = TW, h: float = TH, m: float = 22) -> np.ndarray:
    """1 inside, 0 at the frame's edge: a field reaching the edge would be cut
    there straight — a seam beside the card."""
    return (vfx.smooth(0, m, X) * vfx.smooth(w, w - m, X)
            * vfx.smooth(0, m, Y) * vfx.smooth(h, h - m, Y))


def soft(rgba: np.ndarray, radius: float) -> Image.Image:
    return vfx.image(rgba).filter(ImageFilter.GaussianBlur(radius * vfx.SS))


# ── ravens ──────────────────────────────────────────────────────────────
#
# Ravens of shadow: crisp silhouettes, each shedding dark smoke, eyes lit.
# Over the target the smoke turns into a whirl, the claws leave gashes that
# glow, the feathers go to smoke.

RAVEN = (18, 14, 22)
EYE = (255, 70, 55)
GASH = (200, 30, 45)
SHADOW_RAMP = [
    (0.00, (12, 8, 14, 0)), (0.15, (12, 8, 14, 0)), (0.30, (18, 12, 22, 140)),
    (0.55, (26, 16, 30, 205)), (0.80, (38, 18, 36, 230)), (1.00, (52, 20, 40, 240)),
]


def raven(f: Frame, cx: float, cy: float, s: float, flap: float, a: float = 1.0):
    """A raven facing us, wings spread, `flap` from −1 (down) to 1 (up).
    Returns where its eyes are."""
    col = rgba(RAVEN, 250 * a)
    span = 50 * s
    for side in (-1, 1):
        tipy = cy - flap * 24 * s
        midy = cy - flap * 13 * s - 4 * s
        pts = [(cx + side * 5 * s, cy - 8 * s), (cx + side * span * 0.3, midy - 9 * s),
               (cx + side * span * 0.62, midy - 8 * s - flap * 4 * s), (cx + side * span, tipy)]
        # The primaries: fingers along the trailing edge, tip inward.
        for k in range(6):
            u = (k + 1) / 7
            y = tipy + (midy - tipy) * u
            x = span * (1 - 0.075 * k)
            pts += [(cx + side * x, y + 11 * s), (cx + side * (x - 0.035 * span), y + 3 * s)]
        pts += [(cx + side * span * 0.38, midy + 12 * s), (cx + side * span * 0.18, midy + 10 * s),
                (cx + side * 5 * s, cy + 7 * s)]
        f.poly(f.d, pts, col)
    f.d.ellipse([*f.p(cx - 7 * s, cy - 12 * s), *f.p(cx + 7 * s, cy + 15 * s)], fill=col)
    f.d.ellipse([*f.p(cx - 6 * s, cy - 23 * s), *f.p(cx + 6 * s, cy - 11 * s)], fill=col)
    f.poly(f.d, [(cx - 2.6 * s, cy - 15 * s), (cx + 2.6 * s, cy - 15 * s), (cx, cy - 6 * s)], col)
    f.poly(f.d, [(cx - 5 * s, cy + 13 * s), (cx + 5 * s, cy + 13 * s), (cx + 11 * s, cy + 29 * s),
                 (cx, cy + 25 * s), (cx - 11 * s, cy + 29 * s)], col)
    return [(cx - 2.7 * s, cy - 18 * s, a), (cx + 2.7 * s, cy - 18 * s, a)]


def eyes(img: Image.Image, spots):
    streaks(img, [(x - 0.6, y, x + 0.6, y, a) for x, y, a in spots], EYE, 2.6, 3.2)


def shadow(img: Image.Image, d: np.ndarray):
    """Shadow smoke with the faintest crimson at its edge."""
    vfx.over(img, vfx.ramp(d, SHADOW_RAMP))
    rim = np.exp(-((d - 0.3) / 0.06) ** 2) * 0.3
    vfx.over(img, soft(vfx.tint(rim, (150, 25, 45)), 2))


FLOCK = [((92, 150), 1.0, 0.0), ((168, 118), 0.82, 1.9), ((150, 214), 0.9, 3.7)]


def ravens() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    frames = []
    for i in range(6):
        img = vfx.canvas(TW, TH)
        f = Frame()
        smoke = np.zeros_like(X)
        spots = []
        for (x, y), s, ph in FLOCK:
            bx, by = x + 4 * math.sin(i + ph), y - 6 * math.sin(i * 1.1 + ph)
            flap = math.cos(i / 6 * math.pi * 2 * 1.5 + ph)
            smoke = np.maximum(smoke, np.exp(-(((X - bx) / (36 * s)) ** 2 + ((Y - by - 12 * s) / (26 * s)) ** 2)))
            spots += raven(f, bx, by, s, flap)
        n = vfx.warped(X, Y - i * 7, 10, 120, 8)
        shadow(img, smoke * (0.2 + 1.0 * n) * edge)
        img.alpha_composite(f.render(None, None, halo=1))
        eyes(img, spots)
        frames.append(to_cell(img))
    return join(frames)


def feather(f: Frame, x: float, y: float, ang: float, length: float, a: float):
    c, s_ = math.cos(ang), math.sin(ang)
    pts = []
    for k in range(9):
        u = k / 8
        w = math.sin(u * math.pi) * length * 0.2
        pts.append((x + c * length * u - s_ * w, y + s_ * length * u + c * w))
    for k in range(7, 0, -1):
        u = k / 8
        w = -math.sin(u * math.pi) * length * 0.13
        pts.append((x + c * length * u - s_ * w, y + s_ * length * u + c * w))
    f.poly(f.d, pts, rgba(RAVEN, 235 * a))


def ravens_strike() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
    # Three claw gashes: a dark core, tapering at both ends, in a crimson glow.
    gashes = []
    for (x0, y0), (x1, y1) in (((94, 140), (146, 210)), ((112, 132), (164, 202)), ((130, 126), (180, 194))):
        dd, t = vfx.segment_distance(X, Y, x0, y0, x1, y1)
        taper = np.clip(np.sin(np.pi * t), 0, 1) ** 0.6
        gashes.append((np.exp(-(dd / (2.0 * taper + 0.2)) ** 2) * taper, np.exp(-(dd / 7) ** 2) * taper))
    core = np.maximum.reduce([g[0] for g in gashes])
    glow = np.maximum.reduce([g[1] for g in gashes])
    # whirl radius, whirl, bird ring, bird angle, flap, birds, gashes, feathers
    plan = [(80, 0.5, 105, 0.0, 0.8, 1, 0, 0), (72, 0.8, 80, 0.7, -0.4, 1, 0, 0),
            (62, 1.0, 50, 1.4, 1.0, 1, 0, 0), (50, 1.0, 16, 2.0, -0.7, 1, 1.0, 0),
            (64, 0.85, 60, 2.6, 0.9, 0.9, 1.0, 0.3), (76, 0.55, 98, 3.1, -0.3, 0.5, 0.85, 0.7),
            (84, 0.3, 0, 0, 0, 0, 0.6, 1.0), (88, 0.12, 0, 0, 0, 0, 0.35, 0.6)]
    feathers = [(110, 150, 0.4, 26), (150, 170, 2.2, 22), (125, 190, 4.0, 24), (98, 185, 5.2, 20),
                (160, 140, 1.3, 21), (140, 120, 3.1, 19)]
    frames = []
    for i, (R, whirl, rb, ang, flap, birds, gash, fall) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if gash:
            vfx.over(img, soft(vfx.tint(glow * 0.85 * gash, GASH), 3))
            vfx.over(img, vfx.tint(core * 0.95 * min(1.0, gash + 0.3), (30, 6, 10)))
        sx, sy = vfx.swirl(X, Y, CX, CY, 3.0 + i * 0.6, 40)
        n = vfx.fbm(sx, sy, 16, 130, 5)
        ring = np.exp(-((r - R) / 26) ** 2)
        smoke = ring * whirl * (0.15 + 1.1 * n)
        f = Frame()
        spots = []
        if birds:
            for k in range(3):
                t = ang + k * math.pi * 2 / 3
                bx, by = CX + math.cos(t) * rb * 0.72, CY + math.sin(t) * rb * 0.85
                spots += raven(f, bx, by, 0.92, flap if k != 1 else -flap, birds)
                smoke = np.maximum(smoke, np.exp(-(((X - bx) / 34) ** 2 + ((Y - by - 10) / 24) ** 2)) * birds * 0.9)
        if fall:
            for k, (x, y, fa, ln) in enumerate(feathers):
                fy = y + (i - 4) * 24 + k * 4
                fx = x + 10 * math.sin(i + k)
                gone = max(0.0, (i - 5) * 0.45)
                feather(f, fx, fy, fa + (i - 4) * 0.5, ln, fall * (1 - gone))
                puff = np.exp(-(((X - fx) / 13) ** 2 + ((Y - fy + 6) / 15) ** 2))
                smoke = np.maximum(smoke, puff * vfx.smooth(0.4, 0.75, n) * gone * 1.2)
        shadow(img, smoke * edge)
        img.alpha_composite(f.render(None, None, halo=1))
        eyes(img, spots)
        frames.append(to_cell(img))
    return join(frames)


# ── ice ─────────────────────────────────────────────────────────────────
#
# Faceted icicles in a trail of freezing mist; on the target a crust of real
# crystal facets (Voronoi cells) grows, cracks, and flies apart cell by cell.

ICE = Tone("ice", (20, 40, 60), (110, 180, 230), (205, 238, 255), (255, 255, 255), (60, 110, 150), shadow_a=35)
ICE_MIST = [(0.0, (200, 230, 255, 0)), (0.2, (195, 228, 252, 0)), (0.4, (205, 232, 255, 100)),
            (0.7, (228, 244, 255, 175)), (1.0, (255, 255, 255, 225))]
ICE_LIGHT = (232, 245, 255)
ICE_DARK = (95, 145, 195)
ICE_EDGE = (40, 80, 125)


def icicle(f: Frame, x: float, y: float, length: float, base: float, ang: float = 0.0, a: float = 1.0):
    """Pointing along `ang` from (x, y), its broken back end. Two facets — lit
    above, in shadow below — a white ridge, a glint at the point."""
    c, s = math.cos(ang), math.sin(ang)

    def r(u, v):
        return (x + c * u - s * v, y + s * u + c * v)
    upper = [r(0, -base / 2), r(length * 0.35, -base * 0.42), r(length, 0), r(0, 0)]
    lower = [r(0, 0), r(length, 0), r(length * 0.35, base * 0.42), r(0, base / 2)]
    f.poly(f.d, upper, rgba(ICE_LIGHT, 240 * a))
    f.poly(f.d, lower, rgba(ICE_DARK, 235 * a))
    outline = [r(0, -base / 2), r(length * 0.35, -base * 0.42), r(length, 0),
               r(length * 0.35, base * 0.42), r(0, base / 2), r(-5, base * 0.2), r(2, 0), r(-4, -base * 0.25)]
    f.line(f.d, outline + [outline[0]], rgba(ICE_EDGE, 255 * a), 1.3)
    f.line(f.d, [r(3, 0), r(length * 0.97, 0)], rgba((255, 255, 255), 230 * a), 1.1)
    f.line(f.d, [r(length * 0.2, -base * 0.3), r(length * 0.45, -base * 0.12)], rgba((255, 255, 255), 160 * a), 0.8)


def star(L: Light, x: float, y: float, r: float, a: float):
    L.stroke([(x - r, y), (x + r, y)], [0.7, 0.7], a)
    L.stroke([(x, y - r), (x, y + r)], [0.7, 0.7], a)


VOLLEY = [((70, 148), 128, 16), ((96, 184), 140, 18), ((52, 214), 110, 14)]


def ice_lances() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    frames = []
    for i in range(6):
        img = vfx.canvas(TW, TH)
        n = vfx.warped(X * 0.6 + i * 14, Y, 10, 140, 8)
        mist = np.zeros_like(X)
        f = Frame(ICE)
        rnd = random.Random(30 + i)
        for k, ((x, y), ln, b) in enumerate(VOLLEY):
            x += 3 * math.sin(i * 1.4 + k)
            behind = np.clip(x + 10 - X, 0, None)
            lat = np.exp(-((Y - y) / (5 + behind * 0.13)) ** 2)
            mist = np.maximum(mist, lat * np.exp(-behind / 70) * vfx.smooth(x + 30, x, X))
            icicle(f, x, y, ln, b)
        for _ in range(7):
            star(f.light, rnd.uniform(10, 120), rnd.uniform(130, 230), rnd.uniform(2.5, 5.5), 0.95)
        mist = mist * (0.2 + 1.1 * n) * edge
        m = vfx.ramp(mist, ICE_MIST)
        vfx.over(img, vfx.tint(mist * 0.25, (40, 70, 100)))
        vfx.over(img, m)
        lances = f.render(None, None, halo=1)
        vfx.over(img, vfx.bloom(np.asarray(lances).astype(np.float32), 0.8, 6, 0.8))
        img.alpha_composite(lances)
        frames.append(to_cell(img))
    return join(frames)


def frost() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
    rng = np.random.default_rng(12)
    t = rng.uniform(0, math.pi * 2, 90)
    rad = np.sqrt(rng.uniform(0, 1, 90)) * 104
    pts = np.stack([CX + np.cos(t) * rad, CY + np.sin(t) * rad], axis=1).astype(np.float32)
    f1, f2, idx = vfx.voronoi(X, Y, pts)
    shade = rng.uniform(0.72, 1.12, len(pts)).astype(np.float32)[idx]
    rim = vfx.smooth(2.4, 0.3, f2 - f1)
    grain = vfx.fbm(X, Y, 4, 141, 3)
    edge_n = vfx.fbm(X, Y, 12, 142, 4)
    base = np.array([172, 212, 244], np.float32)
    stuck = [((58, 150), 0.12), ((50, 186), -0.05), ((66, 214), -0.2)]
    # crust radius, mist, cracks, scatter, alpha
    plan = [(24, 1.0, 0, 0, 1), (52, 0.9, 0, 0, 1), (78, 0.8, 0, 0, 1), (96, 0.7, 0, 0, 1),
            (96, 0.6, 1, 0, 1), (96, 0.45, 1, 1, 0.9), (96, 0.3, 0, 2, 0.5), (96, 0.15, 0, 3, 0.18)]
    frames = []
    for i, (gr, mist_a, cracks, scatter, alpha) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        n = vfx.warped(X + i * 6, Y - i * 4, 18, 143, 12)
        grow = vfx.smooth(gr, gr - 14, r * (0.85 + 0.3 * edge_n))
        crust = np.zeros(X.shape + (4,), np.float32)
        light = shade * (0.9 + 0.2 * grain)
        crust[..., :3] = base * light[..., None] + rim[..., None] * (255 - base)
        if cracks:
            # Some facet edges darken into cracks, the rest burn white.
            dark = (shade < 0.85)[..., None] * rim[..., None]
            crust[..., :3] = crust[..., :3] * (1 - dark * 0.75) + dark * np.array(ICE_EDGE) * 0.75
        crust[..., 3] = grow * (0.62 + 0.33 * rim) * 255 * edge
        mist = vfx.ramp(np.exp(-((r - gr * 0.9) / 34) ** 2) * mist_a * (0.2 + 1.0 * n) * edge, ICE_MIST)
        vfx.over(img, vfx.tint(np.exp(-((r - gr * 0.9) / 40) ** 2) * mist_a * 0.2 * edge, (40, 70, 100)))
        if scatter:
            # The crust flies apart cell by cell, outward from the hit, falling.
            out = np.zeros_like(crust)
            push = {1: 9, 2: 20, 3: 28}[scatter]
            drop = {1: 3, 2: 14, 3: 30}[scatter]
            for k, (px, py) in enumerate(pts):
                mask = idx == k
                if not mask.any():
                    continue
                dx, dy = px - CX, py - CY
                ln = math.hypot(dx, dy) or 1
                ox = int((dx / ln * push * (0.4 + ln / 110)) * vfx.SS)
                oy = int((dy / ln * push * (0.4 + ln / 110) + drop) * vfx.SS)
                ys, xs = np.nonzero(mask)
                ty, tx = ys + oy, xs + ox
                ok = (ty >= 0) & (ty < out.shape[0]) & (tx >= 0) & (tx < out.shape[1])
                out[ty[ok], tx[ok]] = crust[ys[ok], xs[ok]]
            # Shards that flew out fade before the frame edge cuts them.
            crust = out
            crust[..., 3] *= edge * vfx.smooth(118, 88, r)
        crust[..., 3] *= alpha
        vfx.over(img, mist)
        vfx.over(img, vfx.bloom(crust, 0.85, 5, 0.8))
        vfx.over(img, crust)
        f = Frame(ICE)
        if i <= 4:
            for (x, y), a in stuck:
                icicle(f, x, y, 76, 12, a)
        if i == 0:
            f.light.dab(CX, CY, 11, 1.0)
            for k in range(10):
                tt = math.pi * 2 * k / 10
                f.light.stroke([(CX + math.cos(tt) * 10, CY + math.sin(tt) * 10),
                                (CX + math.cos(tt) * 40, CY + math.sin(tt) * 40)], [1.6, 0.2], 0.9)
        if i in (3, 4):
            rnd = random.Random(5)
            for _ in range(5):
                star(f.light, CX + rnd.uniform(-80, 80), CY + rnd.uniform(-80, 80), rnd.uniform(3, 6), 1.0)
        img.alpha_composite(f.render(None, None, halo=1))
        frames.append(to_cell(img))
    return join(frames)


# ── soul ────────────────────────────────────────────────────────────────
#
# A whirlpool that pulls inward (noise sampled on twisted coordinates), a
# stream of souls running to the caster, and the caster's aura taking them.

SOUL_RAMP = [(0.0, (40, 120, 120, 0)), (0.2, (30, 90, 100, 0)), (0.35, (40, 140, 140, 150)),
             (0.55, (80, 200, 190, 220)), (0.75, (170, 250, 235, 245)), (1.0, (240, 255, 252, 255))]
VOID = (12, 5, 20)
DUSK = (32, 12, 48)


def soul_vortex() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
    frames = []
    for i, sc in enumerate([0.35, 0.7, 1, 1, 1, 1, 0.6, 0.25]):
        img = vfx.canvas(TW, TH)
        R = 96 * sc
        # Twisted hard and stretched along the twist: long spiral arms, not
        # a speckled disc.
        sx, sy = vfx.swirl(X, Y, CX, CY, 9 + i * 1.1, 26)
        n = vfx.fbm(sx * 0.6, sy * 0.6, 12, 150, 5)
        body = np.clip(1 - r / R, 0, 1)
        vfx.over(img, vfx.tint(body ** 0.45 * (0.75 + 0.25 * n) * edge, DUSK))
        vfx.over(img, vfx.tint(vfx.smooth(R * 0.34, R * 0.12, r), VOID))
        arms = vfx.smooth(0.46, 0.7, n) * body ** 0.35 * vfx.smooth(R * 0.1, R * 0.3, r) * edge
        rim = np.exp(-((r - R * 0.26) / (2 + 1.5 * sc)) ** 2) * vfx.smooth(0.35, 0.7, n)
        light = vfx.ramp(np.maximum(arms * 1.15, rim * 0.9), SOUL_RAMP)
        vfx.over(img, vfx.bloom(light, 0.45, 9, 1.0))
        vfx.over(img, light)
        frames.append(to_cell(img))
    return join(frames)


def soul_stream() -> Image.Image:
    X, Y = vfx.grid(BW, BH)
    spans = [(420, 500), (250, 500), (60, 500), (30, 500), (30, 500), (30, 500), (30, 260), (30, 90)]
    frames = []
    for i, (x0, x1) in enumerate(spans):
        img = vfx.canvas(BW, BH)
        yc = MID + 10 * np.sin(X * 0.03 + i * 0.9)
        vis = vfx.smooth(x0, x0 + 40, X) * vfx.smooth(x1, x1 - 40, X)
        # Sampled further right every frame: the pattern runs left, to the caster.
        n = vfx.warped(X + i * 34, Y, 12, 160, 10)
        w = 7 + 6 * n
        d = np.exp(-((Y - yc) / w) ** 2) * vis * (0.3 + 0.9 * n)
        for k in range(5):
            sx = 500 - ((i * 70 + k * 100) % 470)
            sy = MID + 10 * math.sin(sx * 0.03 + i * 0.9) + 6 * math.sin(k * 2.1)
            head = np.exp(-(((X - sx) / 6) ** 2 + ((Y - sy) / 5) ** 2))
            tail = np.exp(-np.clip(X - sx, 0, None) / 30) * np.exp(-((Y - sy) / 3.5) ** 2) * (X > sx)
            d = np.maximum(d, np.maximum(head * 1.05, tail * 0.75) * vis)
        vfx.over(img, vfx.tint(np.exp(-((Y - yc) / (w * 1.7)) ** 2) * vis * 0.22, (20, 40, 45)))
        light = vfx.ramp(d, SOUL_RAMP)
        vfx.over(img, vfx.bloom(light, 0.45, 7, 1.0))
        vfx.over(img, light)
        frames.append(img.resize((BW, BH), Image.LANCZOS))
    return join(frames)


def soul_glow() -> Image.Image:
    """The caster takes the souls in: a cold flame flares up round them,
    tongues streaming upward, and dies down."""
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    frames = []
    for i in range(6):
        img = vfx.canvas(TW, TH)
        R = 46 + i * 8
        heat = (1.1, 1.0, 0.85, 0.6, 0.38, 0.18)[i]
        # Taller above the centre than below: the flame stands up.
        ry = np.where(Y < CY, (Y - CY) / 1.9, (Y - CY) * 1.2)
        r = np.sqrt((X - CX) ** 2 + ry ** 2)
        streak = vfx.fbm(X * 1.4, Y * 0.42 + i * 18, 9, 171, 5)
        n = vfx.warped(X, Y + i * 10, 12, 172, 8)
        aura = np.clip(1 - r / R, 0, 1) ** 0.6 * (0.25 + 0.75 * streak) * (0.6 + 0.5 * n)
        hollow = vfx.smooth(R * 0.15, R * 0.45, np.sqrt((X - CX) ** 2 + (Y - CY) ** 2))
        d = aura * (0.35 + 0.65 * hollow) * heat * edge
        light = vfx.ramp(d, SOUL_RAMP)
        vfx.over(img, vfx.bloom(light, 0.45, 10, 1.0))
        vfx.over(img, light)
        frames.append(to_cell(img))
    return join(frames)


# ── poison ──────────────────────────────────────────────────────────────
#
# The same field method as fire, through a ramp of rot: black-green murk at
# the edge, acid light where it is thickest. Under the glow lies a dark murk
# layer — it gives the cloud depth and keeps it readable on pale paper.

POISON_RAMP = [
    (0.00, (10, 30, 8, 0)), (0.12, (15, 40, 10, 0)), (0.22, (22, 60, 14, 150)),
    (0.36, (38, 100, 20, 215)), (0.52, (75, 160, 30, 238)), (0.68, (140, 215, 55, 248)),
    (0.84, (200, 250, 120, 255)), (1.05, (235, 255, 210, 255)),
]
MURK = (16, 32, 12)
ACID = (165, 240, 70)


def poison_glob() -> Image.Image:
    """A swirling orb of venom trailing heavy vapour and dropping acid."""
    X, Y = vfx.grid(TW, TH)
    hx, hy, R = 172, 170, 20
    dx, dy = X - hx, Y - hy
    behind = np.clip(-dx, 0, None)
    width = R * 0.9 + behind * 0.25
    frames = []
    for i in range(6):
        swirl = vfx.warped(X + i * 5, Y - i * 3, 9, 77, 10)
        n = vfx.warped(X * 0.6 + i * 12, Y, 13, 78, 11)
        core = np.clip(1 - np.sqrt(dx ** 2 + dy ** 2) / R, 0, 1) ** 0.6 * (0.6 + 0.6 * swirl)
        trail = np.exp(-(dy / width) ** 2) * np.exp(-behind / 70) * vfx.smooth(6, -4, dx)
        edge = vfx.smooth(0, 40, X)
        d = np.maximum(core * 1.05, trail * (0.1 + 1.0 * n) * 0.8) * edge
        # The murk trails behind only, and stays narrow: in front of the orb
        # and wide of it, it filled the frame and was cut by its edges.
        murk = (np.exp(-(dy / (R * 0.8 + behind * 0.14)) ** 2) * np.exp(-behind / 80)
                * vfx.smooth(4, -12, dx) * edge
                * vfx.smooth(0.4, 0.72, vfx.warped(X * 0.7 + i * 9, Y, 16, 79, 10)))
        img = vfx.canvas(TW, TH)
        vfx.over(img, vfx.tint(murk * 0.7, MURK))
        glow = vfx.ramp(d, POISON_RAMP)
        vfx.over(img, vfx.bloom(glow, 0.5, 10, 0.8))
        vfx.over(img, glow)
        rnd = random.Random(40 + i)
        segs = []
        for k in range(3):
            x = hx - 30 - k * 28 - rnd.uniform(0, 8)
            y = hy + 10 + k * 7 + i * 3
            segs.append((x, y, x - 1, y + 9, 0.9))
        streaks(img, segs, ACID, 1.6, 2.5)
        frames.append(to_cell(img))
    return join(frames)


def skull(X, Y, cx: float, cy: float, k: float):
    """A skull in the cloud — not drawn, but carved from it: where its bone is,
    the vapour is a little thicker, and where its eyes are, there is none."""
    def ell(ex, ey, rx, ry):
        return np.clip(1 - np.sqrt(((X - ex) / rx) ** 2 + ((Y - ey) / ry) ** 2), 0, 1)
    body = np.maximum(ell(cx, cy - 6 * k, 30 * k, 28 * k), ell(cx, cy + 20 * k, 19 * k, 13 * k))
    holes = np.maximum.reduce([
        ell(cx - 12 * k, cy - 2 * k, 8.5 * k, 7.5 * k),
        ell(cx + 12 * k, cy - 2 * k, 8.5 * k, 7.5 * k),
        ell(cx, cy + 11 * k, 3.6 * k, 5.5 * k),
    ])
    return vfx.smooth(0.0, 0.25, body), vfx.smooth(0.0, 0.35, holes)


def poison_cloud() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    cx, cy = 128, 150
    # radius, thickness, skull, stain. The widest edge is 1.2 R from the
    # middle: R stays under 96, or the frame edge cuts the cloud straight.
    plan = [(38, 1.35, 0, 0), (54, 1.0, 0, 0.2), (70, 1.0, 0, 0.45), (80, 0.95, 0.3, 0.7),
            (84, 0.9, 0.75, 0.9), (86, 0.85, 0.9, 1.0), (90, 0.6, 0.4, 0.7), (94, 0.32, 0, 0.4)]
    sn = vfx.fbm(X, Y, 13, 91, 5)
    st_r = np.sqrt((X - cx) ** 2 + (Y - 200) ** 2)
    frames = []
    for i, (R, amt, sk, stain) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        # The stain the venom eats into the card, its edge seething.
        if stain:
            v = (1 - st_r / (40 * stain)) + (sn - 0.5) * 0.9
            inside = vfx.smooth(0, 0.06, v)
            # Not a flat patch: eaten deeper in places, pitted, its edge lit.
            pits = vfx.smooth(0.55, 0.7, vfx.fbm(X, Y, 5, 92, 4))
            vfx.over(img, vfx.tint(inside * (0.62 + 0.3 * sn), (14, 26, 8)))
            vfx.over(img, vfx.ramp(inside * pits * 0.5 * stain, POISON_RAMP))
            vfx.over(img, vfx.ramp(np.exp(-((v - 0.02) / 0.05) ** 2) * 0.62 * stain, POISON_RAMP))
        n = vfx.warped(X + i * 4, Y - i * 3, 24, 88, 16)
        base = np.clip(1 - np.sqrt(((X - cx) / (R * 1.2)) ** 2 + ((Y - cy) / (R * 0.85)) ** 2), 0, 1)
        d = base ** 0.55 * (0.2 + 1.0 * n) * amt * 0.95
        if i == 0:
            # The orb bursts: a hot knot in the middle of the first puff.
            knot = np.clip(1 - np.sqrt((X - cx) ** 2 + (Y - cy) ** 2) / 24 * (0.8 + 0.4 * n), 0, 1)
            d = np.maximum(d, knot * 1.15)
        if sk:
            body, holes = skull(X, Y, cx, cy + 4, R / 84)
            d = d + body * 0.3 * sk * base
            d = d * (1 - holes * 0.9 * sk)
        murk = base ** 0.7 * amt * vfx.smooth(0.3, 0.7, n)
        vfx.over(img, vfx.tint(murk * 0.55, MURK))
        glow = vfx.ramp(d, POISON_RAMP)
        vfx.over(img, vfx.bloom(glow, 0.5, 12, 0.85))
        vfx.over(img, glow)
        rnd = random.Random(7)
        segs = []
        if 3 <= i <= 6:
            for k in range(5):
                x = cx + (k - 2) * 24 + rnd.uniform(-6, 6)
                y = cy + R * 0.55 + (i - 3) * 24 + rnd.uniform(0, 14)
                segs.append((x, y, x, y + 10, 1 if i < 6 else 0.5))
        if segs:
            streaks(img, segs, ACID, 1.5, 2.5)
        frames.append(to_cell(img))
    return join(frames)


# ── ghost ───────────────────────────────────────────────────────────────
#
# Ectoplasm round bones: the bones crisp, the spirit stuff a field that flows
# toward the target and at the end is eaten away into dust. On the card,
# cracks that glow cold.

GHOST = Tone("ghost", (20, 25, 35), (140, 180, 205), (215, 235, 245), (250, 253, 255), (60, 70, 80), shadow_a=35)
GHOST_RAMP = [(0.0, (150, 190, 210, 0)), (0.15, (140, 180, 205, 0)), (0.3, (150, 195, 215, 90)),
              (0.5, (185, 220, 235, 160)), (0.7, (220, 240, 248, 210)), (1.0, (250, 253, 255, 240))]
COLD = (150, 220, 255)
GONE = [0, 0, 0, 0, 0, 0.35, 0.7, 1.0]


def ghost_arm() -> Image.Image:
    X, Y = vfx.grid(BW, BH)
    reach = [0.35, 0.75, 1, 1, 1, 1, 1, 1]
    frames = []
    for i in range(8):
        img = vfx.canvas(BW, BH)
        x0, x1 = 30, 30 + (BW - 50) * reach[i]
        yc = MID + 6 * np.sin(X * 0.03 + i * 0.8)
        u = np.clip((X - x0) / (BW - 50), 0, 1)
        w = 10 - 5.5 * u
        vis = vfx.smooth(x0, x0 + 20, X) * vfx.smooth(x1, x1 - 18, X)
        n = vfx.warped(X - i * 10, Y, 10, 180, 9)
        d = np.exp(-((Y - yc) / (w * (0.8 + 0.5 * n))) ** 2) * vis * (0.35 + 0.8 * n)
        wisps = (vfx.smooth(0.62, 0.82, vfx.fbm(X * 1.2, Y * 0.5 + i * 14, 8, 181, 4))
                 * np.exp(-((Y - (yc - 18)) / 12) ** 2) * vis * 0.6)
        d = vfx.dissolve(np.maximum(d, wisps), X, Y, GONE[i], 182)
        vfx.over(img, vfx.tint(d * 0.2, (40, 55, 70)))
        ecto = vfx.ramp(d, GHOST_RAMP)
        vfx.over(img, vfx.bloom(ecto, 0.6, 6, 0.8))
        vfx.over(img, ecto)
        if GONE[i] < 1:
            f = Frame(GHOST, BW, BH)
            pts = []
            x = x0
            while x <= x1:
                pts.append((x, MID + 6 * math.sin(x * 0.03 + i * 0.8), (x - x0) / (BW - 50)))
                x += 4
            if len(pts) > 1:
                for side in (-1, 1):
                    bone = [(px, py + side * 2.6 * (1 - uu)) for px, py, uu in pts]
                    f.light.stroke(bone, [0.8] * len(bone), 0.8 * (1 - GONE[i]))
            img.alpha_composite(f.render(None, None, halo=0))
        frames.append(img.resize((BW, BH), Image.LANCZOS))
    return join(frames)


def finger(L: Light, base, ang: float, curl: float, toward: float, lens, a: float, bones):
    pts = [base]
    x, y = base
    for ln in lens:
        # Gripping, the fingers go down into the card: seen from above they
        # shorten more than they turn. Turned hard, they met in a point and
        # the hand read as a leaf.
        ang += curl * 0.35 * toward
        ln *= 1 - 0.45 * curl
        x, y = x + math.cos(ang) * ln, y + math.sin(ang) * ln
        pts.append((x, y))
    L.stroke(pts, [3.0, 2.6, 2.1, 1.5][: len(pts)], a)
    for q in pts[1:-1]:
        L.dab(q[0], q[1], 2.4, a * 0.8)
    # A claw: the last bone runs on into a hooked point.
    (px, py), (qx, qy) = pts[-2], pts[-1]
    hook = ang + 0.9 * toward
    tip = (qx + math.cos(hook) * 9, qy + math.sin(hook) * 9)
    L.stroke([(qx, qy), tip], [1.6, 0.2], a)
    bones.append(pts + [tip])


def ghost_hand() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    # A grip, not a fist: fully closed, the fingers crossed into a leaf.
    plan = [(-50, 0, 0.5, 0), (-10, 0, 0.85, 0), (0, 0.3, 0.95, 0), (0, 0.6, 1, 0),
            (0, 0.72, 1, 1), (0, 0.72, 1, 1), (0, 0.72, 1, 0.8), (0, 0.72, 1, 0.45)]
    # The cracks, once: their lines in a mask, so the glow round them is a field.
    rnd = random.Random(21)
    crack_lines = []
    for k in range(6):
        x, y = CX + 6, CY
        t = math.pi * 2 * k / 6 + rnd.uniform(-0.4, 0.4)
        pts = [(x, y)]
        for step in range(6):
            t += rnd.uniform(-0.7, 0.7)
            x += math.cos(t) * rnd.uniform(6, 11)
            y += math.sin(t) * rnd.uniform(6, 11)
            pts.append((x, y))
            if step in (2, 4):
                bt = t + rnd.choice((-1, 1)) * rnd.uniform(0.6, 1.1)
                crack_lines.append([(x, y), (x + math.cos(bt) * 10, y + math.sin(bt) * 10)])
        crack_lines.append(pts)
    mask = Image.new("L", (TW * vfx.SS, TH * vfx.SS), 0)
    md = ImageDraw.Draw(mask)
    for ln in crack_lines:
        md.line([(px * vfx.SS, py * vfx.SS) for px, py in ln], fill=255, width=int(1.3 * vfx.SS))
    crack_core = np.asarray(mask).astype(np.float32) / 255
    crack_glow = np.asarray(mask.filter(ImageFilter.GaussianBlur(4 * vfx.SS))).astype(np.float32) / 255
    frames = []
    for i, (dx, curl, a, cracks) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if cracks:
            vfx.over(img, vfx.tint(np.clip(crack_glow * 3, 0, 1) * 0.75 * cracks, COLD))
            vfx.over(img, vfx.tint(crack_core * 0.9 * cracks, (28, 30, 44)))
        f = Frame(GHOST)
        L = f.light
        bones = []
        sc = 1 - 0.08 * max(0.0, curl - 0.6) / 0.4
        wrist, palm = (52 + dx, CY), (92 + dx, CY)
        bases = [(118 + dx, CY + (y - CY) * sc) for y in (136, 156, 178, 200)]
        for b in bases:
            # A broad palm: each bone starts from its own place on the wrist.
            w0 = (wrist[0] + 4, CY + (b[1] - CY) * 0.4)
            L.stroke([w0, b], [2.0, 2.4], a * 0.7)
            bones.append([w0, b])
        for b, ln, sp in zip(bases, [(26, 18, 13), (30, 20, 14), (28, 19, 13), (22, 15, 11)],
                             [-0.55, -0.18, 0.16, 0.52]):
            finger(L, b, sp, curl, 1 if b[1] < CY else -1, ln, a, bones)
        finger(L, (98 + dx, CY - 26), -0.95, curl, 1, (18, 15), a, bones)
        bones.append([(max(30, wrist[0] - 30), CY + 3), wrist, palm])
        # The ectoplasm: the bones drawn thick and blurred, stirred by noise.
        bm = Image.new("L", (TW * vfx.SS, TH * vfx.SS), 0)
        bd = ImageDraw.Draw(bm)
        for ln in bones:
            bd.line([(px * vfx.SS, py * vfx.SS) for px, py in ln], fill=255, width=int(9 * vfx.SS))
        body = np.asarray(bm.filter(ImageFilter.GaussianBlur(7 * vfx.SS))).astype(np.float32) / 255
        n = vfx.warped(X + i * 3, Y - i * 4, 9, 190, 8)
        d = vfx.dissolve(body * (0.45 + 0.9 * n) * a * edge, X, Y, GONE[i], 191)
        vfx.over(img, vfx.tint(d * 0.22, (40, 55, 70)))
        ecto = vfx.ramp(d, GHOST_RAMP)
        vfx.over(img, vfx.bloom(ecto, 0.6, 6, 0.9))
        vfx.over(img, ecto)
        if GONE[i] < 1:
            hand = f.render(None, None, halo=0, alpha=1 - GONE[i])
            img.alpha_composite(hand)
        frames.append(to_cell(img))
    return join(frames)


def save(name: str, img: Image.Image):
    os.makedirs(os.path.abspath(OUT), exist_ok=True)
    path = os.path.join(os.path.abspath(OUT), f"{name}.png")
    img.save(path, "PNG", optimize=True)
    print(path, img.size)


def main():
    for name, paint in (
        ("fireball", fireball), ("fireburst", fireburst),
        ("ravens", ravens), ("ravens-strike", ravens_strike),
        ("ice-lances", ice_lances), ("frost", frost),
        ("soul-vortex", soul_vortex), ("soul-stream", soul_stream), ("soul-glow", soul_glow),
        ("poison-glob", poison_glob), ("poison-cloud", poison_cloud),
        ("ghost-arm", ghost_arm), ("ghost-hand", ghost_hand),
    ):
        save(name, paint())


if __name__ == "__main__":
    main()
