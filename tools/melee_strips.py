#!/usr/bin/env python3
"""House-drawn melee strips: sword, axe, fist, mace — the heavy versions.

Each is one target strip of eight frames: the swing (three frames), the
impact, and what is left (four). The weapon itself is drawn crisp, as shapes
(`Frame`); everything that moves too fast or is not a solid — the smear of
the swing, the flash, sparks, dust, the shock ring — is drawn as a field
(`vfx`), the same way the magic is. Cracks, splinters and cuts are ink with
the paper halo, so they read on a dark photograph as well as on parchment.

    sword-cut   — a diagonal slash: the blade sweeps, a crescent of light
                  behind it; the cut glows white-hot, throws sparks, darkens
    axe-chop    — an overhead chop: the axe falls, bites, a split with
                  cracks opens, splinters fly, dust settles
    fist-blow   — an iron gauntlet in profile with red-hot knuckles: a smear
                  of speed, impact lines, two shock rings, a crater with
                  the print of four knuckles, debris, dust
    mace-crush  — a flanged mace swung in an arc: a crater, sparks of metal,
                  debris, dust

Canvas in the true proportions of the cell (3:4), saved squashed to square
frames: the stage stretches them back. Sword, axe and mace do not turn
toward the target — a slash has no up and down, and an axe turned upside
down would chop from below. The fist does: a punch in profile has to come
from where the striker stands.

    .venv-tools/bin/python tools/melee_strips.py
"""

from __future__ import annotations

import math
import random

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

import vfx
from magic_strips import CX, CY, FIRE_RAMP, TH, TW, Frame, framed, join, save, soft, streaks, to_cell
from spell_strips import Glow, Tone, rgba

N = 8

STEEL = Tone("steel", (20, 24, 32), (120, 160, 220), (205, 225, 255), (255, 255, 255), (40, 40, 50), shadow_a=30)
STEEL_RAMP = [(0.0, (120, 160, 220, 0)), (0.15, (120, 160, 220, 0)), (0.3, (150, 185, 235, 110)),
              (0.55, (200, 222, 250, 190)), (0.8, (235, 245, 255, 235)), (1.0, (255, 255, 255, 250))]
BLUR_RAMP = [(0.0, (90, 90, 100, 0)), (0.2, (90, 90, 100, 0)), (0.45, (120, 120, 130, 90)),
             (0.75, (170, 170, 180, 150)), (1.0, (215, 215, 225, 190))]
SPARK = (255, 214, 140)
DUST = (128, 112, 96)
CRACK = (34, 26, 22)
SPLINTER = (150, 112, 74)
IRON = (62, 60, 70)
IRON_LIT = (170, 172, 186)
IRON_EDGE = (18, 16, 22)
WOOD = (92, 58, 34)


def axes(ang: float):
    """Unit vectors: u along the angle, v a quarter turn clockwise on screen."""
    c, s = math.cos(ang), math.sin(ang)
    return (c, s), (-s, c)


def at(o, u, v, a: float, b: float):
    return (o[0] + u[0] * a + v[0] * b, o[1] + u[1] * a + v[1] * b)


# ── the weapons, as shapes ──────────────────────────────────────────────


def sword(f: Frame, hilt, ang: float, length: float = 118, a: float = 1.0):
    """Hilt at `hilt`, blade along `ang`: two facets, a fuller, a bright edge."""
    u, v = axes(ang)
    w = 9
    blade_l = [at(hilt, u, v, 0, -w / 2), at(hilt, u, v, length - 16, -w / 2), at(hilt, u, v, length, 0), at(hilt, u, v, 0, 0)]
    blade_r = [at(hilt, u, v, 0, 0), at(hilt, u, v, length, 0), at(hilt, u, v, length - 16, w / 2), at(hilt, u, v, 0, w / 2)]
    f.poly(f.d, blade_l, rgba((226, 232, 242), 250 * a))
    f.poly(f.d, blade_r, rgba((132, 144, 164), 250 * a))
    f.line(f.d, [at(hilt, u, v, 6, 0), at(hilt, u, v, length - 26, 0)], rgba((96, 106, 126), 230 * a), 1.4)
    outline = [at(hilt, u, v, 0, -w / 2), at(hilt, u, v, length - 16, -w / 2), at(hilt, u, v, length, 0),
               at(hilt, u, v, length - 16, w / 2), at(hilt, u, v, 0, w / 2)]
    f.line(f.d, outline, rgba(IRON_EDGE, 240 * a), 1.0)
    f.line(f.d, [at(hilt, u, v, 2, -w / 2 + 0.6), at(hilt, u, v, length - 17, -w / 2 + 0.6)], rgba((255, 255, 255), 230 * a), 0.9)
    guard = [at(hilt, u, v, -3, -17), at(hilt, u, v, 3, -17), at(hilt, u, v, 3, 17), at(hilt, u, v, -3, 17)]
    f.poly(f.d, guard, rgba(IRON, 255 * a), rgba(IRON_EDGE, 255 * a), 1.0)
    grip = [at(hilt, u, v, -3, -3), at(hilt, u, v, -24, -3), at(hilt, u, v, -24, 3), at(hilt, u, v, -3, 3)]
    f.poly(f.d, grip, rgba((60, 36, 26), 255 * a), rgba(IRON_EDGE, 255 * a), 0.8)
    pommel = at(hilt, u, v, -28, 0)
    f.disc(f.d, pommel[0], pommel[1], 4.6, rgba(IRON, 255 * a))
    f.disc(f.d, pommel[0] - 1, pommel[1] - 1, 1.6, rgba(IRON_LIT, 220 * a))


AXE_K = 1.45


def axe_edge(pivot, ang: float, length: float = 190, k: float = AXE_K):
    u, v = axes(ang)
    return at(pivot, u, v, length - 24 * k, 56 * k)


def axe(f: Frame, pivot, ang: float, length: float = 190, a: float = 1.0, k: float = AXE_K):
    """A bearded axe on a long haft from `pivot`, the blade on the side it
    swings toward; the head drawn `k` times its plain size."""
    u, v = axes(ang)
    L = length
    hw = 4 * min(k, 1.25)
    haft = [at(pivot, u, v, 30, -hw), at(pivot, u, v, L + 4, -hw), at(pivot, u, v, L + 4, hw), at(pivot, u, v, 30, hw)]
    f.poly(f.d, haft, rgba(WOOD, 255 * a), rgba(IRON_EDGE, 255 * a), 0.9)
    for n in range(3):
        band = L - 58 * k - n * 9
        f.line(f.d, [at(pivot, u, v, band, -hw - 0.5), at(pivot, u, v, band, hw + 0.5)], rgba((40, 26, 18), 255 * a), 1.6)
    head = [(34, -7), (8, -7), (8, 7), (34, 7)]
    blade = [(30, 6), (12, 6), (-4, 30), (6, 50), (24, 58), (44, 60), (62, 52), (48, 30), (38, 8)]
    spike = [(28, -6), (20, -6), (24, -28)]

    def pt(lst):
        return [at(pivot, u, v, L - p * k, q * k) for p, q in lst]
    f.poly(f.d, pt(blade), rgba((92, 96, 108), 255 * a), rgba(IRON_EDGE, 255 * a), 1.1)
    # The ground edge: a lighter band along the cutting edge, its rim white.
    bevel = [(-4, 30), (6, 50), (24, 58), (44, 60), (62, 52), (54, 46), (40, 51), (24, 49), (10, 43), (2, 30)]
    f.poly(f.d, pt(bevel), rgba((196, 202, 214), 250 * a))
    f.line(f.d, pt([(-4, 30), (6, 50), (24, 58), (44, 60), (62, 52)]), rgba((255, 255, 255), 240 * a), 1.4)
    f.poly(f.d, pt(head), rgba(IRON, 255 * a), rgba(IRON_EDGE, 255 * a), 1.0)
    f.poly(f.d, pt(spike), rgba(IRON, 255 * a), rgba(IRON_EDGE, 255 * a), 0.9)


def mace(f: Frame, pivot, ang: float, length: float = 199, a: float = 1.0, k: float = 1.5):
    """A flanged mace: haft from `pivot`, eight flanges round an iron boss.
    Returns the centre of the head."""
    u, v = axes(ang)
    L = length
    haft = [at(pivot, u, v, 40, -4), at(pivot, u, v, L - 10, -4), at(pivot, u, v, L - 10, 4), at(pivot, u, v, 40, 4)]
    f.poly(f.d, haft, rgba((70, 58, 50), 255 * a), rgba(IRON_EDGE, 255 * a), 0.9)
    hx, hy = at(pivot, u, v, L, 0)
    for n in range(8):
        t = ang + math.pi * 2 * n / 8
        c, s = math.cos(t), math.sin(t)
        pts = [(hx + (c * 8 - s * 6) * k, hy + (s * 8 + c * 6) * k), (hx + c * 26 * k, hy + s * 26 * k),
               (hx + (c * 8 + s * 6) * k, hy + (s * 8 - c * 6) * k)]
        f.poly(f.d, pts, rgba(IRON, 255 * a), rgba(IRON_EDGE, 255 * a), 1.0)
        f.line(f.d, [(hx + c * 10 * k, hy + s * 10 * k), (hx + c * 24 * k, hy + s * 24 * k)], rgba(IRON_LIT, 200 * a), 0.9)
    f.disc(f.d, hx, hy, 12 * k, rgba(IRON_EDGE, 255 * a))
    f.disc(f.d, hx, hy, 11 * k, rgba(IRON, 255 * a))
    f.disc(f.d, hx - 3 * k, hy - 3 * k, 4 * k, rgba(IRON_LIT, 230 * a))
    return hx, hy


# ── what an impact leaves, as fields and ink ────────────────────────────


def flash(img, X, Y, x, y, r: float, a: float, warm=(255, 236, 200)):
    d = np.sqrt((X - x) ** 2 + (Y - y) ** 2)
    vfx.over(img, vfx.tint(np.exp(-(d / (r * 2.4)) ** 2) * 0.45 * a, (255, 190, 120)))
    vfx.over(img, vfx.tint(np.exp(-(d / r) ** 2) * 0.95 * a, warm))
    vfx.over(img, vfx.tint(np.exp(-(d / (r * 0.4)) ** 2) * a, (255, 255, 255)))


def shock(img, X, Y, x, y, R: float, a: float, seed: int):
    """A ragged ring of air: pale outside, a dark lip inside, so it reads on
    paper and on a photograph both."""
    d = np.sqrt((X - x) ** 2 + (Y - y) ** 2)
    wob = vfx.fbm(X, Y, 10, seed, 4)
    rr = d * (0.92 + 0.16 * wob)
    gap = vfx.smooth(0.25, 0.6, vfx.fbm(X, Y, 7, seed + 1, 3))
    vfx.over(img, vfx.tint(np.exp(-((rr - R + 5) / 3.5) ** 2) * 0.35 * a * gap, (50, 40, 34)))
    vfx.over(img, vfx.tint(np.exp(-((rr - R) / 4.5) ** 2) * 0.8 * a * gap, (250, 244, 232)))


def dust(img, X, Y, x, y, R: float, a: float, i: int, seed: int, edge):
    d = np.sqrt(((X - x) / 1.25) ** 2 + (Y - y) ** 2)
    n = vfx.warped(X + i * 4, Y - i * 6, 14, seed, 10)
    puff = np.clip(1 - d / R, 0, 1) ** 0.7 * vfx.smooth(0.35, 0.72, n) * a * edge
    vfx.over(img, vfx.tint(puff * 0.6, DUST))
    vfx.over(img, vfx.tint(puff * vfx.smooth(0.6, 0.85, n) * 0.35, (205, 190, 170)))


def crack_lines(rnd: random.Random, x: float, y: float, arms: int, reach: float, spread: float = 0.5,
                rim: float = 0.0, heading: float | None = None, fan: float = math.pi):
    """Cracks starting at a rim round (x, y), not at its centre — a bundle of
    lines out of one point reads as the legs of a spider — and of uneven
    length. With `heading`, they leave in a fan round that direction."""
    lines = []
    for k in range(arms):
        if heading is None:
            t = math.pi * 2 * k / arms + rnd.uniform(-0.35, 0.35)
        else:
            t = heading + rnd.uniform(-fan, fan)
        px, py = x + math.cos(t) * rim * rnd.uniform(0.9, 1.1), y + math.sin(t) * rim * rnd.uniform(0.9, 1.1)
        pts = [(px, py)]
        steps = max(2, int(reach * rnd.uniform(0.35, 1.0) / 9))
        for step in range(steps):
            t += rnd.uniform(-spread, spread)
            px += math.cos(t) * rnd.uniform(6, 11)
            py += math.sin(t) * rnd.uniform(6, 11)
            pts.append((px, py))
            if step in (steps // 3, 2 * steps // 3) and rnd.random() < 0.7:
                bt = t + rnd.choice((-1, 1)) * rnd.uniform(0.6, 1.1)
                lines.append(([(px, py), (px + math.cos(bt) * 12, py + math.sin(bt) * 12)], 0.9))
        lines.append((pts, 1.5))
    return lines


def draw_cracks(f: Frame, lines, a: float, upto: float = 1.0):
    for pts, w in lines:
        n = max(2, int(len(pts) * upto))
        for k in range(n - 1):
            f.line(f.d, pts[k:k + 2], rgba(CRACK, 235 * a), max(0.5, w * (1 - 0.7 * k / max(1, len(pts) - 1))))


class Flying:
    """Sparks or splinters thrown from a point: each has a heading and speed,
    and falls a little more every frame."""

    def __init__(self, rnd: random.Random, x: float, y: float, count: int, aim: float, spread: float,
                 speed=(18, 34), drop: float = 6):
        self.x, self.y, self.drop = x, y, drop
        self.bits = [(aim + rnd.uniform(-spread, spread), rnd.uniform(*speed), rnd.uniform(0.6, 1.0),
                      rnd.uniform(0, math.pi)) for _ in range(count)]

    def where(self, t: float):
        for ang, sp, life, rot in self.bits:
            x = self.x + math.cos(ang) * sp * t
            y = self.y + math.sin(ang) * sp * t + self.drop * t * t
            yield x, y, ang, sp, life, rot

    def sparks(self, img, t: float, a: float):
        segs = []
        for x, y, ang, sp, life, _ in self.where(t):
            tail = sp * 0.35
            fall = self.drop * t * 0.6
            segs.append((x, y, x - math.cos(ang) * tail, y - math.sin(ang) * tail - fall, a * life))
        streaks(img, segs, SPARK, 1.3, 2.6)

    def splinters(self, f: Frame, t: float, a: float):
        for x, y, ang, sp, life, rot in self.where(t):
            r = 2.5 + life * 3
            spin = rot + t * 1.7
            pts = [(x + math.cos(spin + k) * r * m, y + math.sin(spin + k) * r * m)
                   for k, m in ((0, 1.6), (2.2, 0.6), (3.4, 1.0), (4.6, 0.5))]
            f.poly(f.d, pts, rgba(SPLINTER, 240 * a * life), rgba((60, 40, 26), 240 * a * life), 0.8)


def arc_smear(X, Y, centre, r_in: float, r_out: float, a_from: float, a_to: float, a: float,
              ramp, grow_out: bool = True):
    """The smear of a swing: the band a blade sweeps between two angles round
    the hand that holds it, brightest at the leading edge and toward the tip.
    Angles in degrees on screen (y down); a_from is where the swing began."""
    th = np.degrees(np.arctan2(Y - centre[1], X - centre[0])) % 360
    r = np.sqrt((X - centre[0]) ** 2 + (Y - centre[1]) ** 2)
    lo, hi = min(a_from, a_to), max(a_from, a_to)
    inside = vfx.smooth(lo - 1, lo + 1.5, th) * vfx.smooth(hi + 1, hi - 1.5, th)
    lead = np.clip(np.abs(th - a_from) / max(1e-3, abs(a_to - a_from)), 0, 1) ** 1.8
    band = vfx.smooth(r_in, r_in + 12, r) * vfx.smooth(r_out, r_out - 8, r)
    radial = np.clip((r - r_in) / (r_out - r_in), 0, 1) ** (1.2 if grow_out else 0.4)
    d = inside * band * radial * lead * a
    return vfx.ramp(d, ramp), d


# ── sword ───────────────────────────────────────────────────────────────


def sword_cut() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    C = (380, 440)
    Rc = math.hypot(C[0] - CX, C[1] - CY)
    start = 262
    # blade angle, smear from, smear strength, blade, cut glow, cut dark, sparks t
    plan = [(250, 262, 0.7, 1.0, 0, 0, None), (232, 262, 1.0, 1.0, 0, 0, None),
            (214, 262, 1.0, 1.0, 0, 0, None), (200, 250, 0.8, 0.55, 1.0, 0, 0.0),
            (200, 236, 0.35, 0, 0.85, 0.2, 1.0), (200, 236, 0.0, 0, 0.5, 0.6, 2.0),
            (200, 236, 0.0, 0, 0.2, 0.9, 3.0), (200, 236, 0.0, 0, 0.0, 0.6, None)]
    arc = [(C[0] + math.cos(math.radians(t)) * Rc, C[1] + math.sin(math.radians(t)) * Rc)
           for t in np.arange(206, 254, 1.5)]
    rnd = random.Random(4)
    throws = [Flying(rnd, *arc[k], 3, math.radians(rnd.uniform(200, 340)), 0.8, (20, 40), 9)
              for k in range(2, len(arc) - 2, 5)]
    frames = []
    for i, (blade, smear_from, smear_a, blade_a, glow, dark, spark_t) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        f = Frame()
        if dark:
            ws = [1.6 * math.sin(math.pi * k / (len(arc) - 1)) + 0.3 for k in range(len(arc))]
            for (p0, p1, w) in zip(arc, arc[1:], ws):
                f.line(f.d, [p0, p1], rgba((40, 20, 20), 240 * dark), w)
        img.alpha_composite(f.render(None, None, halo=1))
        if smear_a:
            sm, d = arc_smear(X, Y, C, Rc - 55, Rc + 60, smear_from, blade, smear_a, STEEL_RAMP)
            vfx.over(img, vfx.tint(d * 0.25 * edge, (30, 40, 60)))
            sm[..., 3] *= edge
            vfx.over(img, vfx.bloom(sm, 0.7, 6, 0.8))
            vfx.over(img, sm)
        if glow:
            g = Glow(TW, TH)
            ws = [2.6 * math.sin(math.pi * k / (len(arc) - 1)) ** 0.7 + 0.2 for k in range(len(arc))]
            g.stroke(arc, ws, glow)
            vfx.over(img, g.render(STEEL, edge))
        if spark_t is not None:
            for fl in throws:
                fl.sparks(img, 0.35 + spark_t * 0.55, max(0.0, 1 - spark_t * 0.3))
        if blade_a:
            fb = Frame()
            u = (math.cos(math.radians(blade)), math.sin(math.radians(blade)))
            hilt = (C[0] + u[0] * (Rc - 70), C[1] + u[1] * (Rc - 70))
            sword(fb, hilt, math.radians(blade), 130, blade_a)
            img.alpha_composite(fb.render(None, None, halo=1))
        frames.append(to_cell(img))
    return join(frames)


# ── axe ─────────────────────────────────────────────────────────────────


def axe_chop() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    L = 190
    hit = -50
    u, v = axes(math.radians(hit))
    E = (128, 178)
    off = axe_edge((0, 0), math.radians(hit), L)
    P = (E[0] - off[0], E[1] - off[1])
    rnd = random.Random(8)
    # The split runs along the edge; its cracks carry on from its two ends
    # and a few short ones leave its sides.
    head = math.atan2(u[1], u[0])
    split = (crack_lines(rnd, *at(E, u, v, 46, 0), 3, 60, 0.3, 0, head, 0.45)
             + crack_lines(rnd, *at(E, u, v, -46, 0), 3, 60, 0.3, 0, head + math.pi, 0.45)
             + [(pts, 1.0) for pts, _ in crack_lines(rnd, *E, 5, 26, 0.4, 7, head + math.pi / 2, 1.2)]
             + [(pts, 1.0) for pts, _ in crack_lines(rnd, *E, 5, 26, 0.4, 7, head - math.pi / 2, 1.2)])
    chips = Flying(rnd, *E, 12, math.radians(-150), 1.0, (20, 46), 10)
    # axe angle, axe alpha, smear from, smear, impact, split, dust, splinters t, shake
    plan = [(-88, 0.8, -100, 0.6, 0, 0, 0, None, 0), (-72, 1.0, -96, 1.0, 0, 0, 0, None, 0),
            (-58, 1.0, -84, 1.0, 0, 0, 0, None, 0), (hit, 1.0, -64, 0.6, 1.0, 0.6, 0.4, 0.0, 3),
            (hit, 1.0, 0, 0, 0.35, 1.0, 0.8, 1.0, -2), (-62, 0.55, 0, 0, 0, 1.0, 0.9, 2.0, 0),
            (0, 0, 0, 0, 0, 1.0, 0.6, 3.0, 0), (0, 0, 0, 0, 0, 0.8, 0.3, None, 0)]
    frames = []
    for i, (ang, axe_a, sm_from, sm_a, impact, gash, dust_a, chip_t, shake) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        f = Frame()
        if gash:
            # The split: a dark wedge along the edge, torn pale at its lips,
            # cracks running off it.
            g0, g1 = at(E, u, v, -46, 0), at(E, u, v, 46, 0)
            dd, t = vfx.segment_distance(X, Y, *g0, *g1)
            wedge = np.clip(np.sin(np.pi * t), 0, 1) ** 0.5 * 8 + 0.5
            vfx.over(img, vfx.tint(np.exp(-(dd / (wedge + 2.5)) ** 2) * 0.55 * gash * edge, (238, 226, 205)))
            vfx.over(img, vfx.tint(np.exp(-(dd / wedge) ** 4) * 0.95 * gash * edge, (22, 14, 10)))
            draw_cracks(f, split, gash, min(1.0, 0.4 + gash * 0.6))
        if chip_t is not None:
            chips.splinters(f, 0.3 + chip_t * 0.6, max(0.0, 1 - chip_t * 0.28))
        img.alpha_composite(f.render(None, None, halo=1))
        if dust_a:
            dust(img, X, Y, E[0], E[1] - 8, 60 + i * 6, dust_a, i, 210, edge)
        if sm_a:
            sm, d = arc_smear(X, Y, P, L - 70, L + 66, sm_from, ang, sm_a, BLUR_RAMP)
            sm[..., 3] *= edge
            vfx.over(img, sm)
        if impact:
            shock(img, X, Y, *E, 34 + (1 - impact) * 40, impact, 220)
            flash(img, X, Y, *E, 16, impact)
        if axe_a:
            fa = Frame()
            pv = (P[0] + shake, P[1] + abs(shake))
            axe(fa, pv, math.radians(ang), L, axe_a)
            img.alpha_composite(fa.render(None, None, halo=1))
        frames.append(to_cell(img))
    return join(frames)


# ── fist ────────────────────────────────────────────────────────────────
#
# A punch in profile, from the striker's side: an iron gauntlet with
# red-hot knuckles in an ember aura, a smear of speed behind it. On the blow
# a white flash, sharp dark impact lines closing on the point, two shock
# rings; on the card a crater with the print of four knuckles, cracks from
# its rim, debris and dust. This strip DOES turn toward the target — a
# profile punch has to come from where the striker stands.

FIST_K = 1.6
KNUCKLE_HOT = (255, 170, 80)


def gauntlet_side(f: Frame, fx: float, fy: float, s: float = FIST_K, a: float = 1.0, mask=None):
    """An iron fist in profile, knuckles to the right. Returns the points of
    its four knuckle spikes. With `mask` (an ImageDraw on an L image), the
    silhouette is drawn there too — the aura is made from it."""
    edge = rgba(IRON_EDGE, 255 * a)

    def P(x, y):
        return (fx + x * s, fy + y * s)

    def poly(pts, fill, w=1.1):
        f.poly(f.d, [P(*q) for q in pts], rgba(fill, 255 * a), edge, w)
        if mask is not None:
            mask.polygon([(p[0] * vfx.SS, p[1] * vfx.SS) for p in (P(*q) for q in pts)], fill=255)

    def rrect(x0, y0, x1, y1, r, fill):
        f.d.rounded_rectangle([f.p(*P(x0, y0)), f.p(*P(x1, y1))], radius=r * s * 2,
                              fill=rgba(fill, 255 * a), outline=edge, width=2)
        if mask is not None:
            mask.rounded_rectangle([(c * vfx.SS) for c in (*P(x0, y0), *P(x1, y1))], radius=r * s * vfx.SS, fill=255)

    # The bracer, plated, and the flared cuff at the wrist.
    poly([(-110, -15), (-44, -19), (-44, 19), (-110, 15)], (48, 46, 54))
    for k in range(3):
        x = -96 + k * 18
        f.line(f.d, [P(x, -16), P(x, 16)], rgba(IRON_LIT, 110 * a), 1.0)
        f.disc(f.d, *P(x + 6, -11), 1.4 * s, rgba(IRON_LIT, 200 * a))
    f.line(f.d, [P(-108, -13), P(-46, -17)], rgba(IRON_LIT, 170 * a), 1.2)
    poly([(-46, -26), (-32, -29), (-32, 29), (-46, 26)], (58, 56, 66))
    # The back of the hand, then the curled fingers end-on at the front.
    rrect(-34, -27, 12, 27, 8, IRON)
    f.line(f.d, [P(-30, -24), P(8, -24)], rgba(IRON_LIT, 200 * a), 1.3)
    spikes = []
    for k in range(4):
        y0 = -27 + k * 13.5
        rrect(2, y0, 32, y0 + 13, 5, (82, 80, 94))
        f.line(f.d, [P(6, y0 + 3), P(28, y0 + 3)], rgba(IRON_LIT, 210 * a), 1.0)
        cy = y0 + 6.5
        poly([(30, cy - 3.5), (30, cy + 3.5), (41, cy)], (100, 98, 112), 0.9)
        spikes.append(P(36, cy))
    # The thumb wrapped across the front, below.
    rrect(-12, 15, 20, 30, 6, (72, 70, 84))
    f.line(f.d, [P(-8, 18), P(16, 18)], rgba(IRON_LIT, 190 * a), 1.1)
    return spikes


def impact_lines(f: Frame, rnd: random.Random, cx: float, cy: float, r_in: float, r_out: float,
                 count: int, a: float):
    """Sharp dark spikes closing on the point of the blow, of uneven length
    and width, with gaps: the drawn speed of the impact itself."""
    for k in range(count):
        if rnd.random() < 0.18:
            continue
        t = math.pi * 2 * k / count + rnd.uniform(-0.08, 0.08)
        ri = r_in * rnd.uniform(0.9, 1.4)
        ro = r_out * rnd.uniform(0.6, 1.0)
        w = rnd.uniform(0.008, 0.022)
        pts = [(cx + math.cos(t) * ri, cy + math.sin(t) * ri),
               (cx + math.cos(t - w) * ro, cy + math.sin(t - w) * ro),
               (cx + math.cos(t + w) * ro, cy + math.sin(t + w) * ro)]
        f.poly(f.d, pts, rgba((28, 18, 16), 235 * a))


def fist_blow() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    s = FIST_K
    hit_x = CX - 41 * s  # the knuckles' front lands on the middle
    fy = CY
    rnd = random.Random(12)
    knuckles = [(CX - 2, fy + (-20.5 + k * 13.5) * s) for k in range(4)]
    cracks = crack_lines(rnd, CX + 4, CY, 12, 84, 0.35, rim=44)
    chips = Flying(rnd, CX, CY, 16, 0.0, 1.9, (22, 50), 9)
    sparks = Flying(rnd, CX, CY, 16, 0.0, 1.3, (30, 60), 10)
    r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
    # fist x, fist alpha, heat, trail, impact, spikes, rings, crater, dust, flying t
    plan = [(hit_x - 40, 0.9, 0.6, 1.0, 0, 0, (), 0, 0, None),
            (hit_x - 20, 1.0, 0.8, 1.0, 0, 0, (), 0, 0, None),
            (hit_x - 6, 1.0, 1.0, 0.8, 0, 0, (), 0, 0, None),
            (hit_x, 1.0, 1.0, 0.3, 1.0, 1.0, (40,), 0.8, 0.4, 0.0),
            (hit_x - 6, 1.0, 0.7, 0, 0.35, 0.5, (78, 50), 1.0, 0.8, 1.0),
            (hit_x - 22, 0.6, 0.35, 0, 0, 0, (104,), 1.0, 0.95, 2.0),
            (0, 0, 0, 0, 0, 0, (), 1.0, 0.65, 3.0),
            (0, 0, 0, 0, 0, 0, (), 0.8, 0.3, None)]
    crater_n = vfx.fbm(X, Y, 8, 280, 4)
    frames = []
    for i, (fx, fa, heat, trail, impact, spikes, rings, crater, dust_a, fly_t) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if crater:
            rr = r * (0.9 + 0.2 * crater_n)
            bowl = np.clip(1 - rr / 42, 0, 1) ** 0.8
            shade = np.clip(0.5 + (X - CX) / 70, 0, 1)
            vfx.over(img, vfx.tint(np.exp(-((rr - 46) / 7) ** 2) * 0.4 * crater, (110, 80, 55)))
            vfx.over(img, vfx.tint(bowl * (0.35 + 0.35 * shade) * crater, (34, 26, 20)))
            # The print of the four knuckles at the bottom of the crater,
            # still glowing just after the blow.
            for kx, ky in knuckles:
                d = np.sqrt(((X - kx) / 0.8) ** 2 + (Y - ky) ** 2)
                vfx.over(img, vfx.tint(np.clip(1 - d / 7, 0, 1) ** 0.6 * 0.85 * crater, (18, 12, 10)))
                glow = {3: 1.0, 4: 0.8, 5: 0.5, 6: 0.25}.get(i, 0)
                if glow:
                    vfx.over(img, vfx.tint(np.exp(-((d - 6.5) / 1.8) ** 2) * glow, KNUCKLE_HOT))
            vfx.over(img, vfx.tint(np.exp(-((rr - 43) / 2.5) ** 2) * 0.55 * crater * (1 - shade), (245, 236, 220)))
        f = Frame()
        if crater:
            draw_cracks(f, cracks, crater)
        if fly_t is not None:
            chips.splinters(f, 0.3 + fly_t * 0.6, max(0.0, 1 - fly_t * 0.28))
        if spikes:
            impact_lines(f, random.Random(70 + i), CX, CY, 50 + (1 - spikes) * 40, 118, 44, spikes)
        img.alpha_composite(f.render(None, None, halo=1))
        if dust_a:
            dust(img, X, Y, CX - 8, CY, 72 + i * 7, dust_a, i, 290, edge)
        if fly_t is not None and fly_t < 2.5:
            sparks.sparks(img, 0.35 + fly_t * 0.5, max(0.0, 1 - fly_t * 0.4))
        for R in rings:
            shock(img, X, Y, CX, CY, R, impact if impact else 0.6, 300 + int(R))
        if trail and fa:
            # The smear of speed behind the fist: streaked, darker at its far
            # end, fading to nothing before the frame's edge.
            back = np.clip(fx + 10 * s - X, 0, None)
            lat = np.exp(-((Y - fy) / (30 * s)) ** 2)
            streak = vfx.smooth(0.35, 0.75, vfx.fbm(X * 0.12 + i * 3, Y * 2.2, 6, 310, 3))
            d = lat * np.exp(-back / 70) * vfx.smooth(fx + 12 * s, fx - 8 * s, X) * (0.35 + 0.8 * streak) * trail * edge
            vfx.over(img, vfx.ramp(d, BLUR_RAMP))
        if fa:
            fg = Frame()
            mask = Image.new("L", (TW * vfx.SS, TH * vfx.SS), 0)
            md = ImageDraw.Draw(mask)
            spikes_at = gauntlet_side(fg, fx, fy, s, fa, md)
            if heat:
                # The ember aura round the fist, from its own silhouette.
                body = np.asarray(mask.filter(ImageFilter.GaussianBlur(7 * vfx.SS))).astype(np.float32) / 255
                n = vfx.warped(X - i * 8, Y, 9, 320, 8)
                aura = body * (0.3 + 0.7 * n) * heat * edge
                fire = vfx.ramp(aura * 0.75, FIRE_RAMP)
                vfx.over(img, vfx.bloom(fire, 0.4, 8, 0.8))
                vfx.over(img, fire)
            fist = np.asarray(fg.render(None, None, halo=1)).astype(np.float32)
            # The bracer dissolves into the smear instead of being cut by
            # the frame's edge.
            fist[..., 3] *= vfx.smooth(6, 46, X) * edge
            vfx.over(img, fist)
            if heat:
                streaks(img, [(x - 0.8, y, x + 0.8, y, heat) for x, y in spikes_at], KNUCKLE_HOT, 3.0, 3.5)
        if impact:
            flash(img, X, Y, CX, CY, 24, impact)
        frames.append(to_cell(img))
    return join(frames)


# ── mace ────────────────────────────────────────────────────────────────


def mace_crush() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    P = (290, 280)
    H = (128, 165)
    L = math.hypot(H[0] - P[0], H[1] - P[1])
    hit = math.degrees(math.atan2(H[1] - P[1], H[0] - P[0])) % 360
    rnd = random.Random(16)
    cracks = crack_lines(rnd, *H, 13, 80, 0.4, rim=34)
    sparks = Flying(rnd, *H, 14, math.radians(-90), 2.2, (26, 52), 12)
    chips = Flying(rnd, *H, 10, math.radians(-110), 1.6, (16, 36), 9)
    r = np.sqrt((X - H[0]) ** 2 + (Y - H[1]) ** 2)
    # mace angle, alpha, smear from, smear, impact, crater, dust, flying t, shake
    plan = [(hit + 50, 0.85, hit + 64, 0.6, 0, 0, 0, None, 0), (hit + 32, 1.0, hit + 60, 1.0, 0, 0, 0, None, 0),
            (hit + 14, 1.0, hit + 46, 1.0, 0, 0, 0, None, 0), (hit, 1.0, hit + 34, 0.6, 1.0, 0.7, 0.4, 0.0, 3),
            (hit, 1.0, 0, 0, 0.35, 1.0, 0.85, 1.0, -2), (hit + 14, 0.55, 0, 0, 0, 1.0, 0.9, 2.0, 0),
            (0, 0, 0, 0, 0, 1.0, 0.6, 3.0, 0), (0, 0, 0, 0, 0, 0.8, 0.3, None, 0)]
    crater_n = vfx.fbm(X, Y, 8, 250, 4)
    frames = []
    for i, (ang, ma, sm_from, sm_a, impact, crater, dust_a, fly_t, shake) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if crater:
            rr = r * (0.9 + 0.2 * crater_n)
            bowl = np.clip(1 - rr / 34, 0, 1) ** 0.7
            shade = np.clip(0.5 + (Y - H[1]) / 50, 0, 1)
            vfx.over(img, vfx.tint(np.exp(-((rr - 38) / 7) ** 2) * 0.4 * crater, (110, 80, 55)))
            vfx.over(img, vfx.tint(bowl * (0.5 + 0.4 * shade) * crater, (30, 22, 18)))
            vfx.over(img, vfx.tint(np.exp(-((rr - 35) / 2.5) ** 2) * 0.55 * crater * (1 - shade), (245, 236, 220)))
        f = Frame()
        if crater:
            draw_cracks(f, cracks, crater)
        if fly_t is not None:
            chips.splinters(f, 0.3 + fly_t * 0.6, max(0.0, 1 - fly_t * 0.28))
        img.alpha_composite(f.render(None, None, halo=1))
        if dust_a:
            dust(img, X, Y, H[0], H[1] - 6, 66 + i * 6, dust_a, i, 260, edge)
        if fly_t is not None and fly_t < 2.5:
            sparks.sparks(img, 0.35 + fly_t * 0.5, max(0.0, 1 - fly_t * 0.4))
        if sm_a:
            sm, d = arc_smear(X, Y, P, L - 40, L + 34, sm_from, ang, sm_a, BLUR_RAMP)
            sm[..., 3] *= edge
            vfx.over(img, sm)
        if impact:
            shock(img, X, Y, *H, 40 + (1 - impact) * 44, impact, 270)
            flash(img, X, Y, *H, 18, impact)
        if ma:
            fm = Frame()
            mace(fm, (P[0] + shake, P[1] + abs(shake)), math.radians(ang), L, ma)
            img.alpha_composite(fm.render(None, None, halo=1))
        frames.append(to_cell(img))
    return join(frames)


def main():
    for name, paint in (("sword-cut", sword_cut), ("axe-chop", axe_chop),
                        ("fist-blow", fist_blow), ("mace-crush", mace_crush)):
        save(name, paint())


if __name__ == "__main__":
    main()
