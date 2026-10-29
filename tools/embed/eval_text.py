#!/usr/bin/env python3
"""Замер текстовой модели на фразах из tools/search/cases.json.

    .venv-tools/embed/bin/python tools/embed/eval_text.py intfloat/multilingual-e5-small BAAI/bge-m3

Вещь считается найденной по лучшему из её кусков. Верные вещи — по первым 8
знакам id, как в замере сервера. Фразы «ничего не должно находиться» (пустой
`expect`) в этой метрике не участвуют.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
import torch
from sentence_transformers import SentenceTransformer

sys.path.insert(0, str(Path(__file__).parent))
from common import all_chunks  # noqa: E402

ROOT = Path(__file__).parent.parent / "search"
corpus = json.load(open(ROOT / "corpus.json", encoding="utf-8"))
cases = [c for c in json.load(open(ROOT / "cases.json", encoding="utf-8")) if c["expect"]]
rows = all_chunks(corpus)
print(f"{len(rows)} кусков, {len(cases)} фраз")

# e5 просит приставки; bge-m3 — нет
PREFIX = {"e5": ("query: ", "passage: ")}


def prefixes(name):
    return PREFIX["e5"] if "e5" in name else ("", "")


device = "mps" if torch.backends.mps.is_available() else "cpu"
for name in sys.argv[1:]:
    t0 = time.time()
    model = SentenceTransformer(name, device=device)
    model.max_seq_length = 512
    qp, dp = prefixes(name)
    docs = model.encode([dp + r["text"] for r in rows], batch_size=16, normalize_embeddings=True,
                        show_progress_bar=False)
    qs = model.encode([qp + c["q"] for c in cases], batch_size=32, normalize_embeddings=True)
    sims = qs @ docs.T
    ids = np.array([r["id"] for r in rows])
    hit = {1: 0, 3: 0, 6: 0}
    miss = []
    for i, c in enumerate(cases):
        best = {}
        for j in np.argsort(-sims[i])[:60]:
            best.setdefault(ids[j], sims[i][j])
        order = list(best)
        rank = min((k for k, v in enumerate(order) if v[:8] in c["expect"]), default=99)
        for n in hit:
            hit[n] += rank < n
        if rank >= 3:
            miss.append((c["q"], rank + 1 if rank < 99 else None))
    n = len(cases)
    print(f"\n{name}: top1 {hit[1]/n:.1%}  top3 {hit[3]/n:.1%}  top6 {hit[6]/n:.1%}  "
          f"({time.time()-t0:.0f} с)")
    print("  не в первых трёх:", miss[:12])
    del model
