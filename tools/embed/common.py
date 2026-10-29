"""Общее для сборки индекса и замера: как работа, байка, лист и карта режутся на куски текста.

Один кусок — одна строка индекса: `{id, kind, lang, part, text}`. Работа ищется по
названию, описанию, материалу и серии; байка — по заголовку, вводке и абзацам
тела (тело режется, потому что модель читает ограниченную длину, а байка в десять
тысяч знаков иначе потеряла бы всё после первой страницы).
"""
import re

LANGS = ["en", "ru", "uk", "es", "de", "it"]
CHUNK_CHARS = 900
CHUNK_OVERLAP = 150


def split_body(body):
    """Режет текст по абзацам до ~CHUNK_CHARS знаков, соседние куски перекрываются."""
    body = re.sub(r"[#>*]+", "", body or "")
    paras = [p.strip() for p in re.split(r"\n\s*\n", body) if p.strip()]
    out, cur = [], ""
    for p in paras:
        if cur and len(cur) + len(p) > CHUNK_CHARS:
            out.append(cur)
            cur = cur[-CHUNK_OVERLAP:] + "\n" + p
        else:
            cur = (cur + "\n" + p).strip()
    if cur:
        out.append(cur)
    return out


def chunks_of(item):
    """Куски текста одной вещи: [(lang, part, text)]."""
    kind = item["kind"]
    out = []
    if kind == "work":
        names = item.get("names") or {}
        names = {"en": item.get("name")} | names
        shorts = item.get("short") or {}
        full = item.get("full") or {}
        for lang in sorted(set(names) | set(shorts) | set(full)):
            head = names.get(lang) or names.get("en") or ""
            bits = [head, shorts.get(lang) or "", item.get("material") or "",
                    item.get("technique") or "", item.get("series") or ""]
            out.append((lang, "head", ". ".join(b for b in bits if b)))
            if full.get(lang):
                for i, c in enumerate(split_body(full[lang])):
                    out.append((lang, f"full{i}", f"{head}. {c}"))
    elif kind == "card":
        for lang in sorted(set(item.get("title") or {}) | set(item.get("effect") or {})):
            bits = [(item.get("title") or {}).get(lang), (item.get("race") or {}).get(lang),
                    (item.get("effect") or {}).get(lang), (item.get("lore") or {}).get(lang)]
            out.append((lang, "card", ". ".join(b for b in bits if b)))
    else:  # tale, leaf
        for lang in sorted(set(item.get("title") or {}) | set(item.get("body") or {})):
            title = (item.get("title") or {}).get(lang) or ""
            dek = (item.get("dek") or {}).get(lang) or ""
            out.append((lang, "head", ". ".join(b for b in (title, dek) if b)))
            for i, c in enumerate(split_body((item.get("body") or {}).get(lang))):
                out.append((lang, f"body{i}", f"{title}. {c}"))
    return [(l, p, t) for l, p, t in out if t.strip()]


def all_chunks(corpus):
    rows = []
    for group in corpus.values():
        for item in group:
            for lang, part, text in chunks_of(item):
                rows.append({"id": item["id"], "kind": item["kind"], "lang": lang,
                             "part": part, "text": text})
    return rows
