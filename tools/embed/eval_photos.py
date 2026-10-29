#!/usr/bin/env python3
"""Замер поиска фото: название работы на языке X должно находить её фото.

    .venv-tools/embed/bin/python tools/embed/eval_photos.py tools/embed/index tools/embed/index-large

Запросом служит название работы без слов «кукла» (они есть у всех и ничего не
различают). Берутся только работы с названием, которого нет у другой работы. Верно,
если среди первых N фото есть фото этой работы. Метка слабая: название описывает не
всё, что видно на фото, — число годится для сравнения моделей, а не как точность.
"""
import re
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from search import Index  # noqa: E402

DROP = re.compile(r"\b(small|tiny|miniature|authors?|art|doll|artist'?s?|маленькая|крошечная|авторская|кукла|"
                  r"pequeña|muñeca|artística|de autor|kleine|künstlerpuppe|piccola|bambola|d.artista|"
                  r"маленька|крихітна|авторська|лялька)\b[:\s]*", re.I)


def clean(name):
    return DROP.sub("", name).strip(" :-")


for folder in sys.argv[1:]:
    idx = Index(Path(folder))
    works = {i: v for i, v in idx.items.items() if v["kind"] == "work"}
    photo_works = np.array([p["work"] for p in idx.photos])
    for lang in ["ru", "en", "de"]:
        names = {i: clean(v["title"].get(lang, "")) for i, v in works.items()}
        count = {}
        for n in names.values():
            count[n] = count.get(n, 0) + 1
        cases = [(i, n) for i, n in names.items() if n and count[n] == 1 and (photo_works == i).any()]
        qs = idx.im.encode_queries([n for _, n in cases])
        sims = qs @ idx.ivec.T
        hit = {1: 0, 5: 0, 10: 0}
        for (i, _), s in zip(cases, sims):
            order = np.argsort(-s)
            rank = int(np.argmax(photo_works[order] == i))
            for n in hit:
                hit[n] += rank < n
        n = len(cases)
        print(f"{Path(folder).name:12} {lang}  фраз {n:3}  первое фото верно {hit[1]/n:5.1%}  "
              f"в первых 5 {hit[5]/n:5.1%}  в первых 10 {hit[10]/n:5.1%}")
