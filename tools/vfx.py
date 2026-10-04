"""Pixel effects for the motion strips: noise, turbulence, colour ramps.

Fire and vapour drawn as shapes (discs, petals, drops with a shine) read as
a cartoon. Drawn as a FIELD — a density per pixel, stirred by turbulence and
coloured through a heat ramp — they read as fire and vapour. This module is
the field half; `magic_strips.py` composes the spells from it.

Everything is seeded: the same arguments give the same pixels on every run.
"""

from __future__ import annotations

import numpy as np
from PIL import Image, ImageFilter

SS = 2  # supersampling of the fields


def grid(w: int, h: int) -> tuple[np.ndarray, np.ndarray]:
    """Pixel centres of a w×h canvas (true units), at SS× resolution."""
    ys, xs = np.mgrid[0:h * SS, 0:w * SS].astype(np.float32)
    return (xs + 0.5) / SS, (ys + 0.5) / SS


_TABLES: dict[int, np.ndarray] = {}


def _table(seed: int) -> np.ndarray:
    if seed not in _TABLES:
        _TABLES[seed] = np.random.default_rng(seed).random((256, 256)).astype(np.float32)
    return _TABLES[seed]


def noise(x: np.ndarray, y: np.ndarray, seed: int) -> np.ndarray:
    """Smooth value noise in [0, 1], one cell per unit."""
    t = _table(seed)
    x0 = np.floor(x)
    y0 = np.floor(y)
    fx, fy = x - x0, y - y0
    sx = fx * fx * (3 - 2 * fx)
    sy = fy * fy * (3 - 2 * fy)
    xi = x0.astype(np.int64) & 255
    yi = y0.astype(np.int64) & 255
    xj, yj = (xi + 1) & 255, (yi + 1) & 255
    a = t[yi, xi] + (t[yi, xj] - t[yi, xi]) * sx
    b = t[yj, xi] + (t[yj, xj] - t[yj, xi]) * sx
    return a + (b - a) * sy


def fbm(x: np.ndarray, y: np.ndarray, scale: float, seed: int, octaves: int = 5) -> np.ndarray:
    """Fractal noise in roughly [0, 1]: big shapes with finer ones on them."""
    out = np.zeros_like(x)
    amp, total, f = 0.5, 0.0, 1.0 / scale
    for k in range(octaves):
        out += amp * noise(x * f, y * f, seed + k * 17)
        total += amp
        amp *= 0.5
        f *= 2.0
    return out / total


def warped(x: np.ndarray, y: np.ndarray, scale: float, seed: int, push: float) -> np.ndarray:
    """Domain-warped fbm: the noise sampled where another noise points. This is
    what makes it look stirred — curls and tongues instead of blots."""
    qx = fbm(x, y, scale, seed + 101, 3) - 0.5
    qy = fbm(x + 37.0, y - 11.0, scale, seed + 202, 3) - 0.5
    return fbm(x + qx * push, y + qy * push, scale, seed, 5)


def smooth(e0: float, e1: float, v: np.ndarray) -> np.ndarray:
    t = np.clip((v - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def ramp(d: np.ndarray, stops: list[tuple[float, tuple[int, int, int, int]]]) -> np.ndarray:
    """Density → RGBA through colour stops (t, (r, g, b, a))."""
    ts = [t for t, _ in stops]
    out = np.zeros(d.shape + (4,), np.float32)
    for c in range(4):
        out[..., c] = np.interp(d, ts, [col[c] for _, col in stops])
    return out


def tint(alpha: np.ndarray, rgb: tuple[int, int, int]) -> np.ndarray:
    out = np.zeros(alpha.shape + (4,), np.float32)
    out[..., 0], out[..., 1], out[..., 2] = rgb
    out[..., 3] = np.clip(alpha, 0, 1) * 255
    return out


def image(rgba: np.ndarray) -> Image.Image:
    return Image.fromarray(np.clip(rgba, 0, 255).astype(np.uint8), "RGBA")


def over(base: Image.Image, top: np.ndarray | Image.Image) -> Image.Image:
    base.alpha_composite(top if isinstance(top, Image.Image) else image(top))
    return base


def bloom(rgba: np.ndarray, threshold: float, radius: float, strength: float) -> Image.Image:
    """The bright part of a layer, blurred: the light that spills past the
    flame onto what is around it."""
    lum = rgba[..., :3].mean(axis=2) / 255.0
    keep = smooth(threshold, 1.0, lum) * (rgba[..., 3] / 255.0)
    glow = rgba.copy()
    glow[..., 3] = keep * 255 * strength
    return image(glow).filter(ImageFilter.GaussianBlur(radius * SS))


def canvas(w: int, h: int) -> Image.Image:
    return Image.new("RGBA", (w * SS, h * SS), (0, 0, 0, 0))


def swirl(x: np.ndarray, y: np.ndarray, cx: float, cy: float, turn: float,
          falloff: float) -> tuple[np.ndarray, np.ndarray]:
    """Coordinates twisted round a centre, more near it: noise sampled there
    curls into a whirlpool. A polar lookup would leave a seam at ±π; a
    rotation has none."""
    dx, dy = x - cx, y - cy
    r = np.sqrt(dx * dx + dy * dy)
    a = turn / (1 + r / falloff)
    c, s = np.cos(a), np.sin(a)
    return cx + dx * c - dy * s, cy + dx * s + dy * c


def voronoi(x: np.ndarray, y: np.ndarray, pts: np.ndarray):
    """Nearest and second-nearest distances and the cell index — ice facets,
    whose edges are where the two distances meet."""
    d = (x[None] - pts[:, 0, None, None]) ** 2 + (y[None] - pts[:, 1, None, None]) ** 2
    idx = np.argmin(d, axis=0)
    part = np.partition(d, 1, axis=0)
    return np.sqrt(part[0]), np.sqrt(part[1]), idx


def segment_distance(x: np.ndarray, y: np.ndarray, x0: float, y0: float,
                     x1: float, y1: float) -> tuple[np.ndarray, np.ndarray]:
    """Distance to a segment, and how far along it (0..1) the nearest point is."""
    vx, vy = x1 - x0, y1 - y0
    t = np.clip(((x - x0) * vx + (y - y0) * vy) / (vx * vx + vy * vy), 0, 1)
    return np.sqrt((x - x0 - t * vx) ** 2 + (y - y0 - t * vy) ** 2), t


def dissolve(density: np.ndarray, x: np.ndarray, y: np.ndarray, gone: float,
             seed: int) -> np.ndarray:
    """Eaten away by noise: at `gone` 0 nothing is lost, at 1 all of it."""
    if gone <= 0:
        return density
    n = fbm(x, y, 7, seed, 4)
    return density * smooth(gone - 0.05, gone + 0.1, n * 1.1)
