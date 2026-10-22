#!/usr/bin/env python3
"""Выгрузка всех публичных текстов сайта в JSON для переводчиков (es, de, it).

Читает только публичные GET-ручки API (без входа, без POST), по одному
запросу за раз, с паузой. Пишет `tools/i18n/source/<вид>.json` и
`tools/i18n/source/README.md` (счёт и пробелы). Запускать можно сколько угодно:
файлы переписываются целиком.

    python3 tools/i18n/dump.py
    python3 tools/i18n/dump.py --base http://localhost:3000/api/v1
    python3 tools/i18n/dump.py --token <сессия игрока>   # заодно поручения

Устройство файла вида:

    {
      "kind": "...",
      "table": "gazette_leaves",       # куда писать перевод
      "key": "slug",                   # постоянный ключ строки (есть на любой базе)
      "fields": {"title": "title_{lang}", ...},   # поле файла -> колонка
      "original": "ru",                # язык оригинала (по замеру, см. README)
      "items": [ {"key": "...", "id": "<uuid прода>", "title": {"en":..,"ru":..,"uk":..}, ...} ]
    }

Поле с языками — объект {en, ru, uk}; одноязычное поле — строка. Поля внутри
JSON-колонок (черты и способности карт, имена рамок в настройках) описаны в
`json_fields`.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

LANGS = ("en", "ru", "uk")
HERE = Path(__file__).resolve().parent
OUT = HERE / "source"

# ── HTTP ────────────────────────────────────────────────────────────────────


class Client:
    def __init__(self, base: str, delay: float, token: str | None):
        self.base = base.rstrip("/")
        self.delay = delay
        self.token = token
        self.calls = 0

    def get(self, path: str, auth: bool = False):
        url = self.base + path
        headers = {"User-Agent": "gotiga-i18n-dump/1", "Accept": "application/json"}
        if auth and self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        req = urllib.request.Request(url, headers=headers, method="GET")
        if self.calls:
            time.sleep(self.delay)
        self.calls += 1
        for attempt in range(3):
            try:
                with urllib.request.urlopen(req, timeout=30) as r:
                    body = r.read().decode("utf-8")
                    return json.loads(body) if body else None
            except urllib.error.HTTPError as e:
                if e.code in (401, 403, 404):
                    return {"__error__": e.code}
                if attempt == 2:
                    raise
            except urllib.error.URLError:
                if attempt == 2:
                    raise
            time.sleep(1.5 * (attempt + 1))
        return None


def err(x) -> int | None:
    return x.get("__error__") if isinstance(x, dict) else None


# ── помощники ───────────────────────────────────────────────────────────────


def cap(name: str) -> str:
    return name[0].upper() + name[1:]


def tri(obj: dict, camel: str) -> dict:
    """{en, ru, uk} из полей `camelEn`/`camelRu`/`camelUk`."""
    return {lang: obj.get(camel + cap(lang)) for lang in LANGS}


def snake(camel: str) -> str:
    return re.sub(r"(?<!^)([A-Z])", r"_\1", camel).lower()


def tri_fields(names: list[str]) -> dict:
    return {n: snake(n) + "_{lang}" for n in names}


def nonempty(v) -> bool:
    if v is None:
        return False
    if isinstance(v, str):
        return bool(v.strip())
    if isinstance(v, dict):
        return any(nonempty(x) for x in v.values())
    if isinstance(v, list):
        return any(nonempty(x) for x in v)
    return False


TRANSLIT = dict(zip(
    "абвгдеёжзийклмнопрстуфхцчшщъыьэюяіїєґ",
    ["a", "b", "v", "g", "d", "e", "e", "zh", "z", "i", "y", "k", "l", "m", "n",
     "o", "p", "r", "s", "t", "u", "f", "h", "ts", "ch", "sh", "sch", "", "y", "",
     "e", "yu", "ya", "i", "yi", "ye", "g"],
))


def slugish(text: str) -> str:
    s = "".join(TRANSLIT.get(c, c) for c in (text or "").lower())
    return re.sub(r"[^a-z0-9]+", "-", s).strip("-")


def original_of(slug: str | None, title: dict) -> str:
    """Язык, с которого заводили запись: слаг строится из заглавия первой версии."""
    if not slug:
        return "ru" if nonempty(title.get("ru")) else "en"
    s = slug.split("--")[0]
    ru, en = slugish(title.get("ru") or ""), slugish(title.get("en") or "")

    def score(a: str) -> int:
        aw, sw = set(a.split("-")), set(s.split("-"))
        return len(aw & sw)

    return "ru" if score(ru) >= score(en) and score(ru) > 0 else ("en" if score(en) > 0 else "ru")


def source_text(v) -> str:
    """Текст, который переводят: русский, где он есть, иначе английский."""
    if isinstance(v, dict) and set(v) <= set(LANGS):
        return (v.get("ru") or "").strip() or (v.get("en") or "").strip() or (v.get("uk") or "").strip()
    if isinstance(v, str):
        return v.strip()
    return ""


def count_chars(items: list, text_fields: list[str], nested: dict[str, list[str]] | None = None) -> int:
    total = 0
    for it in items:
        for f in text_fields:
            v = it.get(f)
            if isinstance(v, list):
                total += sum(len(source_text(x)) for x in v)
            else:
                total += len(source_text(v))
        for listname, subfields in (nested or {}).items():
            for sub in it.get(listname) or []:
                for f in subfields:
                    total += len(source_text(sub.get(f)))
    return total


# ── виды ────────────────────────────────────────────────────────────────────


def leaves(c: Client):
    """Все листы вестника, включая байки: список по страницам, годы подшивки, главная."""
    seen: dict[str, dict] = {}
    page = 1
    while True:
        d = c.get(f"/gazette?page={page}&perPage=50")
        if err(d) or not d:
            break
        for it in d.get("items", []):
            seen.setdefault(it["slug"], it)
        if page * d.get("perPage", 50) >= d.get("total", 0) or not d.get("items"):
            break
        page += 1
    blotter = c.get("/gazette/blotter")
    years = (blotter or {}).get("years") or []
    for it in (blotter or {}).get("leaves", []) or []:
        seen.setdefault(it["slug"], it)
    for y in years:
        if y == (blotter or {}).get("year"):
            continue
        d = c.get(f"/gazette/blotter?year={y}")
        for it in (d or {}).get("leaves", []) or []:
            seen.setdefault(it["slug"], it)
    for it in (c.get("/gazette/home") or {}).get("leaves", []) or []:
        seen.setdefault(it["slug"], it)
    for it in c.get("/tales") or []:
        seen.setdefault(it["slug"], it)
    # тело целиком — по одному листу
    full = {}
    for slug in seen:
        d = c.get("/gazette/" + urllib.parse.quote(slug))
        full[slug] = d if d and not err(d) else seen[slug]
    return full


def build_leaf_item(x: dict) -> dict:
    title = tri(x, "title")
    return {
        "key": x["slug"],
        "id": x["id"],
        "kind": x.get("kind"),
        "status": x.get("status"),
        "original": original_of(x.get("slug"), title),
        "figurineSlug": x.get("figurineSlug"),
        "title": title,
        "dek": tri(x, "dek"),
        "body": tri(x, "body"),
        "author": x.get("author"),
    }


LEAF_HEADER = {
    "table": "gazette_leaves",
    "key": "slug",
    "fields": tri_fields(["title", "dek", "body"]),
    "limits": {"title": 200, "dek": 500, "body": 12000},
    "notes": "`author` — одна колонка на все языки (имя не переводится), дано для справки.",
}


def cards(c: Client):
    pub = c.get("/battles/cards") or []
    etude = c.get("/battles/etude-cards") or []
    items = []
    seen = set()
    for x in (pub if not err(pub) else []) + (etude if not err(etude) else []):
        if x["slug"] in seen:
            continue
        seen.add(x["slug"])
        items.append({
            "key": x["slug"],
            "id": x["id"],
            "status": x.get("status"),
            "kind": x.get("kind"),
            "figurineSlug": x.get("figurineSlug"),
            "title": tri(x, "title"),
            "type": tri(x, "type"),
            "effect": tri(x, "effect"),
            "lore": tri(x, "lore"),
            "traits": [
                {"index": i, "name": tri(t, "name"), "text": tri(t, "text")}
                for i, t in enumerate(x.get("traits") or [])
            ],
            "abilities": [
                {"index": i, "id": a.get("id"), "name": tri(a, "name")}
                for i, a in enumerate(x.get("abilities") or [])
            ],
        })
    return items


def races(c: Client):
    d = c.get("/battles/races") or []
    return [
        {"key": x["slug"], "id": x["id"], "name": tri(x, "name"), "note": tri(x, "note"),
         "cardCount": x.get("cardCount")}
        for x in (d if not err(d) else [])
    ]


def keywords(c: Client):
    d = c.get("/battles/keywords") or []
    return [
        {"key": x["slug"], "id": x["id"], "name": tri(x, "name"), "rules": tri(x, "rules")}
        for x in (d if not err(d) else [])
    ]


def challenges(c: Client):
    d = c.get("/battles/challenges") or []
    return [
        {"key": x["slug"], "id": x["id"], "status": x.get("status"),
         "title": tri(x, "title"), "note": tri(x, "note")}
        for x in (d if not err(d) else [])
    ]


def errands(c: Client):
    if not c.token:
        return None, "нет --token: ручка /battles/errands отвечает только вошедшему игроку"
    d = c.get("/battles/errands", auth=True)
    if err(d):
        return None, f"/battles/errands ответил {err(d)}"
    rows = d if isinstance(d, list) else d.get("errands", [])
    return [
        {"key": x["slug"], "id": x["id"], "title": tri(x, "title"), "note": tri(x, "note")}
        for x in rows
    ], None


FIG_FIELDS = [
    "name", "shortText", "fullDescription", "material", "technique", "dimensions",
    "series", "edition", "createdPeriod", "careInstructions", "provenanceNote",
    "authenticityNote", "includedItems", "secretText",
]


def figurines(c: Client):
    lst = c.get("/figurines?visible=true&perPage=1000") or {}
    rows = lst.get("items", []) if isinstance(lst, dict) else lst
    items, images, steps = [], [], []
    for row in rows:
        ref = row.get("slug") or row["id"]
        x = c.get("/figurines/" + urllib.parse.quote(ref))
        if err(x) or not x:
            continue
        it = {"key": x.get("slug"), "id": x["id"]}
        for f in FIG_FIELDS:
            it[f] = x.get(f)
        cl = x.get("catalogLists")
        if cl:
            try:
                cl = json.loads(cl)
                custom = (cl.get("featuresCustom") or []) + (cl.get("perfectCustom") or [])
                if custom:
                    it["catalogCustom"] = custom
            except (ValueError, AttributeError):
                pass
        items.append(it)
        for im in x.get("images") or []:
            if nonempty(im.get("altText")):
                images.append({"key": im["id"], "figurineSlug": x.get("slug"), "altText": im["altText"]})
        for st in x.get("processSteps") or []:
            if nonempty(st.get("description")):
                steps.append({"key": st["id"], "figurineSlug": x.get("slug"),
                              "stepType": st.get("stepType"), "description": st["description"]})
    return items, images, steps


def tale_extras(c: Client, tales: list[dict]):
    scenes = []
    for t in tales:
        d = c.get(f"/tales/{t['id']}/scene")
        if not d or err(d):
            continue
        frames = d.get("frames") or []
        caps = [{"index": i, "caption": f.get("caption") or ""} for i, f in enumerate(frames)]
        if any(nonempty(x["caption"]) for x in caps):
            scenes.append({"key": t["key"], "taleId": t["id"], "frames": caps})
    poll = c.get("/tales/poll")
    return scenes, (poll if poll and not err(poll) else None)


def site_settings(c: Client):
    out = {}
    copy = c.get("/settings/copy")
    out["copy"] = copy if copy and not err(copy) else {}
    hc = c.get("/home-content")
    out["homeContent"] = {k: hc.get(k) for k in (
        "title", "kicker", "lead", "heroCaptionTitle", "heroCaptionMeta", "heroCaptionCta")} if hc and not err(hc) else {}
    ap = c.get("/author/profile")
    out["authorProfile"] = {k: ap.get(k) for k in ("name", "tagline", "bio")} if ap and not err(ap) else {}
    pr = c.get("/settings/programme")
    out["programme"] = {"curatorNote": tri(pr, "curatorNote"), "curatorSign": tri(pr, "curatorSign")} \
        if pr and not err(pr) else {}
    rooms = c.get("/showing-rooms") or []
    out["showingRooms"] = [{"key": r["id"], "name": r.get("name")} for r in (rooms if not err(rooms) else [])]
    texts = {}
    for cat in ("author", "workshop"):
        d = c.get(f"/content/texts/{cat}") or []
        texts[cat] = [{"key": t.get("id"), "content": t.get("content")} for t in (d if not err(d) else [])]
    out["contentTexts"] = texts
    hp = c.get("/settings/header-portrait")
    out["headerPortrait"] = None if (hp is None or err(hp)) else hp
    return out


def frames_and_motions(c: Client):
    fr = c.get("/battles/frames") or {}
    frames = [{"key": f["tier"], "name": tri(f, "name")} for f in (fr.get("frames") or [])] if not err(fr) else []
    mo = c.get("/battles/motions") or {}
    motions = [{"key": m["id"], "occasion": m.get("occasion"), "name": tri(m, "name")}
               for m in (mo.get("motions") or [])] if not err(mo) else []
    return frames, motions


# ── запись ──────────────────────────────────────────────────────────────────


def write(kind: str, header: dict, items, text_fields, nested=None, original="ru"):
    doc = {"kind": kind, **header, "original": original, "count": len(items),
           "chars": count_chars(items, text_fields, nested), "items": items}
    (OUT / f"{kind}.json").write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return doc


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--base", default="https://ritunia.com/api/v1")
    ap.add_argument("--delay", type=float, default=0.25, help="пауза между запросами, с")
    ap.add_argument("--token", default=None, help="сессия игрока: нужна только поручениям")
    a = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    c = Client(a.base, a.delay, a.token)
    stats: dict[str, dict] = {}

    def note(doc, extra=""):
        stats[doc["kind"]] = {"count": doc["count"], "chars": doc["chars"],
                              "original": doc["original"], "extra": extra}

    print("листы вестника и байки…", file=sys.stderr)
    all_leaves = leaves(c)
    built = [build_leaf_item(x) for x in all_leaves.values()]
    tales = [x for x in built if x["kind"] == "tale"]
    other = [x for x in built if x["kind"] != "tale"]
    orig_t = sorted({x["original"] for x in tales})
    note(write("tales", {**LEAF_HEADER, "where": "gazette_leaves.kind = 'tale'"}, tales,
               ["title", "dek", "body"], original="/".join(orig_t) or "ru"),
         "оригинал по слагу: " + ", ".join(f"{x['key']}={x['original']}" for x in tales))
    note(write("gazette", {**LEAF_HEADER, "where": "gazette_leaves.kind <> 'tale'"}, other,
               ["title", "dek", "body"], original="en"))

    print("карты…", file=sys.stderr)
    cd = cards(c)
    note(write("cards", {
        "table": "battle_cards", "key": "slug",
        "fields": tri_fields(["title", "type", "effect", "lore"]),
        "limits": {"title": 80, "type": 40, "effect": 400, "lore": 400},
        "json_fields": {
            "traits": "battle_cards.traits (TEXT, JSON-массив CardTrait): nameEn/nameRu/nameUk, textEn/textRu/textUk; по порядку (index)",
            "abilities": "battle_cards.abilities (TEXT, JSON-массив CardAbility): nameEn/nameRu/nameUk; искать по id умения",
        },
        "notes": "Опубликованные (/battles/cards) и карты только для этюдов (/battles/etude-cards).",
    }, cd, ["title", "type", "effect", "lore"], {"traits": ["name", "text"], "abilities": ["name"]}))

    print("расы, ключевые слова, этюды, поручения…", file=sys.stderr)
    note(write("races", {"table": "battle_races", "key": "slug",
                         "fields": tri_fields(["name", "note"]), "limits": {"name": 60, "note": 200}},
               races(c), ["name", "note"]))
    note(write("keywords", {"table": "battle_keywords", "key": "slug",
                            "fields": tri_fields(["name", "rules"]), "limits": {"name": 60, "rules": 300}},
               keywords(c), ["name", "rules"]))
    note(write("challenges", {"table": "battle_challenges", "key": "slug",
                              "fields": tri_fields(["title", "note"]), "limits": {"title": 80, "note": 400}},
               challenges(c), ["title", "note"]))
    er, why = errands(c)
    note(write("errands", {"table": "battle_errands", "key": "slug",
                           "fields": tri_fields(["title", "note"]), "limits": {"title": 120, "note": 400},
                           **({"missing": why} if why else {})},
               er or [], ["title", "note"]), why or "")

    print("работы (по одной)…", file=sys.stderr)
    figs, imgs, steps = figurines(c)
    note(write("figurines", {
        "table": "figurines", "key": "slug",
        "fields": {f: snake(f) for f in FIG_FIELDS},
        "target": "figurine_translations (figurine_id, lang, …) — колонки появятся в волне 2 (E)",
        "notes": "Одноязычные (английский). catalogCustom — свои строки из figurines.catalog_lists (JSON).",
    }, figs, FIG_FIELDS + ["catalogCustom"], original="en"))
    note(write("figurine_images", {"table": "images", "key": "id", "fields": {"altText": "alt_text"},
                                   "notes": "Ключ — UUID изображения прода; на другой базе не найдётся."},
               imgs, ["altText"], original="en"))
    note(write("process_steps", {"table": "process_steps", "key": "id",
                                 "fields": {"description": "description"},
                                 "notes": "Ключ — UUID прода."}, steps, ["description"], original="en"))

    print("сцены и голосование…", file=sys.stderr)
    scenes, poll = tale_extras(c, tales)
    note(write("tale_scenes", {"table": "tale_scenes", "key": "tale_id (по слагу байки)",
                               "fields": {"frames[].caption": "data->frames[i]->caption (JSONB, один язык)"}},
               scenes, [], {"frames": ["caption"]}, original="ru"))
    note(write("tale_polls", {"table": "tale_polls / tale_poll_candidates", "key": "id",
                              "fields": {},
                              "notes": "Своего текста у голосования нет: кандидаты — работы (их названия в figurines.json)."},
               [poll] if poll else [], [], original="en"))

    print("настройки, рамки, движения…", file=sys.stderr)
    st = site_settings(c)
    frames, motions = frames_and_motions(c)
    copy_items = [{"key": k, "text": {lang: (st["copy"].get(lang) or {}).get(k) for lang in LANGS}}
                  for k in sorted({k for lang in LANGS for k in (st["copy"].get(lang) or {})})]
    note(write("copy", {"table": "settings", "key": "key = 'copy_overrides' → JSON {lang: {ключ словаря: текст}}",
                        "fields": {"text": "value->{lang}->{key}"},
                        "notes": "Правки админа поверх словаря. Пустой язык — откат на словарь; новые языки — новые ветки JSON."},
               copy_items, ["text"], original="en"))
    note(write("frames", {"table": "settings", "key": "key = 'battle_frames' → frames[] по tier",
                          "fields": {"name": "nameEn/nameRu/nameUk"},
                          "notes": "Имя ранга рамки, видно игроку на листе карты."},
               frames, ["name"]))
    note(write("motions", {"table": "settings", "key": "key = 'battle_motions' → motions[] по id",
                           "fields": {"name": "nameEn/nameRu/nameUk"},
                           "notes": "Имена движений: видны на столе админа и в студии, игроку на полке не печатаются. Переводить по желанию."},
               motions, ["name"]))
    site_items = []
    for k, v in st["homeContent"].items():
        site_items.append({"key": f"home_content.{k}", "text": v,
                           "where": "app_resources key='home_content' (JSON в file_path)"})
    for k, v in st["authorProfile"].items():
        site_items.append({"key": f"author_profile.{k}", "text": v,
                           "where": "app_resources key='author_profile' (JSON в file_path)"})
    for k, v in st["programme"].items():
        site_items.append({"key": f"programme_settings.{k}", "text": v,
                           "where": "settings key='programme_settings' (JSON, поля …En/…Ru/…Uk)"})
    for r in st["showingRooms"]:
        site_items.append({"key": f"showing_rooms.{r['key']}.name", "text": r["name"],
                           "where": "showing_rooms.name (ключ id прода)"})
    for cat, rows in st["contentTexts"].items():
        for r in rows:
            site_items.append({"key": f"texts.{cat}.{r['key']}", "text": r["content"],
                               "where": "texts.content (category)"})
    note(write("site", {"table": "разные (см. where у строки)", "key": "key",
                        "fields": {"text": "см. where"},
                        "notes": "Тексты главной, автора, витрины показов. Одноязычные поля — английский."},
               site_items, ["text"], original="en"))

    write_readme(stats, c)
    print(f"готово: {c.calls} запросов, файлы в {OUT}", file=sys.stderr)
    return 0


GAPS = """\
## Что не удалось взять и почему

- **Поручения (`battle_errands`)** — `/battles/errands` отвечает только вошедшему
  игроку (401 без сессии), админская ручка закрыта ключом. Тексты заведены
  миграциями (`20260907000000_battle_errands.sql`, `20260908000000_battle_visits.sql`,
  `20260909000000_battle_errand_deeds.sql`, `20260911000000_battle_errands_plain_words.sql`,
  украинский — `20261015000000_ukrainian.sql`); правки хранителя на проде видны
  только с сессией: `dump.py --token <сессия>`.
- **Этюды (`battle_challenges`) и ключевые слова (`battle_keywords`)** — публичные
  ручки на проде отдают пустой список: опубликованных нет (черновики видит только
  админка). Файлы есть, пустые; при появлении опубликованных перезапустить.
- **Листы вестника, кроме байк** — на проде на людях только байки (11 листов);
  черновики, архив и запланированные — только в админке.
- **Скрытые работы, «первый взгляд», «в работе»** — публичный список отдаёт только
  видимые; `first-look` и `in-progress` сейчас пусты.
- **Сцены байк** — `/tales/:id/scene` отвечает `null` для выключенной сцены; у
  кодовых сцен (`taleScenes.ts`) подписи лежат в `src/`, это работа словаря (A/D).
- **Голосование «о ком следующая»** — сейчас не открыто (`null`); своего текста у
  него нет, кандидаты — работы.
- **Картинка автора в шапке** (`/settings/header-portrait`) — 404, не выбрана; текста в
  ней нет.
- **Студия** — галерея и сезон пусты; карты, расы и движения гостей до допуска видны
  только автору и админке.
- **Шаги процесса** — описаний на проде нет ни у одной работы (файл пустой).
- **Письма и записки сервера** — не данные базы, а код (`src-tauri`), это работа B.
- Отклики (`comments`), впечатления (`impressions`) — пишут посетители, не переводятся.

## Замечания к данным прода

- У рас `note_en` и `note_uk` содержат **русский** текст (копия `note_ru`) — переводить
  с `ru`; английский и украинский тоже стоит поправить.
- Раса `witch` (0 карт) — дубль `witches`, имя «Witch» во всех языках; переводить
  незачем, пока её не удалили или не назвали.
- У двух карт для этюдов нет украинского (и одной — `type_uk`).
- Пустые строки (`null`, `""`) в файлах оставлены: так видно, что поле есть, но на
  проде не заполнено.
"""


def write_readme(stats: dict, c: Client):
    lines = [
        "# Тексты прода для перевода (es, de, it)",
        "",
        f"Собрано `tools/i18n/dump.py` с `{c.base}` ({time.strftime('%Y-%m-%d')}), "
        f"{c.calls} GET-запросов. Перезапуск переписывает всё.",
        "",
        "Знаки считаются по тексту, с которого переводят: русский, где он есть, иначе "
        "английский (у работ — только английский). Пробелы и разметка входят в счёт.",
        "",
        "| Вид | Файл | Записей | Знаков | Оригинал |",
        "|---|---|---:|---:|---|",
    ]
    total = 0
    for kind, s in stats.items():
        total += s["chars"]
        lines.append(f"| {kind} | `{kind}.json` | {s['count']} | {s['chars']:,} | {s['original']} |".replace(",", " "))
    lines += ["", f"**Всего: {total:,} знаков.**".replace(",", " "), ""]
    lines += [
        "## Язык оригинала",
        "",
        "- **Байки** — по слагу (он строится из заглавия первой версии): "
        + stats.get("tales", {}).get("extra", ""),
        "- **Карты, расы** — русский и английский заведены вместе; переводить с русского "
        "со сверкой по английскому.",
        "- **Работы, главная, автор, правки словаря, витрины** — английский (полей других "
        "языков нет).",
        "",
        "## Ключ строки",
        "",
        "В шапке каждого файла: `table`, `key` (постоянный ключ — слаг, где он есть),",
        "`fields` (поле файла → колонка, `{lang}` заменить на `es`/`de`/`it`), `limits`",
        "(длина колонки, как у `_uk`). У каждой записи есть и `id` — UUID прода, для сверки.",
        "Ключ — UUID только у фото, шагов процесса и витрин показов: слага у них нет.",
        "",
        GAPS,
    ]
    (OUT / "README.md").write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    sys.exit(main())
