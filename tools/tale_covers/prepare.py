#!/usr/bin/env python3
"""Готовые обложки книг: обрезка до 2:3 и перевод в WebP.

    tools/cutout/.venv/bin/python tools/tale_covers/prepare.py ПАПКА_С_PNG ВЫХОД

Полка и разворот показывают обложку в рамке 2:3 (`object-fit: cover`), поэтому
обрезку выбирает человек, а не браузер: по центру он режет лица. Для каждой
картинки здесь названа байка (слаг на проде), середина кадра по горизонтали
и по вертикали (доли исходной картинки) и сколько точек срезать по краям:
у части картинок по краю идёт светлая линия от экспорта.

Выход: `book-<слаг>.webp` шириной 640 (выше исходной картинки не раздувается
сверх нужного: 640 × 960 — размер, при котором обложка в 260 px на экране с
двойной плотностью не мутнеет).
"""
import sys
from pathlib import Path

from PIL import Image

W, H = 640, 960

# файл (без «книга-» и «.png»): (слаг байки, середина по x, середина по y, срез по краям)
COVERS = {
    "блогерша": ("molodost-bystree-tvoih", 0.47, 0.5, 4),
    "гном-хипстер": ("ubit-a-potom-razorvat", 0.52, 0.5, 4),
    "как у всех": ("vse-kak-u-nas", 0.5, 0.5, 4),
    "непомнящий": ("the-forester-and-his-unforgettable-memory", 0.47, 0.5, 4),
    "пирожки": ("gb", 0.5, 0.5, 4),
    "призрак": ("mir-ne-takoy-kak-tebe", 0.5, 0.36, 4),
    "программист": ("otmeryalschik-ot-boga", 0.45, 0.5, 6),
    "салазар": ("ispolnyu-vse-zhelaniya", 0.51, 0.5, 9),
    "свидетельница": ("ya-veryu", 0.5, 0.5, 4),
    "сосиска": ("ne-odinoko-grustno", 0.5, 0.5, 4),
    "шаманка лечит": ("lechu", 0.5, 0.5, 4),
}


def crop_23(im: Image.Image, cx: float, cy: float, trim: int) -> Image.Image:
    im = im.crop((trim, trim, im.width - trim, im.height - trim))
    w, h = im.size
    if w * 3 > h * 2:  # шире 2:3 — режем по бокам
        cw = round(h * 2 / 3)
        left = min(max(round(cx * w - cw / 2), 0), w - cw)
        return im.crop((left, 0, left + cw, h))
    ch = round(w * 3 / 2)  # выше 2:3 — режем сверху и снизу
    top = min(max(round(cy * h - ch / 2), 0), h - ch)
    return im.crop((0, top, w, top + ch))


def main() -> None:
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    for name, (slug, cx, cy, trim) in COVERS.items():
        im = Image.open(src / f"книга-{name}.png").convert("RGB")
        cut = crop_23(im, cx, cy, trim).resize((W, H), Image.LANCZOS)
        target = out / f"book-{slug}.webp"
        cut.save(target, "WEBP", quality=88, method=6)
        print(f"{target.name}: {im.size} -> {W}x{H}, {target.stat().st_size // 1024} KB")


if __name__ == "__main__":
    main()
