#!/usr/bin/env python3
"""Контактные листы работ сайта для подбора похожих.

    .venv-img/bin/python tools/similar/contact_sheet.py СПИСОК.json ПАПКА [--focus слаг ...]

СПИСОК.json — ответ `GET https://ritunia.com/api/v1/figurines?visible=true`
(объект с `items` или сам список). Номер работы на листах — её место в списке:
по нему сверяют и выбирают.

В ПАПКУ ложатся:
  focus.jpg        работы из --focus крупно, со слагами (если названы);
  sheet1.jpg …     все работы по 21 на лист, с номерами;
  th/N.jpg         скачанные миниатюры.
В вывод печатается таблица «номер, слаг, название, раса материалов».

Миниатюры просятся с обычным User-Agent: прод отвечает 403 на «Python-urllib».
Ничего не пишет за пределы ПАПКИ и ничего не отправляет.
"""
import json
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image, ImageDraw

UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/150.0 Safari/537.36"
CELL = 210
COLS = 7
PER_SHEET = 21


def load(path):
    d = json.load(open(path))
    return d["items"] if isinstance(d, dict) else d


def fetch(url, dest):
    if dest.exists() and dest.stat().st_size > 0:
        return True
    try:
        req = urllib.request.Request(url, headers={"User-Agent": UA})
        with urllib.request.urlopen(req, timeout=25) as r:
            dest.write_bytes(r.read())
        return True
    except Exception as e:  # noqa: BLE001 — лист строится и без одной картинки
        print(f"  не скачалась {url}: {e}", file=sys.stderr)
        return False


def open_img(path):
    try:
        return Image.open(path).convert("RGB")
    except Exception:  # noqa: BLE001
        return Image.new("RGB", (200, 200), "gray")


def sheet(imgs, idx, out):
    rows = (len(idx) + COLS - 1) // COLS
    s = Image.new("RGB", (COLS * CELL, rows * CELL), "white")
    d = ImageDraw.Draw(s)
    for k, n in enumerate(idx):
        im = imgs[n].copy()
        im.thumbnail((CELL - 6, CELL - 6))
        x, y = (k % COLS) * CELL + 3, (k // COLS) * CELL + 3
        s.paste(im, (x, y))
        d.rectangle([x, y, x + 34, y + 20], fill="black")
        d.text((x + 4, y + 4), str(n), fill="white")
    s.save(out, quality=82)


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        print(__doc__)
        sys.exit(2)
    items = load(args[0])
    out = Path(args[1])
    focus = []
    if "--focus" in args:
        focus = args[args.index("--focus") + 1:]
    (out / "th").mkdir(parents=True, exist_ok=True)

    with ThreadPoolExecutor(8) as pool:
        list(pool.map(
            lambda p: fetch(p[1].get("faceImageUrl") or "", out / "th" / f"{p[0]}.jpg"),
            enumerate(items),
        ))
    imgs = [open_img(out / "th" / f"{n}.jpg") for n in range(len(items))]

    for k in range(0, len(items), PER_SHEET):
        sheet(imgs, range(k, min(k + PER_SHEET, len(items))), out / f"sheet{k // PER_SHEET + 1}.jpg")

    if focus:
        by_slug = {i.get("slug"): n for n, i in enumerate(items)}
        missing = [s for s in focus if s not in by_slug]
        if missing:
            print("Нет в списке (скрыта или слаг с опечаткой):", ", ".join(missing), file=sys.stderr)
        ids = [by_slug[s] for s in focus if s in by_slug]
        w = 420
        cols = min(3, max(1, len(ids)))
        rows = (len(ids) + cols - 1) // cols
        f = Image.new("RGB", (cols * w, rows * (w + 24)), "white")
        d = ImageDraw.Draw(f)
        for k, n in enumerate(ids):
            im = imgs[n].copy()
            im.thumbnail((w - 6, w - 6))
            x, y = (k % cols) * w + 3, (k // cols) * (w + 24) + 3
            f.paste(im, (x, y))
            d.text((x + 4, y + w - 4), f"{n}  {items[n].get('slug')}", fill="black")
        f.save(out / "focus.jpg", quality=85)

    for n, i in enumerate(items):
        print(f"{n}\t{i.get('slug')}\t{i.get('name', '').strip()}\t{i.get('material') or ''}\t{i.get('status')}")


if __name__ == "__main__":
    main()
