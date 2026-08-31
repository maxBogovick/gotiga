#!/usr/bin/env python3
"""Режет лист заготовок рамки на отдельные PNG с прозрачным фоном.

Нейросеть отдаёт запчасти рамки одним листом: уголки, планки, накладки,
подписанные буквами. Инструмент находит на листе отдельные предметы и
раскладывает каждый в свой файл, обрезанный по краю рисунка.

    python3 -m venv .venv-tools && .venv-tools/bin/pip install pillow numpy
    .venv-tools/bin/python tools/slice_sheet.py <лист.png> [ещё листы...]

Можно передать папку — разберёт всё, что в ней лежит, каждый лист в свою
папку `<имя>-parts` рядом с исходником. Имена по умолчанию — `01-part`,
`02-part`; свои задаются `--names h1,h2,h3,h4,v1,v2,v3,v4` по порядку
оставленных. `--webp` кладёт рядом ещё и WebP с альфой — тот же формат,
что пишет `POST /admin/battles/frames/art`.

Фон снимается двумя разными способами, и выбирает он сам:

  • если у листа есть настоящий альфа-канал — берётся он, как есть,
    вместе со всеми полупрозрачными краями;
  • если лист непрозрачный (белый фон или нарисованная «шахматка») —
    фоном считается всё светлое и несочное, ДОСВЯЗАННОЕ ДО КРАЯ ХОЛСТА.
    Связность важна: белый блик внутри самоцвета до края не достаёт,
    значит он часть рисунка, а не дырка.

Наивный путь (alpha = 1 − яркость) здесь не годится: он делает
полупрозрачным золото и блики на камнях, то есть ровно то, ради чего
рамку и рисовали.

Тот же разбор живёт в сервере — `src-tauri/server/src/sheet.rs`, откуда им
пользуется вкладка «Ассеты» в админке. Два воплощения одного алгоритма, и это
осознанная цена: здесь он работает по папке на диске, без сервера и без базы.
Числа в обоих местах одни и те же и меряны на одних листах; правка порогов,
сделанная только здесь, разъедется с тем, что увидит хранитель.

Что получится, видно по контактному листу `_contact.png`: на нём все
найденные предметы с номерами, и отброшенные тоже — перечёркнутые.
Номера не «съезжают» от настроек, поэтому по ним можно уточнять
разбор: `--only 1-8`, `--drop 12,13`, `--keep 22` (вернуть то, что
эвристика приняла за подпись).
"""

import argparse
import json
import os
import re
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont


# ── морфология на суммах по окну ───────────────────────────────────────
# Всё, что ниже, — одна и та же коробочная сумма: и размытие края, и
# сжатие маски, и разрастание. Отдельная библиотека ради этого не нужна.

def _win_sum(a, r, axis):
    n = a.shape[axis]
    pad = [(0, 0), (0, 0)]
    pad[axis] = (r, r)
    p = np.pad(a, pad)
    c = np.cumsum(p, axis=axis, dtype=np.float32)
    zeros = list(c.shape)
    zeros[axis] = 1
    c = np.concatenate([np.zeros(zeros, np.float32), c], axis=axis)
    hi = np.take(c, np.arange(2 * r + 1, 2 * r + 1 + n), axis=axis)
    lo = np.take(c, np.arange(0, n), axis=axis)
    return hi - lo


def box_sum(a, r):
    if r <= 0:
        return a.astype(np.float32)
    return _win_sum(_win_sum(a.astype(np.float32), r, 0), r, 1)


def dilate(mask, r):
    return box_sum(mask, r) > 0.5 if r > 0 else mask


def box_blur(a, r):
    if r <= 0:
        return a.astype(np.float32)
    return box_sum(a, r) / float((2 * r + 1) ** 2)


# ── связные области ────────────────────────────────────────────────────
# Разметка по строкам-отрезкам: в строке отрезков сотни, а не миллион
# пикселей, поэтому питоновский цикл здесь допустим. Связность — 8.

def connected_components(mask):
    h, w = mask.shape
    m = mask.astype(np.int8)
    parent = [0]

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[max(ra, rb)] = min(ra, rb)

    rows = []
    prev = None
    for y in range(h):
        d = np.diff(np.concatenate(([0], m[y], [0])))
        starts = np.flatnonzero(d == 1)
        ends = np.flatnonzero(d == -1)          # конец не включается
        lab = np.zeros(len(starts), dtype=np.int64)
        for i in range(len(starts)):
            s, e = starts[i], ends[i]
            cur = 0
            if prev is not None:
                ps, pe, pl = prev
                # +1 по краям — та самая диагональная связность
                for j in np.flatnonzero((ps < e + 1) & (pe > s - 1)):
                    if cur == 0:
                        cur = find(pl[j])
                    else:
                        union(cur, pl[j])
                        cur = find(cur)
            if cur == 0:
                parent.append(len(parent))
                cur = len(parent) - 1
            lab[i] = cur
        prev = (starts, ends, lab)
        rows.append((starts, ends, lab))

    remap = {}
    labels = np.zeros((h, w), dtype=np.int32)
    for y, (starts, ends, lab) in enumerate(rows):
        for i in range(len(starts)):
            root = find(lab[i])
            n = remap.get(root)
            if n is None:
                n = len(remap) + 1
                remap[root] = n
            labels[y, starts[i]:ends[i]] = n
    return labels, len(remap)


# ── фон ────────────────────────────────────────────────────────────────

def component_stats(labels, n, colorful):
    """Высота и доля «цветного» для каждой области — за один проход.

    Считать это в цикле по областям нельзя: каждая проверка `labels == g`
    прочитывает весь лист, и на сотне букв это уже минуты.
    """
    flat = labels.ravel()
    take = flat > 0
    lab = flat[take]
    order = np.argsort(lab, kind="stable")
    lab = lab[order]
    h, w = labels.shape
    yy = np.repeat(np.arange(h, dtype=np.int32), w)[take][order]
    col = colorful.ravel()[take][order].astype(np.float32)
    ids = np.arange(1, n + 1)
    starts = np.searchsorted(lab, ids, side="left")
    ends = np.searchsorted(lab, ids, side="right")
    heights = np.zeros(n + 1, np.int32)
    fracs = np.zeros(n + 1, np.float32)
    for i in range(n):
        a, b = starts[i], ends[i]
        if b > a:
            heights[i + 1] = yy[a:b].max() - yy[a:b].min() + 1
            fracs[i + 1] = col[a:b].mean()
    return heights, fracs


def mask_from_alpha(rgba, threshold):
    """Порог не нулевой намеренно.

    Вокруг рисунка остаётся широкое еле заметное свечение — проценты
    непрозрачности. По нему подпись под планкой оказывается связана с
    планкой, и тогда «H1» уезжает внутрь вырезки. Для 9-slice это
    свечение всё равно вредно: растянутое по стороне, оно видно швами.
    """
    return rgba[..., 3] > threshold


def mask_from_background(rgb, bg_value, bg_sat):
    """Фон — светлое и несочное, дотянувшееся до края холста."""
    f = rgb.astype(np.float32) / 255.0
    v = f.max(2)
    mn = f.min(2)
    s = np.where(v > 0, (v - mn) / np.maximum(v, 1e-6), 0.0)
    bgish = (v >= bg_value) & (s <= bg_sat)

    labels, n = connected_components(bgish)
    border = np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])
    outer = np.unique(border[border > 0])
    background = np.isin(labels, outer)
    return ~background


def bleed_colors(rgb, known, rounds):
    """Растягивает цвет наружу под прозрачный край.

    Без этого по контуру остаётся кайма из фона: браузер при
    сглаживании подмешивает цвет невидимых пикселей.
    """
    out = rgb.astype(np.float32).copy()
    k = known.astype(np.float32)
    for _ in range(rounds):
        if k.min() > 0.5:
            break
        cnt = box_sum(k, 1)
        acc = np.stack([box_sum(out[..., c] * k, 1) for c in range(3)], axis=2)
        fill = (cnt > 0) & (k < 0.5)
        avg = acc / np.maximum(cnt, 1e-6)[..., None]
        out[fill] = avg[fill]
        k = np.where(fill, 1.0, k)
    return np.clip(out, 0, 255).astype(np.uint8)


# ── разбор листа ───────────────────────────────────────────────────────

def read_ranges(spec):
    if not spec:
        return None
    out = set()
    for chunk in spec.replace(" ", "").split(","):
        if not chunk:
            continue
        if "-" in chunk[1:]:
            a, b = chunk.split("-", 1)
            out.update(range(int(a), int(b) + 1))
        else:
            out.add(int(chunk))
    return out


def reading_order(boxes):
    """Слева направо, сверху вниз — но строками, а не по одной координате.

    Предмет попадает в текущую строку, пока перекрывается с ней по
    вертикали хотя бы наполовину своей высоты.
    """
    order = sorted(range(len(boxes)), key=lambda i: (boxes[i][1], boxes[i][0]))
    result, row = [], []
    top = bottom = None
    for i in order:
        x0, y0, x1, y1 = boxes[i]
        # Половина от МЕНЬШЕЙ из двух высот: иначе одна длинная планка,
        # пересекающая лист сверху донизу, растаскивает соседей по строкам.
        if row and y0 >= bottom - min(y1 - y0, bottom - top) * 0.5:
            result.extend(sorted(row, key=lambda j: boxes[j][0]))
            row, top, bottom = [], None, None
        row.append(i)
        top = y0 if top is None else min(top, y0)
        bottom = y1 if bottom is None else max(bottom, y1)
    result.extend(sorted(row, key=lambda j: boxes[j][0]))
    return result


def slug(name):
    s = re.sub(r"[^a-zA-Z0-9]+", "-", name).strip("-").lower()
    return s or "sheet"


def forget_previous(out_dir):
    """Убирает за прошлым прогоном — но только то, что записал сам.

    Иначе повторный разбор с другими именами оставляет рядом два набора
    файлов, и непонятно, какой из них настоящий. Список берётся из
    прошлого parts.json: слепым `rm *.png` по чужой папке не ходим.
    """
    manifest = os.path.join(out_dir, "parts.json")
    if not os.path.exists(manifest):
        return
    try:
        with open(manifest) as fh:
            old = json.load(fh)
    except (ValueError, OSError):
        return
    for e in old.get("parts", []):
        name = e.get("file")
        if not name:
            continue
        for f in (name, os.path.splitext(name)[0] + ".webp"):
            f = os.path.join(out_dir, f)
            if os.path.exists(f):
                os.remove(f)
    contact = os.path.join(out_dir, "_contact.png")
    if os.path.exists(contact):
        os.remove(contact)


def slice_sheet(path, args):
    im = Image.open(path)
    rgba = np.asarray(im.convert("RGBA"))
    h, w = rgba.shape[:2]

    has_alpha = (rgba[..., 3] < 250).mean() > 0.01
    if has_alpha:
        mask = mask_from_alpha(rgba, args.alpha_threshold)
        source_alpha = rgba[..., 3]
        how = "альфа-канал листа"
    else:
        mask = mask_from_background(rgba[..., :3], args.bg_value, args.bg_sat)
        source_alpha = None
        how = "фон по светлоте (V≥%.2f, S≤%.2f)" % (args.bg_value, args.bg_sat)

    print("%s\n  %dx%d, %s" % (os.path.basename(path), w, h, how))

    f = rgba[..., :3].astype(np.float32) / 255.0
    value = f.max(2)
    lo = f.min(2)
    sat = np.where(value > 0, (value - lo) / np.maximum(value, 1e-6), 0.0)
    # Золото и самоцветы: сочное И СВЕТЛОЕ. Порог по светлоте здесь не
    # для красоты — тёмно-коричневая подпись тоже сочная, и её растушёвка
    # к белому фону проходит по одной насыщенности. Но растушёвка,
    # светлея, обесцвечивается: на V=0.6 у неё уже S≈0.2, а у золота 0.45.
    colorful = (sat > 0.35) & (value > 0.60)

    # Мелкую сыпь (шум JPEG, обрывки шахматки) убираем до всего прочего,
    # иначе она склеит соседние предметы при разрастании.
    labels, n = connected_components(mask)
    if n == 0:
        print("  ничего не найдено")
        return
    areas = np.bincount(labels.ravel(), minlength=n + 1)
    speck = np.flatnonzero(areas < args.min_area)
    if len(speck):
        mask &= ~np.isin(labels, speck[speck > 0])
        labels, n = connected_components(mask)
        if n == 0:
            print("  ничего не найдено")
            return

    # Подписи отделяются ДО склейки. Иначе буква, лежащая вплотную к
    # планке, разрастается вместе с ней и уезжает внутрь вырезки — а
    # выкинуть её потом уже нельзя, она стала частью предмета.
    #
    # Подпись — низкая и без цвета. Высота надёжнее яркости: буква
    # тонкая, и половина её пикселей — растушёвка к светлому фону. А вот
    # золота и самоцветов в букве нет.
    #
    # Умолчание в 30 px — замер, а не принцип: на здешних листах подписи
    # не выше 27, а самая тонкая настоящая планка — 33. Зазор узкий,
    # поэтому всё отброшенное остаётся на контактном листе, и ошибку
    # правит --keep, а не переделка правила.
    heights, color_frac = component_stats(labels, n, colorful)
    is_text_label = (heights <= args.text_max_h) & (color_frac < args.text_color)
    is_text_label[0] = False
    if args.keep_text:
        is_text_label[:] = False
    text_mask = is_text_label[labels]

    # Разрастание перед разметкой — то, что склеивает предмет обратно,
    # если он распался на части (шип отдельно от камня). Склейка по
    # форме, а не по рамкам: далёкие соседи не слипнутся. Буквы склеиваются
    # отдельно, между собой — чтобы слово попало на контактный лист целым.
    art_mask = mask & ~text_mask
    grouped, groups = connected_components(dilate(art_mask, args.merge_gap))
    grouped = np.where(art_mask, grouped, 0)
    if text_mask.any():
        g_text, n_text = connected_components(dilate(text_mask, args.merge_gap))
        g_text = np.where(text_mask, g_text, 0)
        grouped = grouped + np.where(g_text > 0, g_text + groups, 0)
        groups += n_text

    boxes, pix, text_group = [], [], []
    for g in range(1, groups + 1):
        ys, xs = np.nonzero(grouped == g)
        if len(xs) == 0:
            continue
        boxes.append((xs.min(), ys.min(), xs.max() + 1, ys.max() + 1))
        pix.append(g)
        text_group.append(bool(text_mask[ys[0], xs[0]]))

    order = reading_order(boxes)
    only = read_ranges(args.only)
    drop = read_ranges(args.drop) or set()
    keep = read_ranges(args.keep) or set()
    names = [s.strip() for s in args.names.split(",")] if args.names else None

    stem = slug(os.path.splitext(os.path.basename(path))[0])
    out_dir = args.out or os.path.join(os.path.dirname(os.path.abspath(path)), stem + "-parts")
    os.makedirs(out_dir, exist_ok=True)
    forget_previous(out_dir)

    entries = []
    for idx, gi in enumerate(order, start=1):
        x0, y0, x1, y1 = boxes[gi]
        sel = grouped == pix[gi]
        area = int(sel.sum())
        reason = "подпись" if text_group[gi] else None
        if only is not None and idx not in only:
            reason = "не в --only"
        if idx in drop:
            reason = "в --drop"
        if idx in keep:
            reason = None

        name = None
        if reason is None:
            kept = len([e for e in entries if e["kept"]])
            if names and kept < len(names):
                name = names[kept]
            name = "%02d-%s" % (idx, slug(name)) if name else "%02d-part" % idx

        entries.append({
            "index": idx, "name": name, "kept": reason is None, "reason": reason,
            "bbox": [int(x0), int(y0), int(x1 - x0), int(y1 - y0)], "area": area,
        })

        if reason is not None:
            continue

        px0, py0 = max(0, x0 - args.pad), max(0, y0 - args.pad)
        px1, py1 = min(w, x1 + args.pad), min(h, y1 + args.pad)
        crop_mask = sel[py0:py1, px0:px1]
        crop_rgb = rgba[py0:py1, px0:px1, :3]

        if source_alpha is not None:
            alpha = np.where(crop_mask, source_alpha[py0:py1, px0:px1], 0).astype(np.float32)
        else:
            alpha = box_blur(crop_mask.astype(np.float32), args.feather) * 255.0

        rgb = bleed_colors(crop_rgb, alpha > 32, args.bleed)
        out = np.dstack([rgb, np.clip(alpha, 0, 255).astype(np.uint8)])
        img = Image.fromarray(out, "RGBA")
        img.save(os.path.join(out_dir, name + ".png"))
        if args.webp:
            img.save(os.path.join(out_dir, name + ".webp"), quality=88, method=6)
        entries[-1]["file"] = name + ".png"

    kept = [e for e in entries if e["kept"]]
    print("  предметов: %d, вырезано %d" % (len(entries), len(kept)))
    for e in entries:
        if e["kept"]:
            print("    %2d  %-22s %4dx%-4d" % (e["index"], e["name"], e["bbox"][2], e["bbox"][3]))
        else:
            print("    %2d  — %s (%dx%d)" % (e["index"], e["reason"], e["bbox"][2], e["bbox"][3]))

    with open(os.path.join(out_dir, "parts.json"), "w") as fh:
        json.dump({"sheet": os.path.basename(path), "size": [w, h], "parts": entries}, fh,
                  ensure_ascii=False, indent=2)

    if not args.no_contact:
        contact_sheet(rgba, grouped, pix, entries, order,
                      os.path.join(out_dir, "_contact.png"))
    print("  → %s" % out_dir)


def contact_sheet(rgba, grouped, pix, entries, order, path, cell=200):
    """Все найденные предметы с номерами — и отброшенные тоже.

    Единственный способ проверить разбор — увидеть его целиком.
    """
    cols = min(6, max(1, len(entries)))
    rows = (len(entries) + cols - 1) // cols
    pad = 10
    sheet = Image.new("RGBA", (cols * (cell + pad) + pad, rows * (cell + pad + 18) + pad),
                      (32, 28, 24, 255))
    draw = ImageDraw.Draw(sheet)
    font = None
    for candidate in ("/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                      "/System/Library/Fonts/Geneva.ttf"):
        try:
            font = ImageFont.truetype(candidate, 13)   # кириллица в причинах
            break
        except OSError:
            continue
    if font is None:
        font = ImageFont.load_default()

    for slot, e in enumerate(entries):
        gi = order[slot]
        x0, y0, bw, bh = e["bbox"]
        sel = grouped[y0:y0 + bh, x0:x0 + bw] == pix[gi]
        rgb = rgba[y0:y0 + bh, x0:x0 + bw, :3]
        alpha = np.where(sel, 255, 0).astype(np.uint8)
        tile = Image.fromarray(np.dstack([rgb, alpha]), "RGBA")
        tile.thumbnail((cell, cell))
        cx = pad + (slot % cols) * (cell + pad)
        cy = pad + (slot // cols) * (cell + pad + 18)
        sheet.alpha_composite(tile, (cx + (cell - tile.width) // 2,
                                     cy + (cell - tile.height) // 2))
        label = "%d  %s" % (e["index"], e["name"] if e["kept"] else "× " + e["reason"])
        draw.text((cx, cy + cell + 2), label, font=font,
                  fill=(230, 220, 205, 255) if e["kept"] else (190, 95, 70, 255))
        if not e["kept"]:
            draw.line([cx, cy, cx + cell, cy + cell], fill=(190, 95, 70, 120), width=2)
    sheet.convert("RGB").save(path)


def main():
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("sheets", nargs="+", help="файлы листов или папка с ними")
    p.add_argument("--out", help="куда класть (по умолчанию <лист>-parts рядом с листом)")
    p.add_argument("--pad", type=int, default=2, help="поле вокруг обрезки, px")
    p.add_argument("--feather", type=int, default=1, help="сглаживание края, px (без альфы)")
    p.add_argument("--bleed", type=int, default=3, help="растяжка цвета под прозрачный край, px")
    p.add_argument("--merge-gap", type=int, default=4, help="через какой зазор склеивать куски, px")
    p.add_argument("--min-area", type=int, default=64, help="мельче этого — сыпь, а не предмет")
    p.add_argument("--alpha-threshold", type=int, default=24,
                   help="альфа выше — рисунок; ниже — остаточное свечение")
    p.add_argument("--bg-value", type=float, default=0.62, help="фон светлее этого (0..1)")
    p.add_argument("--bg-sat", type=float, default=0.20, help="фон бледнее этого (0..1)")
    p.add_argument("--text-max-h", type=int, default=30, help="подписи не выше этого, px")
    p.add_argument("--text-color", type=float, default=0.10,
                   help="доля золота и самоцветов, ниже которой это буква, а не вещь")
    p.add_argument("--keep-text", action="store_true", help="не отбрасывать подписи")
    p.add_argument("--only", help="оставить только эти номера: 1-8,12")
    p.add_argument("--drop", help="выбросить эти номера: 3,9")
    p.add_argument("--keep", help="вернуть эти номера, что бы про них ни решила эвристика")
    p.add_argument("--names", help="имена по порядку: a1,a4,b1,b2")
    p.add_argument("--webp", action="store_true", help="писать ещё и .webp с альфой")
    p.add_argument("--no-contact", action="store_true", help="без контактного листа")
    args = p.parse_args()

    paths = []
    for s in args.sheets:
        if os.path.isdir(s):
            paths += [os.path.join(s, f) for f in sorted(os.listdir(s))
                      if f.lower().endswith((".png", ".jpg", ".jpeg", ".webp"))]
        else:
            paths.append(s)
    if not paths:
        sys.exit("нечего резать")
    if args.out and len(paths) > 1:
        sys.exit("--out указан, а листов несколько: имена файлов перетрут друг друга")

    for path in paths:
        slice_sheet(path, args)


if __name__ == "__main__":
    main()
