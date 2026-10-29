#!/usr/bin/env python3
"""Собирает локальный индекс по смыслу: тексты (работы, байки, вестник, карты) и фото работ.

    .venv-tools/embed/bin/python tools/embed/build.py                  # с локального сервера
    .venv-tools/embed/bin/python tools/embed/build.py --base https://ritunia.com/api/v1
    ... build.py --no-images          только тексты
    ... build.py --rebuild            посчитать всё заново (иначе берётся то, что не менялось)

Результат — `tools/embed/index/` (в git не идёт): векторы `.npy`, строки `.json`,
`meta.json` с названиями моделей. Читает только публичные ручки (GET). Повторный
запуск считает вектора только для новых и изменённых кусков и фото.
"""
import argparse
import hashlib
import json
import sys
import time
import urllib.request
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).parent))
from common import all_chunks  # noqa: E402
from models import ImageModel, TextModel  # noqa: E402

HERE = Path(__file__).parent
LANGS = ["en", "ru", "uk", "es", "de", "it"]


def args():
    p = argparse.ArgumentParser()
    p.add_argument("--base", default="http://localhost:3000/api/v1")
    p.add_argument("--text-model", default="BAAI/bge-m3")
    p.add_argument("--image-model", default="google/siglip2-so400m-patch14-384")
    p.add_argument("--out", default=str(HERE / "index"))
    p.add_argument("--no-images", action="store_true")
    p.add_argument("--rebuild", action="store_true")
    return p.parse_args()


A = args()
OUT = Path(A.out)
OUT.mkdir(parents=True, exist_ok=True)
(OUT / "photos").mkdir(exist_ok=True)


def get(path):
    with urllib.request.urlopen(A.base + path, timeout=60) as r:
        return json.load(r)


def langs(obj, stem):
    out = {}
    for l in LANGS:
        v = obj.get(stem + l.capitalize())
        if v:
            out[l] = v
    return out


def fetch_corpus():
    """Всё, что видит посетитель. Работы — с полным описанием и переводами."""
    works, page = [], 1
    while True:
        d = get(f"/figurines?page={page}&perPage=60")
        works += d["items"]
        if len(works) >= d["total"] or not d["items"]:
            break
        page += 1
    out_works = []
    for i, w in enumerate(works):
        d = get(f"/figurines/{w['id']}")
        tr = d.get("translations") or {}
        names = {l: v["name"] for l, v in tr.items() if v and v.get("name")}
        shorts = {"en": d.get("shortText")} | {l: v["shortText"] for l, v in tr.items() if v and v.get("shortText")}
        full = {"en": d.get("fullDescription")} | {l: v["fullDescription"] for l, v in tr.items() if v and v.get("fullDescription")}
        out_works.append({
            "id": d["id"], "kind": "work", "name": d["name"], "slug": d.get("slug"),
            "names": names, "short": shorts, "full": {l: t for l, t in full.items() if t},
            "material": d.get("material"), "technique": d.get("technique"), "series": d.get("series"),
            "status": d.get("status"),
            "images": [{"id": im["id"], "url": im["url"], "type": im["imageType"]} for im in d.get("images") or []],
        })
        print(f"\rработы: {i+1}/{len(works)}", end="", flush=True)
    print()

    def leaf(l, kind):
        return {"id": l["id"], "kind": kind, "slug": l.get("slug"), "title": langs(l, "title"),
                "dek": langs(l, "dek"), "body": langs(l, "body")}

    tales = [leaf(l, "tale") for l in get("/tales")]
    gaz, page = [], 1
    while True:
        d = get(f"/gazette?page={page}&perPage=60")
        gaz += [leaf(l, "leaf") for l in d["items"] if l.get("kind") != "tale"]
        if page * 60 >= d.get("total", 0) or not d["items"]:
            break
        page += 1
    cards = []
    for c in get("/battles/cards"):
        if c.get("kind") == "structure":
            continue
        cards.append({"id": c["id"], "kind": "card", "title": langs(c, "title"), "race": langs(c, "raceName"),
                      "effect": langs(c, "effect"), "lore": langs(c, "lore"), "slug": c.get("slug")})
    return {"works": out_works, "tales": tales, "gazette": gaz, "cards": cards}


def h(*parts):
    return hashlib.sha1("\x00".join(parts).encode()).hexdigest()


def load_prev(name):
    """Прежний индекс: {хеш → вектор}, если он посчитан теми же моделями."""
    meta = OUT / "meta.json"
    if A.rebuild or not meta.exists():
        return {}
    m = json.load(open(meta))
    if m.get(name + "_model") != getattr(A, name + "_model"):
        return {}
    try:
        vecs = np.load(OUT / f"{name}.npy")
        rows = json.load(open(OUT / f"{name}.json", encoding="utf-8"))
    except FileNotFoundError:
        return {}
    return {r["hash"]: v for r, v in zip(rows, vecs)}


def title_of(item):
    if item["kind"] == "work":
        return {"en": item["name"]} | item["names"]
    return item.get("title") or {}


def main():
    t0 = time.time()
    corpus = fetch_corpus()
    print({k: len(v) for k, v in corpus.items()}, f"({time.time()-t0:.0f} с)")

    items = {}
    for group in corpus.values():
        for it in group:
            items[it["id"]] = {"kind": it["kind"], "slug": it.get("slug"), "title": title_of(it),
                               "status": it.get("status")}
    json.dump(items, open(OUT / "items.json", "w", encoding="utf-8"), ensure_ascii=False)

    # ---- текст
    rows = all_chunks(corpus)
    for r in rows:
        r["hash"] = h(A.text_model, r["lang"], r["text"])
    prev = load_prev("text")
    todo = [i for i, r in enumerate(rows) if r["hash"] not in prev]
    print(f"текст: {len(rows)} кусков, считать {len(todo)}")
    vecs = np.zeros((len(rows), 0), dtype=np.float16)
    if rows:
        new = {}
        if todo:
            tm = TextModel(A.text_model)
            enc = tm.encode_docs([rows[i]["text"] for i in todo])
            new = {rows[i]["hash"]: enc[k] for k, i in enumerate(todo)}
            del tm
        vecs = np.stack([prev.get(r["hash"]) if r["hash"] in prev else new[r["hash"]] for r in rows]).astype(np.float16)
    np.save(OUT / "text.npy", vecs)
    json.dump(rows, open(OUT / "text.json", "w", encoding="utf-8"), ensure_ascii=False)
    meta = {"text_model": A.text_model, "image_model": None, "base": A.base,
            "built": time.strftime("%Y-%m-%d %H:%M:%S")}

    # ---- картинки
    if not A.no_images:
        photos = []
        for w in corpus["works"]:
            for im in w["images"]:
                photos.append({"work": w["id"], "image": im["id"], "type": im["type"], "url": im["url"]})
        for p in photos:
            p["hash"] = h(A.image_model, p["url"])
        prev = load_prev("image")
        todo = [p for p in photos if p["hash"] not in prev]
        print(f"фото: {len(photos)}, считать {len(todo)}")
        new = {}
        if todo:
            im_model = ImageModel(A.image_model)
            paths, got = [], []
            for k, p in enumerate(todo):
                f = OUT / "photos" / (p["image"] + ".jpg")
                if not f.exists():
                    try:
                        req = urllib.request.Request(p["url"], headers={"User-Agent": "gotiga-embed/1"})
                        with urllib.request.urlopen(req, timeout=60) as r:
                            f.write_bytes(r.read())
                    except Exception as e:  # одна недоступная картинка не должна ронять сборку
                        print(f"\n  пропущено {p['url']}: {e}")
                        continue
                paths.append(f)
                got.append(p)
                if k % 50 == 0:
                    print(f"\rзагрузка {k+1}/{len(todo)}", end="", flush=True)
            print()
            todo = got
            enc = im_model.encode_images(paths)
            new = {p["hash"]: enc[k] for k, p in enumerate(todo)}
            del im_model
        photos = [p for p in photos if p["hash"] in prev or p["hash"] in new]
        ivecs = np.stack([prev[p["hash"]] if p["hash"] in prev else new[p["hash"]] for p in photos]).astype(np.float16)
        np.save(OUT / "image.npy", ivecs)
        json.dump(photos, open(OUT / "image.json", "w", encoding="utf-8"), ensure_ascii=False)
        meta["image_model"] = A.image_model
    json.dump(meta, open(OUT / "meta.json", "w"), ensure_ascii=False, indent=1)
    print(f"готово за {time.time()-t0:.0f} с → {OUT}")


main()
