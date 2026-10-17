#!/usr/bin/env python3
"""Покадровые земли: огонь, гроза, могила, хрусталь.

Прежний жар двигал статичные картинки CSS-ом — покачивал, гасил, поднимал, — и
огонь читался свечой: фигура, которая качается, но не горит. Огонь горит
ПОЛЕМ (см. `vfx.py`): плотность на пиксель, взбитая турбулентностью, которая
каждый кадр другая. Поэтому здесь не слои для CSS, а ПОЛОСЫ — `FRAMES` кадров
в ряд, которые CSS листает ступенчато (`steps()`), ни строки JS на кадре. Тот
же приём, что у движений боя (`BATTLE-MOTION.md`).

    tools/cutout/.venv/bin/python tools/ground_fx.py [земля …]

Каждая полоса ЗАМКНУТА: последний кадр переходит в первый без шва. Три способа:
  · круг по шуму — точка выборки идёт по окружности (`loop_noise`): шум кипит
    и возвращается;
  · скольжение с перекрёстным растворением (`rise_noise`) — рисунок едет вверх,
    а когда проехал период, в него вплывает его же начало;
  · частицы с круговой жизнью — у каждой своя фаза, и кадр `t` и кадр `t + 1`
    ставят её в одно место.
Всё засеяно: те же аргументы дают те же пиксели.

Рисует и плитки земель (`ground_tiles.py`) — для тех, что в этом файле.
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter

sys.path.insert(0, str(Path(__file__).parent))
import vfx  # noqa: E402
import ground_tiles as gt  # noqa: E402
from ground_tiles import C, EXTRA, H, W, finish, lit, mix, paint, rgb, soil  # noqa: E402

FW, FH = 240, 320        # кадр полосы: 3 : 4, как клетка
FRAMES = 16
smooth = vfx.smooth


def fgrid() -> tuple[np.ndarray, np.ndarray]:
    """Координаты кадра, но в единицах плитки (W × H): геометрия земли одна."""
    ys, xs = np.mgrid[0:FH, 0:FW].astype(np.float32)
    return (xs + 0.5) * W / FW, (ys + 0.5) * H / FH


def loop_noise(x, y, scale, seed, t, radius, octaves=4, phase=0.0):
    """Шум, который кипит и возвращается: точка выборки ходит по окружности."""
    a = 2 * np.pi * t + phase
    return vfx.fbm(x + radius * np.cos(a), y + radius * np.sin(a), scale, seed, octaves)


def rise_noise(x, y, scale, seed, t, travel, octaves=4, sx=1.0, sy=1.0):
    """Шум, едущий вверх на `travel` за круг. Два экземпляра, сдвинутых на
    период, растворяются друг в друга — так он замыкается; веса подобраны по
    дисперсии, чтобы в середине круга рисунок не тускнел."""
    a = vfx.fbm(x * sx, (y + travel * t) * sy, scale, seed, octaves)
    b = vfx.fbm(x * sx, (y + travel * (t - 1)) * sy, scale, seed, octaves)
    wa, wb = 1 - t, t
    v = (wa * a + wb * b - 0.5) / np.sqrt(wa * wa + wb * wb) + 0.5
    return np.clip(v, 0, 1)


def blob(x, y, px, py, sigma, sy=None):
    sy = sy or sigma
    return np.exp(-(((x - px) / sigma) ** 2 + ((y - py) / sy) ** 2) * 0.5)


def strip(frames: list[np.ndarray]) -> Image.Image:
    """Кадры (FH, FW, 4), значения 0..255, — в одну ленту."""
    # Свет не доходит до кромки кадра: полоса обрезана по клетке, и срез был бы виден.
    yy, xx = np.mgrid[0:FH, 0:FW].astype(np.float32)
    fade = smooth(0.0, 16.0, np.minimum(np.minimum(xx, FW - 1 - xx), np.minimum(yy, FH - 1 - yy)))
    frames = [np.concatenate([f[..., :3], f[..., 3:] * fade[..., None]], axis=-1) for f in frames]
    row = np.concatenate(frames, axis=1)
    return Image.fromarray(np.clip(row, 0, 255).astype(np.uint8), "RGBA")


def layer(col: np.ndarray, alpha: np.ndarray) -> np.ndarray:
    out = np.zeros(alpha.shape + (4,), np.float32)
    out[..., :3] = col
    out[..., 3] = np.clip(alpha, 0, 1) * 255
    return out


def over(base: np.ndarray, top: np.ndarray) -> np.ndarray:
    """Наложение двух непрямых RGBA (float, 0..255)."""
    ta, ba = top[..., 3:4] / 255.0, base[..., 3:4] / 255.0
    oa = ta + ba * (1 - ta)
    rgb_ = (top[..., :3] * ta + base[..., :3] * ba * (1 - ta)) / np.maximum(oa, 1e-4)
    return np.concatenate([rgb_, oa * 255], axis=-1)


def flat(hexstr: str, ref: np.ndarray) -> np.ndarray:
    return np.broadcast_to(rgb(hexstr), ref.shape + (3,)).copy()


# ── Огонь ────────────────────────────────────────────────────────────────────

FIRE_RAMP = [(0.0, (120, 22, 6, 0)), (0.18, (190, 44, 8, 170)), (0.42, (245, 100, 18, 235)),
             (0.7, (255, 170, 52, 250)), (0.9, (255, 224, 130, 255)), (1.0, (255, 250, 214, 255))]


def fire_strip() -> Image.Image:
    """Лава в швах и пламя над ними. Источник огня — сами швы (`embers_geo`):
    пламя стоит там, где на плитке трещина, и тянется вверх, редея."""
    x, y = fgrid()
    g = gt.embers_geo(x, y)
    mask, edge, width = g["mask"], g["edge"], g["width"]
    crack = smooth(width * 1.15, 0.0, edge) * mask
    # Высота пламени над швом: от источника вверх с затуханием, по строкам снизу.
    decay = float(np.exp(-(H / FH) / 38.0))
    reach = np.zeros_like(crack)
    for r in range(FH - 1, -1, -1):
        up = reach[r + 1] * decay if r + 1 < FH else 0.0
        reach[r] = np.maximum(crack[r], up)
    # Пламя слегка раздувается в стороны от шва: источник размыт по горизонтали.
    reach = np.maximum(reach, gaussian_filter(reach, (1.0, 3.0)) * 1.05)
    reach = np.clip(reach, 0, 1) * (y > H * 0.12)

    rng = np.random.default_rng(401)
    cand = np.argwhere(crack > 0.6)
    pick = cand[rng.choice(len(cand), 34, replace=False)]
    sparks = [(float(x[r, c]), float(y[r, c]), rng.uniform(0, 1), rng.uniform(110, 250),
               rng.uniform(5, 15), rng.uniform(1.0, 2.6), rng.uniform(0, 6.28)) for r, c in pick]

    frames = []
    for k in range(FRAMES):
        t = k / FRAMES
        sway = 7.0 * np.sin(2 * np.pi * t + y * 0.025)
        xs = x + sway * smooth(0.0, 0.6, 1 - reach) * 0.0 + sway * (1 - reach) * 0.6
        # Пламя: два шума скольжения с разным шагом и мелкая рябь по кругу.
        tong = rise_noise(xs, y, 30.0, 411, t, 120.0, 5, 1.15, 0.5)
        fine = loop_noise(xs, y * 1.2, 11.0, 421, t, 5.0, 3)
        n = 0.68 * tong + 0.32 * fine
        dens = reach * (0.25 + 1.6 * n) - 0.36
        d = smooth(0.0, 0.7, dens) * 0.92
        # Белый стержень у самого шва: горячее всего там, где источник.
        d = np.clip(d + crack * 0.2 * (0.6 + 0.4 * loop_noise(x, y, 14.0, 431, t, 6.0, 3)), 0, 1)
        fire = vfx.ramp(d ** 1.1, FIRE_RAMP)
        # Лава в швах бурлит: яркость ползёт по шву, не стоит на месте.
        boil = loop_noise(x * 1.4, y, 21.0, 441, t, 14.0, 4, 1.7)
        lava_heat = np.clip(crack * (0.25 + 0.8 * boil), 0, 0.92)
        lava = vfx.ramp(lava_heat, [(0.0, (110, 22, 8, 0)), (0.25, (190, 48, 10, 150)),
                                    (0.6, (255, 120, 28, 210)), (0.9, (255, 205, 100, 245)),
                                    (1.0, (255, 245, 205, 255))])
        frame = over(lava, fire)
        # Искры: живут один круг, у каждой своя фаза, поднимаются и гаснут.
        glow = np.zeros_like(x)
        for sx, sy_, ph, speed, amp, size, psi in sparks:
            u = (t + ph) % 1.0
            px = sx + amp * np.sin(2 * np.pi * u * 1.5 + psi)
            py = sy_ - speed * u
            life = np.sin(np.pi * u) ** 0.8 * (0.65 + 0.35 * np.sin(2 * np.pi * (u * 5 + ph)))
            glow = np.maximum(glow, blob(x, y, px, py, size * 1.5 * (1 - u * 0.4)) * life)
        spark = layer(flat("#ffc25a", x), np.clip(glow * 1.3, 0, 1))
        spark[..., :3] = mix(spark[..., :3], flat("#fff3c8", x), np.clip(glow - 0.45, 0, 1) * 1.8)
        frame = over(frame, spark)
        frame[..., 3] *= smooth(0.0, 0.05, np.minimum(np.minimum(x, W - x), np.minimum(y, H - y)) / W)
        frames.append(frame)
    EXTRA["embers-fire"] = strip(frames)
    return gt.embers()


# ── Гроза ────────────────────────────────────────────────────────────────────

STORM_C = (W * 0.5, H * 0.58)                 # середина плиты
STORM_R = (W * 0.4, H * 0.27)                 # её полуоси
# Громоотводы: основание на плите, остриё над ней. Остриё — то, между чем бьёт.
RODS = [(-0.62, -0.38), (0.62, -0.38), (-0.5, 0.52), (0.5, 0.52)]
ROD_H = 66.0


def rod_points() -> list[tuple[tuple[float, float], tuple[float, float]]]:
    cx, cy = STORM_C
    rx, ry = STORM_R
    return [((cx + a * rx, cy + b * ry), (cx + a * rx, cy + b * ry - ROD_H)) for a, b in RODS]


def storm_tile() -> Image.Image:
    """Плита грозы: тёмный сланец, жилы оплавленного стекла (там, где били
    молнии), четыре железных громоотвода. Свет — только в полосах."""
    x, y = gt.coords()
    cx, cy = STORM_C
    rx, ry = STORM_R
    ux, uy = (x - cx) / rx, (y - cy) / ry
    r = np.sqrt(ux * ux + uy * uy) + (vfx.fbm(x, y, 24.0, 501, 3) - 0.5) * 0.07
    disk = 1 - smooth(0.96, 1.02, r)
    height = 0.5 * smooth(1.02, 0.9, r) + vfx.fbm(x, y, 9.0, 502, 4) * 0.22
    slate = mix(C("#262c37", x), C("#4d586a", x), vfx.fbm(x, y, 20.0, 503, 3))
    col = paint(slate, lit(height, 20.0), 0.34)
    # Расплавленные жилы: ветвящиеся тонкие линии от громоотводов к середине.
    veins = np.zeros_like(x)
    rng = np.random.default_rng(504)
    for (bx, by), _tip in rod_points():
        px, py = bx, by
        for _ in range(7):
            nx = px + (cx - px) * 0.18 + rng.uniform(-14, 14)
            ny = py + (cy - py) * 0.18 + rng.uniform(-9, 9)
            dd, _t = vfx.segment_distance(x, y, px, py, nx, ny)
            veins = np.maximum(veins, smooth(2.6, 0.5, dd))
            px, py = nx, ny
    col = mix(col, C("#9fb7d6", x), veins * 0.55 * disk)
    col = col * (1 - 0.4 * smooth(0.85, 1.0, r))[..., None]
    solid = disk.copy()
    # Громоотводы: железная жердь с шаром на острие и тенью на плиту.
    for (bx, by), (tx, ty) in rod_points():
        dd, tt = vfx.segment_distance(x, y, bx, by, tx, ty)
        shadow = smooth(5.0, 0.5, np.sqrt((x - bx - 9) ** 2 + ((y - by) * 0.6 - 3) ** 2) * 0.0 + dd + 6) * 0
        rod = smooth(3.2, 1.2, dd)
        shade = 0.55 + 0.6 * smooth(-3, 3, (x - bx))
        ironc = flat("#5b6370", x) * shade[..., None]
        col = mix(col, ironc, rod)
        ball = smooth(5.2, 3.6, np.hypot(x - tx, y - ty))
        col = mix(col, flat("#9ea7b6", x) * (0.8 + 0.4 * smooth(-4, 4, tx - x))[..., None], ball)
        solid = np.maximum(solid, np.maximum(rod, ball))
        _ = shadow
    return finish(col, solid, x, y, 10, 15, 8.0, 0.45)


def bolt_points(rng, p0, p1, depth=5, jitter=0.28):
    """Разлом по середине: молния — ломаная, а не прямая."""
    pts = [p0, p1]
    amp = np.hypot(p1[0] - p0[0], p1[1] - p0[1]) * jitter
    for _ in range(depth):
        out = [pts[0]]
        for a, b in zip(pts, pts[1:]):
            mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
            dx, dy = b[0] - a[0], b[1] - a[1]
            ln = max(np.hypot(dx, dy), 1e-3)
            off = rng.normal(0, amp)
            out.append((mx - dy / ln * off, my + dx / ln * off))
            out.append(b)
        pts = out
        amp *= 0.55
    return pts


def draw_bolt(x, y, pts, w):
    """Расстояние до ломаной -> три слоя света: стержень, кайма, ореол."""
    best = np.full_like(x, 1e9)
    for a, b in zip(pts, pts[1:]):
        dd, _t = vfx.segment_distance(x, y, a[0], a[1], b[0], b[1])
        best = np.minimum(best, dd)
    return smooth(w * 1.0, w * 0.2, best), smooth(w * 3.4, w * 0.8, best), smooth(w * 14.0, w * 1.0, best)


# Какие кадры вспыхивают: два удара за круг и дальний отзвук между ними.
STORM_FLASH = [0.04, 0.04, 1.0, 0.7, 0.95, 0.3, 0.08, 0.06, 0.05, 0.1, 0.8, 1.0, 0.45, 0.12, 0.06, 0.05]


def bolt_strip() -> None:
    x, y = fgrid()
    cx, cy = STORM_C
    rx, ry = STORM_R
    ux, uy = (x - cx) / rx, (y - cy) / ry
    plate = (1 - smooth(0.96, 1.05, np.sqrt(ux * ux + uy * uy)))
    rods = rod_points()
    tips = [t for _b, t in rods]
    frames = []
    for k in range(FRAMES):
        f = STORM_FLASH[k]
        rng = np.random.default_rng(600 + k)
        core = np.zeros_like(x)
        mid = np.zeros_like(x)
        halo = np.zeros_like(x)
        # Корона на остриях живёт всегда: слабая, мигает.
        for (tx, ty) in tips:
            fl = 0.35 + 0.65 * (0.5 + 0.5 * np.sin(2 * np.pi * (k / FRAMES * 3 + tx * 0.01)))
            halo = np.maximum(halo, blob(x, y, tx, ty, 9.0) * (0.25 + 0.5 * f) * fl)
            core = np.maximum(core, blob(x, y, tx, ty, 2.2) * (0.5 + 0.5 * f))
        if f > 0.25:
            # Дуги между остриями и удар в середину плиты, несколько за кадр.
            links = [(0, 1), (1, 3), (0, 2), (2, 3), (0, 3), (1, 2)]
            for _ in range(2 if f < 0.9 else 3):
                a, b = links[rng.integers(0, len(links))]
                c_, m_, h_ = draw_bolt(x, y, bolt_points(rng, tips[a], tips[b], 5, 0.22), 1.5)
                core, mid, halo = np.maximum(core, c_ * f), np.maximum(mid, m_ * f), np.maximum(halo, h_ * f)
            tip = tips[rng.integers(0, len(tips))]
            end = (cx + rng.uniform(-0.3, 0.3) * rx, cy + rng.uniform(-0.2, 0.2) * ry)
            c_, m_, h_ = draw_bolt(x, y, bolt_points(rng, tip, end, 5, 0.3), 1.9)
            core, mid, halo = np.maximum(core, c_ * f), np.maximum(mid, m_ * f), np.maximum(halo, h_ * f)
            # Ветки от главного удара.
            for _ in range(2):
                s0 = (tip[0] + (end[0] - tip[0]) * rng.uniform(0.25, 0.7), tip[1] + (end[1] - tip[1]) * rng.uniform(0.25, 0.7))
                s1 = (s0[0] + rng.uniform(-48, 48), s0[1] + rng.uniform(10, 46))
                c_, m_, h_ = draw_bolt(x, y, bolt_points(rng, s0, s1, 4, 0.3), 1.0)
                core, mid, halo = np.maximum(core, c_ * f * 0.8), np.maximum(mid, m_ * f * 0.7), np.maximum(halo, h_ * f * 0.6)
        flash = plate * f * 0.2                       # плита загорается от удара
        col = mix(mix(flat("#7da6ff", x), flat("#cfe0ff", x), np.clip(mid, 0, 1)),
                  flat("#ffffff", x), np.clip(core, 0, 1))
        alpha = np.clip(halo * 0.5 + mid * 0.75 + core + flash, 0, 1)
        frames.append(layer(col, alpha))
    EXTRA["storm-bolt"] = strip(frames)


def sky_strip() -> None:
    """Тучи над плитой, вращающиеся целиком, и косой дождь."""
    x, y = fgrid()
    cx, cy = W * 0.5, H * 0.3
    rng = np.random.default_rng(701)
    rain = [(rng.uniform(0, W), rng.uniform(0, H), rng.uniform(0, 1), rng.uniform(0.8, 1.2)) for _ in range(46)]
    frames = []
    for k in range(FRAMES):
        t = k / FRAMES
        a = 2 * np.pi * t
        c, s = np.cos(a), np.sin(a)
        dx, dy = x - cx, (y - cy) * 1.5
        rxn, ryn = cx + dx * c - dy * s, cy + dx * s + dy * c
        cloud = smooth(0.42, 0.78, vfx.warped(rxn, ryn, 55.0, 711, 38.0))
        dist = np.sqrt(((x - cx) / (W * 0.46)) ** 2 + ((y - cy) / (H * 0.2)) ** 2)
        body = cloud * smooth(1.0, 0.45, dist)
        shade = mix(flat("#2a3243", x), flat("#7b8aa6", x), smooth(0.4, 0.9, vfx.fbm(rxn, ryn, 24.0, 713, 3)))
        sky = layer(shade, body * 0.85)
        drops = np.zeros_like(x)
        for px, py, ph, sp in rain:
            u = (t * sp + ph) % 1.0
            yy = (py + u * H * 1.1) % H
            xx = px + (yy - py) * -0.22
            dd, _t = vfx.segment_distance(x, y, xx, yy, xx - 3.2, yy - 15.0)
            drops = np.maximum(drops, smooth(1.3, 0.2, dd) * np.sin(np.pi * np.clip(yy / H, 0, 1)) ** 0.6)
        rainl = layer(flat("#a9c4e8", x), drops * 0.55 * smooth(0.28, 0.5, y / H))
        frames.append(over(sky, rainl))
    EXTRA["storm-sky"] = strip(frames)


def storm() -> Image.Image:
    bolt_strip()
    sky_strip()
    return storm_tile()


# ── Могила ───────────────────────────────────────────────────────────────────

def grave_tile() -> Image.Image:
    """Проклятая земля: потрескавшийся тёмный грунт, из трещин сочится
    зеленоватый свет, три покосившиеся плиты. Свет и души — в полосе."""
    x, y = gt.coords()
    cx, cy = W * 0.5, H * 0.56
    r = np.sqrt(((x - cx) / (W * 0.44)) ** 2 + ((y - cy) / (H * 0.33)) ** 2) \
        + (vfx.fbm(x, y, 28.0, 801, 3) - 0.5) * 0.2 + (vfx.fbm(x, y, 8.0, 802, 3) - 0.5) * 0.05
    mask = 1 - smooth(0.93, 1.0, r)
    rng = np.random.default_rng(803)
    pts = np.array([(rng.uniform(0.05, 0.95) * W, rng.uniform(0.2, 0.9) * H) for _ in range(13)], np.float32)
    d1, d2, idx = vfx.voronoi(x, y, pts)
    edge = d2 - d1
    seam = smooth(3.0, 0.2, edge)
    height = vfx.fbm(x, y, 9.0, 804, 4) * 0.5 + smooth(0, 14, edge) * 0.4 - seam * 0.5
    dirt = mix(C("#1d1824", x), C("#3b3442", x), vfx.fbm(x, y, 26.0, 805, 3))
    col = paint(dirt, lit(height, 12.0), 0.3)
    col = mix(col, C("#58d9a0", x), seam * 0.35)
    col = col + C("#2fa07a", x) * (smooth(26.0, 0.0, edge) ** 2 * 0.18)[..., None]
    solid = mask.copy()
    # Плиты: каменные, закруглённые сверху, у каждой свой наклон и мох.
    for i, (px, py, wd, ht, lean) in enumerate([(0.3, 0.6, 46, 74, -0.1), (0.62, 0.52, 40, 66, 0.12), (0.52, 0.76, 52, 70, -0.04)]):
        bx, by = W * px, H * py
        xs = (x - bx) - lean * (by - y)
        top = by - ht
        body = smooth(wd / 2 + 1.5, wd / 2 - 1.0, np.abs(xs)) * (y < by) * (y > top + wd / 2)
        cap = smooth(wd / 2 + 1.5, wd / 2 - 1.0, np.hypot(xs, y - (top + wd / 2))) * (y <= top + wd / 2)
        slab = np.clip(body + cap, 0, 1)
        h2 = slab * (0.6 + 0.4 * smooth(-wd / 2, wd / 2, xs)) + vfx.fbm(x, y, 7.0, 810 + i, 3) * 0.12
        sc = paint(mix(C("#3b3a42", x), C("#6f6c78", x), vfx.fbm(x, y, 16.0, 820 + i, 3)), lit(h2, 16.0), 0.36)
        moss = smooth(0.5, 0.68, vfx.fbm(x, y, 11.0, 830 + i, 3)) * smooth(top + 20, by, y)
        sc = mix(sc, C("#4f6a3c", x), moss * 0.55)
        cross = np.maximum(smooth(1.8, 0.6, np.abs(xs)) * (y > top + 18) * (y < top + 46),
                           smooth(1.8, 0.6, np.abs(y - (top + 28))) * (np.abs(xs) < 10))
        sc = mix(sc, C("#1c1a1f", x), cross * 0.6 * slab)
        shadow = smooth(1.15, 0.8, np.hypot((xs - 10) / (wd * 0.8), (y - by - 4) / 12))
        col = col * (1 - 0.45 * shadow * (1 - slab))[..., None]
        col = mix(col, sc, slab)
        solid = np.maximum(solid, slab)
    return finish(col, solid, x, y, 9, 14, 8.0, 0.45)


def grave_strip() -> None:
    """Души: призрачные огни поднимаются из трещин по извивающимся путям,
    у каждого хвост; по земле стелется зелёный туман, который кипит."""
    x, y = fgrid()
    cx, cy = W * 0.5, H * 0.56
    r = np.sqrt(((x - cx) / (W * 0.44)) ** 2 + ((y - cy) / (H * 0.33)) ** 2)
    patch = smooth(1.15, 0.5, r)
    rng = np.random.default_rng(901)
    wisps = [(rng.uniform(0.28, 0.72) * W, rng.uniform(0.58, 0.78) * H, rng.uniform(0, 1),
              rng.uniform(150, 215), rng.uniform(14, 30), rng.uniform(1.0, 2.0), rng.uniform(0, 6.28),
              rng.uniform(0.8, 1.3)) for _ in range(8)]
    frames = []
    for k in range(FRAMES):
        t = k / FRAMES
        fog = smooth(0.4, 0.75, loop_noise(x * 0.8, y * 1.6, 46.0, 911, t, 22.0, 4))
        fog = fog * patch * smooth(0.9, 0.35, (y - H * 0.32) / (H * 0.5))
        frame = layer(mix(flat("#2c6b58", x), flat("#8fe3c0", x), fog), fog * 0.5)
        glow = np.zeros_like(x)
        core = np.zeros_like(x)
        for wx, wy, ph, rise, amp, turns, psi, sz in wisps:
            for tail in range(9):                              # хвост — те же кружки с запаздыванием
                u = (t + ph - tail * 0.018) % 1.0
                px = wx + amp * np.sin(2 * np.pi * u * turns + psi) * (0.3 + u)
                py = wy - rise * u
                life = np.sin(np.pi * u) ** 0.9 * (1 - tail / 10.0)
                s = (7.5 - tail * 0.5) * sz * (0.6 + 0.6 * u)
                glow = np.maximum(glow, blob(x, y, px, py, s, s * 1.5) * life)
                if tail == 0:
                    core = np.maximum(core, blob(x, y, px, py, 2.4 * sz, 3.6 * sz) * life)
        wcol = mix(flat("#46d6a2", x), flat("#e8fff2", x), np.clip(core * 1.4, 0, 1))
        frame = over(frame, layer(wcol, np.clip(glow * 0.62 + core, 0, 1)))
        frames.append(frame)
    EXTRA["grave-spirit"] = strip(frames)


def grave() -> Image.Image:
    grave_strip()
    return grave_tile()


# Медленным землям нужно больше кадров: свет, который сдвигается на волос за
# секунду, при шестнадцати кадрах ходил бы рывками — человек видит ступеньки.
SLOW = 32

# ── Хрусталь ─────────────────────────────────────────────────────────────────

# Друза тёмного горного хрусталя. Один материал, три оттенка дыма: цвет здесь
# не украшение, а порода. Каждый кристалл — шестигранная призма с острием.
# (смещение основания, наклон °, длина, ширина, оттенок)
CRYSTALS = [
    (-92, 16, -62, 74, 30, "ash"),
    (90, 12, 60, 70, 30, "ink"),
    (-58, 6, -30, 132, 46, "smoke"),
    (58, 4, 28, 140, 48, "ink"),
    (0, -2, 3, 228, 74, "smoke"),
    (-28, 12, -14, 176, 58, "ash"),
    (30, 10, 13, 166, 56, "smoke"),
    (-12, 26, -25, 96, 38, "ink"),
]
CRYSTAL_BASE = (W * 0.5, H * 0.76)
PALETTES = {
    "smoke": ((8, 12, 20), (46, 66, 94), (152, 178, 206)),
    "ash": ((14, 13, 16), (72, 68, 78), (176, 170, 186)),
    "ink": ((5, 7, 14), (28, 40, 66), (110, 140, 180)),
}


def crystal_pixels(x, y, light_angle, t):
    """Вся друза одним проходом. Нормаль считается по грани (левая, средняя,
    правая; у острия — с наклоном вверх), свет — диффузный и один резкий блик;
    по оси идёт слабое холодное свечение изнутри, а в толще — тонкие трещины."""
    bx0, by0 = CRYSTAL_BASE
    col = np.zeros(x.shape + (3,), np.float32)
    cover = np.zeros_like(x)
    light = np.array([np.cos(light_angle) * 0.78, np.sin(light_angle) * 0.78, 0.62], np.float32)
    light /= np.linalg.norm(light)
    half_v = light + np.array([0, 0, 1.0], np.float32)
    half_v /= np.linalg.norm(half_v)
    for ci, (ox, oy, ang, ln, wd, pal) in enumerate(CRYSTALS):
        a = np.deg2rad(ang)
        d = np.array([np.sin(a), -np.cos(a)], np.float32)
        pv = np.array([np.cos(a), np.sin(a)], np.float32)
        bx, by = bx0 + ox, by0 + oy
        s_ = (x - bx) * d[0] + (y - by) * d[1]
        u_ = (x - bx) * pv[0] + (y - by) * pv[1]
        tip = wd * 1.3
        body = ln - tip
        half = np.where(s_ <= body, wd / 2, wd / 2 * np.clip(1 - (s_ - body) / tip, 0.0, 1.0))
        half = half * (1 + 0.04 * (vfx.fbm(x, y, 9.0, 970 + ci, 2) - 0.5))
        inside = smooth(half + 1.0, half - 1.0, np.abs(u_)) * smooth(-1.0, 1.0, s_) * smooth(ln + 1.0, ln - 1.0, s_)
        f = np.clip(u_ / np.maximum(half, 1e-3), -1, 1)
        face = np.where(f < -0.5, 0, np.where(f > 0.5, 2, 1))
        phi = np.where(face == 0, np.deg2rad(-62), np.where(face == 2, np.deg2rad(62), np.deg2rad(f * 40)))
        n3 = np.stack([pv[0] * np.sin(phi), pv[1] * np.sin(phi), np.cos(phi)], axis=-1)
        in_tip = (s_ > body)[..., None]
        axis3 = np.array([d[0], d[1], 0.0], np.float32)
        nrm = np.where(in_tip, n3 * np.cos(np.deg2rad(40)) + axis3 * np.sin(np.deg2rad(40)), n3)
        nrm /= np.linalg.norm(nrm, axis=-1, keepdims=True)
        diff = np.clip((nrm * light).sum(-1), 0, 1)
        spec = np.clip((nrm * half_v).sum(-1), 0, 1) ** 70
        dark, mid, bright = (np.array(c, np.float32) for c in PALETTES[pal])
        ts = np.clip(s_ / ln, 0, 1)
        base = mix(np.broadcast_to(dark, x.shape + (3,)).copy(), np.broadcast_to(mid, x.shape + (3,)).copy(), smooth(0.0, 0.6, ts))
        base = mix(base, np.broadcast_to(bright, x.shape + (3,)).copy(), smooth(0.55, 1.0, ts) * 0.4)
        tint = np.random.default_rng(ci * 7 + 3).uniform(0.82, 1.12, 3).astype(np.float32)[face]
        c = base * ((0.2 + 0.95 * diff) * tint)[..., None]
        core = np.exp(-(f / 0.55) ** 2)
        wave = 0.5 + 0.5 * np.cos(2 * np.pi * (ts * 0.9 - t - ci * 0.09))
        c = c + np.array([84, 124, 176], np.float32) * (core * (0.05 + 0.3 * ts) * (0.5 + 0.7 * wave))[..., None]
        crack = smooth(0.025, 0.0, np.abs(vfx.fbm(u_ * 0.3 + ci * 13, s_ * 0.08, 7.0, 990 + ci, 3) - 0.5))
        c = c + bright * (crack * (0.03 + 0.1 * diff))[..., None]
        ridge = smooth(0.05, 0.0, np.abs(np.abs(f) - 0.5)) * 0.45 + smooth(0.08, 0.0, 1 - np.abs(f)) * 0.25
        c = c + bright * (ridge * (0.25 + 0.6 * diff))[..., None] * 0.7
        c = c + np.array([232, 242, 255], np.float32) * (spec * 0.95)[..., None]
        col = mix(col, c, inside)
        cover = np.maximum(cover, inside)
    return np.clip(col, 0, 255), cover


def crystal_floor(x, y):
    cx, cy = CRYSTAL_BASE[0], CRYSTAL_BASE[1] + 6
    r = np.sqrt(((x - cx) / (W * 0.46)) ** 2 + ((y - cy) / (H * 0.16)) ** 2)
    bed = smooth(1.0, 0.82, r + (vfx.fbm(x, y, 20.0, 981, 3) - 0.5) * 0.18)
    h0 = vfx.fbm(x, y, 8.0, 982, 4) * 0.6
    rock = mix(C("#14161c", x), C("#2c303a", x), vfx.fbm(x, y, 18.0, 983, 3))
    col = paint(rock, lit(h0, 12.0), 0.3)
    col = col + np.array([34, 52, 82], np.float32) * (smooth(1.2, 0.0, r) ** 1.6 * 0.5)[..., None]
    return col, bed


def crystal_tile() -> Image.Image:
    x, y = gt.coords()
    floor, bed = crystal_floor(x, y)
    cc, cover = crystal_pixels(x, y, np.arctan2(-0.65, -0.55), 0.15)
    col = mix(floor, cc, cover)
    return finish(col, np.maximum(bed, cover), x, y, 9, 14, 8.0, 0.45)


def crystal_strip() -> None:
    """Свет качается над друзой — не кружит: за круг он уходит вправо и
    возвращается, и блики скользят по граням и гаснут. Внутри дышит холодный
    свет. Никаких звёзд и радуг: хрусталь не украшение."""
    x, y = fgrid()
    tips = []
    for ox, oy, ang, ln, wd, _pal in CRYSTALS[2:7]:
        a = np.deg2rad(ang)
        tips.append((CRYSTAL_BASE[0] + ox + np.sin(a) * ln, CRYSTAL_BASE[1] + oy - np.cos(a) * ln))
    theta0 = np.arctan2(-0.65, -0.55)
    frames = []
    for k in range(SLOW):
        t = k / SLOW
        floor, bed = crystal_floor(x, y)
        cn = loop_noise(x, y * 2.2, 17.0, 991, t, 12.0, 4)
        caustic = smooth(0.08, 0.0, np.abs(cn - 0.5)) * smooth(0.0, 0.6, bed)
        under = layer(np.array([90, 130, 190], np.float32)[None, None, :] * np.ones(x.shape + (1,)), np.clip(caustic * 0.16, 0, 1))
        cc, cover = crystal_pixels(x, y, theta0 + 0.8 * np.sin(2 * np.pi * t), t)
        glint = np.zeros_like(x)
        for si, (tx, ty) in enumerate(tips):
            f = max(0.0, np.cos(2 * np.pi * (t - si * 0.2))) ** 40
            glint = np.maximum(glint, blob(x, y, tx, ty, 3.2) * f)
        frame = over(under, layer(cc, cover))
        frame = over(frame, layer(np.array([236, 244, 255], np.float32)[None, None, :] * np.ones(x.shape + (1,)), np.clip(glint, 0, 1)))
        frames.append(frame)
    EXTRA["crystal-light"] = strip(frames)


def crystal() -> Image.Image:
    crystal_strip()
    return crystal_tile()


# ── Витраж ───────────────────────────────────────────────────────────────────

GLASS_C = (W * 0.5, H * 0.58)
GLASS_HW, GLASS_HH, GLASS_SHEAR = 74.0, 142.0, 0.16
# Готическое окно, как в соборе: рамка из тёмно-красных и синих полос, поле из
# бледных ромбов (в простом переплёте свет почти белый), в середине медальон.
# Цвета глубокие и немногие — витраж нарядный не пестротой, а тем, что тёмное
# стекло горит в светлом.
GLASS_RED, GLASS_BLUE, GLASS_AMBER = (116, 30, 36), (30, 46, 104), (150, 108, 40)


def arch_width(v):
    """Стрельчатая арка: ровные стенки, потом сужение до острия."""
    return np.where(v < 0.62, 1.0, np.clip(1 - ((v - 0.62) / 0.38) ** 1.3, 0.0, 1.0))


def glass_window(x, y, t):
    """Стрельчатое окно, брошенное косым лучом на плиты: (цвет, покрытие, свинец,
    «плотность» стекла — тёмное пропускает меньше света)."""
    cx, cy = GLASS_C
    u = (x - cx - GLASS_SHEAR * (y - cy)) / GLASS_HW
    v = (cy + GLASS_HH - y) / (2 * GLASS_HH)
    lx, ly = u * GLASS_HW, v * 2 * GLASS_HH
    w = arch_width(v)
    rel = np.abs(u) / np.maximum(w, 1e-3)
    inside = smooth(1.03, 0.97, rel) * smooth(-0.015, 0.015, v) * smooth(1.015, 0.985, v)
    # Поле: ромбы под 45°; у каждого свой тон бледного стекла.
    k = 21.0
    ra, rb = (lx + ly) / k, (lx - ly) / k
    ca, cb = np.floor(ra).astype(np.int64), np.floor(rb).astype(np.int64)
    h = ((ca * 73856093) ^ (cb * 19349663)) % 1000 / 1000.0
    h = h.astype(np.float32)
    tone = np.stack([232 + 14 * h, 222 + 10 * h, 190 - 10 * h], axis=-1)
    ph = (((ca * 31 + cb * 17) % 97) / 97.0).astype(np.float32)
    col = tone * (0.88 + 0.07 * np.sin(2 * np.pi * (t + ph)) + 0.1 * h)[..., None]
    dens = np.full_like(x, 0.4)
    lead = np.maximum(smooth(0.1, 0.02, np.abs(ra - np.round(ra))), smooth(0.1, 0.02, np.abs(rb - np.round(rb))))
    # Рамка: полосы тёмно-красного и синего, между ними свинец.
    band = rel > 0.8
    seg = np.floor(ly / 27.0).astype(np.int64) % 2
    bcol = np.where(seg[..., None] == 0, np.array(GLASS_RED, np.float32), np.array(GLASS_BLUE, np.float32))
    col = np.where(band[..., None], bcol * (0.92 + 0.08 * np.sin(2 * np.pi * (t + h)))[..., None], col)
    dens = np.where(band, 0.7, dens)
    seam = np.minimum(ly % 27.0, 27.0 - (ly % 27.0)) / 27.0
    lead = np.where(band, smooth(0.05, 0.015, seam), lead)
    lead = np.maximum(lead, smooth(0.03, 0.0, np.abs(rel - 0.8)))
    # Медальон: синий круг с красным четырёхлистником.
    mu, mv = u * 0.74, (v - 0.44) * 1.6
    rm = np.hypot(mu, mv)
    med = smooth(0.4, 0.37, rm) * (~band)
    petals = np.minimum(np.minimum(np.hypot(mu - 0.17, mv), np.hypot(mu + 0.17, mv)),
                        np.minimum(np.hypot(mu, mv - 0.17), np.hypot(mu, mv + 0.17)))
    petal = smooth(0.15, 0.12, petals)
    mcol = np.array(GLASS_BLUE, np.float32) * (0.92 + 0.08 * np.sin(2 * np.pi * (t + 0.3)))
    col = mix(col, np.broadcast_to(mcol, col.shape).copy(), med)
    col = mix(col, np.broadcast_to(np.array(GLASS_RED, np.float32), col.shape).copy(), petal * med)
    dens = np.where(med > 0.5, 0.72, dens)
    lead = np.maximum(lead, np.maximum(smooth(0.025, 0.0, np.abs(rm - 0.385)), smooth(0.02, 0.0, np.abs(petals - 0.135)) * med))
    # Слуховое окошко над медальоном: янтарное.
    ro = np.hypot(u * 0.9, (v - 0.8) * 2.0)
    col = mix(col, np.broadcast_to(np.array(GLASS_AMBER, np.float32), col.shape).copy(), smooth(0.19, 0.16, ro) * (rel < 0.8))
    dens = np.where((ro < 0.18) & (rel < 0.8), 0.7, dens)
    lead = np.maximum(lead, smooth(0.03, 0.0, np.abs(ro - 0.175)))
    return col, inside, np.clip(lead, 0, 1), dens


def glass_tile() -> Image.Image:
    x, y = gt.coords()
    cx, cy = W * 0.5, H * 0.6
    r = np.sqrt(((x - cx) / (W * 0.45)) ** 2 + ((y - cy) / (H * 0.36)) ** 2) + (vfx.fbm(x, y, 22.0, 1011, 3) - 0.5) * 0.12
    mask = 1 - smooth(0.93, 1.0, r)
    row = np.floor(y / 54.0)
    xs = x + (row % 2) * 40.0
    joint = np.maximum(smooth(38.5, 40.0, np.abs((xs % 80.0) - 40.0)), smooth(25.5, 27.0, np.abs((y % 54.0) - 27.0)))
    stone = mix(C("#1f1e24", x), C("#47454f", x), vfx.fbm(x, y, 30.0, 1012, 3))
    h = vfx.fbm(x, y, 7.0, 1013, 4) * 0.4 - joint * 0.5
    col = paint(stone, lit(h, 12.0), 0.34) * (1 - 0.5 * joint)[..., None]
    wc, wm, lead, dens = glass_window(x, y, 0.0)
    a = gaussian_filter(wm * (1 - lead * 0.95) * dens * 0.62, 1.2)
    col = mix(col, np.clip(col * 0.55 + wc * 0.9, 0, 255), a)
    return finish(col, mask, x, y, 9, 13, 7.0, 0.4)


def glass_strip() -> None:
    """Свет в окне ходит облаками: то ярче, то глуше, стёкла чуть мерцают;
    сверху в луче стоит пыль. Цвета — на камне, а не поверх него: плиты видны
    сквозь свет."""
    x, y = fgrid()
    rng = np.random.default_rng(1021)
    motes = [(rng.uniform(0.2, 0.7) * W, rng.uniform(0.1, 0.55) * H, rng.uniform(0, 1),
              rng.uniform(0, 1), rng.uniform(5, 12), rng.uniform(0.9, 1.7)) for _ in range(30)]
    warm = np.array([214, 192, 152], np.float32)[None, None, :] * np.ones(x.shape + (1,))
    frames = []
    for k in range(SLOW):
        t = k / SLOW
        wc, wm, lead, dens = glass_window(x, y, t)
        cloud = np.clip(0.8 + 0.5 * (loop_noise(x * 0.5, y * 0.5, 70.0, 1031, t, 18.0, 3) - 0.5)
                        + 0.1 * np.sin(2 * np.pi * t), 0.5, 1.05)
        lit_ = wm * (1 - lead * 0.95) * cloud * dens
        a = gaussian_filter(lit_ * 0.68, 1.0)
        frame = over(layer(warm, np.clip(gaussian_filter(lit_, 9.0) * 0.22, 0, 1)), layer(wc, np.clip(a, 0, 1)))
        band = (x - W * 0.02) - (y - H * 0.02) * 0.52
        ray = smooth(0.0, 40.0, band) * smooth(190.0, 120.0, band) * smooth(0.0, 0.2, y / H) * smooth(0.95, 0.5, y / H)
        ray = ray * (0.5 + 0.7 * loop_noise(x * 0.4, y * 1.2, 40.0, 1041, t, 14.0, 3))
        frame = over(layer(warm, np.clip(ray * 0.13, 0, 1)), frame)
        glow = np.zeros_like(x)
        for mx, my, ph, ph2, amp, size in motes:
            px = mx + amp * np.sin(2 * np.pi * (t + ph))
            py = my + amp * 0.7 * np.cos(2 * np.pi * (t + ph))
            tw = 0.3 + 0.7 * (0.5 + 0.5 * np.sin(2 * np.pi * (t * 2 + ph2)))
            ix, iy = int(np.clip(px / W * FW, 0, FW - 1)), int(np.clip(py / H * FH, 0, FH - 1))
            glow = np.maximum(glow, blob(x, y, px, py, size * 0.5) * tw * float(ray[iy, ix] > 0.12))
        frame = over(frame, layer(np.array([255, 244, 220], np.float32)[None, None, :] * np.ones(x.shape + (1,)), np.clip(glow * 0.8, 0, 1)))
        frames.append(frame)
    EXTRA["glass-light"] = strip(frames)


def glass() -> Image.Image:
    glass_strip()
    return glass_tile()


# ── Воронки: разлом и омут ───────────────────────────────────────────────────

def fold(theta: np.ndarray, k: int) -> np.ndarray:
    """Угол, сложенный в сектор `2π/k` зеркально: рисунок, взятый по нему,
    повторяется при повороте на сектор — а значит, поворот на сектор за круг
    замыкает полосу без шва."""
    sec = 2 * np.pi / k
    ph = np.mod(theta, sec)
    return np.where(ph < sec / 2, ph, sec - ph)


def vortex_pixels(x, y, t, cfg):
    """Воронка в эллипсе: (цвет, покрытие, жар у кромки). `cfg` — всё, чем
    разлом отличается от омута: число рук, закрутка, шум, палитра."""
    cx, cy, rx, ry = cfg["geo"]
    dx, dy = (x - cx) / rx, (y - cy) / ry
    r = np.hypot(dx, dy)
    th = np.arctan2(dy, dx)
    k = cfg["arms"]
    thr = th - 2 * np.pi * t / k + cfg["twist"] * np.log(np.maximum(r, 0.04))
    m = fold(thr, k)
    sx, sy = r * np.cos(m) * cfg["scale"], r * np.sin(m) * cfg["scale"]
    n1 = vfx.fbm(sx, sy, 22.0, cfg["seed"], 5)
    n2 = vfx.fbm(sx * 1.7, sy * 1.7, 14.0, cfg["seed"] + 7, 4)
    arms = 0.5 + 0.5 * np.cos(k * thr)
    if cfg["ridged"]:
        foam = smooth(0.5, 0.82, 1 - np.abs(2 * n1 - 1)) * (0.4 + 0.6 * arms)
        v = np.clip(0.28 * arms + 0.55 * foam * smooth(0.12, 0.7, r) * smooth(1.0, 0.7, r), 0, 0.9)
    else:
        v = np.clip(0.55 * n1 + 0.5 * arms * smooth(0.1, 0.8, r) + 0.2 * n2, 0, 1)
    body = smooth(1.06, 0.94, r + (n2 - 0.5) * cfg["jag"])
    rim = smooth(0.74, 0.97, r) * body
    ring = 0.5 + 0.5 * np.cos(2 * np.pi * (r * cfg["rings"] + t)) if cfg["rings"] else 0.0
    v = np.clip(v * smooth(0.04, 0.5, r) + rim * 0.18 + (ring * 0.18 * smooth(0.2, 1.0, r) if cfg["rings"] else 0.0), 0, 1)
    col = vfx.ramp(v, cfg["ramp"])[..., :3]
    col = col * (0.25 + 0.75 * smooth(0.0, 0.5, r))[..., None] + np.array(cfg["void"], np.float32) * (1 - smooth(0.0, 0.5, r))[..., None] * 0.6
    return col, body, rim, r, th


MAELSTROM_CFG = {
    "geo": (W * 0.5, H * 0.55, W * 0.35, H * 0.25), "arms": 4, "twist": 2.3, "scale": 70.0, "seed": 1201,
    "ridged": True, "jag": 0.04, "rings": 3, "void": (2, 10, 14),
    "ramp": [(0.0, (3, 11, 15, 255)), (0.3, (10, 52, 66, 255)), (0.58, (30, 122, 134, 255)),
             (0.86, (150, 214, 212, 255)), (1.0, (226, 246, 242, 255))],
}


def vortex_strip(cfg, name, motes_col, spiral_in=True) -> None:
    x, y = fgrid()
    cx, cy, rx, ry = cfg["geo"]
    rng = np.random.default_rng(cfg["seed"] + 50)
    motes = [(rng.uniform(0, 2 * np.pi), rng.uniform(0, 1), rng.uniform(1.5, 3.0)) for _ in range(30)]
    frames = []
    for k in range(FRAMES):
        t = k / FRAMES
        col, body, rim, r, _th = vortex_pixels(x, y, t, cfg)
        halo = smooth(1.7, 1.0, r) * (1 - body) * (0.28 + 0.12 * np.sin(2 * np.pi * t))
        frame = over(layer(np.array(motes_col, np.float32)[None, None, :] * np.ones(x.shape + (1,)), halo),
                     layer(col, body))
        glow = np.zeros_like(x)
        for a0, ph, size in motes:
            u = (t + ph) % 1.0
            rad = (1.25 - u) if spiral_in else (0.2 + u)
            ang = a0 + 3.2 * u
            px, py = cx + np.cos(ang) * rx * rad, cy + np.sin(ang) * ry * rad
            glow = np.maximum(glow, blob(x, y, px, py, size * (0.6 + 0.6 * (1 - u))) * np.sin(np.pi * u) ** 0.7)
        frame = over(frame, layer(np.array(motes_col, np.float32)[None, None, :] * 0.4 + 150 + 0 * x[..., None], np.clip(glow, 0, 1)))
        frames.append(frame)
    EXTRA[name] = strip(frames)


# ── Разлом ───────────────────────────────────────────────────────────────────

RIFT_C = (W * 0.5, H * 0.54)
RIFT_A, RIFT_B = H * 0.27, W * 0.105          # полувысота и полуширина щели
RIFT_TILT = np.deg2rad(7)


def rift_geometry(x, y, t):
    """Щель: острый лист, чуть наклонённый, края рваные и дышат. `d` — от края
    внутрь (больше нуля — в щели), `xr`, `yr` — координаты вдоль щели."""
    dx, dy = x - RIFT_C[0], y - RIFT_C[1]
    ca, sa = np.cos(RIFT_TILT), np.sin(RIFT_TILT)
    xr, yr = dx * ca + dy * sa, -dx * sa + dy * ca
    v = np.clip(np.abs(yr) / RIFT_A, 0, 1.2)
    ragged = (loop_noise(xr * 0.6, yr * 0.35, 30.0, 1301, t, 6.0, 4) - 0.5) * RIFT_B * 0.9 \
        + (vfx.fbm(xr, yr, 9.0, 1302, 3) - 0.5) * RIFT_B * 0.3
    w = RIFT_B * (1 - np.minimum(v, 1.0) ** 1.6)
    d = w + ragged * np.sqrt(np.maximum(1 - v, 0)) - np.abs(xr)
    d = np.minimum(d, (1 - v) * RIFT_A * 0.6)
    return xr, yr, d


def rift_frame(x, y, t, extras=True):
    """Тёмная щель, из которой утекает холодный свет, и всё, что в неё тянет.
    Один цвет — бледный, почти без оттенка: разлом страшен тем, что он пуст."""
    xr, yr, d = rift_geometry(x, y, t)
    inside = smooth(-1.0, 1.0, d)
    flick = np.clip(0.62 + 0.55 * (loop_noise(xr * 0.25, yr * 0.1, 40.0, 1321, t, 10.0, 3) - 0.5) * 1.5
                    + 0.12 * np.sin(2 * np.pi * t), 0.3, 1.2)
    # Внутри: чернота и редкие тусклые клубы, медленно уходящие вглубь.
    wisp = smooth(0.52, 0.8, loop_noise(xr * 0.9, yr * 0.45, 30.0, 1311, t, 14.0, 4)) * smooth(0.0, RIFT_B * 0.8, d) * 0.5
    void = np.array([5, 4, 10], np.float32)[None, None, :] + np.array([58, 56, 90], np.float32)[None, None, :] * wisp[..., None]
    rng = np.random.default_rng(1331)
    stars = np.zeros_like(x)
    for i in range(24):
        sx_, sy_ = rng.uniform(-0.5, 0.5) * RIFT_B * 1.6, rng.uniform(-0.8, 0.8) * RIFT_A
        px = RIFT_C[0] + sx_ * np.cos(RIFT_TILT) - sy_ * np.sin(RIFT_TILT)
        py = RIFT_C[1] + sx_ * np.sin(RIFT_TILT) + sy_ * np.cos(RIFT_TILT)
        tw = 0.25 + 0.75 * (0.5 + 0.5 * np.sin(2 * np.pi * (t * 2 + rng.uniform(0, 1))))
        stars = np.maximum(stars, blob(x, y, px, py, 1.3) * tw * 0.7)
    void = void + 150.0 * (stars * smooth(4.0, 12.0, d))[..., None]
    er = np.hypot(xr / (RIFT_B * 3.4), yr / (RIFT_A * 1.12))
    spill = smooth(1.0, 0.08, er) ** 1.5 * 0.24 * flick * (1 - inside)
    frame = layer(np.array([104, 122, 186], np.float32)[None, None, :] * np.ones(x.shape + (1,)), np.clip(spill, 0, 1))
    frame = over(frame, layer(void, inside))
    rim = smooth(7.0, 0.0, np.abs(d)) * (0.55 + 0.45 * flick)
    line = smooth(1.7, 0.2, np.abs(d)) * flick
    rim_col = mix(np.broadcast_to(np.array([150, 166, 212], np.float32), x.shape + (3,)).copy(),
                  np.broadcast_to(np.array([240, 244, 255], np.float32), x.shape + (3,)).copy(), np.clip(line, 0, 1))
    frame = over(frame, layer(rim_col, np.clip(rim * 0.75 + line * 0.5, 0, 1)))
    if not extras:
        return frame
    # Обломки плит: кружат вокруг щели и уходят в неё, у каждого свой круг.
    rng = np.random.default_rng(1341)
    for i in range(9):
        a0, rad0, size = rng.uniform(0, 2 * np.pi), rng.uniform(0.55, 1.0), rng.uniform(4.5, 9.5)
        ang = a0 + 2 * np.pi * t * (1 if i % 2 else -1)
        px = RIFT_C[0] + np.cos(ang) * RIFT_B * 5.5 * rad0
        py = RIFT_C[1] + np.sin(ang) * RIFT_A * 0.78 * rad0
        depth = 0.55 + 0.45 * np.sin(ang)                      # дальняя половина круга темнее и мельче
        dd = np.hypot((x - px) / (size * 1.25), (y - py) / size)
        rock = smooth(1.0, 0.75, dd + (vfx.fbm(x, y, 5.0, 1350 + i, 2) - 0.5) * 0.4)
        edge_lit = smooth(1.0, 0.0, np.hypot((x - px + size * 0.4 * np.sign(RIFT_C[0] - px)) / (size * 1.2), (y - py) / size)) * rock
        rcol = np.array([44, 44, 54], np.float32)[None, None, :] * np.ones(x.shape + (1,)) * (0.6 + 0.6 * depth)
        rcol = rcol + np.array([120, 134, 176], np.float32) * (edge_lit * 0.5 * depth)[..., None]
        frame = over(frame, layer(rcol, rock * (0.55 + 0.45 * depth)))
    # Пылинки тянет внутрь.
    rng = np.random.default_rng(1351)
    glow = np.zeros_like(x)
    for i in range(26):
        a0, ph = rng.uniform(0, 2 * np.pi), rng.uniform(0, 1)
        u = (t + ph) % 1.0
        rad = 1.15 * (1 - u)
        px = RIFT_C[0] + np.cos(a0 + 1.4 * u) * RIFT_B * 4.2 * rad
        py = RIFT_C[1] + np.sin(a0 + 1.4 * u) * RIFT_A * 0.7 * rad
        glow = np.maximum(glow, blob(x, y, px, py, 1.7) * np.sin(np.pi * u) ** 0.8)
    frame = over(frame, layer(np.array([196, 208, 238], np.float32)[None, None, :] * np.ones(x.shape + (1,)), np.clip(glow * 0.7, 0, 1)))
    return frame


def rift_tile() -> Image.Image:
    x, y = gt.coords()
    cx, cy = RIFT_C
    r = np.sqrt(((x - cx) / (W * 0.44)) ** 2 + ((y - cy) / (H * 0.34)) ** 2) + (vfx.fbm(x, y, 26.0, 1111, 3) - 0.5) * 0.16
    mask = 1 - smooth(0.93, 1.0, r)
    rng = np.random.default_rng(1112)
    pts = np.array([(rng.uniform(0.05, 0.95) * W, rng.uniform(0.15, 0.95) * H) for _ in range(13)], np.float32)
    d1, d2, _i = vfx.voronoi(x, y, pts)
    edge = d2 - d1
    h0, col0 = soil(x, y, 1113)
    col = paint(col0 * 0.3 + C("#17151c", x) * 0.5, lit(h0 * 0.4, 12.0), 0.3)
    crack = smooth(3.0, 0.2, edge)
    near = smooth(1.3, 0.2, np.hypot((x - cx) / (W * 0.3), (y - cy) / (H * 0.3)))
    col = col + np.array([110, 124, 170], np.float32) * (crack * near * 0.5)[..., None]
    fr = rift_frame(x, y, 0.0, extras=False)
    a = fr[..., 3:4] / 255.0
    col = col * (1 - a) + fr[..., :3] * a
    return finish(col, np.maximum(mask, a[..., 0]), x, y, 9, 14, 8.0, 0.45)


def rift_strip() -> None:
    x, y = fgrid()
    EXTRA["rift-portal"] = strip([rift_frame(x, y, k / SLOW) for k in range(SLOW)])


def rift() -> Image.Image:
    rift_strip()
    return rift_tile()


def maelstrom_tile() -> Image.Image:
    x, y = gt.coords()
    cx, cy, rx, ry = MAELSTROM_CFG["geo"]
    rng = np.random.default_rng(1211)
    stones = np.zeros_like(x)
    hh_ = np.zeros_like(x)
    for i in range(14):                                 # валуны по берегу
        a0 = i / 14 * 2 * np.pi + rng.uniform(-0.12, 0.12)
        sx_, sy_ = cx + np.cos(a0) * rx * 1.2, cy + np.sin(a0) * ry * 1.28
        size = rng.uniform(0.7, 1.25)
        m, hh = gt.dome(x, y, sx_, sy_, 30 * size, 22 * size, 1220 + i)
        stones = np.maximum(stones, m)
        hh_ = np.maximum(hh_, hh * m)
    pcol, body, _rim, _r, _t = vortex_pixels(x, y, 0.0, MAELSTROM_CFG)
    wet = paint(mix(C("#3c4548", x), C("#78858a", x), vfx.fbm(x, y, 9.0, 1213, 3)), lit(hh_, 24.0), 0.38)
    col = mix(wet, pcol, body)
    solid = np.maximum(body, stones)
    return finish(col, solid, x, y, 8, 12, 7.0, 0.42)


def maelstrom() -> Image.Image:
    vortex_strip(MAELSTROM_CFG, "maelstrom-water", (170, 235, 230), spiral_in=True)
    return maelstrom_tile()


GROUNDS = {"embers": fire_strip, "storm": storm, "grave": grave, "crystal": crystal,
           "glass": glass, "rift": rift, "maelstrom": maelstrom}


def main() -> None:
    gt.OUT.mkdir(parents=True, exist_ok=True)
    only = set(sys.argv[1:])
    for name, draw in GROUNDS.items():
        if only and name not in only:
            continue
        tile = draw()
        tile.save(gt.OUT / f"{name}.webp", "WEBP", quality=88, method=6)
        gt.medallion(tile, name).save(gt.OUT / f"{name}-mark.webp", "WEBP", quality=90, method=6)
        print(f"{name}: {tile.width}×{tile.height}")
        for key in [k for k in EXTRA if k.split("-")[0] == name]:
            EXTRA[key].save(gt.OUT / f"{key}.webp", "WEBP", quality=84, method=6)
            print(f"{key}: полоса {EXTRA[key].width}×{EXTRA[key].height}")


if __name__ == "__main__":
    main()
