#!/usr/bin/env python3
"""Картинки снаряжения штурма: крепостная стена, сторожевая башня, таран.

У этих карт нет работы автора, а значит, нет и фотографии. Нарисованы они
тем же способом, что плитки местности (`ground_tiles.py`): карта высот,
освещённая светом сверху слева, окраска по высоте и шуму. Фон — тёмная
комната со свечой, как на фотографиях работ: светлая картинка среди тёмных
фотографий читалась бы карточкой из другой игры.

    tools/cutout/.venv/bin/python tools/siege_art.py [wall|tower|ram …]

Пишет `src-tauri/server/assets/siege/<имя>.webp` (600 × 800, 3 : 4). Всё
засеяно: одни и те же аргументы дают те же пиксели.

Картинки лежат в крейте сервера, а не в `static/` сайта, потому что сервер
встраивает их в себя (`siege.rs`) и при запуске кладёт на свой склад: так
снаряжение штурма приезжает на рабочий сервер вместе с выкладкой, без
ручной загрузки. Стрела — рисунок сцены, она лежит у сайта
(`static/battles/motion/arrow.png`).
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).parent))
import vfx  # noqa: E402
from ground_tiles import lit, paint, mix, rgb  # noqa: E402

W, H = 600, 800
SS = 2
OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "server" / "assets" / "siege"


def coords() -> tuple[np.ndarray, np.ndarray]:
    ys, xs = np.mgrid[0:H * SS, 0:W * SS].astype(np.float32)
    return (xs + 0.5) / SS, (ys + 0.5) / SS


def C(hexstr: str, ref: np.ndarray) -> np.ndarray:
    return np.broadcast_to(rgb(hexstr), ref.shape + (3,)).copy()


def box(x, y, x0, x1, y0, y1, soft=1.0):
    return (vfx.smooth(x0 - soft, x0 + soft, x) * vfx.smooth(x1 + soft, x1 - soft, x)
            * vfx.smooth(y0 - soft, y0 + soft, y) * vfx.smooth(y1 + soft, y1 - soft, y))


def room(x: np.ndarray, y: np.ndarray, floor: float, seed: int) -> np.ndarray:
    """Тёмная комната: стена в тёплой тени, свеча слева сверху, пол."""
    back = mix(C("#120c08", x), C("#2c1f15", x), vfx.fbm(x, y, 160.0, seed, 4))
    glow = np.exp(-(((x - W * 0.12) / (W * 0.55)) ** 2 + ((y - H * 0.18) / (H * 0.45)) ** 2))
    back = mix(back, C("#6a4426", x), glow * 0.55)
    fl = vfx.fbm(x, y * 3.0, 40.0, seed + 1, 4)
    floor_col = mix(C("#1d150f", x), C("#47362a", x), fl)
    floor_col = floor_col * (0.55 + 0.45 * vfx.smooth(H, floor, y))[..., None]
    floor_col = mix(floor_col, C("#7a5634", x), (glow * 0.4)[..., None][..., 0])
    return mix(back, floor_col, vfx.smooth(floor - 2, floor + 2, y))


def masonry(x, y, x0, x1, y0, y1, rows: int, seed: int, a="#5b5148", b="#938670"):
    """Кладка: ряды блоков разной высоты и ширины, выпуклая лицевая сторона,
    тёмный раствор, свой оттенок у каждого камня. Возвращает цвет и высоту."""
    rng = np.random.default_rng(seed)
    hs = rng.uniform(0.8, 1.25, rows)
    bounds = y0 + np.concatenate([[0], np.cumsum(hs)]) / hs.sum() * (y1 - y0)
    row = np.clip(np.digitize(y, bounds) - 1, 0, rows - 1)
    rt, rb = bounds[row], bounds[row + 1]
    bw = np.array([rng.uniform(0.16, 0.24) * W for _ in range(rows)], np.float32)[row]
    off = np.array([rng.uniform(0, W * 0.2) for _ in range(rows)], np.float32)[row]
    u = (x - x0 + off) / bw
    cell = np.floor(u)
    fu = (u - cell) * bw
    edge = np.minimum(np.minimum(fu, bw - fu), np.minimum(y - rt, rb - y) * 1.4)
    tone = vfx.noise(cell * 3.7 + row * 11.3, row * 1.0 + 0.5, seed + 1)
    mortar = vfx.smooth(1.5, 6.0, edge + (vfx.fbm(x, y, 7.0, seed + 2, 3) - 0.5) * 4)
    height = np.clip(edge / 20.0, 0, 1) ** 0.6 + vfx.fbm(x, y, 6.0, seed + 3, 4) * 0.18
    col = mix(C(a, x), C(b, x), tone)
    lichen = vfx.smooth(0.63, 0.74, vfx.fbm(x, y, 30.0, seed + 4))
    col = mix(col, C("#56623a", x), lichen * 0.3)
    col = paint(col, lit(height, 34.0), 0.3)
    col = mix(C("#120d09", x), col, mortar)
    return col, height


def merlons(x, y, x0, x1, top, depth, n: int, gap_share=0.42):
    """Зубцы над кладкой: n зубцов на ширине, просветы между ними."""
    width = x1 - x0
    step = width / n
    u = (x - x0) / step
    frac = u - np.floor(u)
    tooth = vfx.smooth(gap_share * 0.5 - 0.01, gap_share * 0.5 + 0.01, frac) * \
        vfx.smooth(1 - gap_share * 0.5 + 0.01, 1 - gap_share * 0.5 - 0.01, frac)
    return box(x, y, x0, x1, top, top + depth) * tooth


def grade(col, x, y):
    """Под фотографии работ: приглушить, в сепию, затемнить к краям."""
    grey = col.mean(-1, keepdims=True)
    col = grey + (col - grey) * 0.82
    col = col * np.array([1.03, 0.97, 0.88], np.float32)
    cx, cy = x / W - 0.5, y / H - 0.45
    vignette = 1 - 0.55 * np.clip((cx * cx + cy * cy) * 2.0, 0, 1)
    return col * vignette[..., None]


def grain(col, x, y, seed):
    return col * (0.94 + 0.12 * vfx.noise(x * 1.7, y * 1.7, seed))[..., None]


def save(col, x, y, name: str, seed: int):
    col = grain(grade(col, x, y), x, y, seed)
    img = Image.fromarray(np.clip(col, 0, 255).astype(np.uint8), "RGB")
    img = img.resize((W, H), Image.LANCZOS)
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{name}.webp"
    img.save(path, "WEBP", quality=88, method=6)
    print(path)


def shadow_of(solid, dx, dy, blur):
    from scipy.ndimage import gaussian_filter
    sx, sy = int(dx * SS), int(dy * SS)
    sh = np.zeros_like(solid)
    sh[sy:, sx:] = solid[:solid.shape[0] - sy, :solid.shape[1] - sx]
    return gaussian_filter(sh, blur * SS)


# ── Строения на доске: куски одной стены ─────────────────────────────────────
#
# Стена и башня на поле — не карточки, а куски одной стены: клетки одного ряда
# стоят на доске столбиком, и куски обязаны сходиться без шва. Поэтому:
# полоса кладки идёт сверху вниз во всю высоту, рисунок повторяется ровно по
# высоте картинки (ряды плит — 8 на картинку, зубцы — 4, шум сшит по краю),
# по бокам прозрачно — видно землю клетки. Вид сверху, свет сверху слева, как
# у плиток местности. Зубцы — слева: на доске вдоль враг хранителя слева;
# страница поворачивает и отражает кусок по стороне и виду поля.

BAND = (0.30, 0.90)       # дорожка по стене, доли ширины
PARAPET = (0.06, 0.30)    # бруствер с зубцами со стороны врага
TEETH = 4                 # зубцов на картинку; 800 / 4 — шаг 200


def periodic(fn, x, y, *args):
    """Шум, сшитый по высоте: верх картинки продолжает её низ. Два куска
    одного шума смешиваются по высоте, и на краях берётся один и тот же."""
    t = y / H
    return fn(x, y, *args) * (1 - t) + fn(x, y - H, *args) * t


def slabs(x, y, x0, x1, seed, a="#8f8473", b="#b6aa94"):
    """Плиты дорожки: 8 рядов на картинку, по две плиты в ряду, ряды
    вразбежку. Ряд — ровно восьмая часть высоты, поэтому стык кусков
    приходится на шов между рядами."""
    rows = 8
    rh = H / rows
    row = np.floor(y / rh) % rows
    width = x1 - x0
    off = (row % 2) * 0.5
    u = (x - x0) / width * 2 + off
    cell = np.floor(u)
    fu = (u - cell) * width / 2
    fy = (y % rh)
    edge = np.minimum(np.minimum(fu, width / 2 - fu), np.minimum(fy, rh - fy) * 1.3)
    rng = np.random.default_rng(seed)
    tones = rng.uniform(0, 1, (rows, 4))
    tone = tones[row.astype(int), (cell.astype(int) % 4)]
    joint = vfx.smooth(1.4, 5.0, edge + (periodic(vfx.fbm, x, y, 7.0, seed + 1, 3) - 0.5) * 3)
    height = np.clip(edge / 18.0, 0, 1) ** 0.6 + periodic(vfx.fbm, x, y, 6.0, seed + 2, 4) * 0.18
    col = mix(C(a, x), C(b, x), tone.astype(np.float32))
    moss = vfx.smooth(0.64, 0.74, periodic(vfx.fbm, x, y, 26.0, seed + 3))
    col = mix(col, C("#56623a", x), moss * 0.28)
    col = paint(col, lit(height, 26.0), 0.42)
    return mix(C("#1a140f", x), col, joint)


def teeth_mask(x, y, x0, x1):
    """Зубцы бруствера: 4 на картинку, стык кусков — посреди просвета."""
    step = H / TEETH
    f = (y % step) / step
    return box(x, y, x0, x1, -10, H + 10) * vfx.smooth(0.2, 0.23, f) * vfx.smooth(0.8, 0.77, f)


def wall_piece(x, y, seed):
    """Цвет и непрозрачность куска стены во всю высоту картинки."""
    bx0, bx1 = BAND[0] * W, BAND[1] * W
    px0, px1 = PARAPET[0] * W, PARAPET[1] * W
    walk = box(x, y, bx0, bx1, -10, H + 10)
    parapet = box(x, y, px0 + 0.04 * W, px1, -10, H + 10)   # низкий бруствер в просветах
    tooth = teeth_mask(x, y, px0, px1)                      # высокие зубцы
    lip = box(x, y, bx1, bx1 + 0.03 * W, -10, H + 10)       # кромка со стороны города

    walk_col = slabs(x, y, bx0, bx1, seed)
    stone = mix(C("#5b5148", x), C("#938670", x), periodic(vfx.fbm, x, y, 30.0, seed + 5))
    rough = periodic(vfx.fbm, x, y, 5.0, seed + 6, 4)
    par_col = paint(stone * 0.92, lit(rough * 0.4, 20.0), 0.4)
    tooth_col = paint((stone * 0.65 + C("#b0a48e", x) * 0.35), lit(rough * 0.5 + tooth * 0.3, 24.0), 0.5)
    lip_col = stone * 0.62

    col = walk_col
    col = mix(col, lip_col, lip)
    col = mix(col, par_col, parapet)
    # Тень зубцов на дорожку: свет сверху слева, тень вправо и вниз (по кругу).
    sh = np.roll(np.roll(tooth, int(10 * SS), axis=0), int(18 * SS), axis=1)
    from scipy.ndimage import gaussian_filter
    sh = gaussian_filter(sh, (5 * SS, 5 * SS), mode=("wrap", "nearest"))
    col = col * (1 - 0.45 * sh * (1 - tooth))[..., None]
    col = mix(col, tooth_col, tooth)
    # Светлая кромка зубца со стороны света и тёмная — с другой.
    rim = tooth * vfx.smooth(px0 + 6, px0, x)
    col = mix(col, C("#d8c8aa", x), rim * 0.5)
    solid = np.clip(walk + parapet + tooth + lip, 0, 1)
    return col, solid


def ground_shadow(solid, dx):
    """Тень стены на землю справа: только по ширине, по высоте — по кругу."""
    from scipy.ndimage import gaussian_filter
    sh = np.roll(solid, int(dx * SS), axis=1)
    sh[:, : int(dx * SS)] = 0
    return gaussian_filter(sh, (2 * SS, 10 * SS), mode=("wrap", "nearest")) * (1 - solid)


def save_rgba(col, solid, x, y, name, seed):
    alpha = np.clip(solid + ground_shadow(solid, 16) * 0.55, 0, 1)
    shade = solid / np.maximum(alpha, 1e-3)
    col = grain(col, x, y, seed) * shade[..., None]
    rgba = np.dstack([np.clip(col, 0, 255), alpha * 255]).astype(np.uint8)
    img = Image.fromarray(rgba, "RGBA").resize((W, H), Image.LANCZOS)
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{name}.webp"
    img.save(path, "WEBP", quality=90, method=6)
    print(path)


def wall():
    """Кусок крепостной стены: дорожка из плит, бруствер с зубцами."""
    x, y = coords()
    col, solid = wall_piece(x, y, 11)
    save_rgba(col, solid, x, y, "fortress-wall", 5)


def tower():
    """Башня в линии стены: та же стена сверху и снизу и круглая площадка
    с зубцами по кругу посередине — входит в ряд стен без шва."""
    x, y = coords()
    col, solid = wall_piece(x, y, 11)
    cx, cy, r = W * 0.52, H * 0.5, W * 0.47
    d = np.hypot(x - cx, y - cy)
    ang = np.arctan2(y - cy, x - cx)
    body = vfx.smooth(r + 1, r - 1, d)
    # Площадка: кольца плит от центра.
    ring = np.floor(d / (r * 0.28))
    seg = np.floor((ang + np.pi) / (2 * np.pi) * (6 + ring * 4))
    tone = vfx.noise(seg * 3.1 + ring * 7.7, ring + 0.5, 23)
    floor = mix(C("#8f8473", x), C("#b6aa94", x), tone)
    floor = paint(floor, lit(vfx.fbm(x, y, 6.0, 24, 4) * 0.3, 18.0), 0.45)
    joints = vfx.smooth(1.0, 3.5, np.minimum(np.abs((d / (r * 0.28)) % 1 - 0.5) * 2 * r * 0.28 * 0.5,
                                              np.abs(((ang + np.pi) / (2 * np.pi) * (6 + ring * 4)) % 1 - 0.5) * 2 * d * 0.4))
    floor = mix(C("#1a140f", x), floor, joints)
    # Зубцы по кругу: кольцо по краю, прорезанное просветами.
    rim_band = vfx.smooth(r * 0.80, r * 0.83, d) * body
    n = 12
    f = ((ang + np.pi) / (2 * np.pi) * n) % 1
    crenel = vfx.smooth(0.30, 0.34, f) * vfx.smooth(0.70, 0.66, f)
    tooth = rim_band * crenel
    low = rim_band * (1 - crenel)
    stone = mix(C("#5b5148", x), C("#938670", x), vfx.fbm(x, y, 30.0, 25))
    tooth_col = paint((stone * 0.65 + C("#b0a48e", x) * 0.35), lit(vfx.fbm(x, y, 5.0, 26, 4) * 0.5 + tooth * 0.4, 24.0), 0.5)
    # Люк посередине: тёмное дерево.
    hatch = box(x, y, cx - r * 0.18, cx + r * 0.18, cy - r * 0.18, cy + r * 0.18, 1.5)
    wood_col = mix(C("#2e1d11", x), C("#5a3b22", x), vfx.fbm(x * 0.2, y * 4, 3.0, 27, 3))
    planks = vfx.smooth(0.5, 1.5, np.abs(((x - cx) / (r * 0.09)) % 1 - 0.5) * 6)
    wood_col = wood_col * (0.7 + 0.3 * planks)[..., None]

    tcol = floor
    tcol = mix(tcol, stone * 0.8, low)
    sh = np.roll(np.roll(tooth, int(8 * SS), axis=0), int(14 * SS), axis=1)
    from scipy.ndimage import gaussian_filter
    sh = gaussian_filter(sh, 4 * SS)
    tcol = tcol * (1 - 0.45 * sh * (1 - tooth))[..., None]
    tcol = mix(tcol, tooth_col, tooth)
    tcol = mix(tcol, wood_col, hatch)
    # Край башни темнее: она круглая, свет падает слева сверху.
    edge = vfx.smooth(r * 0.9, r, d) * np.clip(0.5 + 0.5 * np.cos(ang - np.pi * 0.25), 0, 1)
    tcol = tcol * (1 - 0.35 * edge)[..., None]
    # Тень башни на стену под ней (вправо-вниз).
    tsh = gaussian_filter(np.roll(np.roll(body, int(12 * SS), axis=0), int(16 * SS), axis=1), 8 * SS)
    col = col * (1 - 0.5 * tsh * (1 - body))[..., None]
    col = mix(col, tcol, body)
    solid = np.clip(solid + body, 0, 1)
    save_rgba(col, solid, x, y, "watchtower", 23)


def beam(x, y, x0, y0, x1, y1, r):
    """Брус от точки к точке: маска и поперечная координата (−1..1) для света."""
    d, _ = vfx.segment_distance(x, y, x0, y0, x1, y1)
    ux, uy = x1 - x0, y1 - y0
    n = np.hypot(ux, uy)
    across = ((x - x0) * (-uy) + (y - y0) * ux) / n / r
    along = ((x - x0) * ux + (y - y0) * uy) / n
    return vfx.smooth(r + 1, r - 1, d), np.clip(across, -1, 1), along


def wood(x, along, across, seed, a="#3b2616", b="#7a5232"):
    grain_ = vfx.fbm(along * 0.15, across * 40.0, 3.0, seed, 4)
    rings = 0.5 + 0.5 * np.sin(across * 9.0 + grain_ * 6.0)
    col = mix(C(a, x), C(b, x), np.clip(grain_ * 0.7 + rings * 0.3, 0, 1))
    shade = 0.35 + 0.85 * np.clip(0.6 - 0.55 * across, 0, 1) * np.sqrt(np.clip(1 - across * across, 0, 1))
    return col * shade[..., None]


def ram():
    """Таран: окованное бревно на цепях под двускатной рамой, на колёсах."""
    x, y = coords()
    floor = H * 0.86
    col = room(x, y, floor, 41)
    parts: list[tuple[np.ndarray, np.ndarray]] = []

    # Рама: две стойки-треугольника и конёк.
    for (ax, ay, bx, by) in [(W * 0.2, H * 0.74, W * 0.42, H * 0.3), (W * 0.64, H * 0.3, W * 0.86, H * 0.74),
                             (W * 0.42, H * 0.3, W * 0.64, H * 0.3), (W * 0.16, H * 0.74, W * 0.9, H * 0.74)]:
        m, ac, al = beam(x, y, ax, ay, bx, by, W * 0.022)
        parts.append((m, wood(x, al, ac, int(ax), "#2e1d11", "#6a4629")))
    # Цепи: тёмные звенья.
    for cx_ in (W * 0.45, W * 0.61):
        m, ac, al = beam(x, y, cx_, H * 0.31, cx_ - W * 0.02, H * 0.5, W * 0.006)
        links = 0.6 + 0.4 * np.sin(al * 0.5)
        parts.append((m * links, C("#2b2622", x) * (0.6 + 0.8 * np.clip(-ac, 0, 1))[..., None]))
    # Бревно: от хвоста справа к голове слева.
    m, ac, al = beam(x, y, W * 0.86, H * 0.55, W * 0.14, H * 0.5, W * 0.055)
    parts.append((m, wood(x, al, ac, 7, "#3a2414", "#8a5c36")))
    # Обручи и окованная голова.
    for k, at in enumerate((0.3, 0.55, 0.78)):
        hx = W * 0.86 + (W * 0.14 - W * 0.86) * at
        hy = H * 0.55 + (H * 0.5 - H * 0.55) * at
        hm, hac, _ = beam(x, y, hx, hy - 1, hx + 1, hy + 1, W * 0.06)
        hoop = hm * vfx.smooth(W * 0.012, W * 0.006, np.abs(x - hx))
        metal = C("#3c3833", x) * (0.4 + 1.1 * np.clip(0.6 - 0.6 * hac, 0, 1))[..., None]
        parts.append((hoop * m, metal))
    head = vfx.smooth(1.02, 0.96, np.hypot((x - W * 0.15) / (W * 0.075), (y - H * 0.5) / (W * 0.07)))
    hshade = np.clip(0.75 - ((x - W * 0.12) / (W * 0.08)) * 0.3 - ((y - H * 0.47) / (W * 0.07)) * 0.6, 0.1, 1.3)
    hcol = C("#4a443d", x) * hshade[..., None]
    hcol = mix(hcol, C("#c9bca6", x), np.exp(-(((x - W * 0.12) / 14) ** 2 + ((y - H * 0.475) / 10) ** 2)) * 0.8)
    parts.append((head, hcol))
    # Колёса.
    for wx in (W * 0.27, W * 0.76):
        r = np.hypot(x - wx, y - H * 0.78)
        rim = vfx.smooth(W * 0.095, W * 0.088, r) * vfx.smooth(W * 0.06, W * 0.067, r)
        ang = np.arctan2(y - H * 0.78, x - wx)
        # Три спицы через ступицу: расстояние до прямой, а не угол — иначе спица
        # сходит на нет к центру.
        spokes = np.zeros_like(x)
        for th in (0.3, 0.3 + np.pi / 3, 0.3 + 2 * np.pi / 3):
            d = np.abs((x - wx) * np.sin(th) - (y - H * 0.78) * np.cos(th))
            spokes = np.maximum(spokes, vfx.smooth(6.0, 4.0, d))
        spokes = spokes * vfx.smooth(W * 0.068, W * 0.062, r)
        hub = vfx.smooth(W * 0.022, W * 0.016, r)
        wheel = np.clip(rim + spokes + hub, 0, 1)
        wc = wood(x, ang * 200, (r - W * 0.08) / (W * 0.02), int(wx), "#2a1a0e", "#64432a")
        parts.append((wheel, wc))

    solid = np.zeros_like(x)
    for m, _ in parts:
        solid = np.maximum(solid, m)
    col = col * (1 - 0.55 * shadow_of(solid, 16, 14, 12))[..., None]
    for m, c in parts:
        col = mix(col, c, np.clip(m, 0, 1))
    # Тёплый отсвет свечи на левом краю предметов.
    glow = np.exp(-(((x - W * 0.12) / (W * 0.5)) ** 2 + ((y - H * 0.2) / (H * 0.5)) ** 2))
    col = col + (C("#b06a30", x) * 0.18) * (solid * glow)[..., None]
    save(col, x, y, "siege-ram", 43)


def arrow():
    """Стрела для удара издалека: полоса из шести кадров 256 × 256 остриём
    вправо, как прочие летящие рисунки (`static/battles/motion/`). Кадры
    отличаются только следом и дрожью оперения: стрела летит, а не машет."""
    from PIL import ImageDraw, ImageFilter
    F, S = 256, 4  # кадр и во сколько раз крупнее рисуется
    strip = Image.new("RGBA", (F * 6, F), (0, 0, 0, 0))
    rng = np.random.default_rng(7)
    for k in range(6):
        big = Image.new("RGBA", (F * S, F * S), (0, 0, 0, 0))
        d = ImageDraw.Draw(big)
        cy = F * S // 2
        x0, x1 = 40 * S, 214 * S
        # След: размытая светлая полоса за древком.
        trail = Image.new("RGBA", big.size, (0, 0, 0, 0))
        td = ImageDraw.Draw(trail)
        td.rectangle([4 * S, cy - 3 * S, x0 + 30 * S, cy + 3 * S],
                     fill=(230, 214, 186, int(60 + 25 * rng.random())))
        trail = trail.filter(ImageFilter.GaussianBlur(5 * S))
        big.alpha_composite(trail)
        # Древко: тёмное дерево с бликом сверху.
        d.rectangle([x0, cy - 3 * S, x1 - 22 * S, cy + 3 * S], fill=(92, 62, 36, 255))
        d.rectangle([x0, cy - 3 * S, x1 - 22 * S, cy - 1 * S], fill=(150, 112, 70, 255))
        # Наконечник: железо, светлая грань сверху.
        tip = [(x1 - 26 * S, cy - 9 * S), (x1, cy), (x1 - 26 * S, cy + 9 * S), (x1 - 20 * S, cy)]
        d.polygon(tip, fill=(58, 56, 54, 255))
        d.polygon([(x1 - 26 * S, cy - 9 * S), (x1, cy), (x1 - 20 * S, cy)], fill=(150, 146, 138, 255))
        # Оперение: два пера, дрожат от кадра к кадру.
        wob = int((rng.random() - 0.5) * 3 * S)
        for sgn in (-1, 1):
            d.polygon([(x0 - 2 * S, cy), (x0 + 30 * S, cy),
                       (x0 + 18 * S, cy + sgn * (13 * S + wob)), (x0 - 10 * S, cy + sgn * (12 * S + wob))],
                      fill=(196, 182, 160, 235) if sgn < 0 else (150, 136, 118, 235))
        d.line([(x0 - 6 * S, cy), (x0 + 30 * S, cy)], fill=(70, 50, 30, 255), width=2 * S)
        frame = big.resize((F, F), Image.LANCZOS)
        strip.alpha_composite(frame, (k * F, 0))
    out = Path(__file__).resolve().parent.parent / "static" / "battles" / "motion" / "arrow.png"
    strip.save(out)
    print(out)


ALL = {"wall": wall, "tower": tower, "ram": ram, "arrow": arrow}

if __name__ == "__main__":
    for name in sys.argv[1:] or list(ALL):
        ALL[name]()
