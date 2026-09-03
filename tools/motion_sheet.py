#!/usr/bin/env python3
"""Собирает лист-заготовку, на котором рисуют детали движений боя.

Движок движений (`BATTLE-MOTION.md`) знает два вида рисунка: ОДИНОЧНЫЙ — то,
что летит от бьющего к цели или ложится на тело одним пятном, — и ПОЛОСА, где
кадры стоят в ряд и проигрываются `steps()`. Этот лист даёт клетки под оба, с
правильными пропорциями и подписями, и отдаёт его непрозрачным PNG.

    .venv-tools/bin/python tools/motion_sheet.py
    .venv-tools/bin/python tools/motion_sheet.py --out листы/ --size 2048

Главная хитрость — в том, что направляющие ИСЧЕЗАЮТ в разборе, и это не
случайность, а расчёт.

`sheet.rs` (и его двойник `tools/slice_sheet.py`) снимает фон одним из двух
способов и выбирает сам: если у листа есть настоящая альфа — берётся она; если
лист непрозрачный — фоном считается всё СВЕТЛОЕ И НЕСОЧНОЕ, досвязанное до края
холста (`v >= bg_value`, `s <= bg_sat`, по умолчанию 0.62 и 0.20).

Отсюда два требования к листу, и оба выполняются здесь:

  • Лист НЕПРОЗРАЧНЫЙ. Будь у него альфа, разбор пошёл бы по ней — и увидел бы
    непрозрачные направляющие как рисунок.
  • Каждый пиксель направляющих — светлый и несочный с запасом (`v >= 0.72`,
    `s <= 0.16` против порогов 0.62 и 0.20). Тогда они сливаются с фоном,
    досвязаны до края и в разбор не попадают вовсе.

Проверяется это не на глаз: пустой лист, пропущенный через разбор, обязан дать
НОЛЬ деталей. Если однажды даст не ноль — направляющие потемнели, и это ошибка
листа, а не разбора.

Из тех же требований — единственное правило для рисующего: **не стирайте
бумагу.** Рисунок кладётся ПОВЕРХ светлого фона, и сохранять надо сведённым,
без прозрачности. Прозрачный экспорт сломает разбор ровно тем, что уведёт его
на альфу.

Дальше одиночные детали живут обычным путём склада: «Ассеты» → загрузить лист →
разбор → отобрать → сохранить с ролью `motion`.

С ПОЛОСОЙ дорога другая, и разойтись они обязаны. Разбор нашёл бы в полосе
столько предметов, сколько в ней кадров, и разложил бы каждый в свой файл —
то есть уничтожил бы ровно то, чем полоса является. Значит, полосу не режут.
Но тогда с неё некому снять бумагу, а деталь с бумагой легла бы на доску
непрозрачным прямоугольником поверх карт. Поэтому:

    .venv-tools/bin/python tools/motion_sheet.py --clean моя-полоса.png

Съём тот же, что в разборе (та же маска, то же сглаживание края, та же растяжка
цвета под прозрачное), но без поиска предметов и без обрезки: холст остаётся
ровно того же размера, потому что кадры делятся по ширине и обрезка сдвинула бы
их все. Готовое грузится кнопкой «Загрузить» прямо в жест.

Отдельные холсты полос инструмент кладёт рядом сам — на них не надо ничего
вырезать. Ширина 1536 выбрана не для красоты: загрузчик деталей ужимает всё,
что шире 1600, и полоса приехала бы пересчитанной — с мылом ровно на стыках
кадров, то есть там, где его видно.
"""

import argparse
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
# Маска берётся у разбора, а не пишется здесь ещё раз. Третье воплощение
# одного алгоритма разъехалось бы с двумя первыми ровно так же, как разъехались
# бы они между собой, — и хранитель увидел бы одно на превью, другое на доске.
from slice_sheet import bleed_colors, box_blur, mask_from_background

# ── Тона ──────────────────────────────────────────────────────────────
#
# Все три обязаны читаться разбором как фон. Запас против порогов взят
# намеренно широкий: лист пересохраняют, а JPEG и уменьшение сдвигают и
# светлоту, и насыщенность.

PAPER = (244, 237, 225)   # v 0.957 · s 0.078
GUIDE = (207, 196, 178)   # v 0.812 · s 0.140
INK = (201, 189, 169)     # v 0.788 · s 0.159

BG_VALUE = 0.62
BG_SAT = 0.20
# Запас, который держит инструмент. Меньше порогов разбора, и намеренно.
SAFE_VALUE = 0.72
SAFE_SAT = 0.16

FONTS = [
    "/System/Library/Fonts/Supplemental/Georgia.ttf",
    "/Library/Fonts/Georgia.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
]


def value_and_sat(rgb):
    """Те же две величины, что считает разбор. Пишутся здесь, а не берутся из
    него, ровно потому же, почему разбор существует дважды: инструмент работает
    по папке, без сервера."""
    r, g, b = (c / 255.0 for c in rgb[:3])
    hi = max(r, g, b)
    lo = min(r, g, b)
    return hi, (0.0 if hi <= 0 else (hi - lo) / hi)


def check_tone(name, rgb):
    v, s = value_and_sat(rgb)
    if v < SAFE_VALUE or s > SAFE_SAT:
        raise SystemExit(
            f"тон «{name}» {rgb} не прочтётся как фон: v={v:.3f} s={s:.3f}, "
            f"надо v>={SAFE_VALUE} и s<={SAFE_SAT}"
        )
    return v, s


def font(size):
    for path in FONTS:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except OSError:
                pass
    return ImageFont.load_default()


def ticks(draw, box, reach=26, weight=3):
    """Метки по углам вместо рамки.

    Замкнутая рамка была бы ошибкой ДВАЖДЫ. Во-первых, она мешает рисовать:
    деталь часто выходит за клетку. Во-вторых — и это существеннее — рамка
    отрезает светлое внутри клетки от края холста, и если её однажды нарисуют
    тоном потемнее, разбор увидит не рисунок, а полный прямоугольник.
    """
    x0, y0, x1, y1 = box
    for cx, dx in ((x0, 1), (x1, -1)):
        for cy, dy in ((y0, 1), (y1, -1)):
            draw.line([(cx, cy), (cx + dx * reach, cy)], fill=GUIDE, width=weight)
            draw.line([(cx, cy), (cx, cy + dy * reach)], fill=GUIDE, width=weight)


def label(draw, xy, text, size=26, fill=INK, anchor="la"):
    draw.text(xy, text, font=font(size), fill=fill, anchor=anchor)


def rightward(draw, box):
    """Подсказка направления внутри клетки полёта.

    Не украшение: `turn: toTarget` поворачивает рисунок от НУЛЯ, а ноль — это
    «вправо». Стрела, нарисованная остриём вверх, полетит боком с любой клетки,
    и заметить это можно только на доске.
    """
    x0, y0, x1, y1 = box
    cy = (y0 + y1) // 2
    left = x0 + (x1 - x0) * 0.22
    right = x1 - (x1 - x0) * 0.22
    draw.line([(left, cy), (right, cy)], fill=GUIDE, width=2)
    for dy in (-14, 14):
        draw.line([(right - 22, cy + dy), (right, cy)], fill=GUIDE, width=2)


def cells(draw, x0, y0, count, side, gap, captions, flight=False):
    for i in range(count):
        cx = x0 + i * (side + gap)
        box = (cx, y0, cx + side, y0 + side)
        ticks(draw, box)
        label(draw, (cx + 8, y0 + side + 10), captions[i], size=24)
        if flight:
            rightward(draw, box)
    return y0 + side


def strip(draw, x0, y0, count, side, title):
    """Полоса кадров.

    Кадры разделены только тонкой чертой, и это правда про то, как полосу
    читает CSS: `background-size: n*100% 100%` делит картинку РОВНО на `n`
    равных частей, а не ищет в ней предметы. Значит, кадр обязан занимать свою
    долю целиком, и зазора между кадрами быть не должно.
    """
    w = count * side
    box = (x0, y0, x0 + w, y0 + side)
    ticks(draw, box, reach=34, weight=3)
    for i in range(1, count):
        x = x0 + i * side
        for y in range(y0 + 6, y0 + side - 6, 18):
            draw.line([(x, y), (x, y + 9)], fill=GUIDE, width=1)
    for i in range(count):
        label(draw, (x0 + i * side + 8, y0 + 8), str(i + 1), size=22)
    label(
        draw,
        (x0, y0 + side + 12),
        f"{title} · вырезать ровно {x0},{y0} — {x0 + w},{y0 + side} "
        f"({w}×{side}, кадр {side}×{side}) · или взять отдельный холст полосы, "
        f"который лежит рядом с этим листом",
        size=22,
    )
    return y0 + side


def build(width=2048):
    """Собирает лист сверху вниз и ОБРЕЗАЕТ холст по последнему блоку.

    Высота не задаётся: она получается. Заданная высота — это число, которое
    расходится с разметкой при первой же правке, и расходится молча: подпись
    налезает на полосу, а полоса уходит за край. Здесь блоки кладутся по
    очереди, курсор считает сам, и лист ровно такой, какой понадобился.
    """
    for name, rgb in (("бумага", PAPER), ("направляющие", GUIDE), ("подписи", INK)):
        check_tone(name, rgb)

    m = width // 32                     # поле листа
    inner = width - 2 * m
    # С запасом: холст обрежется по курсору.
    img = Image.new("RGB", (width, width * 2), PAPER)
    d = ImageDraw.Draw(img)

    y = m
    label(d, (m, y), "ЛИСТ ДВИЖЕНИЙ", size=40)
    y += 52
    label(
        d, (m, y),
        "Рисуйте ПОВЕРХ бумаги и сохраняйте сведённым, без прозрачности: "
        "светлый фон — это то, по чему разбор находит края.",
        size=24,
    )
    y += 32
    label(
        d, (m, y),
        "Направляющие и подписи в разбор не попадают — они светлее порога. "
        "Клетки можно перерастать: обрезка идёт по краю рисунка.",
        size=24,
    )

    side = (inner - 3 * 40) // 4
    gap = 40

    y += 100
    label(d, (m, y), "ПОЛЁТ — то, что идёт от бьющего к цели", size=30)
    y += 38
    label(
        d, (m, y),
        "Остриём ВПРАВО: поворот «вдоль линии к цели» считает вправо нулём.",
        size=24,
    )
    y = cells(
        d, m, y + 44, 4, side, gap,
        ["стрела", "болт", "камень", "дротик"],
        flight=True,
    ) + 44

    y += 70
    label(d, (m, y), "СЛЕД — то, что ложится на тело", size=30)
    y += 38
    label(d, (m, y), "Без направления: этот рисунок не поворачивают.", size=24)
    y = cells(
        d, m, y + 44, 4, side, gap,
        ["удар", "чара", "порча", "исцеление"],
    ) + 44

    y += 70
    label(d, (m, y), "ПОЛОСЫ КАДРОВ", size=30)
    y += 38
    label(
        d, (m, y),
        "Кадры одного размера, встык, без зазора: показ делит картинку ровно "
        "на столько частей, сколько кадров, а не ищет в ней предметы.",
        size=24,
    )

    y += 52
    six = (inner // 6 // 2) * 2
    y = strip(d, m + (inner - six * 6) // 2, y, 6, six, "шесть кадров") + 46

    y += 56
    eight = (inner // 8 // 2) * 2
    y = strip(d, m + (inner - eight * 8) // 2, y, 8, eight, "восемь кадров") + 46

    y += 56
    label(
        d, (m, y),
        "Одиночные детали: «Ассеты» · загрузить лист · разбор · "
        "роль «деталь движения». Бумагу с них снимает сам разбор.",
        size=24,
    )
    y += 32
    label(
        d, (m, y),
        "Полосу не режут — разбор разложил бы её на кадры. Бумагу с неё "
        "снимают отдельно: tools/motion_sheet.py --clean полоса.png",
        size=24,
    )
    y += 32
    label(
        d, (m, y),
        "и грузят кнопкой «Загрузить» прямо в жест.",
        size=24,
    )
    y += 30

    return img.crop((0, 0, width, y + m))


# Ширина отдельной полосы. Не круглое число ради красоты: загрузчик деталей
# (`POST /admin/battles/frames/art`) ужимает всё, что шире 1600, и полоса в
# 1920 приехала бы пересчитанной — с мылом ровно на стыках кадров, то есть там,
# где его видно. 1536 проходит нетронутой и делится нацело на 4, 6, 8 и 12.
STRIP_WIDTH = 1536


def build_strip(count, width=STRIP_WIDTH):
    """Полоса отдельным холстом.

    На большом листе полосу приходится вырезать по меткам, а это единственный
    шаг во всей дороге, где человек может ошибиться на пиксель — и ошибка
    выйдет не сообщением, а кадром, съезжающим при каждом показе. Отдельный
    холст этот шаг убирает: нарисовал и загрузил как есть.
    """
    side = width // count
    width = side * count                # ровно, без остатка
    img = Image.new("RGB", (width, side), PAPER)
    d = ImageDraw.Draw(img)
    for i in range(1, count):
        x = i * side
        for y in range(4, side - 4, 18):
            d.line([(x, y), (x, y + 9)], fill=GUIDE, width=1)
    for i in range(count):
        label(d, (i * side + 8, 8), str(i + 1), size=22)
    label(
        d, (8, side - 34),
        f"{count} кадров по {side}×{side} · рисовать поверх, сохранять сведённым",
        size=22,
    )
    return img


def clean(path, out, bg_value=BG_VALUE, bg_sat=BG_SAT, feather=1, bleed=3):
    """Снять бумагу с готового рисунка, НЕ разрезая его.

    Нужно ровно полосе, и это не удобство, а условие. Одиночную деталь фон
    покидает сам: её вырезает разбор, а он и снимает бумагу. Полосу же не
    режут — иначе она распалась бы на столько предметов, сколько в ней
    кадров, — значит, снять с неё фон некому, и на доску она легла бы
    непрозрачным прямоугольником бумаги поверх карт. Вместе с
    направляющими: их тоже никто бы не убрал.

    Здесь тот же съём, что в разборе, но без поиска предметов и без обрезки:
    холст остаётся ровно того же размера, потому что кадры делятся по ширине
    и обрезка сдвинула бы их все.
    """
    im = Image.open(path).convert("RGB")
    rgb = np.array(im)
    mask = mask_from_background(rgb, bg_value, bg_sat)
    if not mask.any():
        raise SystemExit(f"{path}: на рисунке нет ничего, кроме бумаги")
    # Мягкий край и растяжка цвета под прозрачное — те же, что у разбора,
    # чтобы деталь, снятая здесь, выглядела как деталь, снятая там.
    alpha = box_blur(mask.astype(np.float32), feather) if feather else mask.astype(np.float32)
    colors = bleed_colors(rgb, mask, bleed)
    out_img = Image.fromarray(
        np.dstack([colors, np.clip(alpha * 255.0, 0, 255).astype(np.uint8)]), "RGBA"
    )
    out_img.save(out, "PNG")
    return out_img, float(mask.mean())


def main():
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument("--out", default="tools/sheets/motion-sheet.png",
                   help="куда положить лист")
    p.add_argument("--size", type=int, default=2048, help="ширина листа, px")
    p.add_argument("--strip", type=int, action="append", default=[],
                   help="ещё и отдельный холст полосы на столько кадров "
                        "(можно несколько раз); по умолчанию 6 и 8")
    p.add_argument("--clean", nargs="+", metavar="ФАЙЛ",
                   help="не собирать лист, а СНЯТЬ бумагу с готовых рисунков "
                        "(нужно полосе: её не режут, и фон с неё снять больше "
                        "некому). Кладёт рядом `<имя>-clean.png` с альфой")
    args = p.parse_args()

    if args.clean:
        for path in args.clean:
            stem, _ = os.path.splitext(path)
            out = f"{stem}-clean.png"
            img, share = clean(path, out)
            print(f"{out} · {img.width}×{img.height} · рисунка {share * 100:.1f}%")
        return

    folder = os.path.dirname(args.out) or "."
    os.makedirs(folder, exist_ok=True)
    stem = os.path.splitext(os.path.basename(args.out))[0]

    img = build(args.size)
    # Без альфы намеренно: прозрачность увела бы разбор на другой путь.
    img.save(args.out, "PNG")
    print(f"{args.out} · {img.width}×{img.height}")

    for count in (args.strip or [6, 8]):
        if count < 2 or count > 24:
            raise SystemExit(f"кадров в полосе {count}: надо от 2 до 24")
        band = build_strip(count)
        path = os.path.join(folder, f"{stem}-strip-{count}.png")
        band.save(path, "PNG")
        print(f"{path} · {band.width}×{band.height}")


if __name__ == "__main__":
    main()
