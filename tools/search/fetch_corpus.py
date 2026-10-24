#!/usr/bin/env python3
"""Снимок того, что ищет поиск в шапке: работы, небылицы, листы вестника, карты.

Читает публичные ручки (только GET) и пишет `corpus.json` рядом. Его читают
`examples/search_probe.rs` (замер ранжирования на сервере) и наборы вопросов
`cases.json`. Запуск:

    python3 tools/search/fetch_corpus.py [http://localhost:3000/api/v1]

Тексты берутся целиком: поиск по словам на сервере видит всё тело небылицы, и
со срезанным телом замер не видел бы совпадений, которые видит сервер.
"""
import json
import sys
import urllib.request

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:3000/api/v1"
LANGS = ["En", "Ru", "Uk", "Es", "De", "It"]
BODY_MAX = 100000


def get(path):
    with urllib.request.urlopen(BASE + path, timeout=30) as r:
        return json.load(r)


def langs(obj, stem):
    """`titleEn` … `titleIt` → {"en": …}; пустое не берётся."""
    out = {}
    for l in LANGS:
        v = obj.get(stem + l)
        if v:
            out[l.lower()] = v
    return out


def works():
    items, page = [], 1
    while True:
        d = get(f"/figurines?page={page}&perPage=60")
        items += d["items"]
        if len(items) >= d["total"] or not d["items"]:
            break
        page += 1
    out = []
    for w in items:
        tr = w.get("translations") or {}
        out.append({
            "id": w["id"],
            "kind": "work",
            "name": w["name"],
            "names": {k: v.get("name") for k, v in tr.items() if v and v.get("name")},
            "short": {"en": w.get("shortText")} | {k: v.get("shortText") for k, v in tr.items() if v and v.get("shortText")},
            "material": w.get("material"),
            "technique": w.get("technique"),
            "series": w.get("series"),
            "status": w.get("status"),
        })
    return out


def leaf(l, kind):
    return {
        "id": l["id"],
        "kind": kind,
        "title": langs(l, "title"),
        "dek": langs(l, "dek"),
        "body": {k: v[:BODY_MAX] for k, v in langs(l, "body").items()},
        "work": l.get("figurineName"),
        "works": list((l.get("figurineNameTranslations") or {}).values()),
    }


def tales():
    return [leaf(l, "tale") for l in get("/tales")]


def gazette():
    out, page = [], 1
    while True:
        d = get(f"/gazette?page={page}&perPage=60")
        out += [leaf(l, "leaf") for l in d["items"] if l.get("kind") != "tale"]
        if page * 60 >= d.get("total", 0) or not d["items"]:
            break
        page += 1
    return out


def cards():
    out = []
    for c in get("/battles/cards"):
        if c.get("kind") == "structure":
            continue
        out.append({
            "id": c["id"],
            "kind": "card",
            "title": langs(c, "title"),
            "race": langs(c, "raceName"),
            "effect": langs(c, "effect"),
            "lore": langs(c, "lore"),
            "work": c.get("figurineName"),
            "works": list((c.get("figurineNameTranslations") or {}).values()),
        })
    return out


corpus = {"works": works(), "tales": tales(), "gazette": gazette(), "cards": cards()}
with open(__file__.rsplit("/", 1)[0] + "/corpus.json", "w", encoding="utf-8") as f:
    json.dump(corpus, f, ensure_ascii=False, indent=1)
print({k: len(v) for k, v in corpus.items()})
