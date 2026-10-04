#!/usr/bin/env python3
"""Комплект деталей для конструктора рамок битв: форматы, заготовки, приёмка.

Рамка режима `sliced` собирается из трёх деталей — УГОЛ, кайма ГОРИЗОНТАЛЬНАЯ,
кайма ВЕРТИКАЛЬНАЯ (плюс необязательные накладки). Конструктор ставит их по
четырём врезкам, и все размеры на карте выражены В ПРОЦЕНТАХ КАРТЫ. Отсюда
главное свойство, ради которого написан этот файл:

    отношение сторон каждой детали на карте ПОСТОЯННО и не зависит ни от
    величины карты, ни от экрана. Оно считается из одной величины — толщины
    полосы.

Значит деталь можно нарисовать заранее ровно в том отношении сторон, в каком её
покажет карта, и тогда `stretch` растягивает её в 1.0 — не мылит вовсе. Если
нарисовать «на глаз», растяжение будет любым, и рамка поедет: именно так
выглядели первые сборки — смазанный верх при плотных боках.

Считается всё от ТОЛЩИНЫ ПОЛОСЫ `t`, в процентах ШИРИНЫ карты:

    врезка слева/справа  = t
    врезка сверху/снизу  = t / 1.4          (карта 5 : 7, высота в 1.4 ширины)
    длина каймы H        = (100 - 2t + 2g) / t   толщин
    длина каймы V        = (140 - 2t + 2g) / t   толщин
    угол                 = КВАДРАТ, всегда, при любом t

где `g` — заход каймы под угол (в процентах ширины), нужный лишь затем, чтобы
обрезанный конец каймы прятался под углом и волосяная щель от округления не
светилась.

Два режима, и выбор между ними — это выбор холста:

  РАСТЯНУТЬ (`stretch`). Деталь рисуется ровно в отношении из таблицы. Рисунок
  вдоль стороны не повторяется ни разу — можно вести композицию от угла к
  середине. Но холст выходит вроде 12 : 1, а то и 1 : 18, и такое рисуют
  руками, а не моделью.

  ПОВТОРИТЬ (`tile`). Деталь масштабируется по толщине и повторяется вдоль
  стороны. Холст любой удобный — НО отношение обязано быть длиной полосы,
  ПОДЕЛЁННОЙ НА ЦЕЛОЕ ЧИСЛО. Тогда повторов укладывается ровно столько, сколько
  задумано, и у дальнего конца не остаётся обрубка. Это единственное условие, и
  оно же делает режим пригодным для картиночной модели: при n = 12 кайма H
  почти квадратная.

    .venv-tools/bin/python tools/frame_kit.py spec --thickness 7
    .venv-tools/bin/python tools/frame_kit.py blanks --thickness 7 --out tools/sheets/frames
    .venv-tools/bin/python tools/frame_kit.py check рамка/кайма-h.png --part edgeH -t 7
    .venv-tools/bin/python tools/frame_kit.py draw --palette gilt --motif plait --out /tmp/kit

`check` — не украшение. Он меряет ровно то, из-за чего рамки и разъезжались:
набалдашники на концах каймы, прозрачные поля по краям, несимметричный
поперечный срез, шов у повторяющейся полосы. Деталь, не прошедшую приёмку,
дешевле перерисовать, чем потом двигать ползунками.
"""
from __future__ import annotations

import argparse
import math
import os

import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# ── Геометрия карты ───────────────────────────────────────────────────────
ASPECT = 5 / 7            # ширина ÷ высоту карты
H_OVER_W = 1 / ASPECT     # = 1.4
TUCK = 0.4                # заход каймы под угол, % ширины карты


class Geometry:
    """Всё, что следует из одной толщины полосы."""

    def __init__(self, thickness: float, tuck: float = TUCK):
        self.t = float(thickness)
        self.g = float(tuck)
        self.inset_x = self.t                      # врезка слева/справа, % ширины
        self.inset_y = self.t / H_OVER_W           # врезка сверху/снизу, % высоты
        self.grow_x_h = self.g                     # заход каймы H, % ширины
        self.grow_y_v = self.g / H_OVER_W          # заход каймы V, % высоты
        # Длины полос, в толщинах.
        self.len_h = (100 - 2 * self.t + 2 * self.g) / self.t
        self.len_v = (140 - 2 * self.t + 2 * self.g) / self.t
        # Доводка накладок: коробка меряется разными мерками по двум осям, и
        # квадратной её делает вычисленная поправка, а не подбор.
        self.boss_grow_x = self.inset_x - self.inset_y
        self.boss_grow_y = self.inset_y - self.inset_x

    def canvas(self, part: str, thickness_px: int, repeats: int = 1) -> tuple[int, int]:
        """Холст детали в пикселях. `repeats` = 1 — режим «растянуть»."""
        if part == 'corner':
            return (thickness_px, thickness_px)
        length = (self.len_h if part == 'edgeH' else self.len_v) / repeats
        long_side = int(round(thickness_px * length))
        return (long_side, thickness_px) if part == 'edgeH' else (thickness_px, long_side)


def cmd_spec(args) -> None:
    g = Geometry(args.thickness)
    px = args.pixels
    print(f'Карта 5 : 7 · толщина полосы {g.t:g}% ширины · заход под угол {g.g:g}%\n')
    print('ВРЕЗКИ, их вписать в конструкторе:')
    print(f'  слева / справа   {g.inset_x:.3f} %')
    print(f'  сверху / снизу   {g.inset_y:.3f} %')
    print(f'  заход каймы H    growX {g.grow_x_h:+.3f}')
    print(f'  заход каймы V    growY {g.grow_y_v:+.3f}')
    print(f'  накладка         growX {g.boss_grow_x:+.3f} · growY {g.boss_grow_y:+.3f}')
    print(f'  слои             угол 6 · каймы 4 · накладки 8\n')

    print(f'РЕЖИМ «РАСТЯНУТЬ» — один рисунок на сторону (толщина {px} px):')
    for part, name in (('corner', 'угол   '), ('edgeH', 'кайма H'), ('edgeV', 'кайма V')):
        w, h = g.canvas(part, px)
        ratio = '1 : 1' if part == 'corner' else (
            f'{g.len_h:.3f} : 1' if part == 'edgeH' else f'1 : {g.len_v:.3f}')
        print(f'  {name}  {w:>5} × {h:<5}  ({ratio})')

    print(f'\nРЕЖИМ «ПОВТОРИТЬ» — холст на выбор, отношение обязано быть длиной ÷ целое:')
    print(f'  {"n":>3}  {"кайма H":>16}  {"кайма V":>16}   при толщине {px} px')
    for n in range(2, 13):
        wh, hh = g.canvas('edgeH', px, n)
        wv, hv = g.canvas('edgeV', px, n)
        print(f'  {n:>3}  {g.len_h / n:>7.3f} : 1 {wh:>5}×{hh:<4}  1 : {g.len_v / n:<7.3f}'
              f' {wv:>5}×{hv:<4}')
    print('\n  Угол в обоих режимах — квадрат; повторов у него нет.')


# ── Заготовки ─────────────────────────────────────────────────────────────
#
# Направляющие нарочно ПОЛУПРОЗРАЧНЫЕ и лежат на прозрачном холсте: деталь
# рамки приходит на склад как есть, мимо разбора листов, поэтому прятать
# направляющие от порога фона не нужно — их просто стирают перед сдачей. Тем же
# и отличается этот лист от `motion_sheet.py`, где лист обязан быть непрозрачным.
GUIDE = (52, 37, 28, 70)
GUIDE_FAINT = (52, 37, 28, 34)
LABEL = (52, 37, 28, 150)


def _font(size: int):
    """Шрифт с кириллицей. У встроенного в PIL её нет вовсе, и подпись
    «к кайме H» вышла бы рядом квадратиков — то есть заготовкой без правил."""
    for path in ('/System/Library/Fonts/Supplemental/Arial.ttf',
                 '/System/Library/Fonts/Helvetica.ttc',
                 '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
                 os.path.join(os.path.dirname(os.path.abspath(ImageFont.__file__)),
                              'fonts', 'DejaVuSans.ttf')):
        try:
            return ImageFont.truetype(path, size)
        except (OSError, ValueError):
            continue
    return ImageFont.load_default()


def _guides(img: Image.Image, part: str, g: Geometry) -> None:
    d = ImageDraw.Draw(img)
    w, h = img.size
    font = _font(max(11, min(w, h) // 26))
    thin = max(1, min(w, h) // 256)
    d.rectangle([0, 0, w - 1, h - 1], outline=GUIDE, width=thin * 2)

    # Поперечный срез: где идут продольные обломы. Числа — доли толщины, те же,
    # что у рисованных комплектов дома.
    for v in (0.045, 0.115, 0.150, 0.215, 0.255):
        for u in (v, 1 - v):
            if part == 'edgeV' or part == 'corner':
                x = u * w
                d.line([(x, 0), (x, h)], fill=GUIDE_FAINT, width=thin)
            if part == 'edgeH' or part == 'corner':
                y = u * h
                d.line([(0, y), (w, y)], fill=GUIDE_FAINT, width=thin)

    if part == 'corner':
        # Ус: по нему срез переходит от каймы H к кайме V. Рисунок обязан
        # сойтись на нём сам с собой, иначе угол не стыкуется ни с одной каймой.
        d.line([(0, 0), (w, h)], fill=GUIDE, width=thin)
        d.text((w * 0.56, h * 0.05), 'срез = срез каймы H →', fill=LABEL, font=font)
        d.text((w * 0.05, h * 0.92), 'срез = срез каймы V ↓', fill=LABEL, font=font)
        d.text((w * 0.30, h * 0.46), 'ус', fill=LABEL, font=font)
    else:
        # Концы — самое больное место: там не должно быть набалдашников.
        for u in (0.0, 1.0):
            if part == 'edgeH':
                x = u * (w - thin)
                d.line([(x, 0), (x, h)], fill=GUIDE, width=thin * 2)
            else:
                y = u * (h - thin)
                d.line([(0, y), (w, y)], fill=GUIDE, width=thin * 2)
        pad = max(6, min(w, h) // 40)
        d.text((pad, pad), 'концы — тот же рисунок, что в середине; '
                           'срез симметричен поперёк', fill=LABEL, font=font)


def cmd_blanks(args) -> None:
    g = Geometry(args.thickness)
    os.makedirs(args.out, exist_ok=True)
    made = []
    for part in ('corner', 'edgeH', 'edgeV'):
        n = 1 if part == 'corner' else args.repeats
        w, h = g.canvas(part, args.pixels, n)
        img = Image.new('RGBA', (w, h), (0, 0, 0, 0))
        _guides(img, part, g)
        tag = 'stretch' if n == 1 else f'tile{n}'
        name = f'{part}-t{args.thickness:g}-{tag}-{w}x{h}.png'
        img.save(os.path.join(args.out, name))
        made.append(name)
    readme = os.path.join(args.out, f'ЧИТАТЬ-t{args.thickness:g}.txt')
    with open(readme, 'w') as fh:
        fh.write(RULES.format(t=g.t, ix=g.inset_x, iy=g.inset_y,
                              gh=g.grow_x_h, gv=g.grow_y_v,
                              lh=g.len_h, lv=g.len_v, n=args.repeats))
    for name in made:
        print('заготовка:', os.path.join(args.out, name))
    print('правила:  ', readme)


RULES = """ПРАВИЛА ДЕТАЛИ РАМКИ (толщина полосы {t:g}% ширины карты)

Врезки в конструкторе: слева/справа {ix:.3f}% · сверху/снизу {iy:.3f}%
Заход под угол: кайма H growX {gh:+.3f} · кайма V growY {gv:+.3f}
Длина каймы H {lh:.3f} толщин · каймы V {lv:.3f} толщин · повторов {n}

1. PNG с АЛЬФОЙ. Никакого белого фона: белое станет белым и на карте.

2. Деталь заполняет холст ЦЕЛИКОМ. Прозрачное поле по краю — это дырка в раме,
   а не воздух: конструктор ставит деталь по коробке, а не по рисунку.

3. У каймы концы НЕ ПЛОТНЕЕ середины. Законченная планка с набалдашниками на
   концах — не кайма: растянутая на всю сторону, она приносит свои концы ровно
   в стык с углом. Это и была главная причина прежних огрехов.

4. Поперечный срез каймы СИММЕТРИЧЕН относительно её средней линии (у H верх =
   низ, у V лево = право). Противоположную сторону конструктор рисует зеркалом,
   и при симметричном срезе зеркало невидимо.

5. Кайма для режима «повторить» БЕСШОВНА: левый край продолжает правый.
   Надёжнее всего — нарисовать половину и отзеркалить.

6. Угол рисуется ТОЛЬКО левый верхний. Остальные три конструктор отзеркалит.

7. Угол сходится с каймами по срезу: у ПРАВОГО края квадрата поперечный срез
   обязан совпасть со срезом каймы H, у НИЖНЕГО — со срезом каймы V. Внутри
   квадрата срез переходит от одного к другому по усу (диагональ от внешнего
   угла к внутреннему) — как в настоящей раме.

   Если срезы свести не удаётся (а у писанного красками угла обычно не
   удаётся), стык не выправляют ползунками — его ЗАКРЫВАЮТ: накладка в слот
   `cornerExtra` садится поверх обеих деталей и прячет переход. Так рама и
   устроена в жизни: розетка в локте не украшение, а именно крышка стыка.

8. Приёмка: tools/frame_kit.py check <файл> --part corner|edgeH|edgeV -t {t:g}
"""


# ── Приёмка ───────────────────────────────────────────────────────────────
def cmd_check(args) -> None:
    g = Geometry(args.thickness)
    img = Image.open(args.file).convert('RGBA')
    a = np.asarray(img, np.float32)
    al = a[..., 3] / 255.0
    h, w = al.shape
    solid = al > 0.16
    ok = True

    def say(good: bool, line: str) -> None:
        nonlocal ok
        ok = ok and good
        print(('  ✓ ' if good else '  ✗ ') + line)

    print(f'{os.path.basename(args.file)} — {w}×{h}, деталь «{args.part}»\n')

    # 1. Отношение сторон против таблицы.
    if args.part == 'corner':
        got, want, n = w / h, 1.0, 1
    else:
        length = g.len_h if args.part == 'edgeH' else g.len_v
        got = (w / h) if args.part == 'edgeH' else (h / w)
        n = max(1, round(length / got))
        want = length / n
    off = abs(got - want) / want * 100
    mode = 'растянуть' if n == 1 else f'повторить ×{n}'
    say(off < 1.5, f'отношение {got:.3f} против {want:.3f} ({mode}) — расхождение {off:.2f}%')
    if off >= 1.5 and args.part != 'corner':
        print(f'      ближе всего: {mode}. Холст под него: '
              f'{g.canvas(args.part, h if args.part == "edgeH" else w, n)}')

    # 2. Поля по краям: деталь обязана доходить до края холста.
    ys, xs = np.where(solid)
    if len(xs) == 0:
        say(False, 'деталь пуста — одна прозрачность')
        return
    pads = (int(ys.min()), int(w - 1 - xs.max()), int(h - 1 - ys.max()), int(xs.min()))
    worst = max(pads) / max(w, h) * 100
    say(worst < 1.0, f'поля по краям t/r/b/l = {pads} — {worst:.2f}% холста')

    # 3. Набалдашники: концы плотнее середины — это не кайма.
    if args.part in ('edgeH', 'edgeV'):
        dens = solid.mean(axis=0) if args.part == 'edgeH' else solid.mean(axis=1)
        k = max(1, int(len(dens) * 0.10))
        ends = (dens[:k].mean() + dens[-k:].mean()) / 2
        mid = dens[int(len(dens) * 0.40):int(len(dens) * 0.60)].mean()
        say(ends <= mid * 1.12,
            f'концы {ends:.3f} против середины {mid:.3f} — '
            + ('бегущая кайма' if ends <= mid * 1.12 else 'НАБАЛДАШНИКИ, в стык не годится'))

        # 4. Срез симметричен поперёк — иначе зеркало на другую сторону видно.
        cross = solid.mean(axis=1) if args.part == 'edgeH' else solid.mean(axis=0)
        diff = float(np.abs(cross - cross[::-1]).mean())
        say(diff < 0.06, f'поперечный срез: расхождение с зеркалом {diff:.3f}')

        # 5. Шов у повторяющейся полосы.
        if n > 1:
            rgb = a[..., :3] * al[..., None]
            if args.part == 'edgeH':
                seam = np.abs(rgb[:, 0] - rgb[:, -1]).mean() / 255
                step = np.abs(rgb[:, 1:] - rgb[:, :-1]).mean() / 255
            else:
                seam = np.abs(rgb[0, :] - rgb[-1, :]).mean() / 255
                step = np.abs(rgb[1:, :] - rgb[:-1, :]).mean() / 255
            say(seam < max(0.04, step * 3),
                f'шов при повторе {seam:.4f} против обычного шага {step:.4f}')

    # 6. У угла срез вдоль правого и нижнего краёв — это то, чем он стыкуется.
    if args.part == 'corner':
        # Пересчёт к общей длине: у неквадратного угла края разной длины, а
        # сравнить их надо именно тогда — квадратный до этой проверки и не
        # доходил бы.
        def resample(line, n=256):
            src = np.asarray(line, np.float64)
            return np.interp(np.linspace(0, len(src) - 1, n), np.arange(len(src)), src)

        right, bottom = resample(solid[:, -1]), resample(solid[-1, :])
        diff = float(np.abs(right - bottom).mean())
        say(diff < 0.06, f'правый край против нижнего: расхождение {diff:.3f}')
        print('      (сравните их со срезом своих кайм — стыкуется именно это)')

    print('\nприёмка: ' + ('ПРОЙДЕНА' if ok else 'НЕ ПРОЙДЕНА'))
    raise SystemExit(0 if ok else 1)


# ── Рисованный комплект ───────────────────────────────────────────────────
#
# Тот самый, которым сделаны «Плетёнка», «Бусина», «Лоза», «Зубец» и «Цепь».
# Он же — образец, по которому видно, что именно проверяет `check`.
PALETTES = {
    'gilt':      dict(rule='#4a3512', mid='#8a6a2c', lit='#f0dca0', field='#6b4f1d', strand='#c9a24a'),
    'bone':      dict(rule='#3b3833', mid='#7d766b', lit='#ece5d6', field='#565046', strand='#b8b0a1'),
    'ink':       dict(rule='#241810', mid='#5a4231', lit='#d8c6b1', field='#3a2a1e', strand='#8a6a4c'),
    'verdigris': dict(rule='#1b2c24', mid='#3f6553', lit='#bcd8c8', field='#2a463a', strand='#6f9a83'),
    'rust':      dict(rule='#3a1710', mid='#7a3320', lit='#f0c3a6', field='#4f1f14', strand='#c65f3c'),
}
FIELD0, FIELD1 = 0.255, 0.745
SS = 4


def _rgba(hexstr: str) -> tuple[int, int, int, int]:
    h = hexstr.lstrip('#')
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)


def _profile(pal):
    return [(0.000, 0.045, pal['rule']), (0.045, 0.115, pal['mid']),
            (0.115, 0.150, pal['lit']), (0.150, 0.215, pal['mid']),
            (0.215, 0.255, pal['rule']), (0.255, 0.500, pal['field'])]


def _lut(pal, n=1024):
    """Срез полосы. У каждого продольного облома своя фаска: без неё срез —
    плоские заливки, то есть клипарт, а не точёный профиль."""
    out = np.zeros((n, 4), np.float32)
    for a, b, c in _profile(pal):
        for lo, hi in ((a, b), (1 - b, 1 - a)):
            i0, i1 = int(round(lo * n)), int(round(hi * n))
            if i1 <= i0:
                continue
            t = (np.arange(i1 - i0) + 0.5) / (i1 - i0)
            shade = (0.86 + 0.26 * np.sin(np.pi * t))[:, None]
            px = np.array(c, np.float32)[None, :].repeat(i1 - i0, 0)
            px[:, :3] = np.clip(px[:, :3] * shade, 0, 255)
            out[i0:i1] = px
    return out


def _age(img, seed):
    """Пятна и зерно. Нарочно МЕЛКИЕ: угол и кайма — разные картинки, их поля
    пятен независимы, и крупное пятно дало бы на стыке тональный уступ.
    Возраст, выдающий шов, хуже отсутствия возраста."""
    a = np.asarray(img, np.float32)
    h, w = a.shape[:2]
    rng = np.random.default_rng(seed)
    small = rng.normal(1.0, 0.045, (max(2, h // 10 + 2), max(2, w // 10 + 2)))
    blot = np.asarray(Image.fromarray(np.clip(small * 127, 0, 255).astype(np.uint8))
                      .resize((w, h), Image.BICUBIC), np.float32) / 127.0
    grain = rng.normal(1.0, 0.035, (h, w))
    a[..., :3] *= np.clip(blot * grain, 0.84, 1.10)[..., None]
    return Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), 'RGBA')


def _plait(d, L, T, n, pal):
    p, (y0, y1) = L / n, (FIELD0 * T, FIELD1 * T)
    mid, amp = (y0 + y1) / 2, (y1 - y0) / 2 * 0.62
    for phase, colour in ((0.0, pal['strand']), (0.5, pal['lit'])):
        d.line([(i, mid + amp * math.sin((i / p + phase) * 2 * math.pi)) for i in range(int(L) + 1)],
               fill=colour, width=max(2, int(T * 0.085)), joint='curve')


def _bead(d, L, T, n, pal):
    p, mid = L / n, (FIELD0 + FIELD1) / 2 * T
    r = (FIELD1 - FIELD0) * T * 0.30
    for i in range(n):
        x = (i + 0.5) * p
        d.ellipse([x - r, mid - r, x + r, mid + r], fill=pal['strand'])
        d.ellipse([x - r * .42, mid - r * .42, x + r * .42, mid + r * .42], fill=pal['lit'])
        xb = x + p / 2
        d.rectangle([xb - p * .10, mid - r * .42, xb + p * .10, mid + r * .42], fill=pal['strand'])


def _dentil(d, L, T, n, pal):
    p, (y0, y1) = L / n, (FIELD0 * T, FIELD1 * T)
    for i in range(n):
        x = i * p
        d.rectangle([x + p * .22, y0 + (y1 - y0) * .18, x + p * .78, y1 - (y1 - y0) * .18],
                    fill=pal['strand'])
        d.rectangle([x + p * .22, y0 + (y1 - y0) * .18, x + p * .78, y0 + (y1 - y0) * .34],
                    fill=pal['lit'])


def _vine(d, L, T, n, pal):
    p, (y0, y1) = L / n, (FIELD0 * T, FIELD1 * T)
    mid, amp = (y0 + y1) / 2, (y1 - y0) * 0.20
    d.line([(i, mid + amp * math.sin(i / p * 2 * math.pi)) for i in range(int(L) + 1)],
           fill=pal['strand'], width=max(2, int(T * 0.055)), joint='curve')
    lr = (y1 - y0) * 0.30
    for i in range(n):
        for k, up in ((0.25, True), (0.75, False)):
            x = (i + k) * p
            y = mid + amp * math.sin((i + k) * 2 * math.pi)
            cy = y - lr if up else y + lr
            d.ellipse([x - lr * .85, cy - lr * .62, x + lr * .85, cy + lr * .62],
                      fill=pal['strand'] if up else pal['lit'])


def _chain(d, L, T, n, pal):
    p, (y0, y1) = L / n, (FIELD0 * T, FIELD1 * T)
    mid, r = (y0 + y1) / 2, (y1 - y0) * 0.42
    w = max(2, int(T * 0.05))
    for i in range(n + 1):
        x = i * p
        d.polygon([(x, mid - r), (x + p / 2, mid), (x, mid + r), (x - p / 2, mid)],
                  outline=pal['strand'], width=w)
        d.polygon([(x, mid - r * .34), (x + p * .17, mid), (x, mid + r * .34),
                   (x - p * .17, mid)], fill=pal['lit'])


MOTIFS = {'plait': _plait, 'bead': _bead, 'dentil': _dentil, 'vine': _vine, 'chain': _chain}


def draw_strip(length_units, n_periods, pal, motif, px, vertical=False):
    T, L = px, int(round(px * length_units))
    table = _lut(pal)
    v = (np.arange(T) + 0.5) / T
    row = table[(v * (len(table) - 1)).astype(int)]
    img = Image.fromarray(np.repeat(row[None, :, :], L, 0).transpose(1, 0, 2).astype(np.uint8),
                          'RGBA')
    big = Image.new('RGBA', (L * SS, T * SS), (0, 0, 0, 0))
    MOTIFS[motif](ImageDraw.Draw(big), L * SS, T * SS, n_periods, pal)
    img.alpha_composite(big.resize((L, T), Image.LANCZOS))
    img = _age(img, 101 + (7 if vertical else 0))
    return img.rotate(90, expand=True) if vertical else img


def draw_corner(pal, px, boss='lozenge'):
    """Угол: точный ус из того же среза, что и каймы, плюс розетка в локте.
    У правого края коробки срез — ровно срез каймы H, у нижнего — каймы V,
    поэтому стык сходится по построению, а не по подгонке."""
    T = px * SS
    xs, ys = (np.arange(T) + 0.5)[None, :], (np.arange(T) + 0.5)[:, None]
    v = np.where(ys <= xs, ys / T, xs / T)
    table = _lut(pal)
    img = Image.fromarray(table[(v * (len(table) - 1)).astype(int)].astype(np.uint8), 'RGBA')
    d = ImageDraw.Draw(img)
    c, r = T / 2, (FIELD1 - FIELD0) * T * 0.46
    if boss == 'disc':
        d.ellipse([c - r, c - r, c + r, c + r], fill=pal['strand'])
        d.ellipse([c - r * .46, c - r * .46, c + r * .46, c + r * .46], fill=pal['lit'])
    elif boss == 'square':
        d.rectangle([c - r * .8, c - r * .8, c + r * .8, c + r * .8], fill=pal['strand'])
        d.rectangle([c - r * .36, c - r * .36, c + r * .36, c + r * .36], fill=pal['lit'])
    else:
        d.polygon([(c, c - r), (c + r, c), (c, c + r), (c - r, c)], fill=pal['strand'])
        d.polygon([(c, c - r * .45), (c + r * .45, c), (c, c + r * .45), (c - r * .45, c)],
                  fill=pal['lit'])
    return _age(img.resize((px, px), Image.LANCZOS), 202)


def draw_boss(pal, px, shape='lozenge'):
    T = px * SS
    img = Image.new('RGBA', (T, T), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c, r = T / 2, T * 0.44
    if shape == 'disc':
        for k, colour in ((1.0, pal['rule']), (0.78, pal['strand']), (0.34, pal['lit'])):
            d.ellipse([c - r * k, c - r * k, c + r * k, c + r * k], fill=colour)
    else:
        for k, colour in ((1.0, pal['rule']), (0.80, pal['strand']), (0.40, pal['lit'])):
            d.polygon([(c, c - r * k), (c + r * k * .72, c), (c, c + r * k), (c - r * k * .72, c)],
                      fill=colour)
    return img.resize((px, px), Image.LANCZOS)


def cmd_draw(args) -> None:
    g = Geometry(args.thickness)
    pal = {k: _rgba(v) for k, v in PALETTES[args.palette].items()}
    os.makedirs(args.out, exist_ok=True)
    n_h = args.periods
    n_v = max(1, round(n_h * g.len_v / g.len_h))
    parts = {
        'corner': draw_corner(pal, args.pixels, args.boss),
        'edgeH': draw_strip(g.len_h, n_h, pal, args.motif, args.pixels),
        'edgeV': draw_strip(g.len_v, n_v, pal, args.motif, args.pixels, vertical=True),
        'boss': draw_boss(pal, args.pixels, args.boss),
    }
    for name, img in parts.items():
        path = os.path.join(args.out, f'{args.palette}-{args.motif}-{name}.png')
        img.save(path)
        print(f'{name:>7}  {img.size[0]:>5}×{img.size[1]:<5}  {path}')
    print(f'\nпериодов: H {n_h} · V {n_v} (шаг орнамента совпадает на обеих сторонах)')
    print('дальше: «Ассеты» → загрузить с ролями corner / sideH / sideV / accent')



# ── Разрез целой рамы ─────────────────────────────────────────────────────
#
# Самый короткий путь к красивой раме: картиночная модель рисует РАМУ ЦЕЛИКОМ
# — это она умеет, — а на детали её режем здесь.
#
# Выигрыш не в удобстве, а в стыке: угол и каймы вырезаны из ОДНОГО рисунка,
# поэтому их поперечные срезы совпадают сами собой. Ровно то условие, которое у
# порознь нарисованных деталей выполнить почти невозможно.
#
# И толщина полосы здесь не назначается, а ВЫЧИСЛЯЕТСЯ: берётся такая, при
# которой каймы растягиваются ровно в 1.0. Пропорции художника сохраняются, а
# мыла нет вовсе. Если рама нарисована в 5 : 7, то же самое сходится и по
# вертикали; при другом отношении расхождение печатается числом.
def _cut_alpha(img, bg_value=0.62, bg_sat=0.20):
    """Снять фон и ВЫБИТЬ ОКНО.

    Разбор листов снимает светлое, досвязанное до края холста, — и правильно
    делает: иначе белёсый блик внутри самоцвета стал бы дыркой. Но у рамы
    ровно одна такая дырка задумана — её окно, и до края холста оно не
    дотягивается. Пока окно считалось рисунком, полоса меряется в пол-рамы.

    Поэтому к фону добавляется самая большая ЗАМКНУТАЯ светлая область, и
    только если она размером с окно (шире трети холста и выше трети). Блик на
    самоцвете столько не занимает, а окно занимает всегда."""
    import slice_sheet as ss
    window = None
    rgba = np.asarray(img.convert('RGBA'))
    if rgba[..., 3].min() < 200:
        mask = ss.mask_from_alpha(rgba, 24)
    else:
        mask = ss.mask_from_background(rgba[..., :3], bg_value, bg_sat)
        f = rgba[..., :3].astype(np.float32) / 255.0
        v, mn = f.max(2), f.min(2)
        sat = np.where(v > 0, (v - mn) / np.maximum(v, 1e-6), 0.0)
        holes = (v >= bg_value) & (sat <= bg_sat) & mask
        labels, n = ss.connected_components(holes)
        h, w = mask.shape
        for i in range(1, n + 1):
            ys, xs = np.where(labels == i)
            if len(xs) and (xs.max() - xs.min()) > w / 3 and (ys.max() - ys.min()) > h / 3:
                mask = mask & ~(labels == i)
                window = labels == i
    rgb = ss.bleed_colors(rgba[..., :3], mask, 6)
    out = np.dstack([rgb, np.where(mask, 255, 0).astype(np.uint8)])
    return Image.fromarray(out, 'RGBA'), mask, window


def window_insets(hole, margin=1.5):
    """Врезки по ВПИСАННОМУ в окно прямоугольнику, а не по габаритам дыры.

    Габариты дыры — это не окно. У богатой рамы внутрь свисают колонны, листья
    и гроздья; белое обходит их сверху и снизу, поэтому габаритная рамка шире
    просвета, и содержимое карты уезжает под резьбу. Меряется поэтому не
    крайний белый пиксель, а тот, дальше которого белое идёт почти во всех
    строках: перцентиль по строкам и столбцам. `margin` — ещё немного воздуха,
    потому что упереться текстом в резьбу тоже некрасиво.
    """
    h, w = hole.shape
    rows = np.where(hole.any(axis=1))[0]
    cols = np.where(hole.any(axis=0))[0]
    if not len(rows) or not len(cols):
        return None
    # Внутри окна, а не по всей раме: строки и столбцы берём только те, что
    # окно пересекают, иначе перцентиль считался бы по пустоте.
    lefts, rights = [], []
    for y in rows:
        xs = np.where(hole[y])[0]
        lefts.append(xs[0]); rights.append(xs[-1])
    tops, bottoms = [], []
    for x in cols:
        ys = np.where(hole[:, x])[0]
        tops.append(ys[0]); bottoms.append(ys[-1])
    left = float(np.percentile(lefts, 88))
    right = float(np.percentile(rights, 12))
    top = float(np.percentile(tops, 88))
    bottom = float(np.percentile(bottoms, 12))
    return (top / h * 100 + margin, (w - 1 - right) / w * 100 + margin,
            (h - 1 - bottom) / h * 100 + margin, left / w * 100 + margin)


def _runs(mask, axis):
    """Толщина полосы вдоль стороны: длина ПЕРВОГО непрерывного отрезка
    рисунка, считая внутрь.

    Не «от края холста»: у рамы с фигурным верхом до края доходят не все
    столбцы, и отрезок от края дал бы ноль на большинстве из них."""
    m = mask if axis == 0 else mask.T
    h, w = m.shape
    out = np.zeros(w, np.int32)
    for x in range(w):
        col = m[:, x]
        if not col.any():
            continue
        start = int(np.argmax(col))
        gap = np.argmin(col[start:])
        out[x] = (h - start) if col[start:].all() else int(gap)
    return out


def _reach(mask, axis, lo, hi, cap):
    """Докуда ВГЛУБЬ карты достаёт рисунок этой стороны.

    Коробка каймы НЕ обязана равняться толщине полосы — вот чего я сперва не
    увидел. Коробка может быть глубже, а рисунок в ней прозрачен всюду, где
    ничего не нарисовано. Тогда подвеска у притолоки и картуш посреди стороны
    помещаются целиком, вместо того чтобы срезаться по толщине планки.

    Мерится поэтому не полоса, а САМАЯ ГЛУБОКАЯ точка первого отрезка на
    участке между углами. `cap` — потолок: если какой-то столбец окажется
    сквозным, он утащил бы коробку на всю карту."""
    m = mask if axis == 0 else mask.T
    h = m.shape[0]
    deep = 0
    for x in range(int(lo), int(hi)):
        col = m[:, x]
        if not col.any():
            continue
        start = int(np.argmax(col))
        if col[start:].all():
            continue
        end = start + int(np.argmin(col[start:]))
        deep = max(deep, min(end, cap))
    return max(1, deep)


def cmd_cut(args) -> None:
    img = Image.open(args.file)
    img, mask, window = _cut_alpha(img)
    ys, xs = np.where(mask)
    if len(xs) == 0:
        raise SystemExit('пусто: фон снят целиком — рама, видимо, светлее порога')
    x0c, y0c = int(xs.min()), int(ys.min())
    img = img.crop((x0c, y0c, int(xs.max()) + 1, int(ys.max()) + 1))
    mask = mask[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
    H, W = mask.shape
    if window is not None:
        window = window[y0c:int(ys.max()) + 1, x0c:int(xs.max()) + 1]
    print(f'рама {W}×{H} (отношение {W / H:.3f}; у карты {ASPECT:.3f})')

    top, left = _runs(mask, 0), _runs(mask, 1)
    # Толщину берём НИЖНИМ перцентилем по середине стороны: медиану задрал бы
    # картуш, который у хорошей рамы как раз посередине и стоит.
    t_top = float(np.percentile(top[int(W * .20):int(W * .80)], 30))
    t_left = float(np.percentile(left[int(H * .20):int(H * .80)], 30))
    t_px = (t_top + t_left) / 2
    if t_px < 4:
        raise SystemExit('полоса не нашлась: рама без сплошного края?')

    def extent(run, t, span):
        """Докуда достаёт угловой убор: первое место, где полоса выходит на
        ровную толщину и держит её."""
        hold = max(4, int(span * .03))
        flat = run <= t * 1.18
        for i in range(int(span * .02), int(span * .45)):
            if flat[i:i + hold].all():
                return i
        return int(t * 1.6)

    S = max(extent(top, t_top, W), extent(left, t_left, H))
    # Бегущая рама или СОЧИНЁННАЯ: у бегущей угловой убор около одной толщины и
    # посреди стороны ничего не вспухает. Ниже этого порога 9-slice ещё
    # собирается, не раздувая коробок втрое.
    # Картуш посреди стороны выше полосы — в кайму он не влезет и обрежется.
    # Молчать об этом нельзя: срезанный купол виден на карте сразу.
    swell = float(np.percentile(top[int(W * .40):int(W * .60)], 80)) / t_top
    if swell > 1.25:
        print(f'  ⚠ посреди верхней стороны убор в {swell:.1f} толщины — в кайму он не'
              f' влезет и будет срезан.\n    Либо просите у модели РОВНУЮ кайму без'
              f' картуша, либо вырежьте картуш\n    отдельно и наденьте его в слот'
              f' sideMidH.')
    k = S / t_px
    composed = k > 2.0 or swell > 1.5
    print(f'полоса {t_px:.1f} px · угловой убор {S} px ({k:.2f} толщины)'
          + (' · рама СОЧИНЁННАЯ' if composed else ' · рама бегущая'))

    # Толщина в процентах карты, при которой растяжение кайм равно 1.0.
    g_u = TUCK / 100
    r_h = (W - 2 * S) / t_px
    t_pct = (1 + 2 * g_u) * 100 / (r_h + 2 * k)
    g = Geometry(t_pct)
    s_pct = t_pct * k
    r_v_src = (H - 2 * S) / t_px
    r_v_want = (1.4 - 2 * s_pct / 100 + 2 * g_u) / (t_pct / 100)
    skew = abs(r_v_src - r_v_want) / r_v_want * 100

    os.makedirs(args.out, exist_ok=True)
    px = args.pixels
    parts = {
        'corner': (img.crop((0, 0, S, S)), (px, px)),
        'edgeH': (img.crop((S, 0, W - S, int(round(t_top)))),
                  (int(round(px * r_h)), px)),
        'edgeV': (img.crop((0, S, int(round(t_left)), H - S)),
                  (px, int(round(px * r_v_src)))),
    }
    stem = os.path.splitext(os.path.basename(args.file))[0]
    for name, (crop, size) in parts.items():
        path = os.path.join(args.out, f'{stem}-{name}.png')
        crop.resize(size, Image.LANCZOS).save(path)
        print(f'  {name:>7}  {size[0]:>5}×{size[1]:<5}  {path}')

    print(f'\nВПИСАТЬ В КОНСТРУКТОР (толщина полосы {t_pct:.3f}% ширины карты):')
    print(f'  врезки     слева/справа {g.inset_x:.3f} % · сверху/снизу {g.inset_y:.3f} %')
    print(f'  угол       growX {s_pct - g.inset_x:+.3f} · growY '
          f'{(s_pct - g.inset_x) / H_OVER_W:+.3f}   (коробка под угловой убор)')
    print(f'  кайма H    growX {g.inset_x - s_pct + TUCK:+.3f}')
    print(f'  кайма V    growY {(g.inset_y - s_pct / H_OVER_W + TUCK / H_OVER_W):+.3f}')
    print(f'  слои       угол 6 · каймы 4')
    print(f'  режим      растянуть (stretch) — детали уже в нужном отношении')
    if skew > 3:
        print(f'\n  ⚠ по вертикали растяжение разойдётся на {skew:.1f}%: рама нарисована'
              f' в {W / H:.3f}, а карта {ASPECT:.3f}. Просите модель об --ar 5:7.')
    else:
        print(f'\n  по вертикали расхождение {skew:.1f}% — растяжение честное на обеих осях.')

    # ── Окно: то, что нужно режиму «целой картинкой» ──────────────────────
    #
    # Рама, у которой стороны — КОМПОЗИЦИЯ (колонны, медальоны, подвески), а не
    # бегущая кайма, в разрезе теряет рисунок: кайму растянет, а угловой убор
    # придётся раздувать коробкой втрое. Такую раму носят ЦЕЛИКОМ, и тогда от
    # неё нужно ровно одно число — где у неё дыра.
    ins = window_insets(window) if window is not None else None
    if ins:
        print('\nЛИБО ЦЕЛИКОМ (frameMode: overlay) — врезки по ПРОСВЕТУ окна:')
        print(f'  insetTop {ins[0]:.3f} · insetRight {ins[1]:.3f}'
              f' · insetBottom {ins[2]:.3f} · insetLeft {ins[3]:.3f}')
        print(f'  значки     costX {ins[3] + 7:.1f} costY {ins[0] + 5:.1f}'
              f' · powerX {100 - ins[1] - 7:.1f} powerY {100 - ins[2] - 5:.1f}')
        if composed:
            print('  ← этой раме советую именно так: стороны у неё композиция, а не')
            print('    бегущая кайма, и в разрезе рисунок разъедется.')
    else:
        print('\n  окно не нашлось — внутри рамы не сплошная светлая дыра?')



def cmd_slice(args) -> None:
    """Разрезать целую раму на ТРИ ДЕТАЛИ конструктора и посчитать всё, что
    надо вписать, чтобы они сошлись обратно.

    От `cut` отличается одним, но решающим: коробки деталей здесь не равны
    толщине полосы. Угловая коробка — по угловому убору, коробки кайм — по
    самой глубокой точке своей стороны. Поэтому медальон, подвеска и свисающий
    лист остаются целыми, а не срезаются по планке.

    Врезки при этом отвечают за ДРУГОЕ — за просвет, в котором стоит текст, — и
    это тоже вылезло не сразу: у богатой рамы просвет вдвое уже полосы вместе с
    убором, и одно число за двоих не отвечает. Детали ставятся от врезок
    доводками `grow`.
    """
    img = Image.open(args.file)
    img, mask, hole = _cut_alpha(img)
    ys, xs = np.where(mask)
    x0c, y0c, x1c, y1c = int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())
    img = img.crop((x0c, y0c, x1c + 1, y1c + 1))
    mask = mask[y0c:y1c + 1, x0c:x1c + 1]
    if hole is not None:
        hole = hole[y0c:y1c + 1, x0c:x1c + 1]
    H, W = mask.shape

    top, left = _runs(mask, 0), _runs(mask, 1)
    t_top = float(np.percentile(top[int(W * .20):int(W * .80)], 30))
    t_left = float(np.percentile(left[int(H * .20):int(H * .80)], 30))

    def extent(run, t, span):
        hold = max(4, int(span * .03))
        flat = run <= t * 1.18
        for i in range(int(span * .02), int(span * .45)):
            if flat[i:i + hold].all():
                return i
        return int(t * 1.6)

    S = max(extent(top, t_top, W), extent(left, t_left, H))
    deep_h = _reach(mask, 0, S, W - S, int(H * .42))
    deep_v = _reach(mask, 1, S, H - S, int(W * .42))
    print(f'рама {W}×{H} · угловой убор {S} px · верх уходит вглубь на {deep_h} px'
          f' · бок на {deep_v} px')

    # Одна мерка на все три детали: во сколько раз рисунок сядет на карту.
    # Берётся по горизонтальной кайме, и ею же меряются угол и бок — иначе у
    # деталей одной рамы оказалась бы разная толщина.
    g_u = TUCK / 100
    r_h = (W - 2 * S) / deep_h
    k = S / deep_h
    s_u = k * (1 + 2 * g_u) / (r_h + 2 * k)
    scale = s_u / S
    deep_h_u, deep_v_u = deep_h * scale, deep_v * scale
    run_v_src = (H - 2 * S) * scale
    run_v_want = 1.4 - 2 * s_u + 2 * g_u
    skew = abs(run_v_src - run_v_want) / run_v_want * 100

    ins = window_insets(hole) if hole is not None else None
    if not ins:
        raise SystemExit('окно не нашлось — врезки под текст считать не от чего')
    it, ir, ib, il = ins

    os.makedirs(args.out, exist_ok=True)
    stem = os.path.splitext(os.path.basename(args.file))[0]
    px = args.pixels
    cuts = {
        'corner': (img.crop((0, 0, S, S)), (px, px)),
        'sideH': (img.crop((S, 0, W - S, deep_h)),
                  (int(round(px * (W - 2 * S) / S)), int(round(px * deep_h / S)))),
        'sideV': (img.crop((0, S, deep_v, H - S)),
                  (int(round(px * deep_v / S)), int(round(px * (H - 2 * S) / S)))),
    }
    paths = {}
    for name, (crop, size) in cuts.items():
        paths[name] = os.path.join(args.out, f'{stem}-{name}.png')
        crop.resize(size, Image.LANCZOS).save(paths[name])
        print(f'  {name:>7}  {size[0]:>5}×{size[1]:<5}  {paths[name]}')

    sx, sy = s_u * 100, s_u * 100 / H_OVER_W          # угловая коробка, % ширины и высоты
    hy = deep_h_u * 100 / H_OVER_W                    # глубина верхней каймы, % высоты
    vx = deep_v_u * 100                               # глубина боковой каймы, % ширины
    print(f'\nВПИСАТЬ В КОНСТРУКТОР (режим sliced):')
    print(f'  врезки     top {it:.3f} · right {ir:.3f} · bottom {ib:.3f} · left {il:.3f}'
          f'   (просвет под текст)')
    print(f'  угол       growX {sx - il:+.3f} · growY {sy - it:+.3f}   слой 6')
    print(f'  кайма H    growX {il - sx + TUCK:+.3f} · growY {hy - it:+.3f}   слой 4')
    print(f'  кайма V    growX {vx - il:+.3f} · growY {it - sy + TUCK / H_OVER_W:+.3f}   слой 4')
    print(f'  fit stretch · turn mirror у всех трёх')
    print(f'\n  по вертикали расхождение {skew:.1f}%'
          + ('' if skew <= 3 else ' — рама нарисована не в 5 : 7'))

    if args.json:
        out = {
            'insetTop': it, 'insetRight': ir, 'insetBottom': ib, 'insetLeft': il,
            'corner': {'growX': sx - il, 'growY': sy - it},
            'sideH': {'growX': il - sx + TUCK, 'growY': hy - it},
            'sideV': {'growX': vx - il, 'growY': it - sy + TUCK / H_OVER_W},
            'files': paths, 'skew': skew,
        }
        with open(args.json, 'w') as fh:
            import json
            json.dump(out, fh, ensure_ascii=False, indent=1)
        print('  числа:', args.json)


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest='cmd', required=True)

    s = sub.add_parser('spec', help='точные форматы деталей')
    s.add_argument('-t', '--thickness', type=float, default=7.0, help='толщина полосы, %% ширины')
    s.add_argument('--pixels', type=int, default=256, help='толщина полосы в пикселях рисунка')
    s.set_defaults(func=cmd_spec)

    s = sub.add_parser('blanks', help='холсты-заготовки с направляющими')
    s.add_argument('-t', '--thickness', type=float, default=7.0)
    s.add_argument('--pixels', type=int, default=256)
    s.add_argument('--repeats', type=int, default=1, help='1 — растянуть, больше — повторить')
    s.add_argument('--out', default='tools/sheets/frames')
    s.set_defaults(func=cmd_blanks)

    s = sub.add_parser('check', help='приёмка готовой детали')
    s.add_argument('file')
    s.add_argument('--part', choices=('corner', 'edgeH', 'edgeV'), required=True)
    s.add_argument('-t', '--thickness', type=float, default=7.0)
    s.set_defaults(func=cmd_check)

    s = sub.add_parser('cut', help='разрезать целую раму на детали')
    s.add_argument('file')
    s.add_argument('--pixels', type=int, default=512, help='толщина полосы в пикселях')
    s.add_argument('--out', default='/tmp/frame-cut')
    s.set_defaults(func=cmd_cut)

    s = sub.add_parser('slice', help='разрезать раму на детали конструктора')
    s.add_argument('file')
    s.add_argument('--pixels', type=int, default=512, help='угловая коробка в пикселях')
    s.add_argument('--out', default='/tmp/frame-slice')
    s.add_argument('--json', default=None, help='куда сложить числа')
    s.set_defaults(func=cmd_slice)

    s = sub.add_parser('draw', help='рисованный комплект дома')
    s.add_argument('-t', '--thickness', type=float, default=5.2)
    s.add_argument('--pixels', type=int, default=160)
    s.add_argument('--palette', choices=tuple(PALETTES), default='gilt')
    s.add_argument('--motif', choices=tuple(MOTIFS), default='plait')
    s.add_argument('--boss', choices=('lozenge', 'disc', 'square'), default='lozenge')
    s.add_argument('--periods', type=int, default=17)
    s.add_argument('--out', default='/tmp/frame-kit')
    s.set_defaults(func=cmd_draw)

    args = p.parse_args()
    args.func(args)


if __name__ == '__main__':
    main()
