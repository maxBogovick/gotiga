#!/usr/bin/env python3
"""Готовые поисковые движки на тех же данных и фразах, что и поиск сайта.

Записи собираются так же, как в `examples/search_probe.rs` (`docs()`): название на всех
языках плюс названия работ — в поле name, описания и тексты на всех языках — в body.
Дальше те же `cases.json` и то же деление на «подбор» (чётные фразы) и «проверку»
(нечётные), что у пробы, поэтому числа сравнимы с её строками «подбор» и «проверка».

    docker run -d --name gotiga-trial-meili -p 127.0.0.1:7700:7700 \
        -e MEILI_ENV=development getmeili/meilisearch:latest
    docker run -d --name gotiga-trial-manticore -p 127.0.0.1:9308:9308 manticoresearch/manticore:latest

    python3 tools/search/try_engines.py meili
    python3 tools/search/try_engines.py manticore --variant stem
    SEARCH_DIR=/путь/к/снимку python3 tools/search/try_engines.py meili

Без вектора: оба движка ищут только по словам, поэтому сравнивать надо со словами
самого сайта (`search_probe --no-vector`), а не с гибридом.
"""
import argparse
import json
import os
import re
import statistics
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).parent
DIR = Path(os.environ.get("SEARCH_DIR", HERE))
JUNK = ["pizza recipe", "bitcoin price", "football score", "dragon", "weather tomorrow", "how to fix a car",
        "asdf qwer zxcv", "курс доллара", "ракетный двигатель", "президент франции", "футбол",
        "квантовая физика формулы", "best laptop 2026", "space rocket", "stock market"]
PROBES = ["weapon", "weapons", "оружие", "axe", "зеленая", "зелёный", "бабушка", "красная шапочка", "ведьмы", "gnomes with axes"]


def vals(o):
    return [v for v in (o or {}).values() if isinstance(v, str) and v]


def first(o, langs):
    for l in langs:
        v = (o or {}).get(l)
        if v:
            return v
    return ""


def cut(t, n):
    return (t or "")[:n]


def build_docs(corpus):
    """Те же записи, что `docs()` в search_probe.rs."""
    out = []
    for w in corpus["works"]:
        name = w.get("name") or ""
        craft = " ".join([w.get("material") or "", w.get("technique") or "", w.get("series") or ""])
        en = first(w.get("short"), ["en", "ru"])
        out.append({"uid": w["id"], "kind": "work", "label": name,
                    "passage": f"{name}. {cut(en, 300)}. {craft.strip()}",
                    "name": " ".join([name] + vals(w.get("names"))),
                    "body": " ".join(vals(w.get("short")) + [craft])})
    for key, kind in (("tales", "tale"), ("gazette", "leaf")):
        for l in corpus[key]:
            works = [l.get("work") or ""] + [x for x in (l.get("works") or []) if isinstance(x, str)]
            en_title = first(l.get("title"), ["en", "ru"])
            out.append({"uid": l["id"], "kind": kind, "label": en_title,
                        "passage": f"{en_title}. {first(l.get('dek'), ['en', 'ru'])}. {cut(first(l.get('body'), ['en', 'ru']), 500)}",
                        "name": " ".join(vals(l.get("title")) + works),
                        "body": " ".join(vals(l.get("dek")) + vals(l.get("body")))})
    for c in corpus["cards"]:
        works = [c.get("work") or ""] + [x for x in (c.get("works") or []) if isinstance(x, str)]
        en_title = first(c.get("title"), ["en", "ru"])
        race = first(c.get("race"), ["en", "ru"])
        out.append({"uid": c["id"], "kind": "card", "label": en_title,
                    "passage": f"{en_title} — {race}. {c.get('work') or ''}. {cut(first(c.get('effect'), ['en', 'ru']), 200)}. {cut(first(c.get('lore'), ['en', 'ru']), 200)}",
                    "name": " ".join(vals(c.get("title")) + works),
                    "body": " ".join(vals(c.get("race")) + vals(c.get("effect")) + vals(c.get("lore")))})
    return out


def http(method, url, body=None, form=None, timeout=60):
    data = None
    headers = {}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    if form is not None:
        data = form.encode()
    req = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            raw = r.read()
    except urllib.error.HTTPError as e:
        raise SystemExit(f"{url}: {e.code} {e.read().decode(errors='replace')[:400]}")
    return json.loads(raw) if raw else {}


# ---------------------------------------------------------------- Meilisearch
class Meili:
    base = "http://127.0.0.1:7700"

    def __init__(self, variant):
        self.variant = variant

    def wait(self, task, tolerate=False):
        uid = task["taskUid"]
        while True:
            t = http("GET", f"{self.base}/tasks/{uid}")
            if t["status"] in ("succeeded", "failed", "canceled"):
                if t["status"] != "succeeded" and not tolerate:
                    raise SystemExit(f"Meilisearch: {t}")
                return
            time.sleep(0.2)

    def load(self, docs):
        self.wait(http("DELETE", f"{self.base}/indexes/docs"), tolerate=True)  # нет индекса — не ошибка
        self.wait(http("POST", f"{self.base}/indexes", {"uid": "docs", "primaryKey": "uid"}))
        settings = {"searchableAttributes": ["name", "body"], "displayedAttributes": ["uid", "label"]}
        if "nofuzzy" in self.variant:
            settings["typoTolerance"] = {"enabled": False}
        if "synonyms" in self.variant:
            settings["synonyms"] = {
                "weapon": ["axe", "sword", "spear", "dagger", "топор", "меч", "оружие"],
                "weapons": ["axe", "sword", "топор", "меч", "оружие"],
                "оружие": ["топор", "меч", "axe", "sword", "weapon", "hacha", "axt"],
            }
        self.wait(http("PATCH", f"{self.base}/indexes/docs/settings", settings))
        self.wait(http("POST", f"{self.base}/indexes/docs/documents", docs))

    def search(self, q, limit=40):
        body = {"q": q, "limit": limit, "showRankingScore": True, "attributesToRetrieve": ["uid", "label"]}
        if "freq" in self.variant:
            body["matchingStrategy"] = "frequency"  # при нехватке совпадений сначала отбрасывает самые частые слова
        r = http("POST", f"{self.base}/indexes/docs/search", body)
        return [(h["uid"], h.get("_rankingScore", 0.0), h.get("label", "")) for h in r["hits"]]


# ---------------------------------------------------------------- Manticore
class Manticore:
    base = "http://127.0.0.1:9308"

    def __init__(self, variant):
        self.variant = variant
        self.ids = {}

    def sql(self, query):
        return http("POST", f"{self.base}/sql?mode=raw", form="query=" + urllib.request.quote(query, safe=""))

    def load(self, docs):
        self.sql("DROP TABLE IF EXISTS docs")
        opts = ""
        if "stem" in self.variant:
            opts += " morphology='libstemmer_en,libstemmer_ru,libstemmer_de,libstemmer_es,libstemmer_it'"
        if "infix" in self.variant:
            opts += " min_infix_len='3'"
        r = self.sql(f"CREATE TABLE docs (name text, body text, uid string attribute, label string attribute){opts}")
        if r[0].get("error"):
            raise SystemExit(f"Manticore: {r[0]['error']}")
        lines = []
        for i, d in enumerate(docs, 1):
            self.ids[i] = d
            lines.append(json.dumps({"insert": {"table": "docs", "id": i, "doc": {
                "name": d["name"], "body": d["body"], "uid": d["uid"], "label": d["label"]}}}))
        req = urllib.request.Request(f"{self.base}/bulk", data=("\n".join(lines) + "\n").encode(), method="POST",
                                     headers={"Content-Type": "application/x-ndjson"})
        res = json.load(urllib.request.urlopen(req, timeout=120))
        if res.get("errors"):
            raise SystemExit(f"Manticore bulk: {str(res)[:300]}")

    def search(self, q, limit=40):
        words = re.findall(r"\w+", q.lower().replace("ё", "е"))
        if not words:
            return []
        query = " | ".join(words)  # любое из слов; ранжирует BM25
        option = "ranker=bm25, field_weights=(name=4, body=1)"
        if "fuzzy" in self.variant:
            option += ", fuzzy=1, distance=2"
            query = " ".join(words)
        sql = f"SELECT uid, label, weight() AS w FROM docs WHERE MATCH('{query.replace(chr(39), ' ')}') LIMIT {limit} OPTION {option}"
        r = self.sql(sql)[0]
        if r.get("error"):
            raise SystemExit(f"Manticore: {r['error']}\n{sql}")
        return [(row["uid"], float(row["w"]), row["label"]) for row in r.get("data", [])]


# ---------------------------------------------------------------- вектор и гибрид
class Hybrid:
    """Движок (слова) + вектор e5-small, как на сайте: два списка по 24 вещи, слияние RRF с k=60."""

    def __init__(self, engine, docs):
        import numpy as np
        from sentence_transformers import SentenceTransformer
        self.np, self.engine, self.docs = np, engine, docs
        self.model = SentenceTransformer("intfloat/multilingual-e5-small", device="mps")
        self.model.max_seq_length = 256
        self.vecs = self.model.encode(["passage: " + d["passage"] for d in docs], normalize_embeddings=True, batch_size=16)

    def search(self, q, limit=40):
        np = self.np
        qv = self.model.encode(["query: " + q], normalize_embeddings=True)[0]
        order = np.argsort(-(self.vecs @ qv))[:24]
        vec_list = [self.docs[i]["uid"] for i in order]
        words = self.engine.search(q, 24)
        score = {}
        for lst in (vec_list, [h[0] for h in words]):
            for rank, uid in enumerate(lst):
                score[uid] = score.get(uid, 0.0) + 1.0 / (60 + rank + 1)
        label = {d["uid"]: d["label"] for d in self.docs}
        ranked = sorted(score.items(), key=lambda x: -x[1])[:limit]
        return [(uid, sc, label[uid]) for uid, sc in ranked]


# ---------------------------------------------------------------- замер
def rank_of(hits, expect):
    for i, (uid, _, _) in enumerate(hits):
        if any(uid.startswith(e) for e in expect):
            return i
    return None


class Tally:
    def __init__(self):
        self.n = self.at1 = self.at3 = self.at6 = 0
        self.rr = 0.0

    def add(self, r):
        self.n += 1
        if r is not None:
            self.at1 += r == 0
            self.at3 += r < 3
            self.at6 += r < 6
            self.rr += 1 / (r + 1)

    def line(self, name):
        p = lambda x: 100 * x / max(self.n, 1)
        return f"{name:<12} {self.n:>3}  первая {p(self.at1):5.1f}%  в трёх {p(self.at3):5.1f}%  в шести {p(self.at6):5.1f}%  MRR {self.rr / max(self.n, 1):.3f}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("engine", choices=["meili", "manticore"])
    ap.add_argument("--variant", default="")
    ap.add_argument("-v", action="store_true", help="печатать фразы, где верная вещь не первая")
    ap.add_argument("--hybrid", action="store_true", help="добавить вектор e5 (нужен .venv-tools/embed)")
    a = ap.parse_args()

    corpus = json.load(open(DIR / "corpus.json", encoding="utf-8"))
    cases = json.load(open(DIR / "cases.json", encoding="utf-8"))
    docs = build_docs(corpus)
    eng = (Meili if a.engine == "meili" else Manticore)(a.variant)
    eng.load(docs)
    if a.hybrid:
        eng = Hybrid(eng, docs)
    print(f"{a.engine} [{a.variant or 'по умолчанию'}{', + вектор' if a.hybrid else ''}]: {len(docs)} записей")

    tune, check, by_tag, times = Tally(), Tally(), {}, []
    top_scores = []
    for i, c in enumerate(cases):
        if not c["expect"]:
            continue
        t = time.time()
        hits = eng.search(c["q"])
        times.append((time.time() - t) * 1000)
        r = rank_of(hits, c["expect"])
        (tune if i % 2 == 0 else check).add(r)
        by_tag.setdefault(c["tag"], Tally()).add(r)
        if hits:
            top_scores.append(hits[0][1])
        if a.v and (r is None or r >= 1):
            print(f"  — {c['q']!r}: верная на месте {None if r is None else r + 1}; первая: {hits[0][2] if hits else '—'}")
    print(tune.line("подбор"))
    print(check.line("проверка"))
    for tag in sorted(by_tag):
        print(by_tag[tag].line(tag))
    print(f"время запроса: {statistics.mean(times):.1f} мс в среднем, {max(times):.0f} мс наибольшее")

    neg = [c["q"] for c in cases if not c["expect"]] + JUNK
    shown = []
    junk_top = []
    for q in neg:
        h = eng.search(q)
        shown.append((q, len(h)))
        junk_top.append(h[0][1] if h else 0.0)
    print("мусорные запросы: сколько вещей вернул движок (сайт показывает 0 у всех, кроме трёх):")
    print("  " + " · ".join(f"{q}: {n}" for q, n in shown))
    lo = min(top_scores) if top_scores else 0
    print(f"оценка лучшей вещи: у нужных запросов от {lo:.3f} (медиана {statistics.median(top_scores):.3f}), у мусорных до {max(junk_top):.3f}")

    print("отдельные запросы (три первых):")
    for q in PROBES:
        h = eng.search(q, 3)
        print(f"  {q:<18} " + " | ".join(f"{lab[:26]} {sc:.2f}" for _, sc, lab in h) if h else f"  {q:<18} —")


if __name__ == "__main__":
    sys.exit(main())
