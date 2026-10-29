#!/usr/bin/env python3
"""Реранкер вместо порогов: отделяет ли он нужное от мусора, и не портит ли порядок.

    .venv-tools/embed/bin/python tools/embed/eval_rerank.py [BAAI/bge-reranker-v2-m3]

Схема: быстрый поиск (bge-m3) берёт 30 вещей, реранкер читает пары «запрос + лучший
кусок вещи» и выдаёт оценку релевантности 0..1. Печатается: порядок на 129 фразах
из tools/search/cases.json, оценка лучшей вещи у мусорных запросов и у запросов про
оружие, и на каком пороге мусор и нужное расходятся.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
import torch
from sentence_transformers import CrossEncoder, SentenceTransformer

sys.path.insert(0, str(Path(__file__).parent))
from common import all_chunks  # noqa: E402

name = sys.argv[1] if len(sys.argv) > 1 else "BAAI/bge-reranker-v2-m3"
R = Path(__file__).parent.parent / "search"
corpus = json.load(open(R / "corpus.json", encoding="utf-8"))
cases = json.load(open(R / "cases.json", encoding="utf-8"))
rows = all_chunks(corpus)
label = {i["id"]: ((i.get("title") or {}).get("ru") or i.get("name") or i["id"]) for g in corpus.values() for i in g}
kind = {i["id"]: i["kind"] for g in corpus.values() for i in g}
dev = "mps" if torch.backends.mps.is_available() else "cpu"

dense = SentenceTransformer("BAAI/bge-m3", device=dev)
dense.max_seq_length = 512
D = dense.encode([r["text"] for r in rows], batch_size=16, normalize_embeddings=True)
ids = np.array([r["id"] for r in rows])
ce = CrossEncoder(name, device=dev, max_length=512)


def candidates(q, k=30):
    s = D @ dense.encode([q], normalize_embeddings=True)[0]
    best = {}
    for j in np.argsort(-s):
        best.setdefault(ids[j], j)
        if len(best) >= k:
            break
    return list(best.items())  # (id, индекс лучшего куска)


def rerank(q, k=30):
    cand = candidates(q, k)
    sc = ce.predict([(q, rows[j]["text"]) for _, j in cand], batch_size=16, show_progress_bar=False)
    return sorted(((float(s), i) for (i, _), s in zip(cand, sc)), reverse=True)


t0 = time.time()
pos = [c for c in cases if c["expect"]]
hit = {1: 0, 3: 0, 6: 0}
top_pos = []
for c in pos:
    out = rerank(c["q"])
    order = [i for _, i in out]
    rank = next((k for k, i in enumerate(order) if i[:8] in c["expect"]), 99)
    for n in hit:
        hit[n] += rank < n
    top_pos.append(out[0][0])
print(f"{name}: top1 {hit[1]/len(pos):.1%}  top3 {hit[3]/len(pos):.1%}  top6 {hit[6]/len(pos):.1%}  "
      f"({(time.time()-t0)/len(pos):.2f} с на запрос)")

junk = [c["q"] for c in cases if not c["expect"]] + [
    "pizza recipe", "bitcoin price", "football score", "dragon", "weather tomorrow", "how to fix a car",
    "asdf qwer zxcv", "курс доллара", "ракетный двигатель", "президент франции", "футбол",
    "квантовая физика формулы", "best laptop 2026", "stock market"]
js = sorted(((rerank(q)[0][0], q) for q in junk), reverse=True)
print("мусор, оценка лучшей вещи (по убыванию):", [(round(s, 3), q) for s, q in js[:6]])
print("нужное, наименьшие оценки лучшей вещи:", [(round(s, 3), c["q"]) for s, c in sorted(zip(top_pos, pos), key=lambda x: x[0])[:6]])
for q in ["weapon", "оружие", "sword", "axe", "меч"]:
    out = rerank(q)
    print(f"  {q}: ", [(round(s, 3), kind[i], label[i][:22]) for s, i in out[:4]])
