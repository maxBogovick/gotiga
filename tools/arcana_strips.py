#!/usr/bin/env python3
"""House-drawn strips for the rest of a battle: three more attacks, two
spells, and the moments that are not attacks at all.

    meteor        — target: a burning stone falls on the card; a blast, a
                    molten crater, rocks, a column of smoke
    chains        — target: a rift opens on the card, chains burst out of
                    it, cross over the card, tighten, and sink back
    reaper        — target: a spectral scythe sweeps across in a crescent
                    of cold light; the soul is pulled half out and snaps back
    shadow-spikes — target: a pool of shadow, black crystal spikes erupt
                    through the card and shatter
    bats          — flight: a swarm of bats
    bats-swarm    — target: they circle, cover the card, bite, disperse
    healing       — target (mend): a column of gold light, runes rising,
                    feathers falling, the cracks sealed with gold
    summoning     — striker (arrive): a circle of fire draws itself, a
                    pillar of flame, the card stands up in it
    death         — target (fall): the card burns from its edges in, ash
                    rises, a pale wisp of soul leaves upward
    ward          — target (spell, a shield cast): a rune shield blooms,
                    a blow ripples across it, it settles into the card

The same method as `magic_strips.py` and `melee_strips.py`: solid things
are shapes, everything that glows, smokes or moves too fast is a field.
Cell strips are drawn in the true 3:4 proportions of the cell and saved
squashed to square frames.

    .venv-tools/bin/python tools/arcana_strips.py
"""

from __future__ import annotations

import math
import random

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

import vfx
from magic_strips import (CX, CY, FIRE_RAMP, GHOST_RAMP, SHADOW_RAMP, SMOKE, TH, TW, Frame,
                          framed, join, save, soft, streaks, to_cell)
from melee_strips import Flying, IRON_EDGE, crack_lines, dust, flash, shock
from spell_strips import Glow, Tone, rgba

N = 8

GOLD = Tone("gold", (60, 40, 10), (230, 170, 60), (255, 225, 140), (255, 252, 235), (120, 80, 20), shadow_a=25)
AZURE = Tone("ward", (15, 25, 40), (90, 150, 230), (190, 220, 255), (255, 255, 255), (40, 60, 90), shadow_a=25)
EMBERS = Tone("embers", (50, 20, 10), (220, 90, 30), (255, 190, 100), (255, 248, 225), (90, 30, 10), shadow_a=25)
CRIMSON = Tone("crimson", (30, 6, 12), (170, 30, 60), (255, 130, 130), (255, 235, 235), (40, 6, 12), shadow_a=25)
VIOLET = Tone("violet", (20, 10, 30), (120, 60, 200), (200, 160, 255), (250, 245, 255), (30, 12, 50), shadow_a=25)
GOLD_RAMP = [(0.0, (230, 170, 60, 0)), (0.15, (230, 170, 60, 0)), (0.3, (240, 185, 80, 110)),
             (0.55, (255, 210, 120, 190)), (0.8, (255, 238, 190, 235)), (1.0, (255, 252, 240, 250))]
WARD_RAMP = [(0.0, (90, 150, 230, 0)), (0.12, (90, 150, 230, 0)), (0.3, (110, 170, 240, 100)),
             (0.55, (160, 205, 255, 180)), (0.8, (215, 235, 255, 230)), (1.0, (255, 255, 255, 250))]


def rock(f: Frame, x: float, y: float, r: float, spin: float, a: float, col=(58, 50, 46)):
    pts = [(x + math.cos(spin + k * 1.1) * r * m, y + math.sin(spin + k * 1.1) * r * m)
           for k, m in enumerate((1.0, 0.75, 1.1, 0.8, 0.95, 0.7))]
    f.poly(f.d, pts, rgba(col, 245 * a), rgba(IRON_EDGE, 245 * a), 0.9)


def rocks(f: Frame, fly: Flying, t: float, a: float, col=(58, 50, 46)):
    for x, y, _ang, _sp, life, rot in fly.where(t):
        rock(f, x, y, 2.5 + life * 3.5, rot + t * 1.5, a * life, col)


def rune(g: Glow, x: float, y: float, h: float, seed: int, a: float):
    """A rune of two to four straight strokes in an h-tall box."""
    rnd = random.Random(seed)
    w = h * 0.6
    g.stroke([(x, y - h / 2), (x, y + h / 2)], [0.9, 0.9], a)
    for _ in range(rnd.choice((1, 2, 3))):
        y0 = y - h / 2 + rnd.uniform(0, h * 0.6)
        side = rnd.choice((-1, 1))
        g.stroke([(x, y0), (x + side * w * 0.6, y0 + rnd.uniform(-h * 0.3, h * 0.3))], [0.8, 0.8], a)


# ── meteor ──────────────────────────────────────────────────────────────


def meteor() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    dx, dy = -0.48, 0.88
    ln = math.hypot(dx, dy)
    dx, dy = dx / ln, dy / ln
    hit = (CX, CY + 12)
    rnd = random.Random(31)
    debris = Flying(rnd, *hit, 14, -math.pi / 2, 1.6, (24, 52), 12)
    embers = Flying(rnd, *hit, 16, -math.pi / 2, 1.0, (18, 40), -3)
    r = np.sqrt((X - hit[0]) ** 2 + (Y - hit[1]) ** 2)
    cn = vfx.fbm(X, Y, 12, 33, 5)
    v = (1 - r / 46) + (cn - 0.5) * 0.8
    veins_n = vfx.fbm(X, Y, 7, 34, 4)
    # head distance back from impact, blast radius, heat, smoke, crater, glow, ring, flying t
    plan = [(165, 0, 0, 0, 0, 0, 0, None), (110, 0, 0, 0, 0, 0, 0, None), (55, 0, 0, 0, 0, 0, 0, None),
            (0, 52, 1.4, 0.2, 0.6, 1.0, 46, 0.0), (None, 66, 0.9, 0.55, 1.0, 1.0, 86, 1.0),
            (None, 40, 0.5, 0.85, 1.0, 0.8, 0, 2.0), (None, 0, 0, 0.75, 1.0, 0.55, 0, 3.0),
            (None, 0, 0, 0.4, 0.8, 0.3, 0, None)]
    frames = []
    for i, (back, blast, heat, smoke_a, crater, glow, ring, fly_t) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if crater:
            halo = vfx.smooth(-0.25, 0.0, v) * (1 - vfx.smooth(0, 0.08, v))
            vfx.over(img, vfx.tint(halo * 0.4 * crater, (95, 48, 20)))
            vfx.over(img, vfx.tint(vfx.smooth(0, 0.08, v) * 0.9 * crater, (26, 16, 11)))
            rim = np.exp(-((v - 0.02) / 0.05) ** 2)
            veins = vfx.smooth(0.6, 0.7, veins_n) * vfx.smooth(0.08, 0.3, v) * 0.75
            vfx.over(img, vfx.ramp((rim * 0.8 + veins) * glow, FIRE_RAMP))
        f = Frame()
        if fly_t is not None:
            rocks(f, debris, 0.35 + fly_t * 0.6, max(0.0, 1 - fly_t * 0.25))
        img.alpha_composite(f.render(None, None, halo=1))
        if smoke_a:
            scy = hit[1] - 40 - i * 14
            rs = np.sqrt(((X - hit[0]) / 1.2) ** 2 + (Y - scy) ** 2)
            sm = (np.clip(1 - rs / (50 + i * 10), 0, 1) ** 0.7
                  * vfx.smooth(0.35, 0.75, vfx.warped(X, Y + i * 12, 20, 35, 14)) * smoke_a * edge)
            vfx.over(img, vfx.tint(sm * 0.8, SMOKE))
        if blast:
            dyb = (Y - (hit[1] - i * 3))
            base = np.clip(1 - np.sqrt((X - hit[0]) ** 2 + dyb ** 2) / blast, 0, 1)
            n = vfx.warped(X, Y + i * 9, 16, 36, 12)
            fire = vfx.ramp(base ** 0.65 * heat * (0.45 + 0.95 * n), FIRE_RAMP)
            vfx.over(img, vfx.bloom(fire, 0.45, 14, 1.0))
            vfx.over(img, fire)
        if ring:
            shock(img, X, Y, *hit, ring, 1.0 if i == 3 else 0.6, 37 + i)
        if back is not None:
            hx, hy = hit[0] - dx * back, hit[1] - dy * back
            along = (X - hx) * dx + (Y - hy) * dy
            lat = -(X - hx) * dy + (Y - hy) * dx
            behind = np.clip(-along, 0, None)
            R = 17
            width = R * 0.9 + behind * 0.2
            n = vfx.warped(along * 0.55 - i * 16, lat, 14, 3, 9)
            rough = vfx.warped(X + i * 7, Y - i * 5, 7, 4, 6)
            core = np.clip(1 - np.sqrt((X - hx) ** 2 + (Y - hy) ** 2) / R * (0.85 + 0.35 * rough), 0, 1)
            trail = np.exp(-(lat / width) ** 2) * np.exp(-behind / 120) * vfx.smooth(6, -4, along)
            d = np.maximum(core ** 0.5 * 1.15, trail * (0.15 + 1.25 * n)) * edge
            smoke = (np.exp(-(lat / (width * 1.3)) ** 2) * vfx.smooth(30, 90, behind) * np.exp(-behind / 180)
                     * vfx.smooth(0.45, 0.75, vfx.warped(along * 0.7 - i * 10, lat, 18, 9, 10))) * edge
            vfx.over(img, vfx.tint(smoke * 0.75, SMOKE))
            fire = vfx.ramp(d, FIRE_RAMP)
            vfx.over(img, vfx.bloom(fire, 0.5, 12, 0.9))
            vfx.over(img, fire)
            fr = Frame()
            rock(fr, hx + dx * 3, hy + dy * 3, 9, i * 0.7, 0.85, (70, 40, 26))
            img.alpha_composite(fr.render(None, None, halo=0))
            streaks(img, [(hx + dx * 6 - 1, hy + dy * 6, hx + dx * 6 + 1, hy + dy * 6, 1.0)], (255, 200, 120), 4, 4)
        if fly_t is not None and fly_t < 3:
            segs = []
            for x, y, ang, sp, life, _ in embers.where(0.3 + fly_t * 0.6):
                segs.append((x, y, x, y + 4, max(0.0, 1 - fly_t * 0.3) * life))
            streaks(img, segs, (255, 196, 96), 1.2, 2.5)
        if i == 3:
            flash(img, X, Y, *hit, 26, 1.0)
        frames.append(to_cell(img))
    return join(frames)


# ── chains ──────────────────────────────────────────────────────────────

RUST = (82, 66, 58)
RUST_LIT = (186, 150, 118)


def bezier(p0, p1, p2, n: int = 60):
    return [((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0],
             (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1]) for t in (k / n for k in range(n + 1))]


def chain(f: Frame, pts, grow: float, a: float):
    """Links along a path: one face-on, one edge-on, alternately, and a hook
    at the leading end."""
    cum = [0.0]
    for p, q in zip(pts, pts[1:]):
        cum.append(cum[-1] + math.hypot(q[0] - p[0], q[1] - p[1]))
    total = cum[-1] * grow
    s, k, j = 0.0, 0, 0
    last = None
    while s < total:
        while j < len(cum) - 2 and cum[j + 1] < s:
            j += 1
        seg = max(1e-6, cum[j + 1] - cum[j])
        u = (s - cum[j]) / seg
        x = pts[j][0] + (pts[j + 1][0] - pts[j][0]) * u
        y = pts[j][1] + (pts[j + 1][1] - pts[j][1]) * u
        tx, ty = (pts[j + 1][0] - pts[j][0]) / seg, (pts[j + 1][1] - pts[j][1]) / seg
        if k % 2 == 0:
            ring = [(x + tx * 6.5 * math.cos(t) - ty * 3.6 * math.sin(t), y + ty * 6.5 * math.cos(t) + tx * 3.6 * math.sin(t))
                    for t in (m / 14 * math.pi * 2 for m in range(15))]
            f.line(f.d, ring, rgba(IRON_EDGE, 250 * a), 3.4)
            f.line(f.d, ring, rgba(RUST, 250 * a), 2.0)
            f.line(f.d, ring[1:6], rgba(RUST_LIT, 220 * a), 0.9)
        else:
            f.line(f.d, [(x - tx * 6, y - ty * 6), (x + tx * 6, y + ty * 6)], rgba(IRON_EDGE, 250 * a), 3.6)
            f.line(f.d, [(x - tx * 5, y - ty * 5), (x + tx * 5, y + ty * 5)], rgba(RUST, 250 * a), 2.0)
        last = (x, y, tx, ty)
        s += 8.5
        k += 1
    if last:
        x, y, tx, ty = last
        hook = [(x + tx * 4, y + ty * 4), (x + tx * 14 - ty * 2, y + ty * 14 + tx * 2),
                (x + tx * 16 + ty * 8, y + ty * 16 - tx * 8), (x + tx * 10 + ty * 9, y + ty * 10 - tx * 9),
                (x + tx * 12 + ty * 4, y + ty * 12 - tx * 4), (x + tx * 6, y + ty * 6 + 2)]
        f.poly(f.d, hook, rgba(RUST, 250 * a), rgba(IRON_EDGE, 250 * a), 1.0)


def chains() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    r = np.sqrt((X - CX) ** 2 + (Y - CY - 10) ** 2)
    routes = [((CX - 70, CY + 60), (CX - 10, CY - 10), (CX + 78, CY - 108)),
              ((CX + 70, CY + 62), (CX + 10, CY - 6), (CX - 80, CY - 104)),
              ((CX - 76, CY + 10), (CX, CY + 40), (CX + 92, CY + 6)),
              ((CX + 74, CY - 30), (CX, CY - 60), (CX - 90, CY - 40))]
    # rift, grow, tighten, alpha, sparks t
    plan = [(0.6, 0.0, 0, 0, None), (1.0, 0.4, 0, 1, None), (1.0, 0.8, 0, 1, None), (1.0, 1.0, 0, 1, None),
            (1.0, 1.0, 1.0, 1, 0.0), (0.9, 1.0, 1.0, 1, 1.0), (0.6, 0.5, 0.5, 0.7, None), (0.25, 0.0, 0, 0, None)]
    rnd = random.Random(41)
    spark = Flying(rnd, CX, CY - 20, 12, -math.pi / 2, 2.4, (20, 40), 10)
    frames = []
    for i, (rift, grow, tight, a, spark_t) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        sx, sy = vfx.swirl(X, Y, CX, CY + 10, 4 + i * 0.8, 30)
        n = vfx.fbm(sx, sy, 12, 42, 5)
        R = 64 * rift
        if R:
            pool = np.clip(1 - r / R, 0, 1) ** 0.4 * edge
            vfx.over(img, vfx.tint(pool * (0.75 + 0.25 * n), (14, 4, 10)))
            rim = np.exp(-((r - R * 0.92) / 5) ** 2) * vfx.smooth(0.35, 0.7, n) * edge
            vfx.over(img, vfx.ramp(rim * 0.9, [(0.0, (170, 30, 60, 0)), (0.3, (170, 30, 60, 120)),
                                              (0.7, (240, 90, 90, 220)), (1.0, (255, 200, 190, 250))]))
            vfx.over(img, soft(vfx.tint(rim * 0.6, (200, 40, 70)), 5))
        if grow:
            f = Frame()
            g = Glow(TW, TH)
            for p0, c, p1 in routes:
                c = (c[0] + (CX - c[0]) * 0.5 * tight, c[1] + (CY - c[1]) * 0.5 * tight)
                p1 = (p1[0] + (CX - p1[0]) * 0.12 * tight, p1[1] + (CY - p1[1]) * 0.12 * tight)
                pts = bezier(p0, c, p1)
                chain(f, pts, grow, a)
                g.stroke(pts[: max(2, int(len(pts) * grow))], [1.2] * len(pts), 0.35 * a, core=False)
            vfx.over(img, g.render(CRIMSON, edge))
            img.alpha_composite(f.render(None, None, halo=1))
        if spark_t is not None:
            spark.sparks(img, 0.35 + spark_t * 0.6, 1 - spark_t * 0.4)
        frames.append(to_cell(img))
    return join(frames)


# ── reaper ──────────────────────────────────────────────────────────────


def scythe(f: Frame, C, th: float, a: float):
    """A scythe swung round C: the snath along the radius, the blade at its
    end bent toward the way it swings (decreasing angle)."""
    t = math.radians(th)
    u = (math.cos(t), math.sin(t))
    lead = (math.sin(t), -math.cos(t))
    p0 = (C[0] + u[0] * 150, C[1] + u[1] * 150)
    p1 = (C[0] + u[0] * 282, C[1] + u[1] * 282)
    f.line(f.d, [p0, p1], rgba((40, 44, 56), 210 * a), 5)
    f.line(f.d, [p0, p1], rgba((120, 140, 160), 150 * a), 2)
    outer, inner = [], []
    for k in range(13):
        q = k / 12
        along = 128 * q
        bend = 34 * q * q
        x = p1[0] + lead[0] * along - u[0] * bend
        y = p1[1] + lead[1] * along - u[1] * bend
        w = 15 * (1 - q) ** 0.8
        outer.append((x + u[0] * w * 0.3, y + u[1] * w * 0.3))
        inner.append((x - u[0] * w, y - u[1] * w))
    blade = outer + inner[::-1]
    f.poly(f.d, blade, rgba((150, 175, 190), 200 * a), rgba((30, 34, 44), 230 * a), 1.0)
    f.line(f.d, outer, rgba((240, 250, 255), 240 * a), 1.2)


def reaper() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    C = (CX + 20, CY + 250)
    Rc = 268
    # scythe angle, smear from, smear, scythe alpha, cut glow, soul rise, soul alpha
    plan = [(312, 322, 0.6, 0.8, 0, 0, 0), (290, 322, 1.0, 1.0, 0, 0, 0), (262, 318, 1.0, 1.0, 0, 0, 0),
            (232, 300, 0.7, 0.7, 1.0, 0, 0), (0, 0, 0, 0, 0.7, 24, 0.8), (0, 0, 0, 0, 0.45, 60, 1.0),
            (0, 0, 0, 0, 0.25, 28, 0.55), (0, 0, 0, 0, 0.1, 0, 0)]
    from melee_strips import arc_smear
    cut = [(C[0] + math.cos(math.radians(t)) * (Rc + 12), C[1] + math.sin(math.radians(t)) * (Rc + 12))
           for t in np.arange(236, 304, 2)]
    frames = []
    for i, (th, sm_from, sm_a, sc_a, cut_a, rise, soul) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if cut_a:
            g = Glow(TW, TH)
            ws = [2.2 * math.sin(math.pi * k / (len(cut) - 1)) ** 0.7 + 0.2 for k in range(len(cut))]
            g.stroke(cut, ws, cut_a)
            vfx.over(img, g.render(AZURE, edge))
        if soul:
            # The soul, half pulled out: a tall stretched figure of pale
            # vapour, its head drawn upward, torn at the edges, still rooted
            # in the card. No face: dots for eyes made it a nursery ghost.
            n = vfx.warped(X, Y + i * 8, 8, 51, 10)
            top = CY - 40 - rise
            u = np.clip((Y - top) / max(1, CY + 30 - top), 0, 1)
            sway = 6 * np.sin(Y * 0.06 + i) * u
            width = 9 + 26 * np.sin(np.clip(u * 1.25, 0, 1) * np.pi / 2) ** 0.8 * (1 - 0.65 * u)
            body = np.exp(-((X - CX - sway) / width) ** 2) * vfx.smooth(top - 8, top + 10, Y) * vfx.smooth(CY + 40, CY + 10, Y)
            arms = sum(np.exp(-((X - CX - side * (20 + 16 * (Y - top - 34) / 40)) / 5) ** 2)
                       * vfx.smooth(top + 26, top + 40, Y) * vfx.smooth(top + 84, top + 56, Y) for side in (-1, 1))
            hollow = sum(np.exp(-(((X - CX - side * 5) / 2.6) ** 2 + ((Y - top - 12) / 4.5) ** 2)) for side in (-1, 1))
            d = np.maximum(body, arms * 0.7) * (0.3 + 0.9 * n) * soul * edge * (1 - 0.8 * hollow)
            ghost = vfx.ramp(d, GHOST_RAMP)
            vfx.over(img, vfx.tint(d * 0.22, (30, 40, 55)))
            vfx.over(img, vfx.bloom(ghost, 0.55, 8, 0.9))
            vfx.over(img, ghost)
        if sm_a:
            sm, dd = arc_smear(X, Y, C, Rc - 40, Rc + 40, sm_from, th, sm_a, GHOST_RAMP)
            sm[..., 3] *= edge
            vfx.over(img, vfx.tint(dd * 0.2 * edge, (30, 40, 55)))
            vfx.over(img, vfx.bloom(sm, 0.6, 6, 0.8))
            vfx.over(img, sm)
        if sc_a:
            f = Frame()
            scythe(f, C, th, sc_a)
            img.alpha_composite(f.render(None, None, halo=1))
        frames.append(to_cell(img))
    return join(frames)


# ── shadow spikes ───────────────────────────────────────────────────────

OBSIDIAN_DARK = (22, 14, 32)
OBSIDIAN = (64, 46, 96)
OBSIDIAN_EDGE = (196, 160, 255)


def shadow_spikes() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    pool_c = (CX, CY + 46)
    r = np.sqrt(((X - pool_c[0]) / 1.3) ** 2 + (Y - pool_c[1]) ** 2)
    rnd = random.Random(61)
    spikes = []
    for k in range(9):
        bx = pool_c[0] + rnd.uniform(-68, 68)
        by = pool_c[1] + rnd.uniform(-10, 18)
        h = rnd.uniform(70, 150) * (1 - abs(bx - pool_c[0]) / 160)
        ang = math.radians(-90 + (bx - pool_c[0]) * 0.32 + rnd.uniform(-8, 8))
        spikes.append((bx, by, h, ang, rnd.uniform(9, 15)))
    spikes.sort(key=lambda s: s[1])
    # pool, grow, alpha, flash, shatter, dust
    plan = [(0.5, 0, 0, 0, 0, 0), (0.9, 0.35, 1, 0, 0, 0), (1.0, 1.0, 1, 1.0, 0, 0), (1.0, 1.0, 1, 0.3, 0, 0),
            (0.9, 1.0, 1, 0, 0.35, 0.3), (0.7, 1.0, 0.75, 0, 1.0, 0.7), (0.4, 1.0, 0.35, 0, 1.8, 0.6),
            (0.15, 0, 0, 0, 0, 0.3)]
    frames = []
    for i, (pool, grow, a, fl, shatter, dust_a) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        n = vfx.warped(X + i * 4, Y, 10, 62, 8)
        if pool:
            R = 70 * pool
            body = np.clip(1 - r / R, 0, 1) ** 0.5 * (0.6 + 0.5 * n) * edge
            vfx.over(img, vfx.ramp(body * 1.2, SHADOW_RAMP))
            rim = np.exp(-((r - R * 0.85) / 5) ** 2) * vfx.smooth(0.4, 0.7, n) * edge
            vfx.over(img, soft(vfx.tint(rim * 0.8, (130, 70, 220)), 3))
        if dust_a:
            vfx.over(img, vfx.ramp(np.clip(1 - np.sqrt((X - CX) ** 2 + (Y - CY + 10) ** 2) / (90 + i * 6), 0, 1)
                                   * vfx.smooth(0.4, 0.75, vfx.warped(X, Y + i * 10, 16, 63, 10)) * dust_a * edge,
                                   SHADOW_RAMP))
        if grow:
            f = Frame()
            g = Glow(TW, TH)
            for k, (bx, by, h, ang, w) in enumerate(spikes):
                hh = h * grow
                u = (math.cos(ang), math.sin(ang))
                v = (-u[1], u[0])
                base_l = (bx + v[0] * w / 2, by + v[1] * w / 2)
                base_r = (bx - v[0] * w / 2, by - v[1] * w / 2)
                tip = (bx + u[0] * hh, by + u[1] * hh)
                mid = (bx + u[0] * hh * 0.15, by + u[1] * hh * 0.15)
                pieces = [(base_l, mid, tip, OBSIDIAN_DARK), (mid, base_r, tip, OBSIDIAN)]
                if shatter:
                    # Broken into three, flung out and down.
                    for q in range(3):
                        t0, t1 = q / 3, (q + 1) / 3
                        off = shatter * (12 + q * 10)
                        side = 1 if bx > CX else -1
                        ox, oy = side * off * 0.8, off * 0.6 + shatter * shatter * 8
                        quad = [(base_l[0] + (tip[0] - base_l[0]) * t0 + ox, base_l[1] + (tip[1] - base_l[1]) * t0 + oy),
                                (base_l[0] + (tip[0] - base_l[0]) * t1 + ox, base_l[1] + (tip[1] - base_l[1]) * t1 + oy),
                                (base_r[0] + (tip[0] - base_r[0]) * t1 + ox, base_r[1] + (tip[1] - base_r[1]) * t1 + oy),
                                (base_r[0] + (tip[0] - base_r[0]) * t0 + ox, base_r[1] + (tip[1] - base_r[1]) * t0 + oy)]
                        f.poly(f.d, quad, rgba(OBSIDIAN_DARK, 245 * a), rgba(OBSIDIAN_EDGE, 200 * a), 0.8)
                    continue
                for p0, p1, p2, col in pieces:
                    f.poly(f.d, [p0, p1, p2], rgba(col, 250 * a))
                f.line(f.d, [base_l, tip, base_r], rgba(IRON_EDGE, 250 * a), 1.0)
                f.line(f.d, [mid, tip], rgba(OBSIDIAN_EDGE, 230 * a), 0.9)
                g.stroke([mid, tip], [1.0, 0.4], 0.6 * a, core=False)
            vfx.over(img, g.render(VIOLET, edge))
            img.alpha_composite(f.render(None, None, halo=1))
        if fl:
            flash(img, X, Y, CX, CY - 20, 30, fl, (220, 200, 255))
        frames.append(to_cell(img))
    return join(frames)


# ── bats ────────────────────────────────────────────────────────────────

BAT = (20, 14, 22)


def bat(f: Frame, cx: float, cy: float, s: float, flap: float, a: float = 1.0):
    col = rgba(BAT, 250 * a)
    for side in (-1, 1):
        lift = flap * 10 * s
        pts = [(cx + side * 4 * s, cy - 2 * s), (cx + side * 16 * s, cy - 10 * s - lift),
               (cx + side * 34 * s, cy - 6 * s - lift * 1.3), (cx + side * 29 * s, cy + 3 * s - lift * 0.8),
               (cx + side * 25 * s, cy - 1 * s - lift * 0.6), (cx + side * 22 * s, cy + 8 * s - lift * 0.4),
               (cx + side * 16 * s, cy + 4 * s - lift * 0.2), (cx + side * 11 * s, cy + 10 * s),
               (cx + side * 4 * s, cy + 6 * s)]
        f.poly(f.d, pts, col)
    f.d.ellipse([*f.p(cx - 4.5 * s, cy - 6 * s), *f.p(cx + 4.5 * s, cy + 8 * s)], fill=col)
    f.poly(f.d, [(cx - 4 * s, cy - 5 * s), (cx - 3.5 * s, cy - 12 * s), (cx - 1 * s, cy - 6 * s)], col)
    f.poly(f.d, [(cx + 4 * s, cy - 5 * s), (cx + 3.5 * s, cy - 12 * s), (cx + 1 * s, cy - 6 * s)], col)
    return [(cx - 1.6 * s, cy - 3 * s, a), (cx + 1.6 * s, cy - 3 * s, a)]


def bat_eyes(img, spots):
    streaks(img, [(x - 0.4, y, x + 0.4, y, a) for x, y, a in spots], (255, 60, 50), 1.6, 2.2)


def bats() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    rnd = random.Random(71)
    flock = [(CX + rnd.uniform(-70, 70), CY + rnd.uniform(-60, 60), rnd.uniform(0.45, 0.75), rnd.uniform(0, 6))
             for _ in range(16)]
    frames = []
    for i in range(6):
        img = vfx.canvas(TW, TH)
        smoke = np.zeros_like(X)
        f = Frame()
        spots = []
        for x, y, s, ph in flock:
            bx, by = x + 6 * math.sin(i * 1.3 + ph), y + 5 * math.cos(i * 1.1 + ph)
            spots += bat(f, bx, by, s, math.cos(i * 2.1 + ph), 1.0)
            smoke = np.maximum(smoke, np.exp(-(((X - bx) / (26 * s)) ** 2 + ((Y - by - 4) / (18 * s)) ** 2)))
        n = vfx.warped(X, Y - i * 6, 10, 72, 8)
        vfx.over(img, vfx.ramp(smoke * (0.2 + 0.9 * n) * edge, SHADOW_RAMP))
        img.alpha_composite(f.render(None, None, halo=1))
        bat_eyes(img, spots)
        frames.append(to_cell(img))
    return join(frames)


def bats_swarm() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    rnd = random.Random(73)
    swarm = [(rnd.uniform(0, math.pi * 2), rnd.uniform(0.6, 1.1), rnd.uniform(0.4, 0.7), rnd.uniform(0, 6))
             for _ in range(18)]
    bites = [(CX + rnd.uniform(-50, 50), CY + rnd.uniform(-70, 70)) for _ in range(6)]
    r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
    # ring radius, spin, bats alpha, mist, bites
    plan = [(110, 0.0, 0.8, 0.5, 0), (90, 0.6, 1, 0.8, 0), (70, 1.3, 1, 1.0, 0), (40, 2.0, 1, 1.0, 0.6),
            (30, 2.6, 1, 1.0, 1.0), (60, 3.2, 0.9, 0.8, 1.0), (100, 3.8, 0.5, 0.5, 0.8), (120, 4.3, 0.15, 0.25, 0.5)]
    frames = []
    for i, (R, spin, ba, mist, bite) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if bite:
            f0 = Frame()
            for bx, by in bites:
                for d in (-2.5, 2.5):
                    f0.line(f0.d, [(bx + d, by - 3), (bx + d + 0.6, by + 3)], rgba((150, 20, 30), 230 * bite), 1.4)
            img.alpha_composite(f0.render(None, None, halo=1))
        sx, sy = vfx.swirl(X, Y, CX, CY, 3 + i * 0.7, 40)
        n = vfx.fbm(sx, sy, 14, 74, 5)
        ring = np.exp(-((r - R) / 34) ** 2) + np.clip(1 - r / max(20, R), 0, 1) * 0.5
        vfx.over(img, vfx.ramp(ring * mist * (0.2 + 1.0 * n) * edge, SHADOW_RAMP))
        f = Frame()
        spots = []
        for t0, rr, s, ph in swarm:
            t = t0 + spin
            bx, by = CX + math.cos(t) * R * rr * 0.75, CY + math.sin(t) * R * rr
            spots += bat(f, bx, by, s, math.cos(i * 2.3 + ph), ba)
        img.alpha_composite(f.render(None, None, halo=1))
        bat_eyes(img, spots)
        frames.append(to_cell(img))
    return join(frames)


# ── healing ─────────────────────────────────────────────────────────────


def gold_feather(f: Frame, x: float, y: float, ang: float, length: float, a: float):
    c, s_ = math.cos(ang), math.sin(ang)
    pts = []
    for k in range(9):
        u = k / 8
        w = math.sin(u * math.pi) * length * 0.2
        pts.append((x + c * length * u - s_ * w, y + s_ * length * u + c * w))
    for k in range(7, 0, -1):
        u = k / 8
        w = -math.sin(u * math.pi) * length * 0.16
        pts.append((x + c * length * u - s_ * w, y + s_ * length * u + c * w))
    f.poly(f.d, pts, rgba((255, 228, 160), 230 * a), rgba((170, 120, 40), 230 * a), 0.8)
    f.line(f.d, [(x, y), (x + c * length, y + s_ * length)], rgba((200, 150, 60), 230 * a), 0.8)


def healing() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    rnd = random.Random(81)
    seams = crack_lines(rnd, CX, CY + 6, 7, 70, 0.4, rim=8)
    runes = [(CX + rnd.uniform(-80, 80), CY + rnd.uniform(-20, 90), rnd.randrange(1000)) for _ in range(8)]
    feathers = [(CX + rnd.uniform(-70, 70), rnd.uniform(-20, 80), rnd.uniform(0, 6), rnd.uniform(18, 26)) for _ in range(5)]
    # column width, column, seams glow, runes rise, feathers t, halo ring
    plan = [(10, 0.6, 0, None, 0.0, 0), (22, 1.0, 0, 0, 1.0, 0), (34, 1.0, 0.9, 1, 2.0, 0),
            (34, 0.9, 1.0, 2, 3.0, 0.6), (28, 0.7, 0.8, 3, 4.0, 1.0), (20, 0.45, 0.5, 4, 5.0, 0.7),
            (12, 0.2, 0.25, 5, 6.0, 0.3), (0, 0, 0, None, None, 0)]
    frames = []
    for i, (cw, col, seam, rise, ft, halo) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        if col:
            n = vfx.fbm(X * 1.4, Y * 0.35 - i * 20, 9, 82, 4)
            beam = np.exp(-((X - CX) / cw) ** 2) * vfx.smooth(0, 140, Y) * (0.55 + 0.6 * n) * col * edge
            light = vfx.ramp(beam, GOLD_RAMP)
            vfx.over(img, vfx.bloom(light, 0.5, 12, 0.9))
            vfx.over(img, light)
        if halo:
            r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
            ring = np.exp(-((r - (50 + i * 10)) / 6) ** 2) * halo * edge
            vfx.over(img, vfx.ramp(ring * 0.9, GOLD_RAMP))
        g = Glow(TW, TH)
        if seam:
            for pts, w in seams:
                g.stroke(pts, [w * 0.8] * len(pts), seam)
        if rise is not None:
            for k, (x, y, seed) in enumerate(runes):
                yy = y - rise * 22 - k * 3
                aa = max(0.0, min(1.0, 1.2 - rise * 0.22)) * (0.6 + 0.4 * ((k + rise) % 2))
                rune(g, x, yy, 12, seed, aa)
        vfx.over(img, g.render(GOLD, edge))
        if ft is not None:
            f = Frame()
            for k, (x, y, ang, ln) in enumerate(feathers):
                fy = y + ft * 26
                if fy > TH - 30:
                    continue
                gold_feather(f, x + 10 * math.sin(ft + k), fy, ang + ft * 0.5, ln, max(0.0, 1 - ft * 0.13))
            fi = f.render(None, None, halo=0)
            vfx.over(img, vfx.bloom(np.asarray(fi).astype(np.float32), 0.6, 4, 0.8))
            img.alpha_composite(fi)
        frames.append(to_cell(img))
    return join(frames)


# ── summoning ───────────────────────────────────────────────────────────


def summoning() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    rnd = random.Random(91)
    runes = [rnd.randrange(1000) for _ in range(12)]
    embers = Flying(rnd, CX, CY + 20, 18, -math.pi / 2, 0.6, (24, 50), -2)
    # outer ring drawn, inner, runes, sigil, pillar, embers t, circle alpha
    plan = [(0.4, 0, 0, 0, 0, None, 1), (1, 0.6, 0.5, 0, 0, None, 1), (1, 1, 1, 1, 0, None, 1),
            (1, 1, 1, 1, 1.0, 0.0, 1), (1, 1, 1, 1, 0.9, 1.0, 1), (1, 1, 1, 0.7, 0.5, 2.0, 0.8),
            (1, 1, 1, 0.4, 0.2, 3.0, 0.5), (1, 1, 1, 0.1, 0, None, 0.2)]
    frames = []
    for i, (outer, inner, rn, sigil, pillar, ft, ca) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        g = Glow(TW, TH)

        def arc(R, frac, a, w=1.4):
            pts = [(CX + math.cos(-math.pi / 2 + t) * R, CY + math.sin(-math.pi / 2 + t) * R)
                   for t in np.linspace(0, math.pi * 2 * frac, max(3, int(120 * frac)))]
            g.stroke(pts, [w] * len(pts), a)
        arc(96, outer, ca, 1.8)
        if inner:
            arc(80, inner, ca, 1.1)
        if rn:
            for k, seed in enumerate(runes[: int(12 * rn)]):
                t = -math.pi / 2 + k / 12 * math.pi * 2 + i * 0.05
                rune(g, CX + math.cos(t) * 88, CY + math.sin(t) * 88, 9, seed, ca)
        if sigil:
            rot = i * 0.25
            for k in range(3):
                t0 = rot + k * math.pi * 2 / 3
                pts = [(CX + math.cos(t0 + q) * 56, CY + math.sin(t0 + q) * 56) for q in np.linspace(0, 1.6, 30)]
                g.stroke(pts, [1.3] * len(pts), sigil * ca)
            tri = [(CX + math.cos(-math.pi / 2 + k * math.pi * 2 / 3) * 72, CY + math.sin(-math.pi / 2 + k * math.pi * 2 / 3) * 72)
                   for k in range(4)]
            g.stroke(tri, [1.0] * 4, sigil * ca * 0.8)
        vfx.over(img, g.render(EMBERS, edge))
        if pillar:
            n = vfx.fbm(X * 1.5, Y * 0.32 + i * 22, 9, 92, 5)
            col = np.exp(-((X - CX) / (46 * pillar + 8)) ** 2) * vfx.smooth(0, 120, Y) * vfx.smooth(CY + 110, CY + 60, Y)
            fire = vfx.ramp(col * (0.35 + 0.85 * n) * pillar * edge, FIRE_RAMP)
            vfx.over(img, vfx.bloom(fire, 0.45, 14, 1.0))
            vfx.over(img, fire)
        if ft is not None:
            segs = [(x, y, x, y + 5, max(0.0, 1 - ft * 0.28) * life) for x, y, _a, _s, life, _r in embers.where(0.3 + ft * 0.6)]
            streaks(img, segs, (255, 196, 96), 1.2, 2.5)
        if i == 3:
            flash(img, X, Y, CX, CY, 30, 0.8)
        frames.append(to_cell(img))
    return join(frames)


# ── death ───────────────────────────────────────────────────────────────


def death() -> Image.Image:
    """The card burns from its edges in. The card itself is faded by the
    body gesture (`sink`) at the same time; this strip lays the burn over it:
    the burnt part dark, the edge of the fire glowing, ash and embers going
    up, and at the end a pale wisp of soul."""
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y, m=10)
    # The strip lies in a box 1.3 cells big (`DEATH_SIZE` in battles.ts):
    # the card is the middle 1/1.3 of it, a hair inside.
    hw, hh = TW / 2 / 1.3 * 0.97, TH / 2 / 1.3 * 0.97
    inside = np.minimum.reduce([X - (CX - hw), (CX + hw) - X, Y - (CY - hh), (CY + hh) - Y])
    n = vfx.fbm(X, Y, 14, 101, 5)
    burn = inside + (n - 0.5) * 44
    flakes = vfx.fbm(X, Y, 22, 106, 3)
    card = vfx.smooth(-1, 2, inside)
    rnd = random.Random(103)
    ash = [(CX + rnd.uniform(-hw, hw), CY + rnd.uniform(-hh, hh), rnd.uniform(0.6, 1.0), rnd.uniform(0, 6))
           for _ in range(30)]
    # front, char crumbled, soul rise, soul alpha
    plan = [(10, 0, 0, 0), (26, 0, 0, 0), (46, 0, 0, 0), (68, 0, 0, 0), (92, 0.1, 10, 0.6),
            (120, 0.4, 34, 1.0), (150, 0.75, 62, 0.7), (170, 1.0, 90, 0.3)]
    frames = []
    for i, (front, crumbled, rise, soul) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        char_a = 1 - crumbled
        burnt = vfx.smooth(front - 2, front - 10, burn) * card
        # Burnt through, the char does not fade flat: it breaks into big
        # flakes of ash whose edges still smoulder. Fine noise here reads as
        # television static, not as ash.
        if crumbled:
            cut = crumbled * 1.1 - 0.05
            keep = vfx.smooth(cut - 0.03, cut + 0.03, flakes)
            smoulder = np.exp(-((flakes - cut) / 0.025) ** 2) * burnt * (1 - crumbled * 0.6)
            vfx.over(img, vfx.tint(burnt * keep * 0.92, (22, 15, 12)))
            vfx.over(img, vfx.ramp(smoulder * 0.7, FIRE_RAMP))
        else:
            vfx.over(img, vfx.tint(burnt * 0.92, (22, 15, 12)))
        lit = np.exp(-((burn - front) / 4.5) ** 2) * card
        tongues = (vfx.smooth(0.5, 0.8, vfx.fbm(X * 1.2, Y * 0.5 + i * 18, 7, 102, 4))
                   * np.exp(-((burn - front - 8) / 10) ** 2) * card)
        fire = vfx.ramp(np.maximum(lit * 0.9, tongues * 0.75) * (1 if front < 180 else 0), FIRE_RAMP)
        vfx.over(img, vfx.bloom(fire, 0.45, 8, 0.9))
        vfx.over(img, fire)
        f = Frame()
        for x, y, life, ph in ash:
            d = min(x - (CX - hw), (CX + hw) - x, y - (CY - hh), (CY + hh) - y)
            if d > front - 6:
                continue
            up = (front - d) * 0.6 * life
            ax, ay = x + 6 * math.sin(ph + i), y - up
            if ay < 12:
                continue
            pts = [(ax + math.cos(ph + k * 2.1) * 2.6, ay + math.sin(ph + k * 2.1) * 2.6) for k in range(3)]
            f.poly(f.d, pts, rgba((90, 82, 78), 220 * life * max(0.2, char_a + 0.2)))
        img.alpha_composite(f.render(None, None, halo=0))
        segs = []
        for x, y, life, ph in ash[:14]:
            d = min(x - (CX - hw), (CX + hw) - x, y - (CY - hh), (CY + hh) - y)
            if abs(d - front) < 30:
                segs.append((x, y - 4, x, y, life))
        streaks(img, segs, (255, 190, 90), 1.2, 2.5)
        if soul:
            ns = vfx.warped(X, Y + i * 10, 10, 104, 9)
            sy = CY - 20 - rise
            wisp = (np.exp(-((X - CX - 6 * np.sin(Y * 0.05 + i)) / (14 + (Y - sy).clip(0) * 0.12)) ** 2)
                    * vfx.smooth(sy - 30, sy, Y) * vfx.smooth(sy + 110, sy + 30, Y))
            d = wisp * (0.35 + 0.8 * ns) * soul * edge
            ghost = vfx.ramp(d, GHOST_RAMP)
            vfx.over(img, vfx.tint(d * 0.2, (30, 40, 55)))
            vfx.over(img, vfx.bloom(ghost, 0.55, 8, 0.9))
            vfx.over(img, ghost)
        frames.append(to_cell(img))
    return join(frames)


# ── ward ────────────────────────────────────────────────────────────────


def ward() -> Image.Image:
    X, Y = vfx.grid(TW, TH)
    edge = framed(X, Y)
    r = np.sqrt((X - CX) ** 2 + (Y - CY) ** 2)
    # An energy lattice: three families of lines at 60°.
    lat = np.zeros_like(X)
    for k in range(3):
        t = k * math.pi / 3
        q = (X * math.cos(t) + Y * math.sin(t)) / 14
        lat = np.maximum(lat, np.exp(-((q - np.round(q)) / 0.06) ** 2))
    hit = (CX - 66, CY - 14)
    dh = np.sqrt((X - hit[0]) ** 2 + (Y - hit[1]) ** 2)
    rnd = random.Random(111)
    runes = [rnd.randrange(1000) for _ in range(10)]
    sparks = Flying(rnd, *hit, 14, math.pi, 1.2, (20, 44), 8)
    # scale, shield alpha, ripple radius, cracks, settle
    plan = [(0.35, 0.7, 0, 0, 0), (1.0, 1.0, 0, 0, 0), (1.0, 1.0, 14, 0, 0), (1.0, 0.95, 46, 0.8, 0),
            (0.95, 0.85, 90, 1.0, 0), (0.85, 0.6, 0, 0.6, 0.6), (0.78, 0.35, 0, 0.3, 1.0), (0.74, 0.1, 0, 0, 0.5)]
    cracks = crack_lines(rnd, *hit, 5, 60, 0.3, rim=4, heading=0.0, fan=0.9)
    frames = []
    for i, (sc, a, ripple, crack, settle) in enumerate(plan):
        img = vfx.canvas(TW, TH)
        R = 92 * sc
        disc = vfx.smooth(R + 2, R - 4, r)
        n = vfx.fbm(X + i * 5, Y - i * 3, 12, 112, 4)
        d = disc * (0.18 + 0.25 * lat * (0.5 + 0.5 * n)) + np.exp(-((r - R) / 3.5) ** 2) * 0.95
        if ripple:
            d = d + np.exp(-((dh - ripple) / 5) ** 2) * disc * 0.7 * (1 - ripple / 120)
            d = d + np.exp(-(dh / 14) ** 2) * disc * (1 - ripple / 100)
        d = d * a * edge
        light = vfx.ramp(np.clip(d, 0, 1.05), WARD_RAMP)
        vfx.over(img, vfx.tint(d * 0.18, (20, 35, 60)))
        vfx.over(img, vfx.bloom(light, 0.55, 9, 0.9))
        vfx.over(img, light)
        g = Glow(TW, TH)
        for k, seed in enumerate(runes):
            t = -math.pi / 2 + k / 10 * math.pi * 2 + i * 0.12
            rune(g, CX + math.cos(t) * (R - 12), CY + math.sin(t) * (R - 12), 8 * sc, seed, a * 0.9)
        if crack:
            for pts, w in cracks:
                g.stroke(pts, [w * 0.7] * len(pts), crack)
        if settle:
            # The shield sinks into the card: its outline glows and fades.
            hw, hh = TW / 2 / 1.5, TH / 2 / 1.5
            rect = [(CX - hw, CY - hh), (CX + hw, CY - hh), (CX + hw, CY + hh), (CX - hw, CY + hh), (CX - hw, CY - hh)]
            g.stroke(rect, [1.6] * 5, settle)
        vfx.over(img, g.render(AZURE, edge))
        if 2 <= i <= 4:
            sparks.sparks(img, 0.3 + (i - 2) * 0.55, 1 - (i - 2) * 0.35)
        if i == 2:
            flash(img, X, Y, *hit, 14, 0.9, (220, 235, 255))
        frames.append(to_cell(img))
    return join(frames)


def main():
    for name, paint in (("meteor", meteor), ("chains", chains), ("reaper", reaper),
                        ("shadow-spikes", shadow_spikes), ("bats", bats), ("bats-swarm", bats_swarm),
                        ("healing", healing), ("summoning", summoning), ("death", death), ("ward", ward)):
        save(name, paint())


if __name__ == "__main__":
    main()
