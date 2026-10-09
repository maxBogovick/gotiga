#!/usr/bin/env python3
"""Плитки местности для поля боя: стена, овраг, яма, болото, укрытие, холм,
родник.

Рисуются не фигурами, а ПОЛЕМ — как огонь и яд в `magic_strips.py`: у каждой
земли есть карта высот, её освещает свет сверху слева (тот же, что падает в
комнату этюда), и окраска берётся по высоте и по шуму. Плоская заливка на
клетке читалась наклейкой; освещённая высота читается землёй.

На каждую землю два файла:
  <name>.webp       — плитка 3 : 4 во всю клетку, края растворены в прозрачность,
                      чтобы ложиться и на бумагу стола, и на тёмное сукно;
  <name>-mark.webp  — медальон: круглый срез той же плитки в золотой оправе.
                      Его ставят над картой, когда клетку занимает тело.

    tools/cutout/.venv/bin/python tools/ground_tiles.py

Подвижные слои (болото, родник, овраг, холм) лежат рядом
и запечены на полную силу — сколько их видно, решает CSS: <земля>-<слой>.webp. Рисует их
этот же файл, двигает CSS в `BattleGroundMark.svelte`.

Всё засеяно: те же аргументы дают те же пиксели при каждом запуске.
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter
from scipy.ndimage import gaussian_filter

sys.path.insert(0, str(Path(__file__).parent))
import vfx  # noqa: E402

W, H = 360, 480          # плитка, 3 : 4 — пропорция клетки
SS = 2                   # рисуется вдвое крупнее и ужимается: без лесенки
OUT = Path(__file__).resolve().parent.parent / "static" / "battles" / "ground"
LIGHT = np.array([-0.55, -0.65, 0.52])  # сверху слева
LIGHT = LIGHT / np.linalg.norm(LIGHT)


def coords(w: int = W, h: int = H) -> tuple[np.ndarray, np.ndarray]:
    ys, xs = np.mgrid[0:h * SS, 0:w * SS].astype(np.float32)
    return (xs + 0.5) / SS, (ys + 0.5) / SS


def lit(height: np.ndarray, relief: float) -> np.ndarray:
    """Освещённость по карте высот: нормаль из уклона, скалярно со светом."""
    gy, gx = np.gradient(height * relief)
    n = np.stack([-gx, -gy, np.ones_like(height)], axis=-1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    return np.clip((n * LIGHT).sum(-1), 0.0, 1.0)


def paint(albedo: np.ndarray, light: np.ndarray, ambient: float = 0.38) -> np.ndarray:
    shade = ambient + (1.0 - ambient) * light / LIGHT[2]
    return albedo * np.clip(shade, 0.0, 1.6)[..., None]


def mix(a: np.ndarray, b: np.ndarray, t: np.ndarray) -> np.ndarray:
    return a + (b - a) * t[..., None]


def rgb(hexstr: str) -> np.ndarray:
    h = hexstr.lstrip("#")
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], np.float32)


def soil(x: np.ndarray, y: np.ndarray, seed: int) -> tuple[np.ndarray, np.ndarray]:
    """Земля: высота и цвет утоптанного грунта с мелкими камешками."""
    big = vfx.fbm(x, y, 90.0, seed)
    fine = vfx.fbm(x, y, 9.0, seed + 5, 4)
    pebbles = vfx.smooth(0.72, 0.8, vfx.fbm(x, y, 6.0, seed + 9, 3))
    height = big * 0.6 + fine * 0.25 + pebbles * 0.35
    tone = vfx.fbm(x, y, 40.0, seed + 3)
    col = mix(np.broadcast_to(rgb("#4a3a2c"), x.shape + (3,)).copy(),
              np.broadcast_to(rgb("#7a6650"), x.shape + (3,)).copy(), tone)
    col = mix(col, np.broadcast_to(rgb("#8c7c66"), x.shape + (3,)).copy(), pebbles * 0.7)
    return height, col


def edge_fade(x: np.ndarray, y: np.ndarray, w: int, h: int, soft: float, seed: int) -> np.ndarray:
    """Край плитки растворяется неровно — клетка не вырезана ножницами."""
    d = np.minimum(np.minimum(x, w - x), np.minimum(y, h - y))
    wobble = (vfx.fbm(x, y, 18.0, seed) - 0.5) * soft * 0.9
    return vfx.smooth(0.0, soft, d + wobble)


def grade(col: np.ndarray, x: np.ndarray, y: np.ndarray) -> np.ndarray:
    """Под свет комнаты: приглушить, чуть увести в сепию и затемнить к краям
    клетки. Яркая плитка на тёмном сукне читалась наклейкой из другой игры."""
    grey = col.mean(-1, keepdims=True)
    col = grey + (col - grey) * 0.8
    col = col * np.array([1.02, 0.97, 0.9], np.float32)
    cx, cy = x / W - 0.5, y / H - 0.5
    vignette = 1 - 0.38 * np.clip((cx * cx + cy * cy) * 2.2, 0, 1)
    return col * 0.86 * vignette[..., None]


def to_image(col: np.ndarray, alpha: np.ndarray, x: np.ndarray | None = None,
             y: np.ndarray | None = None) -> Image.Image:
    if x is not None:
        col = grade(col, x, y)
    rgba = np.dstack([np.clip(col, 0, 255), np.clip(alpha, 0, 1) * 255]).astype(np.uint8)
    img = Image.fromarray(rgba, "RGBA")
    return img.resize((img.width // SS, img.height // SS), Image.LANCZOS)


def finish(col: np.ndarray, solid: np.ndarray, x: np.ndarray, y: np.ndarray,
           dx: float = 12.0, dy: float = 18.0, blur: float = 9.0, strength: float = 0.5) -> Image.Image:
    """Предмет на прозрачном фоне с мягкой падающей тенью. Прежняя плитка несла
    сплошную землю во всю клетку: на светлой бумаге стола это тёмный грязный
    прямоугольник, и предмет в нём тонул. Теперь рисунок — сам предмет, тень
    падает вниз и вправо (свет сверху слева), а клетка вокруг пуста."""
    solid = np.clip(solid, 0, 1).astype(np.float32)
    sx, sy = int(dx * SS), int(dy * SS)
    sh = np.zeros_like(solid)
    sh[sy:, sx:] = solid[:solid.shape[0] - sy, :solid.shape[1] - sx]
    shadow = gaussian_filter(sh, blur * SS) * strength * (1 - solid)
    alpha = solid + shadow
    colp = col * (solid / np.maximum(alpha, 1e-3))[..., None]
    return to_image(colp, alpha, x, y)


# Подвижные слои: рисуются вместе с землёй, а двигает их CSS (BattleGroundMark).
EXTRA: dict[str, Image.Image] = {}


def layer(hexstr: str, alpha: np.ndarray, x: np.ndarray) -> Image.Image:
    """Слой одного цвета с заданной альфой — без земли и без тени, ровно того,
    что должно двигаться."""
    return to_image(C(hexstr, x), alpha)


def blades(x: np.ndarray, y: np.ndarray, spots: list[tuple[float, float]], seed: int,
           reach: tuple[float, float] = (11.0, 20.0), n: int = 5) -> np.ndarray:
    """Пучки травы: у каждого веер коротких стеблей из одной точки."""
    rng = np.random.default_rng(seed)
    out = np.zeros_like(x)
    for bx, by in spots:
        for _ in range(n):
            lean = rng.uniform(-0.55, 0.55)
            ln = rng.uniform(*reach)
            dd, _t = vfx.segment_distance(x, y, bx, by, bx + lean * ln, by - ln)
            out = np.maximum(out, vfx.smooth(1.5, 0.4, dd))
    return out


def vine(x: np.ndarray, y: np.ndarray, bx: float, by: float, length: float,
         seed: int) -> tuple[np.ndarray, np.ndarray]:
    """Плеть плюща, свисающая вниз: извилистый стебель и листья по обе стороны."""
    rng = np.random.default_rng(seed)
    ph = rng.uniform(0, 6.28)
    steps = 14
    pts = [(bx + 7.0 * np.sin(i / steps * 5.0 + ph), by + i / steps * length) for i in range(steps + 1)]
    stem = np.zeros_like(x)
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        dd, _t = vfx.segment_distance(x, y, x0, y0, x1, y1)
        stem = np.maximum(stem, vfx.smooth(2.2, 0.9, dd))
    leaf = np.zeros_like(x)
    for i in range(2, steps + 1):
        px, py = pts[i]
        for side in (-1, 1):
            if rng.random() < 0.3:
                continue
            ang = np.deg2rad(side * rng.uniform(25, 60))
            sz = rng.uniform(0.8, 1.25)
            cx_, cy_ = px + side * 5.0 * sz, py + rng.uniform(-3, 3)
            dx, dy = x - cx_, y - cy_
            u = (dx * np.cos(ang) + dy * np.sin(ang)) / (7.0 * sz)
            v = (-dx * np.sin(ang) + dy * np.cos(ang)) / (3.8 * sz)
            leaf = np.maximum(leaf, vfx.smooth(1.0, 0.78, np.sqrt(u * u + v * v)))
    return stem, leaf


# ── Земли ─────────────────────────────────────────────────────────────────────


def C(hexstr: str, ref: np.ndarray) -> np.ndarray:
    """Цвет, растянутый до формы картинки."""
    return np.broadcast_to(rgb(hexstr), ref.shape + (3,)).copy()


def stone_palette(idx: np.ndarray, n: int, seed: int, a: str, b: str) -> np.ndarray:
    """Каждому камню — свой оттенок из пары: одинаковые камни читаются штампом."""
    t = np.random.default_rng(seed).uniform(0, 1, n)[idx]
    return mix(C(a, idx.astype(np.float32)), C(b, idx.astype(np.float32)), t.astype(np.float32))


def dome(x: np.ndarray, y: np.ndarray, cx: float, cy: float, rx: float, ry: float,
         seed: int) -> tuple[np.ndarray, np.ndarray]:
    """Один валун: маска с неровным краем и купол высоты (0..1)."""
    wob = (vfx.fbm(x, y, 14.0, seed, 3) - 0.5) * 0.28
    r = np.sqrt(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2) + wob
    return vfx.smooth(1.0, 0.94, r), np.clip(1 - r, 0, 1) ** 0.6


def wall() -> Image.Image:
    """Стена — отдельно стоящий кусок обветренной кладки: плоская крышка из плит,
    передняя грань из неровных блоков с раствором, выщербленный верх, осыпь у
    концов и тень на землю. Силуэт неровный — это руина, а не плита с краями."""
    x, y = coords()
    h0, col0 = soil(x, y, 11)
    ground = paint(col0, lit(h0 * 0.3, 14.0))
    capt = H * 0.085

    def parts(xx: np.ndarray, yy: np.ndarray):
        broken = vfx.smooth(0.55, 0.6, vfx.fbm(xx, xx * 0 + 7, 70.0, 13, 3))
        top = H * 0.2 + broken * H * 0.13 + (vfx.fbm(xx, xx * 0 + 1, 9.0, 14, 3) - 0.5) * 8
        wob = (vfx.fbm(yy, yy * 0 + 3, 25.0, 15, 3) - 0.5) * 18
        left, right = W * 0.07 + wob, W * 0.93 - wob * 0.8
        base = H * 0.84 + (vfx.fbm(xx, xx * 0 + 9, 20.0, 16, 3) - 0.5) * 9
        inx = vfx.smooth(left - 1, left + 1, xx) * vfx.smooth(right + 1, right - 1, xx)
        cap = inx * vfx.smooth(top - 1, top + 1, yy) * vfx.smooth(top + capt + 1, top + capt - 1, yy)
        face = inx * vfx.smooth(top + capt - 1, top + capt + 1, yy) * vfx.smooth(base + 1, base - 1, yy)
        return cap, face, top, base

    cap, face, top, base = parts(x, y)
    capb = top + capt

    # Грань: ряды блоков разной высоты, в каждом ряду свои ширины и сдвиг;
    # у каждого блока выпуклая лицевая сторона и свой оттенок, раствор тёмный.
    rng = np.random.default_rng(14)
    y0, y1 = H * 0.285, H * 0.84
    heights = rng.uniform(0.8, 1.25, 5)
    bounds = y0 + np.concatenate([[0], np.cumsum(heights)]) / heights.sum() * (y1 - y0)
    row = np.clip(np.digitize(y, bounds) - 1, 0, 4)
    row_top, row_bot = bounds[row], bounds[row + 1]
    bw = np.array([rng.uniform(78, 120) for _ in range(5)], np.float32)[row]
    off = np.array([rng.uniform(0, 120) for _ in range(5)], np.float32)[row]
    u = (x + off) / bw
    cell = np.floor(u)
    fu = (u - cell) * bw
    dx = np.minimum(fu, bw - fu)
    dy = np.minimum(y - row_top, row_bot - y)
    edge = np.minimum(dx, dy * 1.4)
    tone = vfx.noise(cell * 3.7 + row * 11.3, row * 1.0 + 0.5, 19)
    mortar = vfx.smooth(1.2, 5.0, edge + (vfx.fbm(x, y, 6.0, 17, 3) - 0.5) * 3)
    fh = np.clip(edge / 16.0, 0, 1) ** 0.6 + vfx.fbm(x, y, 5.0, 18, 4) * 0.16
    facec = mix(C("#5b5148", x), C("#938670", x), tone)
    lichen = vfx.smooth(0.62, 0.72, vfx.fbm(x, y, 24.0, 20))
    facec = mix(facec, C("#5d6a3a", x), lichen * 0.35 * vfx.smooth(top + 120, base, y))
    facec = paint(facec, lit(fh, 30.0), 0.34)
    facec = mix(C("#15100c", x), facec, mortar)
    facec = facec * (1 - 0.55 * vfx.smooth(capb + 26, capb, y))[..., None]       # тень под крышкой
    facec = facec * (1 - 0.5 * vfx.smooth(base - 40, base, y))[..., None]        # у земли темнее

    # Крышка: длинные плиты, светлее грани — на неё падает свет.
    cp = np.array([(c * 96 + rng.uniform(-14, 14), capt * 0.5) for c in range(-1, 6)], np.float32)
    c1, c2, cidx = vfx.voronoi(x, y - top, cp)
    cedge = c2 - c1
    slab = vfx.smooth(0.8, 3.5, cedge)
    capc = stone_palette(cidx, len(cp), 21, "#8f8473", "#b6aa94")
    caph = np.clip(cedge / 14.0, 0, 1) ** 0.6 + vfx.fbm(x, y, 5.0, 22, 4) * 0.2
    capc = paint(capc, lit(caph, 20.0), 0.5)
    capc = mix(C("#2a2219", x), capc, slab)
    capc = mix(capc, C("#e6dcc6", x), vfx.smooth(top + 7, top, y) * 0.55)       # светлая кромка
    capc = mix(capc, C("#1b1510", x), vfx.smooth(capb - 4, capb, y) * 0.7)      # губа крышки

    # Тень блока на земле: вниз и вправо.
    col = mix(mix(ground, facec, face), capc, cap)
    solid = np.clip(cap + face, 0, 1)

    # Осыпь: упавшие камни у основания и у концов.
    for i, (px, py, rad) in enumerate([(0.09, 0.90, 30), (0.2, 0.935, 18), (0.9, 0.9, 28),
                                       (0.79, 0.945, 16), (0.96, 0.95, 13), (0.5, 0.955, 12)]):
        m, hh = dome(x, y, W * px, H * py, rad, rad * 0.78, 30 + i)
        sh = vfx.smooth(1.1, 0.85, np.hypot((x - W * px - 7) / rad, (y - H * py - 9) / (rad * 0.78)))
        col = col * (1 - 0.45 * sh * (1 - m) * (1 - solid))[..., None]
        sc = paint(mix(C("#70675a", x), C("#a29683", x), vfx.fbm(x, y, 8.0, 40 + i, 3)),
                   lit(hh, rad * 1.3), 0.4)
        col = mix(col, sc, m)
        solid = np.maximum(solid, m)
    # Плющ свисает с крышки (две группы качаются порознь), трава пробивается у основания.
    ci = lambda fx_: int(fx_ * W * SS)  # noqa: E731
    vine_groups = [[(0.13, 90), (0.31, 120)], [(0.66, 105), (0.86, 80)]]
    for g, vs in enumerate(vine_groups):
        stem_all, leaf_all = np.zeros_like(x), np.zeros_like(x)
        for k, (fx_, ln) in enumerate(vs):
            by_ = float(top[0, ci(fx_)] + capt - 4)
            st, lf = vine(x, y, W * fx_, by_, ln, 400 + g * 10 + k)
            stem_all, leaf_all = np.maximum(stem_all, st), np.maximum(leaf_all, lf)
        tone = vfx.fbm(x, y, 9.0, 410 + g, 3)
        icol = mix(C("#33411f", x), mix(C("#4c6330", x), C("#8aa352", x), tone), leaf_all)
        EXTRA[f"wall-ivy-{g}"] = to_image(icol, np.maximum(stem_all, leaf_all) * 0.95, x, y)
    grass = blades(x, y, [(W * f, H * (0.87 + 0.02 * ((i * 7) % 3))) for i, f in
                          enumerate([0.05, 0.16, 0.33, 0.46, 0.6, 0.74, 0.9])], 420, (12, 22))
    gc = mix(C("#4a5a26", x), C("#9aa65a", x), vfx.fbm(x, y, 6.0, 421, 2))
    EXTRA["wall-grass"] = to_image(gc, grass * 0.92, x, y)
    return finish(col, solid, x, y)


def ravine() -> Image.Image:
    """Овраг — извилистая пропасть поперёк клетки. К левому и правому краю она
    выходит на середину и одной ширины, поэтому соседние клетки сходятся в одну
    длинную расщелину. Дальняя стена освещена и уходит во тьму, ближняя кромка
    нависает; по обоим краям крошится камень."""
    x, y = coords()
    h0, col0 = soil(x, y, 51)
    ground = paint(col0, lit(h0 * 0.3, 14.0))
    ends = vfx.smooth(0, 70, x) * vfx.smooth(0, 70, W - x)
    yc = H * 0.5 + ends * (vfx.fbm(x, x * 0 + 2, 100.0, 53, 3) - 0.5) * 170
    hw = 74 + ends * (vfx.fbm(x, x * 0 + 5, 70.0, 55, 3) - 0.5) * 36
    jag = ((vfx.fbm(x, y, 8.0, 57, 4) - 0.5) * 14 + (vfx.fbm(x, y, 24.0, 58, 3) - 0.5) * 12) * ends
    dist = np.abs(y - yc) - hw + jag           # < 0 внутри
    gorge = vfx.smooth(1.5, -1.5, dist)
    t = np.clip((y - (yc - hw)) / (2 * hw), 0, 1)   # 0 у дальней кромки, 1 у ближней

    # Дальняя стена: слои и вертикальные трещины, свет сверху слева.
    # Порода — неровные глыбы с чёрными швами, а не полосы: полосы читались доской.
    rng = np.random.default_rng(59)
    bp = np.array([(rng.uniform(-20, W + 20), rng.uniform(0, H)) for _ in range(46)], np.float32)
    b1, b2, bidx = vfx.voronoi(x, y * 1.5, bp * np.array([1, 1.5], np.float32))
    seam = vfx.smooth(0.8, 4.5, b2 - b1)
    chunk = np.random.default_rng(60).uniform(0, 1, len(bp))[bidx].astype(np.float32)
    strata = chunk * 0.6 + vfx.fbm(x, y, 9.0, 61, 4) * 0.4
    crack = 1 - seam
    rh = np.clip((b2 - b1) / 14.0, 0, 1) ** 0.6 * 0.8 + vfx.fbm(x, y, 6.0, 62, 4) * 0.25
    rockc = mix(C("#6a5744", x), C("#c4ac86", x), strata)
    rockc = paint(rockc, lit(rh, 22.0), 0.5)
    rockc = mix(rockc, C("#0d0907", x), crack * 0.85)
    rockc = rockc * (1 - 0.95 * vfx.smooth(0.08, 0.8, t))[..., None]
    void = mix(C("#120d0a", x), C("#030201", x), vfx.smooth(0.4, 1.0, t))
    mist = vfx.smooth(0.55, 0.72, vfx.fbm(x, y * 2.0, 30.0, 61)) * vfx.smooth(0.5, 0.85, t) * 0.2
    void = mix(void, C("#6b665e", x), mist)
    inside = mix(rockc, void, vfx.smooth(0.45, 0.82, t))
    # Ближняя кромка нависает над пустотой: под ней тьма.
    inside = inside * (1 - 0.65 * vfx.smooth(-26, 0, dist) * (t > 0.5))[..., None]
    col = mix(ground, inside, gorge)

    # Кромки снаружи: светлая губа и осыпь.
    outside = 1 - gorge
    lip = vfx.smooth(16, 0, dist) * outside
    col = mix(col, C("#c4b390", x), lip * 0.42)
    grit = vfx.smooth(0.64, 0.7, vfx.fbm(x, y, 4.0, 63, 3)) * vfx.smooth(20, 3, dist) * outside
    col = mix(col, C("#8f806b", x), grit * 0.7)
    # Туман на дне — два слоя разного рисунка, которые сменяют друг друга (CSS).
    # К боковым краям клетки он сходит на нет: сдвигаясь, слой иначе показал бы
    # обрез там, где пропасть продолжает соседняя клетка.
    # Он лежит только внутри пропасти и не доходит до кромки: у светлой губы
    # туман выглядел бы паром над землёй, а не мглой в глубине.
    deepzone = vfx.smooth(-4, -24, dist) * vfx.smooth(0.3, 0.7, t)
    for tag, seed in (("a", 301), ("b", 302)):
        fog = vfx.smooth(0.42, 0.72, vfx.warped(x * 0.5, y, 55.0, seed, 28.0))
        EXTRA[f"ravine-mist-{tag}"] = layer("#a8a398", fog * deepzone * vfx.smooth(0, 70, x) * vfx.smooth(0, 70, W - x), x)
    for fx_ in (0.25, 0.5, 0.75):
        i_ = int(fx_ * W * SS)
        print(f"ravine path x={fx_ * 100:.0f}%: центр {yc[0, i_] / H * 100:.0f}%, полуширина {hw[0, i_] / H * 100:.0f}%")
    # Обломки на краю: валуны с тенью, вразброс по обе стороны.
    solid = vfx.smooth(17, 11, dist)       # сама пропасть и полоса осыпающейся кромки
    rng = np.random.default_rng(64)
    for i in range(11):
        px = rng.uniform(12, W - 12)
        side = -1 if i % 2 == 0 else 1
        pyc = H * 0.5 + (np.interp(px, [0, 70, W - 70, W], [0, 1, 1, 0]) *
                         (vfx.fbm(np.array([px], np.float32), np.array([2.0], np.float32), 100.0, 53, 3)[0] - 0.5) * 170)
        pw = 74 + (np.interp(px, [0, 70, W - 70, W], [0, 1, 1, 0]) *
                   (vfx.fbm(np.array([px], np.float32), np.array([5.0], np.float32), 70.0, 55, 3)[0] - 0.5) * 36)
        py = pyc + side * (pw + rng.uniform(8, 18))
        rad = rng.uniform(9, 19)
        m, hh = dome(x, y, px, py, rad, rad * 0.8, 70 + i)
        sh = vfx.smooth(1.1, 0.85, np.hypot((x - px - 5) / rad, (y - py - 7) / (rad * 0.8)))
        col = col * (1 - 0.4 * sh * (1 - m) * outside)[..., None]
        sc = paint(mix(C("#7b705f", x), C("#b0a48d", x), vfx.fbm(x, y, 7.0, 80 + i, 3)), lit(hh, rad * 1.3), 0.4)
        col = mix(col, sc, m * outside)
        solid = np.maximum(solid, m)
    return finish(col, solid, x, y, 6, 10, 7.0, 0.4)


def pit() -> Image.Image:
    """Провал посреди клетки: обложенный камнями край, колья, темнота."""
    x, y = coords()
    h0, col0 = soil(x, y, 71)
    cx, cy = W * 0.5, H * 0.53
    rx, ry = W * 0.33, H * 0.2
    wob = (vfx.fbm(x, y, 22.0, 73) - 0.5) * 0.16
    r = np.sqrt(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2) + wob
    hole = 1 - vfx.smooth(0.92, 1.0, r)
    depth = 1 - vfx.smooth(0.0, 1.0, r)
    # Камни по краю — разной величины, вразброс, не бусы.
    rng = np.random.default_rng(75)
    stones = np.zeros_like(x)
    for i in range(11):
        a0 = (i / 11 + rng.uniform(-0.03, 0.03)) * 2 * np.pi
        size = rng.uniform(0.55, 1.25)
        sx = cx + np.cos(a0) * rx * rng.uniform(1.02, 1.2)
        sy = cy + np.sin(a0) * ry * rng.uniform(1.05, 1.25)
        tilt = rng.uniform(0, np.pi)
        dx, dy = x - sx, y - sy
        u = (dx * np.cos(tilt) + dy * np.sin(tilt)) / (rx * 0.24 * size)
        v = (-dx * np.sin(tilt) + dy * np.cos(tilt)) / (ry * 0.32 * size)
        lump = 1 - np.sqrt(u * u + v * v) - (vfx.fbm(x, y, 9.0, 77 + i, 3) - 0.5) * 0.18
        # Купол, а не ступенька: высота растёт к середине камня — свет даёт объём.
        stones = np.maximum(stones, np.clip(lump, 0, 1) ** 0.6 * rng.uniform(0.75, 1.0))
    stones = stones * (1 - hole)
    height = h0 * 0.3 + stones * 0.9 - hole * (0.4 + depth * 1.6)
    col = paint(col0, lit(height, 16.0))
    stonec = mix(np.broadcast_to(rgb("#6c6358"), x.shape + (3,)).copy(),
                 np.broadcast_to(rgb("#a39684"), x.shape + (3,)).copy(), vfx.fbm(x, y, 8.0, 79, 3))
    col = mix(col, paint(stonec, lit(height, 16.0)), vfx.smooth(0.0, 0.12, stones))
    # Внутренняя стенка: свет на дальней (нижней) стороне, тьма к дну.
    inner = np.clip((y - cy) / ry * 0.5 + 0.5, 0, 1)
    wallc = mix(np.broadcast_to(rgb("#070504"), x.shape + (3,)).copy(),
                np.broadcast_to(rgb("#3b2c20"), x.shape + (3,)).copy(), inner * vfx.smooth(0.25, 0.95, r))
    col = mix(col, wallc, hole)
    # Холодный пар поднимается из провала: два слоя разного рисунка, CSS их
    # поднимает и гасит. Слой лежит там, где темно, — на камнях он был бы дымом.
    core = vfx.smooth(1.0, 0.45, r)
    for tag, seed in (("a", 321), ("b", 322)):
        wisp = vfx.smooth(0.45, 0.72, vfx.warped(x * 0.7, y * 1.3, 40.0, seed, 22.0))
        EXTRA[f"pit-vapor-{tag}"] = layer("#8d949a", wisp * core, x)
    solid = np.maximum(hole, vfx.smooth(0.0, 0.12, stones))
    # Колья: три заострённых тёмных клина со дна.
    for i, (px, lean) in enumerate([(0.38, -0.12), (0.52, 0.05), (0.64, 0.14)]):
        bx = W * px
        top = cy - ry * (0.15 + 0.12 * i % 2)
        bot = cy + ry * 0.55
        tt = np.clip((y - top) / (bot - top), 0, 1)
        lx = bx + lean * (bot - y)
        halfw = 3.2 * tt + 0.3
        stake = vfx.smooth(halfw + 1.2, halfw, np.abs(x - lx)) * (y > top) * (y < bot) * hole
        shade = 0.55 + 0.45 * vfx.smooth(lx + halfw, lx - halfw, x)
        stakec = rgb("#7a5a3a") * shade[..., None]
        col = mix(col, stakec, stake)
        solid = np.maximum(solid, stake)
    return finish(col, solid, x, y, 8, 12, 7.0, 0.4)


def mire() -> Image.Image:
    """Стоячая вода: тёмные омуты, ряска, блики, осока по краю."""
    x, y = coords()
    rr = np.sqrt(((x - W * 0.5) / (W * 0.47)) ** 2 + ((y - H * 0.52) / (H * 0.38)) ** 2) \
        + (vfx.fbm(x, y, 40.0, 103, 3) - 0.5) * 0.3 + (vfx.fbm(x, y, 12.0, 104, 3) - 0.5) * 0.08
    murk = vfx.warped(x, y, 120.0, 91, 40.0)
    # Ряска — редкими островками у краёв, а не ковром: под ней должна читаться вода.
    border = 1 - vfx.smooth(0.18, 0.42, np.minimum(np.minimum(x / W, 1 - x / W), np.minimum(y / H, 1 - y / H)) * 2)
    # Островки — где низкочастотный шум высок, а внутри них ряска мелкими
    # листками, сквозь которые видна вода.
    region = vfx.smooth(0.6, 0.68, vfx.fbm(x, y, 45.0, 93, 4) + border * 0.15)
    leaves = vfx.smooth(0.58, 0.64, vfx.fbm(x, y, 3.5, 95, 3))
    weed = region * leaves
    height = weed * 0.5
    water = mix(np.broadcast_to(rgb("#141b16"), x.shape + (3,)).copy(),
                np.broadcast_to(rgb("#34402c"), x.shape + (3,)).copy(), murk)
    col = paint(water, lit(height, 10.0), 0.75)
    col = mix(col, np.broadcast_to(rgb("#6d7a3c"), x.shape + (3,)).copy(), weed * 0.9)
    # Отражение неба: широкий мягкий отсвет сверху и тонкие блики ряби.
    sky = vfx.smooth(0.0, 1.0, 1 - y / H) * (0.25 + 0.2 * vfx.fbm(x, y, 60.0, 96)) * (1 - weed)
    col = mix(col, np.broadcast_to(rgb("#8f9a86"), x.shape + (3,)).copy(), sky * 0.5)
    # Блики — два слоя с разным рисунком, которые сменяют друг друга (CSS):
    # рябь не стоит на месте, и ни один блик не вылезает за берег.
    inpond = vfx.smooth(1.0, 0.94, rr) * vfx.smooth(0.9, 0.68, rr)
    for tag, seed in (("a", 97), ("b", 98)):
        gl = vfx.smooth(0.72, 0.8, vfx.fbm(x * 0.3, y * 2.6, 12.0, seed, 3)) * (1 - weed) * inpond
        EXTRA[f"mire-glint-{tag}"] = layer("#d8ddc4", gl, x)
    # Осока: тонкие тёмные стебли пучками по краям.
    reeds = np.zeros_like(x)
    reeds_by_side = [np.zeros_like(x), np.zeros_like(x)]   # левая и правая осока качаются порознь
    rng = np.random.default_rng(99)
    for _ in range(26):
        side = rng.random()
        bx = (rng.random() * 0.22 if side < 0.5 else 0.78 + rng.random() * 0.22) * W
        by = (0.35 + rng.random() * 0.6) * H
        hgt = H * (0.12 + rng.random() * 0.16)
        lean = (rng.random() - 0.5) * 0.5
        tt = np.clip((by - y) / hgt, 0, 1)
        lx = bx + lean * (by - y)
        halfw = 1.6 * (1 - tt) + 0.4
        stem = vfx.smooth(halfw + 1.0, halfw, np.abs(x - lx)) * (y <= by) * (y >= by - hgt)
        reeds = np.maximum(reeds, stem)
        k = 0 if bx < W * 0.5 else 1
        reeds_by_side[k] = np.maximum(reeds_by_side[k], stem)
    reedc = mix(np.broadcast_to(rgb("#2b2e1a"), x.shape + (3,)).copy(),
                np.broadcast_to(rgb("#6d6a3a"), x.shape + (3,)).copy(), vfx.smooth(0.3, 0.9, (H - y) / H))
    # Осока — отдельные слои (качаются), а не часть плитки.
    EXTRA["mire-reeds-l"] = to_image(reedc, reeds_by_side[0])
    EXTRA["mire-reeds-r"] = to_image(reedc, reeds_by_side[1])
    # Пруд — пятно воды с илистым берегом, а не вся клетка зелёной.
    pond = vfx.smooth(1.0, 0.94, rr)
    col = mix(col, C("#3a3425", x), vfx.smooth(0.84, 0.98, rr) * 0.8)
    return finish(col, pond, x, y, 8, 12, 7.0, 0.4)


def cover() -> Image.Image:
    """Укрытие — низкий вал из навалённых валунов поперёк клетки: два ряда
    крупных округлых камней, у каждого тень на землю и свой оттенок, мох сверху.
    Прежняя ровная кладка плавала в пустоте и читалась полосой."""
    x, y = coords()
    h0, col0 = soil(x, y, 111)
    col = paint(col0, lit(h0 * 0.3, 14.0))
    solid = np.zeros_like(x)
    rng = np.random.default_rng(112)
    stones = [(W * 0.27, H * 0.6, 74, 52), (W * 0.74, H * 0.58, 66, 48),
              (W * 0.5, H * 0.46, 54, 40),
              (W * 0.2, H * 0.8, 62, 46), (W * 0.52, H * 0.78, 78, 54), (W * 0.86, H * 0.8, 56, 42)]
    stones.sort(key=lambda s_: s_[1])
    for i, (sx, sy, rx, ry) in enumerate(stones):          # от дальних к ближним
        sh = vfx.smooth(1.15, 0.8, np.hypot((x - sx - 12) / rx, (y - sy - 18) / ry))
        col = col * (1 - 0.55 * sh)[..., None]
        m, hh = dome(x, y, sx, sy, rx, ry, 120 + i)
        hh = hh + vfx.fbm(x, y, 16.0, 150 + i, 3) * 0.08
        # Трещины и сколы: тонкие тёмные линии по камню.
        crack = vfx.smooth(0.7, 0.76, vfx.fbm(x * 1.4, y * 0.8, 12.0, 155 + i, 3))
        base = mix(C("#675f53", x), C("#a89c88", x), vfx.fbm(x, y, 70.0, 160 + i, 2) * 0.5
                   + float(rng.uniform(0, 0.5)))
        moss = vfx.smooth(0.55, 0.68, vfx.fbm(x, y, 16.0, 170 + i, 3)) * vfx.smooth(sy + ry * 0.1, sy - ry * 0.7, y)
        base = mix(base, C("#5c6a38", x), moss * 0.5)
        sc = paint(base, lit(hh, rx * 1.15), 0.3)
        sc = mix(sc, C("#1c1611", x), crack * 0.6)
        sc = sc * (1 - 0.4 * vfx.smooth(0.55, 1.0, np.hypot((x - sx) / rx, (y - sy) / ry)))[..., None]
        sc = mix(sc, C("#d8cdb6", x), vfx.smooth(0.2, 0.0, np.hypot((x - sx + rx * 0.3) / rx, (y - sy + ry * 0.4) / ry)) * 0.16)
        col = mix(col, sc, m)
        solid = np.maximum(solid, m)
    # Трава в щелях между камнями — две группы по высоте, качаются порознь.
    gaps = [(0.07, 0.72), (0.37, 0.86), (0.69, 0.85), (0.95, 0.68), (0.5, 0.64),
            (0.14, 0.93), (0.62, 0.95), (0.9, 0.93), (0.38, 0.5), (0.78, 0.44)]
    gaps.sort(key=lambda g_: g_[1])
    gc = mix(C("#4a5a26", x), C("#9aa65a", x), vfx.fbm(x, y, 6.0, 431, 2))
    for g in range(2):
        part = gaps[g * 5:(g + 1) * 5]
        print(f"cover-grass-{g}: pivot y = {np.mean([p[1] for p in part]) * 100:.0f}%")
        a = blades(x, y, [(W * fx_, H * fy_) for fx_, fy_ in part], 432 + g, (14, 26))
        EXTRA[f"cover-grass-{g}"] = to_image(gc, a * 0.92, x, y)
    return finish(col, solid, x, y, 14, 20)


def hill() -> Image.Image:
    """Холм — травяной бугор на земле: у него есть край, склон со светом
    сверху слева, тень на землю справа внизу, редкие камни на гребне и пучки
    травы вокруг подошвы. Прежний зелёный квадрат во всю клетку был лугом."""
    x, y = coords()
    h0, col0 = soil(x, y, 121)
    ground = paint(col0, lit(h0 * 0.3, 14.0))
    cx, cy, rx, ry = W * 0.5, H * 0.56, W * 0.43, H * 0.29

    def mound_r(xx, yy):
        wob = (vfx.fbm(xx, yy, 40.0, 123, 3) - 0.5) * 0.3 + (vfx.fbm(xx, yy, 12.0, 124, 3) - 0.5) * 0.08
        return np.sqrt(((xx - cx) / rx) ** 2 + ((yy - cy) / ry) ** 2) + wob

    r = mound_r(x, y)
    mask = vfx.smooth(1.0, 0.95, r)
    h = np.clip(1 - r, 0, 1) ** 0.75
    blades = vfx.fbm(x * 1.6, y * 0.35, 2.6, 125, 3)
    height = h * 1.0 + blades * 0.05 + vfx.fbm(x, y, 30.0, 126, 3) * 0.12
    tone = vfx.fbm(x, y, 28.0, 127, 3)
    base = mix(C("#394322", x), C("#7e8a44", x), tone * 0.7 + blades * 0.3)
    base = mix(base, C("#8c7c4a", x), vfx.smooth(0.58, 0.7, vfx.fbm(x, y, 38.0, 128)) * 0.45)
    # Контурные линии склона — как на карте местности, тихо.
    contour = (0.5 + 0.5 * np.cos(h * 2 * np.pi * 3.2)) ** 14 * vfx.smooth(0.0, 0.1, h)
    col = paint(base, lit(height, 200.0), 0.2)
    col = mix(col, C("#222a12", x), contour * 0.35)
    # Подошва темнее, гребень светлее — склон круглится.
    col = col * (0.78 + 0.35 * h)[..., None]
    col = mix(col, C("#2a2f18", x), vfx.smooth(0.82, 1.0, r) * 0.5)
    col = mix(ground, col, mask)
    solid = mask.copy()

    # Камни на гребне.
    for i, (px, py, rad) in enumerate([(0.47, 0.44, 26), (0.58, 0.41, 17), (0.4, 0.5, 13)]):
        m, hh = dome(x, y, W * px, H * py, rad, rad * 0.75, 180 + i)
        sh = vfx.smooth(1.15, 0.85, np.hypot((x - W * px - 6) / rad, (y - H * py - 9) / (rad * 0.75)))
        col = col * (1 - 0.4 * sh * (1 - m))[..., None]
        sc = paint(mix(C("#756c5e", x), C("#aa9e8a", x), vfx.fbm(x, y, 8.0, 190 + i, 3)), lit(hh, rad * 1.3), 0.4)
        col = mix(col, sc, m)
        solid = np.maximum(solid, m)

    # Пучки травы вокруг подошвы — три группы по высоте на плитке, каждая
    # качается по-своему (CSS), а не часть плитки: стоячая трава читалась рисунком.
    rng = np.random.default_rng(130)
    spots = []
    for _ in range(22):
        a = rng.uniform(0, 2 * np.pi)
        k = rng.uniform(0.97, 1.12)
        blades = [(rng.uniform(-0.55, 0.55), rng.uniform(11, 20)) for _ in range(5)]
        spots.append((cy + np.sin(a) * ry * k, cx + np.cos(a) * rx * k, blades))
    spots.sort(key=lambda t_: t_[0])
    groups = [np.zeros_like(x) for _ in range(3)]
    for i, (by, bx, blades) in enumerate(spots):
        g = i * 3 // len(spots)
        for lean, ln in blades:
            dd, _t = vfx.segment_distance(x, y, bx, by, bx + lean * ln, by - ln)
            groups[g] = np.maximum(groups[g], vfx.smooth(1.5, 0.4, dd))
    tc = mix(C("#4a5a26", x), C("#9aa65a", x), vfx.fbm(x, y, 6.0, 131, 2))
    for g in range(3):
        ys_ = [sp[0] for sp in spots[g * len(spots) // 3:(g + 1) * len(spots) // 3]]
        print(f"hill-grass-{g}: pivot y = {np.mean(ys_) / H * 100:.0f}%")
        EXTRA[f"hill-grass-{g}"] = to_image(tc, groups[g] * 0.92, x, y)
    # Ветер по склону: светлые стебли проступают то в одном месте, то в другом.
    inner = mask * vfx.smooth(0.95, 0.7, r)
    for tag, seed in (("a", 311), ("b", 312)):
        streak = vfx.smooth(0.6, 0.7, vfx.fbm(x * 1.6, y * 0.35, 2.6, seed, 3)) \
            * vfx.smooth(0.45, 0.62, vfx.fbm(x, y, 45.0, seed + 5, 3))
        EXTRA[f"hill-wind-{tag}"] = layer("#c3cf7c", streak * inner, x)
    return finish(col, solid, x, y, 14, 20)


def spring() -> Image.Image:
    """Родник: каменная чаша, в ней тёмная прозрачная вода и круги от ключа."""
    x, y = coords()
    h0, col0 = soil(x, y, 131)
    cx, cy = W * 0.5, H * 0.52
    r = np.sqrt(((x - cx) / (W * 0.3)) ** 2 + ((y - cy) / (H * 0.2)) ** 2)
    water = 1 - vfx.smooth(0.88, 0.95, r)
    ring = vfx.smooth(0.85, 0.95, r) * vfx.smooth(1.35, 1.12, r)
    ang = np.arctan2(y - cy, x - cx)
    cut = vfx.smooth(0.15, 0.3, np.abs(np.sin(ang * 6 + vfx.fbm(x, y, 20.0, 133) * 2)))
    stones = ring * cut
    height = h0 * 0.3 + stones * 0.9 - water * 0.3
    col = paint(col0, lit(height, 14.0))
    stonec = mix(np.broadcast_to(rgb("#6f685d"), x.shape + (3,)).copy(),
                 np.broadcast_to(rgb("#b0a592"), x.shape + (3,)).copy(), vfx.fbm(x, y, 7.0, 135, 3))
    col = mix(col, paint(stonec, lit(height, 14.0)), np.clip(ring * 1.3, 0, 1) * (0.5 + 0.5 * cut))
    # Вода: глубже к середине, круги — тонкие светлые кольца.
    deep = 1 - vfx.smooth(0.0, 0.9, r)
    waterc = mix(np.broadcast_to(rgb("#2c4a4c"), x.shape + (3,)).copy(),
                 np.broadcast_to(rgb("#0c1a1c"), x.shape + (3,)).copy(), deep)
    wob = (vfx.fbm(x, y, 18.0, 139) - 0.5) * 0.08
    # Круги от ключа — отдельный слой (расходятся, CSS); на плитке остаётся тихая вода.
    ring_w = vfx.smooth(0.07, 0.0, np.abs(r + wob - 0.8)) * water
    EXTRA["spring-ring"] = layer("#bfdedb", ring_w, x)
    rng = np.random.default_rng(140)
    spark = np.zeros_like(x)
    for _ in range(7):
        a = rng.uniform(0, 2 * np.pi); d = rng.uniform(0.25, 0.7)
        px, py = cx + np.cos(a) * W * 0.3 * d, cy + np.sin(a) * H * 0.2 * d
        spark = np.maximum(spark, vfx.smooth(5.0, 0.8, np.hypot(x - px, (y - py) * 1.6)))
    centre = vfx.smooth(9.0, 1.5, np.hypot(x - cx, (y - cy) * 1.5))
    sky_glint = vfx.smooth(0.4, 0.0, np.hypot((x - cx + W * 0.08) / (W * 0.18), (y - cy + H * 0.06) / (H * 0.06)))
    EXTRA["spring-glint"] = layer("#e2f3f1", np.clip(sky_glint * 0.4 + spark * 0.7 + centre * 0.75, 0, 1) * water, x)
    ripples = 0.0
    sky = vfx.smooth(0.4, 0.0, np.hypot((x - cx + W * 0.08) / (W * 0.18), (y - cy + H * 0.06) / (H * 0.06)))
    waterc = mix(waterc, np.broadcast_to(rgb("#9cc2c0"), x.shape + (3,)).copy(), np.clip(sky * 0.18, 0, 1))
    col = mix(col, waterc, water)
    return finish(col, np.maximum(water, np.clip(ring * 1.6, 0, 1)), x, y, 8, 12, 7.0, 0.4)


GROUNDS = {
    "wall": wall,
    "ravine": ravine,
    "pit": pit,
    "mire": mire,
    "cover": cover,
    "hill": hill,
    "spring": spring,
}


def medallion(tile: Image.Image, ground: str, size: int = 112) -> Image.Image:
    """Круглый срез плитки в золотой оправе — знак над занятой клеткой."""
    # У укрытия суть в нижней трети, у прочих — в середине.
    box_h = tile.width
    top = int(tile.height * (0.5 if ground == "cover" else 0.5) - box_h / 2)
    if ground == "cover":
        top = int(tile.height * 0.78 - box_h / 2)
    crop = tile.crop((0, max(0, top), tile.width, max(0, top) + box_h))
    back = Image.new("RGBA", crop.size, (52, 40, 30, 255))
    back.alpha_composite(crop)
    s = size * SS
    face = back.resize((s, s), Image.LANCZOS)
    ys, xs = np.mgrid[0:s, 0:s].astype(np.float32)
    r = np.hypot(xs - s / 2 + 0.5, ys - s / 2 + 0.5) / (s / 2)
    disk = vfx.smooth(0.84, 0.8, r)
    rim = vfx.smooth(0.8, 0.84, r) * vfx.smooth(1.0, 0.95, r)
    arr = np.asarray(face).astype(np.float32)
    # Оправа: золото, светлее сверху слева.
    gold_light = 0.65 + 0.35 * np.clip(((s / 2 - xs) + (s / 2 - ys)) / s, -1, 1)
    gold = np.dstack([212 * gold_light, 176 * gold_light, 106 * gold_light])
    col = arr[..., :3] * disk[..., None] + gold * rim[..., None]
    inner_shadow = vfx.smooth(0.6, 0.8, r) * disk * 0.35
    col = col * (1 - inner_shadow[..., None])
    alpha = np.clip(disk + rim, 0, 1)
    out = Image.fromarray(np.dstack([np.clip(col, 0, 255), alpha * 255]).astype(np.uint8), "RGBA")
    return out.resize((size, size), Image.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, draw in GROUNDS.items():
        tile = draw()
        tile.save(OUT / f"{name}.webp", "WEBP", quality=88, method=6)
        medallion(tile, name).save(OUT / f"{name}-mark.webp", "WEBP", quality=90, method=6)
        print(f"{name}: {tile.width}×{tile.height}")
    for name, img in EXTRA.items():
        img.save(OUT / f"{name}.webp", "WEBP", quality=88, method=6)
        print(f"{name}: слой")


if __name__ == "__main__":
    main()
