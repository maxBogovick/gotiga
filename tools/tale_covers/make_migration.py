#!/usr/bin/env python3
"""SQL с обложками книг и надписями на них.

    python3 -I tools/tale_covers/make_migration.py ПРОД.json [--out файл.sql]

ПРОД.json — ответ `GET /api/v1/tales` с рабочего сервера: из него берутся
названия (на длинном названии кегль надписи меньше). Печатает SQL в `--out`
или в stdout. В `src-tauri/server/migrations/` кладётся отдельным шагом, после
просмотра на локальной базе: применённые миграции не правят.

Обложка ставится только там, где её ещё нет: загруженная хозяином вручную
остаётся. Сами картинки лежат в `src-tauri/server/assets/covers/` и
кладутся на склад при запуске (`tale_covers.rs`).
"""
import json
import math
import sys
from pathlib import Path

# слаги — те же, что в prepare.py
SLUGS = [
    "molodost-bystree-tvoih",
    "ubit-a-potom-razorvat",
    "vse-kak-u-nas",
    "the-forester-and-his-unforgettable-memory",
    "gb",
    "mir-ne-takoy-kak-tebe",
    "otmeryalschik-ot-boga",
    "ispolnyu-vse-zhelaniya",
    "ya-veryu",
    "ne-odinoko-grustno",
    "lechu",
]

# Вид «Б1»: кремовые буквы с тенью прямо на картинке, без таблички.
TITLE_INK = "#f6ead0"
AUTHOR_INK = "#ecd9a8"
TITLE_WIDTH = 84.0   # % ширины обложки
CHAR_EM = 0.54       # ширина средней буквы Cormorant с запасом на перенос слов, в кегле
AUTHOR_Y = 94.5      # середина имени автора, % высоты
TITLE_BOTTOM = 84.7  # нижний край названия, % высоты
RATIO = 1.5          # высота обложки / ширина
NO_PLATE = {"on": False, "color": "#f3ead9", "opacity": 0.55, "blur": 1.5, "radius": 1.5, "pad": 3.0}


def wrap_lines(text: str, size: float) -> int:
    """Сколько строк займёт название при кегле: слова целиком, ширина буквы — `CHAR_EM`."""
    lines, used = 1, 0.0
    for word in text.split():
        w = len(word) * CHAR_EM * size
        gap = CHAR_EM * size * 0.5
        if used and used + gap + w > TITLE_WIDTH:
            lines, used = lines + 1, w
        else:
            used += (gap if used else 0) + w
    return lines


def title_label(titles: list[str]) -> dict:
    """Кегль по самому длинному названию из всех языков: две строки, а если не выходит — три."""
    # Крупно — в две строки; длинное название лучше в три строки покрупнее,
    # чем в две мелкие: мелкий текст на картинке не читается.
    chosen = None
    for size, most in ((10.0, 2), (9.0, 2), (8.0, 2), (8.0, 3), (7.2, 3), (6.4, 3), (5.6, 4)):
        lines = max(wrap_lines(t, size) for t in titles)
        chosen = (size, lines)
        if lines <= most:
            break
    size, lines = chosen
    height = lines * size * 1.08  # в долях ширины
    y = TITLE_BOTTOM - height / RATIO / 2
    return {
        "shown": True, "text": "", "x": 50, "y": round(y, 1), "width": TITLE_WIDTH,
        "font": "cormorant", "size": size, "weight": 700, "lineHeight": 1.08,
        "spacing": 0.01, "italic": False, "shadow": 1, "align": "center",
        "color": TITLE_INK, "plate": NO_PLATE,
    }


def author_label() -> dict:
    return {
        "shown": True, "text": "", "x": 50, "y": AUTHOR_Y, "width": 60,
        "font": "cormorant", "size": 5.6, "weight": 600, "lineHeight": 1.15,
        "spacing": 0.1, "italic": True, "shadow": 1, "align": "center",
        "color": AUTHOR_INK, "plate": NO_PLATE,
    }


def sql_str(s: str) -> str:
    return "'" + s.replace("'", "''") + "'"


def main() -> None:
    args = sys.argv[1:]
    out = Path(args[args.index("--out") + 1]) if "--out" in args else None
    tales = {t["slug"]: t for t in json.load(open(args[0]))}
    lines = [
        "-- Обложки книг небылиц и надписи на них (название и имя автора).",
        "-- Картинки лежат в assets/covers/ и кладутся на склад при запуске.",
        "-- Обложка ставится только там, где её ещё нет: загруженная вручную остаётся.",
        "-- Пустой текст надписи: название — байки на языке читателя, автор — из листа.",
        "",
    ]
    for slug in SLUGS:
        tale = tales[slug]
        titles = [tale[k] for k in ("titleEn", "titleRu", "titleUk") if tale.get(k)]
        text = {"title": title_label(titles), "author": author_label()}
        lines += [
            f"UPDATE gazette_leaves",
            f"   SET book_cover = {sql_str(f'/static/covers/book-{slug}.webp')},",
            f"       book_cover_text = {sql_str(json.dumps(text, ensure_ascii=False, separators=(',', ':')))}::jsonb",
            f" WHERE slug = {sql_str(slug)} AND kind = 'tale' AND book_cover IS NULL;",
            "",
        ]
    sql = "\n".join(lines)
    if out:
        out.write_text(sql)
        print(f"{out}: {len(SLUGS)} обложек")
    else:
        print(sql)


if __name__ == "__main__":
    main()
