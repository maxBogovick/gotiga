#!/usr/bin/env python3
"""Data texts in the other languages → SQL migrations.

  python3 tools/i18n/make_migration.py                  # check, then write the files
  python3 tools/i18n/make_migration.py --check          # check only, write nothing
  python3 tools/i18n/make_migration.py --stamp 20261105000000 --only tales,cards
  python3 tools/i18n/make_migration.py --stamp 20261022000002 \
      --only figurines,figurine_images --langs ru,uk    # works in Russian/Ukrainian

Languages are es, de, it by default. Russian and Ukrainian (`--langs ru,uk`)
are accepted only for the works and their photo descriptions: everywhere else
ru is the original and uk has its own migrations. The file name carries the
languages (`..._figurine_translations_ru_uk.sql`).

Reads tools/i18n/source/<kind>.json (what is on the live site, the key of every
row) and tools/i18n/content/{es,de,it}/<kind>.json (the translations), checks
them and writes into src-tauri/server/migrations/:

  <stamp>    _content_es_de_it.sql               tales, gazette, cards (with the
                                                   ability and trait names inside
                                                   the JSON), races, keywords,
                                                   etudes, errands, frame names,
                                                   motion names, copy overrides
  <stamp+1>  _figurine_translations_es_de_it.sql  works and photo descriptions
  <site-stamp> _site_translations.sql             home page texts, showing room
                                                   names (own stamp: the columns
                                                   come in 20261021000000)

A file larger than SPLIT_BYTES is cut into parts, each with the next stamp.

Every statement finds its row by a permanent key (slug, frame tier, motion id,
photo id) and fills a column only while it is empty, so the output is safe to
run more than once and never overwrites what the keeper wrote by hand.

The output is deterministic: the same inputs give the same bytes.

**sqlx checksums.** A migration that has been applied anywhere must never
change: sqlx compares checksums and refuses to start. When translations change
after the files were applied, generate NEW files with a later stamp
(`--stamp YYYYMMDDHHMMSS`, optionally `--only kind,...`); the old files stay as
they are. Because every statement fills only empty columns, a new file adds
missing texts but does not replace texts already written by an earlier one.

Standard library only.
"""
import argparse
import json
import re
import sys
from datetime import datetime, timedelta
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SOURCE = HERE / "source"
CONTENT = HERE / "content"
MIGRATIONS = ROOT / "src-tauri" / "server" / "migrations"
BATTLES_RS = ROOT / "src-tauri" / "server" / "src" / "battles.rs"

LANGS = ("es", "de", "it")       # rebound by --langs in main()
EXTRA_LANGS = ("ru", "uk")         # allowed only for FIGURINE_KINDS
LANG_NAMES = {"ru": "Russian", "uk": "Ukrainian", "es": "Spanish", "de": "German", "it": "Italian"}
RUSSIAN_ONLY = re.compile(r"[ыэъёЫЭЪЁ]")  # letters Ukrainian does not have
DEFAULT_STAMP = "20261020000002"
DEFAULT_SITE_STAMP = "20261021000001"
SPLIT_BYTES = 1_500_000

CONTENT_KINDS = ("tales", "gazette", "cards", "races", "keywords", "challenges", "errands",
                 "frames", "motions", "copy", "site")
FIGURINE_KINDS = ("figurines", "figurine_images")
ALL_KINDS = CONTENT_KINDS + FIGURINE_KINDS

CYRILLIC = re.compile(r"[Ѐ-ӿ]")
META = {"key", "id", "index"}  # fields in a content item that are not texts

# Limits not written as CHECK constraints in SQL. Frame names have no limit in
# the server code; 60 is the same as every other name in the game. Copy
# overrides have no per-key limit either (the whole setting is capped at
# 500 KiB). Photo descriptions: CHECK in 20261020000001 (1..1000), parsed too.
FRAME_NAME_MAX = 60
COPY_TEXT_MAX = 2000


class Problems:
    def __init__(self):
        self.errors, self.warnings = [], []

    def err(self, msg):
        self.errors.append(msg)

    def warn(self, msg):
        self.warnings.append(msg)


# ── limits from the migrations and the server code ──

def sql_limits():
    """{(table, column): max length} from every CHECK (... char_length(col) <= N)."""
    out = {}
    alter = re.compile(r"ALTER TABLE (\w+) ADD CONSTRAINT \w+\s+CHECK \((?:\w+ IS NULL OR )?"
                       r"char_length\((\w+)\) <= (\d+)\)")
    inline = re.compile(r"^\s+(\w+)\s+TEXT(?: NOT NULL)? CHECK \((?:\w+ IS NULL OR )?char_length\(\1\) "
                        r"(?:<= (\d+)|BETWEEN \d+ AND (\d+))\)", re.M)
    table = re.compile(r"CREATE TABLE IF NOT EXISTS (\w+) \((.*?)\n\);", re.S)
    for f in sorted(MIGRATIONS.glob("*.sql")):
        text = f.read_text()
        for t, c, n in alter.findall(text):
            out[(t, c)] = int(n)
        for t, body in table.findall(text):
            for c, a, b in inline.findall(body):
                out[(t, c)] = int(a or b)
    return out


def rust_const(name, default):
    m = re.search(rf"pub const {name}: usize = (\d+);", BATTLES_RS.read_text()) if BATTLES_RS.exists() else None
    return int(m.group(1)) if m else default


def seeded_errand_slugs():
    """Errands are not on the public API (login only): their slugs are taken
    from the migrations that seeded and translated them."""
    slugs = set()
    for f in sorted(MIGRATIONS.glob("*.sql")):
        for m in re.finditer(r"UPDATE battle_errands\b.*?WHERE slug = '([a-z0-9-]+)'", f.read_text(), re.S):
            slugs.add(m.group(1))
    return slugs


# ── reading ──

def read_json(path):
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        return None
    except ValueError as e:
        sys.exit(f"{path}: not valid JSON: {e}")


def text_of(v):
    return v.strip() if isinstance(v, str) else ""


def keyed(items, p, where):
    out = {}
    for it in items:
        k = it.get("key")
        if k is None:
            p.err(f"{where}: an item has no key")
            continue
        k = str(k)
        if k in out:
            p.err(f"{where}: key {k!r} twice")
        out[k] = it
    return out


def check_text(p, where, value, limit, lang=None):
    if not isinstance(value, str):
        p.err(f"{where}: not a string ({type(value).__name__})")
        return False
    if lang == "uk" and RUSSIAN_ONLY.search(value):
        p.err(f"{where}: Russian letter {RUSSIAN_ONLY.search(value).group(0)!r} in the Ukrainian text")
        return False
    if lang not in EXTRA_LANGS and CYRILLIC.search(value):
        bad = CYRILLIC.search(value).group(0)
        p.err(f"{where}: Cyrillic letter {bad!r} in the translation")
        return False
    if limit is not None and len(value) > limit:
        p.err(f"{where}: {len(value)} characters, the column takes {limit}")
        return False
    return True


# ── SQL helpers ──

def lit(s):
    return "'" + s.replace("'", "''") + "'"


def pick_tag(base, texts):
    n = 0
    while True:
        tag = base if n == 0 else f"{base}{n}"
        if not any(f"${tag}$" in t for t in texts):
            return tag
        n += 1


class Sql:
    """A list of statements; a text is kept apart until the tag is chosen."""

    def __init__(self):
        self.parts = []  # each statement is a list of str | ("q", text)
        self.counts = {}

    def add(self, kind, parts):
        self.parts.append(parts)
        self.counts[kind] = self.counts.get(kind, 0) + 1


def Q(s):
    return ("q", s)


def empty(col):
    return f"COALESCE({col}, '') = ''"


def fill_col(col, text):
    return [f"{col} = CASE WHEN {empty(col)} THEN ", Q(text), f" ELSE {col} END"]


def join(parts_lists, sep):
    out = []
    for i, p in enumerate(parts_lists):
        if i:
            out.append(sep)
        out.extend(p)
    return out


# ── kinds ──

def column_kind(p, sql, kind, src, tr, limits, extra_where=""):
    """Tables with one column per language: find the row by slug, fill empty columns."""
    table, fields = src["table"], src["fields"]
    hdr_limits = src.get("limits", {})
    src_items = keyed(src.get("items", []), p, f"source/{kind}")
    allowed = set(fields)
    for lang in LANGS:
        doc = tr.get(lang)
        if doc is None:
            continue
        items = keyed(doc.get("items", []), p, f"content/{lang}/{kind}")
        for key in sorted(items):
            it = items[key]
            where = f"content/{lang}/{kind}.json [{key}]"
            if key not in src_items and kind != "errands":
                p.err(f"{where}: no such key in source/{kind}.json")
                continue
            extra = set(it) - allowed - set(src.get("json_fields", {})) - META
            if extra:
                p.err(f"{where}: unknown fields {sorted(extra)}")
            sets = []
            for f in sorted(allowed & set(it)):
                v = it[f]
                if v is None or text_of(v) == "":
                    continue
                col = fields[f].replace("{lang}", lang)
                if limits.get((table, col)) is None:
                    p.err(f"{where}: column {table}.{col} has no CHECK in the migrations (schema missing?)")
                    continue
                limit = min(x for x in (hdr_limits.get(f), limits[(table, col)]) if x is not None)
                if check_text(p, f"{where}.{f}", v, limit):
                    sets.append(fill_col(col, v))
            if kind == "cards":
                sets_json = card_json(p, it, src_items.get(key, {}), lang, where)
                for col_parts in sets_json:
                    sets.append(col_parts)
            if not sets:
                continue
            stmt = [f"-- {key} ({lang})\nUPDATE {table}\n   SET "] + join(sets, ",\n       ") + \
                   [f"\n WHERE slug = {lit(key)}{extra_where};\n"]
            sql.add(kind, stmt)
        # coverage
        for key, s in sorted(src_items.items()):
            orig = src.get("original", "en")
            for f in fields:
                sv = s.get(f)
                has = any(text_of(sv.get(l)) for l in ("ru", "en")) if isinstance(sv, dict) else text_of(sv)
                if has and not text_of(items.get(key, {}).get(f)):
                    p.warn(f"{lang}/{kind} [{key}].{f}: no translation (original {orig})")


TRAIT_NAME_MAX = rust_const("TRAIT_NAME_MAX", 60)
TRAIT_TEXT_MAX = rust_const("TRAIT_TEXT_MAX", 200)
ABILITY_NAME_MAX = rust_const("ABILITY_NAME_MAX", 60)
MOTION_NAME_MAX = rust_const("MOTION_NAME_MAX", 60)


def card_json(p, it, src_item, lang, where):
    """SET fragments for `abilities` / `traits` (TEXT holding a JSON array).

    An element is matched by ability id (or, for an ability without id and for
    traits, by its place in the array) and the nested field is written only
    while it is empty. The column is rewritten only when an element changes."""
    out = []
    L = lang.capitalize()
    abilities = it.get("abilities")
    if abilities is not None:
        src_ab = {a.get("index"): a for a in src_item.get("abilities", [])}
        cases = []
        for a in abilities:
            w = f"{where}.abilities[{a.get('index')}]"
            s = src_ab.get(a.get("index"))
            if s is None or (s.get("id") or "") != (a.get("id") or ""):
                p.err(f"{w}: id {a.get('id')!r} at index {a.get('index')} does not match the source")
                continue
            name = a.get("name")
            if text_of(name) == "" or not check_text(p, f"{w}.name", name, ABILITY_NAME_MAX):
                continue
            match = (f"e.a->>'id' = {lit(a['id'])}" if a.get("id")
                     else f"COALESCE(e.a->>'id', '') = '' AND e.ord = {int(a['index']) + 1}")
            cases.append((match, f"name{L}", name))
        if cases:
            out.append(json_array_fill("abilities", cases))
    traits = it.get("traits")
    if traits is not None:
        src_tr = {t.get("index"): t for t in src_item.get("traits", [])}
        cases = []
        for t in traits:
            w = f"{where}.traits[{t.get('index')}]"
            if t.get("index") not in src_tr:
                p.err(f"{w}: no trait at this index in the source")
                continue
            for f, limit, field in (("name", TRAIT_NAME_MAX, f"name{L}"), ("text", TRAIT_TEXT_MAX, f"text{L}")):
                v = t.get(f)
                if text_of(v) and check_text(p, f"{w}.{f}", v, limit):
                    cases.append((f"e.ord = {int(t['index']) + 1}", field, v))
        if cases:
            out.append(json_array_fill("traits", cases))
    return out


def json_array_fill(col, cases):
    """col = the same array with each matched element's empty field filled."""
    expr = ["e.a"]
    for match, field, text in cases:
        expr = (["CASE WHEN ", match, f" AND COALESCE(e.a->>{lit(field)}, '') = '' THEN ("] + expr +
                [f") || jsonb_build_object({lit(field)}, ", Q(text), ") ELSE ("] + expr + [") END"])
    any_empty = " OR ".join(f"({m} AND COALESCE(e.a->>{lit(f)}, '') = '')" for m, f, _ in cases)
    return ([f"{col} = CASE WHEN {col} IS NOT NULL AND jsonb_typeof({col}::jsonb) = 'array' AND EXISTS ("
             f"SELECT 1 FROM jsonb_array_elements({col}::jsonb) WITH ORDINALITY AS e(a, ord) WHERE {any_empty}) "
             f"THEN (SELECT jsonb_agg("] + expr +
            [f" ORDER BY e.ord) FROM jsonb_array_elements({col}::jsonb) WITH ORDINALITY AS e(a, ord))::text "
             f"ELSE {col} END"])


def settings_array_kind(p, sql, kind, src, tr, setting, array, match_field, limit, numeric):
    """frames / motions: names inside settings JSON, `{array: [{match_field, nameXx}]}`."""
    src_items = keyed(src.get("items", []), p, f"source/{kind}")
    for lang in LANGS:
        doc = tr.get(lang)
        if doc is None:
            continue
        L = lang.capitalize()
        items = keyed(doc.get("items", []), p, f"content/{lang}/{kind}")
        for key in sorted(items, key=lambda k: (len(k), k)):
            it = items[key]
            where = f"content/{lang}/{kind}.json [{key}]"
            if key not in src_items:
                p.err(f"{where}: no such key in source/{kind}.json")
                continue
            extra = set(it) - {"name"} - META
            if extra:
                p.err(f"{where}: unknown fields {sorted(extra)}")
            name = it.get("name")
            if text_of(name) == "" or not check_text(p, f"{where}.name", name, limit):
                continue
            field = f"name{L}"
            m = (f"(e.a->>{lit(match_field)}) = {lit(str(int(key)))}" if numeric
                 else f"e.a->>{lit(match_field)} = {lit(key)}")
            cond = f"{m} AND COALESCE(e.a->>{lit(field)}, '') = ''"
            stmt = [f"-- {kind} {key} ({lang})\nUPDATE settings\n   SET value = jsonb_set(value::jsonb, "
                    f"'{{{array}}}', (SELECT jsonb_agg(CASE WHEN {cond} THEN e.a || jsonb_build_object({lit(field)}, ",
                    Q(name), f") ELSE e.a END ORDER BY e.ord) FROM jsonb_array_elements(value::jsonb->{lit(array)}) "
                    f"WITH ORDINALITY AS e(a, ord)))::text\n WHERE key = {lit(setting)}\n"
                    f"   AND jsonb_typeof(value::jsonb->{lit(array)}) = 'array'\n"
                    f"   AND EXISTS (SELECT 1 FROM jsonb_array_elements(value::jsonb->{lit(array)}) "
                    f"WITH ORDINALITY AS e(a, ord) WHERE {cond});\n"]
            sql.add(kind, stmt)


def copy_kind(p, sql, src, tr):
    """settings.copy_overrides = {lang: {dictionary key: text}}. A translation is
    written only for a key the keeper overrode in English (otherwise the
    dictionary of that language is already right) and only while it is unset."""
    src_items = keyed(src.get("items", []), p, "source/copy")
    for lang in LANGS:
        doc = tr.get(lang)
        if doc is None:
            continue
        items = keyed(doc.get("items", []), p, f"content/{lang}/copy")
        for key in sorted(items):
            it = items[key]
            where = f"content/{lang}/copy.json [{key}]"
            if key not in src_items:
                p.err(f"{where}: no such key in source/copy.json")
                continue
            text = it.get("text")
            if text_of(text) == "" or not check_text(p, f"{where}.text", text, COPY_TEXT_MAX):
                continue
            stmt = [f"-- copy {key} ({lang})\nUPDATE settings\n   SET value = (value::jsonb || jsonb_build_object("
                    f"{lit(lang)}, COALESCE(value::jsonb->{lit(lang)}, '{{}}'::jsonb) || jsonb_build_object({lit(key)}, ",
                    Q(text), f")))::text\n WHERE key = 'copy_overrides'\n"
                    f"   AND COALESCE(value::jsonb->'en'->>{lit(key)}, '') <> ''\n"
                    f"   AND COALESCE(value::jsonb->{lit(lang)}->>{lit(key)}, '') = '';\n"]
            sql.add("copy", stmt)


# Site texts may come in Russian and Ukrainian too: on the live site they exist
# only in English. Read only when content/<lang>/site.json is there.
SITE_LANGS = ("ru", "uk") + LANGS
SITE_TEXT_MAX = 500  # showing_rooms CHECK is 200; checked separately below
HOME_FIELDS = ("title", "kicker", "lead", "heroCaptionTitle", "heroCaptionMeta", "heroCaptionCta")
UUID_RE = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")


def site_kind(p, sql, src):
    """Home texts (`app_resources.home_content`, JSON keys `kickerEs` …) and
    showing room names (`showing_rooms.name_es` …, from
    20261021000000_showing_room_names.sql). A translation is written only while
    the English it was made from is still the text on the site and while the
    translation is empty: changed English means the translation is stale."""
    src_items = keyed(src.get("items", []), p, "source/site")
    for lang in SITE_LANGS:
        doc = read_json(CONTENT / lang / "site.json")
        if doc is None:
            continue
        if doc.get("lang") not in (None, lang):
            p.err(f"content/{lang}/site.json says lang {doc.get('lang')!r}")
        items = keyed(doc.get("items", []), p, f"content/{lang}/site")
        suffix = lang[0].upper() + lang[1:]
        for key in sorted(items):
            where = f"content/{lang}/site.json [{key}]"
            if key not in src_items:
                p.err(f"{where}: no such key in source/site.json")
                continue
            text = items[key].get("text")
            original = text_of(src_items[key].get("text"))
            if text_of(text) == "":
                continue
            if lang in ("ru", "uk"):
                ok = isinstance(text, str) and len(text) <= SITE_TEXT_MAX
                if not ok:
                    p.err(f"{where}.text: not a string or longer than {SITE_TEXT_MAX}")
                    continue
            elif not check_text(p, f"{where}.text", text, SITE_TEXT_MAX):
                continue
            if original == "":
                p.warn(f"{where}: the English text is empty on the site; translation skipped")
                continue
            head, _, rest = key.partition(".")
            if head == "home_content" and rest in HOME_FIELDS:
                field = f"{rest}{suffix}"
                stmt = [f"-- site {key} ({lang})\nUPDATE app_resources\n"
                        f"   SET file_path = jsonb_set(file_path::jsonb, '{{{field}}}', to_jsonb(",
                        Q(text), "::text))::text\n WHERE key = 'home_content'\n"
                        # CASE keeps the JSON cast away from the other rows of
                        # app_resources (plain file paths): AND does not promise
                        # an order of evaluation, CASE does.
                        f"   AND (CASE WHEN key = 'home_content' THEN file_path::jsonb->>{lit(rest)} END) = ",
                        Q(original), "\n"
                        f"   AND (CASE WHEN key = 'home_content' THEN COALESCE(file_path::jsonb->>{lit(field)}, '') END) = '';\n"]
                sql.add("site", stmt)
            elif head == "showing_rooms" and rest.endswith(".name") and UUID_RE.fullmatch(rest[:-5]):
                if len(text) > 200:
                    p.err(f"{where}.text: {len(text)} characters, the column takes 200")
                    continue
                col = f"name_{lang}"
                stmt = [f"-- site {key} ({lang})\nUPDATE showing_rooms\n   SET ", *fill_col(col, text),
                        f"\n WHERE id = {lit(rest[:-5])}\n   AND name = ", Q(original), ";\n"]
                sql.add("site", stmt)
            else:
                p.err(f"{where}: the server has no per-language place for this key")


def figurines_kind(p, sql, src, tr, limits):
    fields = src["fields"]
    src_items = keyed(src.get("items", []), p, "source/figurines")
    cols = [fields[f] for f in fields]
    for lang in LANGS:
        doc = tr.get(lang)
        if doc is None:
            continue
        items = keyed(doc.get("items", []), p, f"content/{lang}/figurines")
        for key in sorted(items):
            it = items[key]
            where = f"content/{lang}/figurines.json [{key}]"
            if key not in src_items:
                p.err(f"{where}: no such key in source/figurines.json")
                continue
            extra = set(it) - set(fields) - META
            if extra:
                p.err(f"{where}: unknown fields {sorted(extra)}")
            vals = {}
            for f in fields:
                v = it.get(f)
                if v is None or text_of(v) == "":
                    if text_of(src_items[key].get(f)):
                        p.warn(f"{lang}/figurines [{key}].{f}: no translation")
                    continue
                col = fields[f]
                if ("figurine_translations", col) not in limits:
                    p.err(f"{where}: figurine_translations.{col} has no CHECK (schema missing?)")
                    continue
                if check_text(p, f"{where}.{f}", v, limits[("figurine_translations", col)], lang):
                    vals[col] = v
            if not vals:
                continue
            used = [c for c in cols if c in vals]
            sel = []
            for c in used:
                sel += [", ", Q(vals[c])]
            upd = join([[f"{c} = COALESCE(NULLIF(figurine_translations.{c}, ''), EXCLUDED.{c})"] for c in used],
                       ",\n       ")
            stmt = ([f"-- {key} ({lang})\nINSERT INTO figurine_translations (figurine_id, lang, {', '.join(used)})\n"
                     f"SELECT id, {lit(lang)}"] + sel +
                    [f"\n  FROM figurines WHERE slug = {lit(key)}\nON CONFLICT (figurine_id, lang) DO UPDATE\n   SET "] +
                    upd + [";\n"])
            sql.add("figurines", stmt)


def images_kind(p, sql, src, tr, limits):
    src_items = keyed(src.get("items", []), p, "source/figurine_images")
    limit = limits.get(("figurine_image_translations", "alt_text"), 1000)
    uuid = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
    for lang in LANGS:
        doc = tr.get(lang)
        if doc is None:
            continue
        items = keyed(doc.get("items", []), p, f"content/{lang}/figurine_images")
        rows = []
        for key in sorted(items):
            it = items[key]
            where = f"content/{lang}/figurine_images.json [{key}]"
            if key not in src_items:
                p.err(f"{where}: no such key in source/figurine_images.json")
                continue
            if not uuid.match(key):
                p.err(f"{where}: key is not a UUID")
                continue
            extra = set(it) - {"altText"} - META
            if extra:
                p.err(f"{where}: unknown fields {sorted(extra)}")
            v = it.get("altText")
            if text_of(v) == "":
                if text_of(src_items[key].get("altText")):
                    p.warn(f"{lang}/figurine_images [{key}]: no translation")
                continue
            if check_text(p, f"{where}.altText", v, limit, lang):
                rows.append((key, v))
        # One statement per language: a VALUES list joined to `images`, so a
        # photo that is not in this database (ids are the live site's) is skipped.
        if rows:
            vals = []
            for i, (k, v) in enumerate(rows):
                vals += [("" if i == 0 else ",\n       ") + f"({lit(k)}::uuid, ", Q(v), ")"]
            stmt = ([f"-- photo descriptions ({lang}): {len(rows)}\n"
                     f"INSERT INTO figurine_image_translations (image_id, figurine_id, lang, alt_text)\n"
                     f"SELECT i.id, i.figurine_id, {lit(lang)}, t.alt\n  FROM (VALUES "] + vals +
                    [") AS t(id, alt)\n  JOIN images i ON i.id = t.id\nON CONFLICT (image_id, lang) DO NOTHING;\n"])
            sql.add("figurine_images", stmt)
            sql.counts["figurine_images"] = sql.counts.get("figurine_images", 1) - 1 + len(rows)


# ── output ──

HEAD_CONTENT = """-- {langs} for the data texts: tales, gazette leaves,
-- cards (with ability and trait names inside the JSON), races, keywords,
-- etudes, errands, frame names, motion names, copy overrides.
--
-- GENERATED by tools/i18n/make_migration.py from tools/i18n/source/ and
-- tools/i18n/content/{dirs}/. Do not edit by hand.
--
-- Safe to run more than once and safe next to the keeper's own work:
--   * a row is found by its permanent key (slug, frame tier, motion id), not
--     by id: ids differ between the live site and a developer's database;
--   * a column (or a field inside JSON) is filled only while it is empty, so a
--     text the keeper has already written or corrected is never overwritten;
--   * `updated_at` is left alone, so the translation is not reported as an edit;
--   * a key that does not exist in this database simply matches no row.
-- Copy overrides are translated only where the keeper overrode the English text.
--
-- sqlx keeps a checksum of every applied migration: this file must not change
-- once applied anywhere. For newer translations generate a NEW file with a
-- later stamp: make_migration.py --stamp YYYYMMDDHHMMSS [--only kind,...].
"""

HEAD_FIGURINES = """-- {langs} for the works and their photo descriptions:
-- rows of figurine_translations and figurine_image_translations
-- (schema: 20261020000001_figurine_translations.sql).
--
-- GENERATED by tools/i18n/make_migration.py from tools/i18n/source/ and
-- tools/i18n/content/{dirs}/. Do not edit by hand.
--
-- Safe to run more than once and safe next to the keeper's own work:
--   * a work is found by its slug; a slug missing in this database inserts nothing;
--   * an existing translation row keeps every non-empty column (COALESCE), only
--     empty ones are filled, and its `updated_at` is left alone;
--   * a photo is found by its id, which is the live site's: the rows are joined
--     to `images`, so on a database without those photos nothing is inserted;
--     an existing description is kept (ON CONFLICT DO NOTHING).
--
-- sqlx keeps a checksum of every applied migration: this file must not change
-- once applied anywhere. For newer translations generate a NEW file with a
-- later stamp: make_migration.py --stamp YYYYMMDDHHMMSS [--only kind,...].
"""


HEAD_SITE = """-- Home page texts and showing room names in the other languages:
-- JSON keys `kickerEs` … in app_resources.home_content, columns name_es … of
-- showing_rooms (schema: 20261021000000_showing_room_names.sql).
--
-- GENERATED by tools/i18n/make_migration.py from tools/i18n/source/site.json and
-- tools/i18n/content/<lang>/site.json. Do not edit by hand.
--
-- Safe to run more than once and safe next to the keeper's own work:
--   * a translation is written only while it is empty;
--   * and only while the English it was made from is still the text on the
--     site (the keeper changed the English: the translation is stale, skipped);
--   * a showing room is found by its id, which is the live site's: on another
--     database nothing matches.
--
-- sqlx keeps a checksum of every applied migration: this file must not change
-- once applied anywhere. For newer translations generate a NEW file:
-- make_migration.py --site-stamp YYYYMMDDHHMMSS --only site.
"""


def render(head, sql, tagbase):
    texts = [p[1] for st in sql.parts for p in st if isinstance(p, tuple)]
    tag = pick_tag(tagbase, texts)
    stmts = ["".join(f"${tag}${p[1]}${tag}$" if isinstance(p, tuple) else p for p in st) for st in sql.parts]
    return head, tag, stmts


def write_files(head, tag, stmts, name, stamp, out_dir, dry):
    """Split into parts under SPLIT_BYTES; returns [(path, bytes, statements)]."""
    note = f"--\n-- Texts are dollar-quoted (${tag}$ ... ${tag}$); the tag occurs in no text.\n"
    parts, cur, size = [], [], 0
    for s in stmts:
        b = len(s.encode()) + 1
        if cur and size + b > SPLIT_BYTES:
            parts.append(cur)
            cur, size = [], 0
        cur.append(s)
        size += b
    if cur:
        parts.append(cur)
    out = []
    for i, part in enumerate(parts):
        suffix = "" if len(parts) == 1 else f"_part{i + 1}"
        path = out_dir / f"{stamp}_{name}{suffix}.sql"
        body = head + note + ("" if len(parts) == 1 else f"-- Part {i + 1} of {len(parts)}.\n") + \
            "\n" + "\n".join(part)
        if not dry:
            if path.exists() and path.read_text() != body and not FORCE:
                sys.exit(f"refusing to rewrite {path.name}: it differs from what is on disk, and an applied "
                         f"migration must never change (sqlx checksums). Generate a new file with "
                         f"--stamp, or pass --force if you are sure it is applied nowhere.")
            path.write_text(body)
        out.append((path, len(body.encode()), len(part)))
        stamp = next_stamp(stamp)
    return out, stamp


FORCE = False


def next_stamp(stamp):
    return (datetime.strptime(stamp, "%Y%m%d%H%M%S") + timedelta(seconds=1)).strftime("%Y%m%d%H%M%S")


def main():
    global LANGS
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--stamp", default=DEFAULT_STAMP, help="timestamp of the first file (YYYYMMDDHHMMSS)")
    ap.add_argument("--only", default="", help=f"comma-separated kinds: {','.join(ALL_KINDS)}")
    ap.add_argument("--langs", default=",".join(LANGS),
                    help="comma-separated languages (default es,de,it; ru,uk only with --only figurines,figurine_images)")
    ap.add_argument("--site-stamp", default=DEFAULT_SITE_STAMP,
                    help="timestamp of the site texts file (after 20261021000000_showing_room_names)")
    ap.add_argument("--check", action="store_true", help="check only, write nothing")
    ap.add_argument("--force", action="store_true",
                    help="allow rewriting an existing migration file that differs (only if applied nowhere)")
    ap.add_argument("--strict", action="store_true", help="missing translations are errors too")
    ap.add_argument("--out", default=str(MIGRATIONS), help="directory for the files")
    ap.add_argument("--quiet", action="store_true", help="do not list the missing translations")
    a = ap.parse_args()
    global FORCE
    FORCE = a.force
    if not re.fullmatch(r"\d{14}", a.stamp):
        sys.exit("--stamp must be YYYYMMDDHHMMSS")
    only = [k for k in a.only.split(",") if k]
    bad = [k for k in only if k not in ALL_KINDS]
    if bad:
        sys.exit(f"unknown kinds: {bad}; known: {', '.join(ALL_KINDS)}")
    kinds = only or list(ALL_KINDS)
    langs = tuple(l for l in a.langs.split(",") if l)
    unknown = [l for l in langs if l not in LANG_NAMES]
    if unknown or not langs or len(set(langs)) != len(langs):
        sys.exit(f"--langs: unknown or repeated languages {unknown or langs}; known: {', '.join(LANG_NAMES)}")
    if any(l in EXTRA_LANGS for l in langs):
        if not only or any(k not in FIGURINE_KINDS for k in only):
            sys.exit(f"--langs {','.join(l for l in langs if l in EXTRA_LANGS)}: only with "
                     f"--only {','.join(FIGURINE_KINDS)} (elsewhere ru is the original and uk has its own migrations)")
    LANGS = langs
    names = [LANG_NAMES[l] for l in LANGS]
    lang_words = names[0] if len(names) == 1 else ", ".join(names[:-1]) + " and " + names[-1]
    suffix = "_".join(LANGS)

    p = Problems()
    limits = sql_limits()
    errands = seeded_errand_slugs()

    def load(kind):
        src = read_json(SOURCE / f"{kind}.json")
        tr = {lang: read_json(CONTENT / lang / f"{kind}.json") for lang in LANGS}
        for lang, d in tr.items():
            if d is not None and d.get("lang") not in (None, lang):
                p.err(f"content/{lang}/{kind}.json says lang {d.get('lang')!r}")
        return src, tr

    content, figs, site = Sql(), Sql(), Sql()
    for kind in kinds:
        src, tr = load(kind)
        if src is None:
            if any(tr.values()):
                p.err(f"content for {kind} but no source/{kind}.json")
            continue
        if kind in ("tales", "gazette"):
            where = " AND kind = 'tale'" if kind == "tales" else " AND kind <> 'tale'"
            column_kind(p, content, kind, src, tr, limits, where)
        elif kind in ("cards", "races", "keywords", "challenges"):
            column_kind(p, content, kind, src, tr, limits)
        elif kind == "errands":
            for lang, d in tr.items():
                for it in (d or {}).get("items", []):
                    if str(it.get("key")) not in errands:
                        p.err(f"content/{lang}/errands.json [{it.get('key')}]: no such errand in the migrations")
            column_kind(p, content, kind, src, tr, limits)
        elif kind == "frames":
            settings_array_kind(p, content, kind, src, tr, "battle_frames", "frames", "tier", FRAME_NAME_MAX, True)
        elif kind == "motions":
            settings_array_kind(p, content, kind, src, tr, "battle_motions", "motions", "id", MOTION_NAME_MAX,
                                False)
        elif kind == "copy":
            copy_kind(p, content, src, tr)
        elif kind == "site":
            site_kind(p, site, src)
        elif kind == "figurines":
            figurines_kind(p, figs, src, tr, limits)
        elif kind == "figurine_images":
            images_kind(p, figs, src, tr, limits)

    if p.warnings and not a.quiet:
        print(f"warnings ({len(p.warnings)}):")
        for w in p.warnings:
            print("  " + w)
    if a.strict:
        p.errors += [w for w in p.warnings if "no translation" in w]
    if p.errors:
        print(f"\nERRORS ({len(p.errors)}), nothing written:", file=sys.stderr)
        for e in p.errors:
            print("  " + e, file=sys.stderr)
        sys.exit(1)

    stamp = a.stamp
    out_dir = Path(a.out)
    written = []
    dirs = "{" + ",".join(LANGS) + "}" if len(LANGS) > 1 else LANGS[0]
    for head, sql, name, tagbase in ((HEAD_CONTENT, content, f"content_{suffix}", "tr"),
                                     (HEAD_FIGURINES, figs, f"figurine_translations_{suffix}", "tr")):
        head = head.replace("{langs}", lang_words).replace("{dirs}", dirs)
        if not sql.parts:
            continue
        h, tag, stmts = render(head, sql, tagbase)
        files, stamp = write_files(h, tag, stmts, name, stamp, out_dir, a.check)
        written.append((files, sql.counts))
    # Site texts go into their own file with their own stamp: the columns of
    # showing_rooms appear only in 20261021000000, after the content files.
    if site.parts:
        h, tag, stmts = render(HEAD_SITE, site, "tr")
        files, _ = write_files(h, tag, stmts, "site_translations", a.site_stamp, out_dir, a.check)
        written.append((files, site.counts))
    print()
    for files, counts in written:
        for path, size, n in files:
            print(f"{'would write' if a.check else 'wrote'} {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path}"
                  f"  {size / 1024:.0f} KiB, {n} statements")
        print(f"  rows per kind ({', '.join(LANGS)}): " + ", ".join(f"{k} {v}" for k, v in counts.items()))
    if not written:
        print("nothing to write")


if __name__ == "__main__":
    main()
