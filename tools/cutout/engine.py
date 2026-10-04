"""Ядро вырезания: модель, маска, очистка, проверки, сборка итогового файла.

Здесь нет ни командной строки, ни сервера — только то, что делается с одной
картинкой. Командная строка (`cutout.py`) и страница проверки (`review.py`)
пользуются одними и теми же функциями, поэтому файл, исправленный щелчком,
собирается ровно так же, как собранный в пакете.
"""

from __future__ import annotations

import io
import os
from collections import OrderedDict
from dataclasses import dataclass, asdict

import numpy as np
from PIL import Image, ImageCms, ImageFilter, ImageOps

try:  # фото с iPhone (HEIC). Без пакета остальные форматы работают как прежде.
    import pillow_heif

    pillow_heif.register_heif_opener()
    HEIF = True
except Exception:  # pragma: no cover
    HEIF = False

Image.MAX_IMAGE_PIXELS = 200_000_000  # снимок со среднеформатной камеры — не бомба

_SRGB = ImageCms.createProfile("sRGB")
_SRGB_BYTES = ImageCms.ImageCmsProfile(_SRGB).tobytes()

INPUT_EXTS = {".jpg", ".jpeg", ".png", ".webp", ".tif", ".tiff", ".bmp", ".avif"}
if HEIF:
    INPUT_EXTS |= {".heic", ".heif"}

# Модели, о которых известно, как они режут фигурки (замер на 48 фото архива).
# Основная держит отдельные пряди и тонкое (черенок метлы). Облегчённая чаще
# сама снимает руку (рука осталась на 3–4 фото из 48 против 5–6), но теряет
# тонкие предметы и часть волосков — а метла есть у половины ведьм. ISNet —
# черновик: мутный ореол и рука чаще остаётся.
MODELS = {
    "birefnet-general": "лучшее качество: пряди, тонкие предметы; ≈10 с на фото",
    "birefnet-general-lite": "≈7 с на фото; чаще снимает руку, но теряет тонкое (черенок метлы) и часть прядей",
    "isnet-general-use": "черновик, ≈1,5 с на фото; края мутнее, рука чаще остаётся",
}
# Секунд на фото целиком (модель, очистка края, запись PNG), Apple M5, процессор.
SECONDS = {"birefnet-general": 10.0, "birefnet-general-lite": 7.2, "isnet-general-use": 1.5}
DEFAULT_MODEL = "birefnet-general"
FAST_MODEL = "isnet-general-use"


# ── настройки сборки ─────────────────────────────────────────────────────

@dataclass
class Render:
    """Как из маски собирается итоговый файл. Модель сюда не входит: сменив
    только эти поля, файл пересобирают из сохранённой маски, не запуская
    нейросеть заново."""

    format: str = "png"          # png | webp | jpg
    bg: str | None = None        # None — прозрачный фон, иначе цвет CSS (#f8f1e7)
    crop: bool = False           # обрезать по фигурке
    pad: float = 5.0             # поле вокруг фигурки при обрезке, % её размера
    square: bool = False         # квадратный холст (с обрезкой)
    max_size: int = 0            # длинная сторона итога, 0 — как у исходника
    quality: int = 92            # webp / jpg
    clean_edges: bool = True     # снять цвет старого фона с полупрозрачных краёв
    specks: bool = True          # убрать мелкие отдельные крошки

    def ext(self) -> str:
        return {"jpg": ".jpg", "webp": ".webp"}.get(self.format, ".png")

    def as_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: dict) -> "Render":
        known = {k: d[k] for k in cls.__dataclass_fields__ if k in d}
        return cls(**known)


# ── чтение ───────────────────────────────────────────────────────────────

def load_image(path: str) -> tuple[Image.Image, bytes | None]:
    """RGB в sRGB с учётом поворота из EXIF; второе значение — профиль sRGB
    для записи в итог (или None, если исходник профиля не нёс).

    Снимки с телефона идут в Display P3. Оставить P3 в итоге нельзя: фон
    `--bg "#f8f1e7"` задан в sRGB, и на файле с пометкой P3 пергамент вышел бы
    другого оттенка, чем на сайте. Поэтому цвета переводятся в sRGB по профилю
    исходника — тем же путём, каким браузер показал бы его, — а не
    отбрасываются (без перевода красный камзол стал бы бурым)."""
    im = Image.open(path)
    im = ImageOps.exif_transpose(im)
    icc = im.info.get("icc_profile")
    if im.mode in ("RGBA", "LA", "PA") or (im.mode == "P" and "transparency" in im.info):
        im = im.convert("RGBA")
        base = Image.new("RGBA", im.size, (255, 255, 255, 255))
        base.alpha_composite(im)
        im = base.convert("RGB")
    elif im.mode not in ("RGB", "CMYK"):
        im = im.convert("RGB")

    if icc:
        try:
            src = ImageCms.ImageCmsProfile(io.BytesIO(icc))
            name = (ImageCms.getProfileDescription(src) or "").lower()
            if im.mode == "CMYK" or "srgb" not in name.replace(" ", ""):
                im = ImageCms.profileToProfile(im, src, _SRGB, renderingIntent=0, outputMode="RGB")
            return im, _SRGB_BYTES
        except Exception:
            pass  # битый профиль: читаем пиксели как есть
    if im.mode != "RGB":
        im = im.convert("RGB")
    return im, None


# ── маска ────────────────────────────────────────────────────────────────

class Cutter:
    """Обёртка над моделью rembg. Модель грузится при первой картинке, а не
    при создании: `--dry-run` и пустая папка не должны ждать полминуты."""

    def __init__(self, model: str = DEFAULT_MODEL):
        self.model = model
        self._session = None

    def _load(self):
        if self._session is None:
            from rembg import new_session

            self._session = new_session(self.model, providers=["CPUExecutionProvider"])
        return self._session

    def mask(self, img: Image.Image) -> np.ndarray:
        """Альфа 0..1 размером с картинку."""
        m = self._load().predict(img)[0]
        if m.size != img.size:
            m = m.resize(img.size, Image.Resampling.LANCZOS)
        return np.asarray(m, dtype=np.float32) / 255.0


def drop_specks(alpha: np.ndarray, rel: float = 0.01) -> np.ndarray:
    """Убирает отдельные кусочки меньше `rel` от самого крупного.

    Порог относительный, а не в пикселях: две курицы рядом с фигуркой — это
    10% её площади и остаются, а пылинка фона или обводка снятой руки — сотые
    доли и уходят. Полупрозрачная дымка, не дотягивающая до 0.5, в счёт
    кусков не идёт и гасится вместе с тем куском, к которому прилегает."""
    from scipy import ndimage

    solid = alpha > 0.5
    labels, n = ndimage.label(solid)
    if n <= 1:
        return alpha
    sizes = ndimage.sum(solid, labels, index=np.arange(1, n + 1))
    keep = np.zeros(n + 1, dtype=bool)
    keep[1:] = sizes >= sizes.max() * rel
    if keep[1:].all():
        return alpha
    # всё, что ближе к оставленному куску, чем к выброшенному, — его дымка
    kept_mask = keep[labels]
    dist_keep = ndimage.distance_transform_edt(~kept_mask)
    dropped = solid & ~kept_mask
    dist_drop = ndimage.distance_transform_edt(~dropped)
    out = alpha.copy()
    out[dist_drop < dist_keep] = 0.0
    return out


def foreground(img: Image.Image, alpha: np.ndarray) -> np.ndarray:
    """Цвета без примеси фона (uint8 RGB).

    В полупрозрачной пряди пиксель — смесь волоса и серой стены. Если его так
    и положить на пергамент, вокруг головы встанет серая кайма. Оценка
    переднего плана (pymatting) пересчитывает цвет, каким он был бы без стены."""
    from pymatting import estimate_foreground_ml

    rgb = np.asarray(img, dtype=np.float64) / 255.0
    f = estimate_foreground_ml(rgb, alpha.astype(np.float64))
    return np.clip(f * 255.0 + 0.5, 0, 255).astype(np.uint8)


# ── проверки ─────────────────────────────────────────────────────────────

FLAGS = {
    "edge": "фигура касается края кадра — возможно, осталась рука",
    "parts": "несколько отдельных частей — проверьте, всё ли это фигурка",
    "small": "вырезано очень мало — возможно, фигурка не найдена",
    "large": "вырезано почти всё — возможно, остался фон",
    "collage": "похоже на коллаж из нескольких кадров",
}


def analyze(img: Image.Image, alpha: np.ndarray) -> tuple[list[str], dict]:
    """Признаки, по которым результат стоит посмотреть глазами.

    Ни один из них не означает «плохо»: на крупном плане фигурка и должна
    касаться края. Это порядок просмотра, а не приговор."""
    from scipy import ndimage

    h, w = alpha.shape
    solid = alpha > 0.5
    coverage = float(solid.mean())
    flags: list[str] = []

    # Рука почти всегда входит в кадр сбоку или сверху; низ кадра — пол, и
    # фигурка в полный рост стоит на нём, касаясь края лишь на крупных планах.
    band = max(2, int(min(h, w) * 0.004))
    sides = {
        "left": solid[:, :band].any(axis=1).mean(),
        "right": solid[:, -band:].any(axis=1).mean(),
        "top": solid[:band, :].any(axis=0).mean(),
    }
    if max(sides.values()) > 0.04:
        flags.append("edge")

    labels, n = ndimage.label(solid)
    parts = 0
    if n:
        sizes = ndimage.sum(solid, labels, index=np.arange(1, n + 1))
        parts = int((sizes >= sizes.max() * 0.03).sum())
        if parts > 1:
            flags.append("parts")

    if coverage < 0.03:
        flags.append("small")
    elif coverage > 0.75:
        flags.append("large")

    if _looks_like_collage(img):
        flags.append("collage")

    stats = {
        "coverage": round(coverage, 4),
        "parts": parts,
        "edge": {k: round(float(v), 3) for k, v in sides.items()},
    }
    return flags, stats


def _looks_like_collage(img: Image.Image) -> bool:
    """Коллаж узнаётся по ровной полосе-разделителю через весь кадр.

    На живом фото строка или столбец пикселей не бывают одноцветными от края
    до края: их пересекает фигурка, складка ткани, свет. Полоса между кадрами
    коллажа — бывает."""
    g = np.asarray(img.convert("L").resize((300, max(1, round(300 * img.height / img.width)))), dtype=np.float32)
    h, w = g.shape

    def flat(lines: np.ndarray) -> bool:
        n = lines.shape[0]
        inner = lines[int(n * 0.08): int(n * 0.92)]
        return bool((inner.std(axis=1) < 2.5).any()) if inner.size else False

    return flat(g) or flat(g.T)


# ── сборка ───────────────────────────────────────────────────────────────

def parse_color(value: str) -> tuple[int, int, int]:
    from PIL import ImageColor

    return ImageColor.getrgb(value)[:3]


def compose(img: Image.Image, alpha: np.ndarray, r: Render, cleaned: bool = False) -> Image.Image:
    """Итоговая картинка в памяти (RGBA или RGB при залитом фоне).

    `cleaned` — крошки уже сняты (`final_alpha`), второй раз не считать."""
    a = alpha if cleaned or not r.specks else drop_specks(alpha)
    rgb = foreground(img, a) if r.clean_edges else np.asarray(img, dtype=np.uint8)
    a8 = np.clip(a * 255.0 + 0.5, 0, 255).astype(np.uint8)
    out = Image.fromarray(np.dstack([rgb, a8]), "RGBA")

    if r.crop:
        out = _crop(out, a8, r.pad, r.square)

    if r.max_size and max(out.size) > r.max_size:
        k = r.max_size / max(out.size)
        out = out.resize((max(1, round(out.width * k)), max(1, round(out.height * k))), Image.Resampling.LANCZOS)

    bg = r.bg if r.bg not in (None, "", "transparent", "none") else None
    if bg is None and r.format == "jpg":
        bg = "#ffffff"  # у JPEG нет прозрачности; белое лучше чёрного
    if bg is not None:
        base = Image.new("RGBA", out.size, parse_color(bg) + (255,))
        base.alpha_composite(out)
        out = base.convert("RGB")
    return out


def final_alpha(alpha: np.ndarray, r: Render) -> np.ndarray:
    """Маска в том виде, в каком она ляжет в файл: по ней же считаются пометки."""
    return drop_specks(alpha) if r.specks else alpha


def _crop(out: Image.Image, a8: np.ndarray, pad: float, square: bool) -> Image.Image:
    ys, xs = np.nonzero(a8 > 10)  # совсем слабую дымку в рамку не считаем
    if not len(xs):
        return out
    x0, x1, y0, y1 = xs.min(), xs.max() + 1, ys.min(), ys.max() + 1
    p = round(max(x1 - x0, y1 - y0) * pad / 100.0)
    x0, y0, x1, y1 = x0 - p, y0 - p, x1 + p, y1 + p
    if square:
        side = max(x1 - x0, y1 - y0)
        cx, cy = (x0 + x1) // 2, (y0 + y1) // 2
        x0, y0 = cx - side // 2, cy - side // 2
        x1, y1 = x0 + side, y0 + side
    # Поле может выйти за кадр: холст достраивается прозрачным, а не
    # прижимается к краю — иначе у фигурки у края кадра поле было бы кривым.
    canvas = Image.new("RGBA", (int(x1 - x0), int(y1 - y0)), (0, 0, 0, 0))
    canvas.paste(out.crop((max(0, x0), max(0, y0), min(out.width, x1), min(out.height, y1))),
                 (int(max(0, -x0)), int(max(0, -y0))))
    return canvas


def save(out: Image.Image, path: str, r: Render, icc: bytes | None) -> None:
    """Пишет атомарно: прерванная запись не оставляет полфайла под верным
    именем, которое повторный запуск принял бы за готовое."""
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    kw: dict = {}
    if icc:
        kw["icc_profile"] = icc
    tmp = path + ".part"
    if r.format == "jpg":
        out.convert("RGB").save(tmp, "JPEG", quality=r.quality, optimize=True, progressive=True, **kw)
    elif r.format == "webp":
        out.save(tmp, "WEBP", quality=r.quality, method=4, **kw)
    else:
        out.save(tmp, "PNG", optimize=False, compress_level=6, **kw)
    os.replace(tmp, path)


def save_mask(alpha: np.ndarray, path: str) -> None:
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".part"
    Image.fromarray(np.clip(alpha * 255.0 + 0.5, 0, 255).astype(np.uint8), "L").save(tmp, "PNG")
    os.replace(tmp, path)


def load_mask(path: str) -> np.ndarray:
    return np.asarray(Image.open(path).convert("L"), dtype=np.float32) / 255.0


# ── щелчок: SAM ──────────────────────────────────────────────────────────

class Clicker:
    """Выделение предмета по щелчку (Segment Anything).

    Самая долгая часть SAM — разбор картинки (≈2 с), а ответ на щелчок —
    сотые доли секунды. rembg делает оба шага на каждый вызов, поэтому
    разбор здесь сделан отдельно и запоминается: второй и третий щелчок по
    той же картинке приходят сразу."""

    INPUT = (684, 1024)  # высота, ширина входа кодировщика из сборки rembg

    def __init__(self):
        self._sess = None
        self._cache: OrderedDict[str, tuple] = OrderedDict()

    def _load(self):
        if self._sess is None:
            from rembg import new_session

            self._sess = new_session("sam", providers=["CPUExecutionProvider"])
        return self._sess

    def prepare(self, key: str, img: Image.Image) -> None:
        if key in self._cache:
            self._cache.move_to_end(key)
            return
        sess = self._load()
        ih, iw = self.INPUT
        scale = min(iw / img.width, ih / img.height)
        nw, nh = max(1, round(img.width * scale)), max(1, round(img.height * scale))
        canvas = np.zeros((ih, iw, 3), dtype=np.float32)
        canvas[:nh, :nw] = np.asarray(img.convert("RGB").resize((nw, nh), Image.Resampling.BILINEAR), dtype=np.float32)
        name = sess.encoder.get_inputs()[0].name
        emb = sess.encoder.run(None, {name: canvas})[0]
        self._cache[key] = (emb, scale, img.size, (nw, nh))
        while len(self._cache) > 4:
            self._cache.popitem(last=False)

    def select(self, key: str, img: Image.Image, x: float, y: float) -> np.ndarray:
        """Маска предмета под точкой (x, y в пикселях исходника), 0..1."""
        self.prepare(key, img)
        emb, scale, (w, h), (nw, nh) = self._cache[key]
        sess = self._load()
        coords = np.array([[[x * scale, y * scale], [0.0, 0.0]]], dtype=np.float32)
        labels = np.array([[1, -1]], dtype=np.float32)
        masks, scores, _ = sess.decoder.run(None, {
            "image_embeddings": emb,
            "point_coords": coords,
            "point_labels": labels,
            "mask_input": np.zeros((1, 1, 256, 256), dtype=np.float32),
            "has_mask_input": np.zeros(1, dtype=np.float32),
            "orig_im_size": np.array(self.INPUT, dtype=np.float32),
        })
        best = int(np.argmax(scores[0])) if scores.shape[-1] > 1 else 0
        logits = masks[0, best, :nh, :nw].astype(np.float32)
        big = Image.fromarray(logits, "F").resize((w, h), Image.Resampling.BILINEAR)
        lg = np.asarray(big, dtype=np.float32)
        # мягкий край в пару пикселей вместо ступеньки
        return 1.0 / (1.0 + np.exp(-np.clip(lg * 2.0, -30, 30)))


def grow(mask: np.ndarray, px: int) -> np.ndarray:
    """Расширяет маску на px пикселей с мягким краем — чтобы от снятой руки не
    оставалось обводки в один пиксель."""
    m = Image.fromarray((mask > 0.5).astype(np.uint8) * 255, "L")
    if px > 0:
        m = m.filter(ImageFilter.MaxFilter(px * 2 + 1))
    m = m.filter(ImageFilter.GaussianBlur(max(1, px // 2)))
    return np.asarray(m, dtype=np.float32) / 255.0


def thumb_bytes(img: Image.Image, width: int, fmt: str = "WEBP", quality: int = 85) -> bytes:
    im = img
    if width and im.width > width:
        im = im.resize((width, max(1, round(im.height * width / im.width))), Image.Resampling.LANCZOS)
    buf = io.BytesIO()
    if fmt == "JPEG":
        im.convert("RGB").save(buf, "JPEG", quality=quality)
    else:
        im.save(buf, "WEBP", quality=quality, method=4)
    return buf.getvalue()
