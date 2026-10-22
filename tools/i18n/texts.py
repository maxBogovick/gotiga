"""Spanish, German and Italian texts for the card set and etude loaders.

The translations live in tools/i18n/content/{es,de,it}/<kind>.json (the same
files make_migration.py turns into SQL). The loaders in tools/card_set and
tools/etudes write per-language fields through the admin API; this module
gives them the `…Es/…De/…It` fields for a slug. Nothing is invented: a text
missing in content/ is simply not in the result.

Standard library only.
"""
import json
from functools import lru_cache
from pathlib import Path

CONTENT = Path(__file__).resolve().parent / "content"
LANGS = ("es", "de", "it")


@lru_cache(maxsize=None)
def _items(kind, lang):
    try:
        doc = json.loads((CONTENT / lang / f"{kind}.json").read_text())
    except FileNotFoundError:
        return {}
    return {str(it.get("key")): it for it in doc.get("items", [])}


def _fields(kind, slug, mapping):
    """mapping: content field → API field prefix (e.g. "title" → "title")."""
    out = {}
    for lang in LANGS:
        it = _items(kind, lang).get(slug) or {}
        for src, prefix in mapping.items():
            v = it.get(src)
            if isinstance(v, str) and v.strip():
                out[prefix + lang.capitalize()] = v
    return out


def card_fields(slug):
    """titleEs, effectEs, typeEs, loreEs … for a card slug."""
    return _fields("cards", slug, {"title": "title", "effect": "effect", "type": "type", "lore": "lore"})


def card_abilities(slug, abilities):
    """The card's abilities with nameEs/nameDe/nameIt filled from content/ by
    ability id (an empty name already there is replaced, a set one is kept)."""
    out = []
    for i, a in enumerate(abilities):
        a = dict(a)
        for lang in LANGS:
            tr = (_items("cards", lang).get(slug) or {}).get("abilities") or []
            hit = next((t for t in tr if (a.get("id") and t.get("id") == a.get("id"))
                        or (not a.get("id") and t.get("index") == i)), None)
            key = "name" + lang.capitalize()
            if hit and (hit.get("name") or "").strip() and not (a.get(key) or "").strip():
                a[key] = hit["name"]
        out.append(a)
    return out


def race_fields(slug):
    """nameEs, noteEs … for a race slug."""
    return _fields("races", slug, {"name": "name", "note": "note"})


def challenge_fields(slug):
    """titleEs, noteEs … for an etude slug (content/<lang>/challenges.json)."""
    return _fields("challenges", slug, {"title": "title", "note": "note"})


def carry(body, existing, prefixes):
    """A PUT carries the whole row: fill every `<prefix>Es/De/It` the body does
    not have from what the server already has, so an update never wipes a
    translation written there by hand."""
    if not existing:
        return body
    for p in prefixes:
        for lang in LANGS:
            k = p + lang.capitalize()
            if k not in body and (existing.get(k) or "").strip():
                body[k] = existing[k]
    return body
