#!/usr/bin/env python3
"""House-drawn spell strips: lightning from the caster to the target.

One drawing, five tones (`TONES`): warm (the house colours), red, violet,
green, and black. Each tone gives two files:

    lightning[-tone].png — a beam (`whom: beam`): the stage stretches each
                    frame from the middle of the caster (left edge) to the
                    middle of the target (right edge) and turns it toward the
                    target. Frames are 512×160, not the square 256 of the
                    melee strips: a beam across four cells is stretched far
                    wider than a card, and a 256-wide frame blurs there. The
                    file is served from static, not uploaded, so the 1600 px
                    upload squeeze does not apply to it.
    scorch[-tone].png — lies on the target (`whom: target`), 256×256 frames
                    like the melee strips: the hit, the scorch, smoke, a
                    trace. The stage stretches it to the 3:4 cell, so it is
                    drawn squashed to 3/4 in height and lands on the card
                    round.

Black is not a colour of the same light but its inverse: a black channel
with a thin pale rim, in dark smoke; two channels twisting round each other,
and eight frames instead of six. Its motion in `battles.ts` reads the 8.

The zigzag comes from a seeded generator, so the PNG is the same on every
run: a match is replayed, and the second showing must match the first.

    .venv-tools/bin/python tools/spell_strips.py
"""

from __future__ import annotations

import math
import os
import random
from dataclasses import dataclass

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

import vfx

N = 6

INK = (52, 37, 28)
EMBER = (198, 95, 60)
EMBER_DEEP = (111, 59, 36)
WAX = (226, 160, 112)
PAPER = (248, 241, 231)

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                   "..", "static", "battles", "motion")

# Drawn at 2× and reduced once: PIL draws without antialiasing.
S = 2

Pt = tuple[float, float]


def layer(fw: int, fh: int, n: int = N) -> tuple[Image.Image, ImageDraw.ImageDraw]:
    img = Image.new("RGBA", (fw * n * S, fh * S), (0, 0, 0, 0))
    return img, ImageDraw.Draw(img)


def rgba(c, a: float):
    return (*c, max(0, min(255, int(a))))


def haloed(glow: Image.Image, ink: Image.Image, reach: int) -> Image.Image:
    """Ink over a paper halo, over the ember glow.

    A card is sometimes white paper and sometimes a dark photograph; ink alone
    vanishes on the second. The halo is the ink's own silhouette, widened and
    softened — the same light ring the action seals wear on the board.
    """
    a = ink.getchannel("A").filter(ImageFilter.MaxFilter(reach * 2 + 1))
    a = a.filter(ImageFilter.GaussianBlur(reach * 0.6)).point(lambda v: int(v * 0.55))
    halo = Image.new("RGBA", ink.size, (*PAPER, 0))
    halo.putalpha(a)
    glow.alpha_composite(halo)
    glow.alpha_composite(ink)
    return glow


# ── lightning: the beam ─────────────────────────────────────────────────

BW, BH = 512, 160
MID = BH / 2


def zigzag(a: Pt, b: Pt, rough: float, depth: int, rnd: random.Random) -> list[Pt]:
    """Midpoint displacement: a jagged line from a to b, sideways only."""
    pts = [a, b]
    for level in range(depth):
        out = [pts[0]]
        spread = rough / (1.45 ** level)
        for p, q in zip(pts, pts[1:]):
            mx, my = (p[0] + q[0]) / 2, (p[1] + q[1]) / 2
            dx, dy = q[0] - p[0], q[1] - p[1]
            ln = math.hypot(dx, dy) or 1
            off = rnd.uniform(-spread, spread)
            out += [(mx - dy / ln * off, my + dx / ln * off), q]
        pts = out
    # The ends stay where they are; the middle stays inside the frame.
    return [(x, min(BH - 14, max(14, y))) for x, y in pts]


HOT = (255, 251, 242)  # the core: paper heated white, not a cold blue


@dataclass(frozen=True)
class Tone:
    """The colours of one lightning and how thick each layer of it lies.

    shadow — a breath under the light: on parchment a pale stroke needs
             something darker beside it to read at all (for black: the smoke);
    haze   — the wide coloured glow;
    skin   — the bright rim round the channel;
    core   — the channel itself, sharp;
    deep   — the heart of the scorch it leaves.
    """

    name: str
    shadow: tuple[int, int, int]
    haze: tuple[int, int, int]
    skin: tuple[int, int, int]
    core: tuple[int, int, int]
    deep: tuple[int, int, int]
    soot: tuple[int, int, int] = INK
    shadow_r: float = 1.4
    shadow_a: float = 40
    shadow_blur: float = 2
    haze_r: float = 3.8
    haze_a: float = 150
    skin_r: float = 1.5
    skin_blur: float = 1.6
    core_r: float = 0.5
    dark: bool = False


TONES = [
    Tone("", INK, EMBER, WAX, HOT, EMBER_DEEP),
    Tone("red", (60, 12, 12), (205, 35, 35), (255, 125, 105), (255, 243, 237), (110, 18, 18)),
    Tone("violet", (40, 15, 60), (140, 60, 220), (208, 165, 255), (251, 245, 255), (70, 25, 110)),
    Tone("green", (15, 45, 20), (50, 185, 85), (172, 255, 160), (244, 255, 240), (25, 90, 40)),
    # Black: the core is the darkest thing on the card, the rim is the only
    # light in it, and the shadow is smoke wide enough to dim what it crosses.
    Tone("black", (10, 4, 14), (150, 15, 70), (238, 220, 255), (8, 4, 12), (8, 4, 10),
         soot=(14, 8, 16), shadow_r=3.6, shadow_a=85, shadow_blur=5,
         haze_r=2.8, haze_a=160, skin_r=1.1, skin_blur=0.5, core_r=0.62, dark=True),
]


class Light:
    """Four layers of one stroke, each blurred on its own and laid in order:
    shadow, haze, skin, core (see `Tone`).

    One Light is one FRAME: the wide smoke and haze at the caster's end would
    otherwise spill into the previous frame, and the strip would show a cut
    edge on every step.
    """

    def __init__(self, fw: int, fh: int, tone: Tone):
        self.tone = tone
        self.shadow, self.sd = layer(fw, fh, 1)
        self.ember, self.ed = layer(fw, fh, 1)
        self.wax, self.wd = layer(fw, fh, 1)
        self.core, self.cd = layer(fw, fh, 1)

    def dab(self, x: float, y: float, w: float, a: float, *, core: bool = True):
        """One round dab of the stroke at canvas point (x, y), width w."""
        t = self.tone

        def disc(d, r, col):
            d.ellipse([(x - r) * S, (y - r) * S, (x + r) * S, (y + r) * S], fill=col)
        disc(self.sd, w * t.shadow_r, rgba(t.shadow, t.shadow_a * a))
        disc(self.ed, w * t.haze_r, rgba(t.haze, t.haze_a * a))
        disc(self.wd, w * t.skin_r, rgba(t.skin, 210 * a))
        if core:
            disc(self.cd, w * t.core_r, rgba(t.core, 255 * a))

    def spark(self, pts: list[Pt], w: float, a: float):
        """A crackle off the channel: rim and haze only, no core — on black
        lightning these are the only bright lines, and they carry it."""
        t = self.tone
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            steps = max(1, int(math.hypot(x1 - x0, y1 - y0) / 0.8))
            for k in range(steps):
                u = k / steps
                x, y = x0 + (x1 - x0) * u, y0 + (y1 - y0) * u
                for d, r, col in ((self.ed, w * 3.2, rgba(t.haze, 110 * a)),
                                  (self.wd, w * 0.9, rgba(t.skin, 255 * a))):
                    d.ellipse([(x - r) * S, (y - r) * S, (x + r) * S, (y + r) * S], fill=col)

    def smoke(self, x: float, y: float, w: float, a: float):
        """Only the shadow layer: black lightning leaves dark air behind."""
        t = self.tone
        r = w * t.shadow_r
        self.sd.ellipse([(x - r) * S, (y - r) * S, (x + r) * S, (y + r) * S],
                        fill=rgba(t.shadow, t.shadow_a * a))

    def stroke(self, pts: list[Pt], widths: list[float], a: float, *,
               core: bool = True, only_smoke: bool = False):
        """A tapered line: dabs close enough to read as one stroke."""
        for (x0, y0), (x1, y1), w0, w1 in zip(pts, pts[1:], widths, widths[1:]):
            steps = max(1, int(math.hypot(x1 - x0, y1 - y0) / 0.8))
            for k in range(steps):
                u = k / steps
                x, y, w = x0 + (x1 - x0) * u, y0 + (y1 - y0) * u, w0 + (w1 - w0) * u
                if only_smoke:
                    self.smoke(x, y, w, a)
                else:
                    self.dab(x, y, w, a, core=core)

    def flatten(self, out_w: int, out_h: int) -> Image.Image:
        """Out at final size; `join` lays the frames side by side."""
        t = self.tone
        img = self.shadow.filter(ImageFilter.GaussianBlur(t.shadow_blur * S))
        img.alpha_composite(self.ember.filter(ImageFilter.GaussianBlur(9 * S)))
        img.alpha_composite(self.wax.filter(ImageFilter.GaussianBlur(t.skin_blur * S)))
        img.alpha_composite(self.core.filter(ImageFilter.GaussianBlur(0.4 * S)))
        return img.resize((out_w, out_h), Image.LANCZOS)


def join(frames: list[Image.Image]) -> Image.Image:
    w, h = frames[0].size
    out = Image.new("RGBA", (w * len(frames), h), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        out.paste(f, (i * w, 0))
    return out


def tree(a: Pt, b: Pt, rough: float, rnd: random.Random, gen: int = 0) -> list[tuple[list[Pt], int]]:
    """A channel and its branches, and their branches: real lightning forks
    forward, toward where it is going, and every fork is thinner."""
    path = zigzag(a, b, rough, 5 if gen == 0 else 4, rnd)
    out = [(path, gen)]
    if gen < 2:
        heading = math.atan2(b[1] - a[1], b[0] - a[0])
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        for _ in range(4 if gen == 0 else 2):
            i = rnd.randint(int(len(path) * 0.12), int(len(path) * 0.8))
            p = path[i]
            turn = rnd.choice((-1, 1)) * rnd.uniform(0.35, 0.75)
            reach = length * rnd.uniform(0.16, 0.3)
            end = (p[0] + math.cos(heading + turn) * reach, p[1] + math.sin(heading + turn) * reach)
            if not 26 < end[1] < BH - 26:
                # A fork that would leave the frame turns the other way: the
                # frame edge would flatten it into a comb.
                turn = -turn
                end = (p[0] + math.cos(heading + turn) * reach, p[1] + math.sin(heading + turn) * reach)
            end = (end[0], min(BH - 26, max(26, end[1])))
            out += tree(p, end, rough * 0.4, rnd, gen + 1)
    return out


def widths(path: list[Pt], base: float, gen: int, salt: float) -> list[float]:
    """The main channel breathes along its length; a branch tapers to nothing."""
    n = len(path)
    out = []
    for i in range(n):
        u = i / max(1, n - 1)
        if gen == 0:
            out.append(base * (0.78 + 0.22 * math.cos(u * 11 + salt)) * (1.15 - 0.3 * u))
        else:
            out.append(base * (0.55 ** gen) * (1 - u) ** 0.7)
    return out


# ── the glow renderer ───────────────────────────────────────────────────
#
# Lightning is lines of light, so it is drawn as lines into masks and lit by
# blurring them at three widths: a sharp core, a tight bright skin, a wide
# coloured haze. Round dabs of light read as a drawn stroke; stacked blurs
# read as light thrown on what is around it.

class Glow:
    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        size = (w * vfx.SS, h * vfx.SS)
        self.body, self.core, self.spark, self.smoke = (Image.new("L", size, 0) for _ in range(4))
        self.bd, self.cd, self.sd, self.md = (ImageDraw.Draw(m) for m in (self.body, self.core, self.spark, self.smoke))

    @staticmethod
    def _line(d: ImageDraw.ImageDraw, pts, ws, a: float, k: float = 1.0):
        q = vfx.SS
        for (x0, y0), (x1, y1), w0, w1 in zip(pts, pts[1:], ws, ws[1:]):
            w = max(0.5, (w0 + w1) / 2 * k) * q
            d.line([(x0 * q, y0 * q), (x1 * q, y1 * q)], fill=int(255 * a), width=max(1, int(w)))
            r = w / 2
            d.ellipse([x1 * q - r, y1 * q - r, x1 * q + r, y1 * q + r], fill=int(255 * a))

    def stroke(self, pts, ws, a: float, *, core: bool = True, only_smoke: bool = False):
        if only_smoke:
            self._line(self.md, pts, [w * 2.2 for w in ws], a)
            return
        self._line(self.bd, pts, ws, a)
        if core:
            self._line(self.cd, pts, ws, a, 0.42)

    def sparkle(self, pts, w: float, a: float):
        self._line(self.sd, pts, [w] * len(pts), a)

    def dot(self, x: float, y: float, r: float, a: float):
        q = vfx.SS
        self.bd.ellipse([(x - r) * q, (y - r) * q, (x + r) * q, (y + r) * q], fill=int(255 * a))
        r2 = r * 0.5
        self.cd.ellipse([(x - r2) * q, (y - r2) * q, (x + r2) * q, (y + r2) * q], fill=int(255 * a))

    @staticmethod
    def _arr(m: Image.Image, blur: float = 0) -> np.ndarray:
        if blur:
            m = m.filter(ImageFilter.GaussianBlur(blur * vfx.SS))
        return np.asarray(m).astype(np.float32) / 255

    def render(self, tone: Tone, edge: np.ndarray | None = None) -> Image.Image:
        g1, g2, g3 = (self._arr(self.body, b) for b in (1.3, 5, 15))
        s1, s2 = self._arr(self.spark, 1), self._arr(self.spark, 5)
        sm = self._arr(self.smoke, 7)
        k = edge if edge is not None else 1.0

        def lay(img, v, col):
            vfx.over(img, vfx.tint(np.clip(v, 0, 1) * k, col))
        img = vfx.canvas(self.w, self.h)
        if tone.dark:
            lay(img, sm * 1.6 + g3 * 2.6 * 0.8, tone.shadow)
            lay(img, g2 * 2.4 * 0.7, tone.haze)
            lay(img, g1 * 2.4, tone.skin)
            lay(img, s2 * 2 * 0.6, tone.haze)
            lay(img, s1 * 2, tone.skin)
            lay(img, self._arr(self.body, 0.6) * 1.7 - 0.4, tone.core)
        else:
            # A dark breath under the light: on parchment a pale stroke needs
            # something darker beside it, or the white core is paper on paper.
            lay(img, self._arr(self.body, 2.5) * 2.2 * 0.38 + sm * 0.6, tone.shadow)
            lay(img, g3 * 2.4 * 0.8, tone.haze)
            lay(img, g2 * 2.6 * 0.95, tone.haze)
            lay(img, g1 * 2.2, tone.skin)
            lay(img, s2 * 2 * 0.6, tone.haze)
            lay(img, s1 * 2, tone.skin)
            lay(img, self._arr(self.core, 0.4), tone.core)
        return img


def bolt(g: Glow, branches, base: float, a: float, *, upto: float = 1.0, core: bool = True,
         salt: float = 0.0, only_smoke: bool = False):
    for path, gen in branches:
        n = max(2, int(len(path) * upto)) if gen == 0 else len(path)
        if gen > 0 and upto < 1 and path[0][0] > BW * upto:
            continue
        g.stroke(path[:n], widths(path, base, gen, salt)[:n], a * (1 if gen == 0 else 0.85),
                 core=core, only_smoke=only_smoke)


def arcs(g: Glow, path: list[Pt], rnd: random.Random, a: float, count: int = 4):
    """Thin side arcs leaping beside the channel: the air round a stroke is
    not still, and one clean channel reads as a drawn line."""
    for _ in range(count):
        i = rnd.randrange(2, len(path) - 8)
        j = min(len(path) - 1, i + rnd.randrange(4, 9))
        (x0, y0), (x1, y1) = path[i], path[j]
        side = rnd.choice((-1, 1)) * rnd.uniform(8, 20)
        mid = ((x0 + x1) / 2, (y0 + y1) / 2 + side)
        pts = zigzag((x0, y0), mid, 5, 3, rnd)[:-1] + zigzag(mid, (x1, y1), 5, 3, rnd)
        g.sparkle(pts, 0.9, a * rnd.uniform(0.6, 1.0))


def crackle(g: Glow, path: list[Pt], rnd: random.Random, a: float, every: int = 3):
    """Bright crackles off a black channel. Irregular on purpose — evenly
    spaced ticks of one length read as the legs of a centipede."""
    for i in range(2, len(path) - 2):
        if rnd.random() > 1 / every:
            continue
        (x, y), (nx, ny) = path[i], path[i + 1]
        heading = math.atan2(ny - y, nx - x)
        ang = heading + rnd.choice((-1, 1)) * rnd.uniform(0.45, 1.25)
        length = rnd.uniform(10, 32)
        pts = [(x, y)]
        for k in range(1, rnd.choice((3, 4)) + 1):
            u = k / 4
            jog = rnd.uniform(-4, 4)
            pts.append((x + math.cos(ang) * length * u - math.sin(ang) * jog,
                        y + math.sin(ang) * length * u + math.cos(ang) * jog))
        pts = [(px, min(BH - 10, max(10, py))) for px, py in pts]
        g.sparkle(pts, rnd.uniform(1.0, 1.8), a * rnd.uniform(0.7, 1.0))


def paint_lightning(tone: Tone) -> Image.Image:
    return paint_black_lightning(tone) if tone.dark else paint_bright_lightning(tone)


# The beam starts this far inside the caster and stops this far short of the
# target's middle. Not zero: the glow round the hand is wider than the stroke,
# and at the frame edge it would be cut straight — a vertical seam on the card.
EDGE_IN = 34
EDGE_OUT = 26


def beam_edge() -> np.ndarray:
    X, Y = vfx.grid(BW, BH)
    return (vfx.smooth(0, 18, X) * vfx.smooth(BW, BW - 18, X)
            * vfx.smooth(0, 14, Y) * vfx.smooth(BH, BH - 14, Y))


def paint_bright_lightning(tone: Tone) -> Image.Image:
    a, b = (EDGE_IN, MID), (BW - EDGE_OUT, MID)
    first = tree(a, b, 40, random.Random(7))
    second = tree(a, b, 36, random.Random(23))
    edge = beam_edge()
    frames = []
    for i in range(N):
        g = Glow(BW, BH)
        rnd = random.Random(300 + i)
        if i == 0:
            # The leader: a thin stroke feeling its way out of the hand.
            g.dot(EDGE_IN, MID, 5, 0.8)
            bolt(g, first, 1.4, 0.75, upto=0.55, salt=1)
        elif i == 1:
            # The stroke: the whole tree at once, white-hot, the air arcing
            # round it, a flash where it lands.
            g.dot(EDGE_IN, MID, 8, 1.0)
            g.dot(b[0], b[1], 4, 1.0)
            bolt(g, first, 4.6, 1.0, salt=1)
            arcs(g, first[0][0], rnd, 1.0, 5)
        elif i == 2:
            g.dot(EDGE_IN, MID, 7, 0.9)
            g.dot(b[0], b[1], 3.5, 0.9)
            bolt(g, first, 1.6, 0.4, core=False, salt=1)
            bolt(g, second, 4.2, 1.0, salt=4)
            arcs(g, second[0][0], rnd, 0.9, 4)
        elif i == 3:
            bolt(g, first, 2.8, 0.8, salt=1)
            arcs(g, first[0][0], rnd, 0.5, 2)
        elif i == 4:
            bolt(g, second, 1.6, 0.4, core=False, salt=4)
            bolt(g, second, 2.0, 0.35, only_smoke=True, salt=4)
        frames.append(g.render(tone, edge).resize((BW, BH), Image.LANCZOS))
    return join(frames)


BLACK_N = 8


def paint_black_lightning(tone: Tone) -> Image.Image:
    """Eight frames: the leader; one channel; the other; both braided; a
    flicker; the dark air it leaves; the air thinning; nothing."""
    a, b = (EDGE_IN, MID), (BW - EDGE_OUT, MID)
    # Branches are fewer and thicker than on bright lightning: a black
    # hairline reads as a pencil scribble, not as a fork.
    first = [(p, gg) for p, gg in tree(a, b, 34, random.Random(11)) if gg < 2]
    second = [(p, gg) for p, gg in tree(a, b, 34, random.Random(41)) if gg < 2]
    main1, main2 = first[0][0], second[0][0]
    edge = beam_edge()
    frames = []
    for i in range(BLACK_N):
        g = Glow(BW, BH)
        rnd = random.Random(100 + i)
        if i == 0:
            g.dot(EDGE_IN, MID, 7, 0.9)
            bolt(g, first, 2.6, 0.85, upto=0.5, salt=2)
            crackle(g, main1[: len(main1) // 2], rnd, 0.7, every=5)
        elif i == 1:
            g.dot(EDGE_IN, MID, 11, 1.0)
            bolt(g, first, 6.0, 1.0, salt=2)
            crackle(g, main1, rnd, 1.0)
        elif i == 2:
            g.dot(EDGE_IN, MID, 10, 1.0)
            bolt(g, first, 3.0, 0.5, only_smoke=True, salt=2)
            bolt(g, second, 5.6, 1.0, salt=5)
            crackle(g, main2, rnd, 1.0)
        elif i == 3:
            # Both at once: two black channels braided between the hands.
            g.dot(EDGE_IN, MID, 12, 1.0)
            bolt(g, second, 4.6, 0.95, salt=5)
            bolt(g, first, 6.2, 1.0, salt=2)
            crackle(g, main1, rnd, 1.0, every=2)
        elif i == 4:
            bolt(g, first, 3.4, 0.85, salt=2)
            bolt(g, second, 2.6, 0.6, only_smoke=True, salt=5)
            crackle(g, main1, rnd, 0.6, every=4)
        elif i == 5:
            bolt(g, first, 4.0, 0.75, only_smoke=True, salt=2)
            crackle(g, main1, rnd, 0.35, every=6)
        elif i == 6:
            bolt(g, first, 3.6, 0.4, only_smoke=True, salt=2)
        frames.append(g.render(tone, edge).resize((BW, BH), Image.LANCZOS))
    return join(frames)


# ── on the target: a flash and the figure it burns ──────────────────────
#
# A lightning strike leaves a branching figure on the surface it hits — a
# Lichtenberg figure. Here it first burns with light, then stays as a burn.
# Drawn on a canvas in the true proportions of the cell (3:4) and saved
# squashed to a square frame: the stage stretches it back.

TW, TH = 256, 341
CX, CY = 128, 170


def lichtenberg(rnd: random.Random, arms: int = 9, length: float = 92):
    paths = []

    def grow(x, y, ang, length, gen):
        pts, gone = [(x, y)], 0.0
        while gone < length:
            step = rnd.uniform(4, 8)
            ang += rnd.uniform(-0.45, 0.45)
            x, y = x + math.cos(ang) * step, y + math.sin(ang) * step
            gone += step
            pts.append((x, y))
            if gen < 3 and rnd.random() < 0.22:
                grow(x, y, ang + rnd.choice((-1, 1)) * rnd.uniform(0.4, 0.9),
                     (length - gone) * rnd.uniform(0.4, 0.7), gen + 1)
        paths.append((pts, gen))

    for k in range(arms):
        grow(CX, CY, math.pi * 2 * k / arms + rnd.uniform(-0.25, 0.25), length * rnd.uniform(0.7, 1.0), 0)
    return paths


def figure(g: Glow, paths, a: float, core: bool = True):
    for pts, gen in paths:
        w0 = (2.3, 1.5, 1.0, 0.7)[gen]
        ws = [w0 * (1 - 0.7 * k / len(pts)) for k in range(len(pts))]
        g.stroke(pts, ws, a, core=core)


def paint_scorch(tone: Tone) -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = (vfx.smooth(0, 22, X) * vfx.smooth(TW, TW - 22, X)
            * vfx.smooth(0, 22, Y) * vfx.smooth(TH, TH - 22, Y))
    r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
    paths = lichtenberg(random.Random(17))
    burn_mask = Image.new("L", (TW * vfx.SS, TH * vfx.SS), 0)
    bd = ImageDraw.Draw(burn_mask)
    for pts, gen in paths:
        bd.line([(x * vfx.SS, y * vfx.SS) for x, y in pts], fill=255,
                width=max(1, int((2.6, 1.8, 1.2, 0.9)[gen] * vfx.SS)))
    burn = np.asarray(burn_mask.filter(ImageFilter.GaussianBlur(0.8 * vfx.SS))).astype(np.float32) / 255
    char_n = vfx.fbm(X, Y, 9, 18, 4)
    char = np.clip(1 - r / 30, 0, 1) ** 0.7 * (0.6 + 0.6 * char_n)
    # flash, figure alight, burn, smoke
    plan = [(1.0, 1.0, 0.0, 0), (0.45, 0.85, 0.3, 0), (0.0, 0.45, 0.75, 0),
            (0.0, 0.2, 0.9, 0.5), (0.0, 0.0, 0.85, 1.0), (0.0, 0.0, 0.45, 0.4)]
    frames = []
    for i, (flash, lit, burnt, smoke) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if burnt:
            vfx.over(img, vfx.tint(np.clip(burn * 1.4, 0, 1) * 0.8 * burnt * edge, tone.soot))
            vfx.over(img, vfx.tint(char * 0.85 * burnt, tone.deep))
        if smoke:
            wisp = (vfx.smooth(0.55, 0.8, vfx.fbm(X * 1.3, Y * 0.4 + i * 22, 8, 19, 4))
                    * np.clip(1 - np.sqrt((X - CX) ** 2 + ((Y - CY + 30) / 1.8) ** 2) / 70, 0, 1))
            vfx.over(img, vfx.tint(wisp * 0.45 * smoke * edge, tone.soot))
        g = Glow(TW, TH)
        if lit:
            figure(g, paths, lit, core=lit > 0.5)
        vfx.over(img, g.render(tone, edge))
        if flash:
            # The flash is light falling off, not a disc: a drawn disc read
            # as a button sewn on the card.
            vfx.over(img, vfx.tint(np.exp(-(r / (70 * flash)) ** 2) * 0.6 * flash * edge, tone.haze))
            vfx.over(img, vfx.tint(np.exp(-(r / (26 * flash + 4)) ** 2) * 0.9 * edge, tone.skin))
            vfx.over(img, vfx.tint(np.exp(-(r / (11 * flash + 3)) ** 2), tone.core))
        frames.append(img.resize((256, 256), Image.LANCZOS))
    return join(frames)


def save(name: str, img: Image.Image):
    os.makedirs(os.path.abspath(OUT), exist_ok=True)
    path = os.path.join(os.path.abspath(OUT), f"{name}.png")
    img.save(path, "PNG", optimize=True)
    print(path, img.size)


def main():
    for tone in TONES:
        suffix = f"-{tone.name}" if tone.name else ""
        save(f"lightning{suffix}", paint_lightning(tone))
        save(f"scorch{suffix}", paint_scorch(tone))


if __name__ == "__main__":
    main()
