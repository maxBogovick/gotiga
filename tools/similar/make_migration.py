#!/usr/bin/env python3
"""Миграция с выбором похожих работ и список непокрытых работ.

Миграция:
    python3 -I tools/similar/make_migration.py СПИСОК.json ВЫБОР.json [--name суффикс]

  ВЫБОР.json: {"слаг-новой-работы": ["слаг1", "слаг2", "слаг3"], …}
  Три разных слага, самая близкая первой, не сама работа, все — работы из СПИСКА
  (то есть на людях). Файл ложится в src-tauri/server/migrations/ под первым
  свободным номером после всех существующих и печатается вместе со списком для
  ручного выбора в админке.

Непокрытые:
    python3 -I tools/similar/make_migration.py СПИСОК.json --missing

  Работы из списка, у которых нет похожих: слага нет ни в одной миграции
  `*figurine_similar*` и (если прод уже отдаёт `similarIds`) поле пусто.

Файла из migrations/ не правит никогда: sqlx сверяет контрольную сумму
применённых миграций, и правка уже применённой роняет запуск сервера.
"""
import datetime
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MIGRATIONS = ROOT / "src-tauri" / "server" / "migrations"
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def load(path):
    d = json.load(open(path))
    return d["items"] if isinstance(d, dict) else d


def covered_slugs():
    """Слаги, которым уже назначены похожие какой-либо миграцией."""
    out = set()
    for f in MIGRATIONS.glob("*figurine_similar*.sql"):
        for line in f.read_text().splitlines():
            m = re.match(r"\s*\('([a-z0-9-]+)',\s*'", line)
            if m:
                out.add(m.group(1))
    return out


def next_prefix():
    names = sorted(p.name for p in MIGRATIONS.glob("*.sql"))
    last = names[-1].split("_")[0] if names else "0"
    today = datetime.date.today().strftime("%Y%m%d")
    candidate = int(today + "000000")
    return str(max(candidate, int(last) + 1)).zfill(14)


def missing(items):
    done = covered_slugs()
    has_field = any("similarIds" in i for i in items)
    out = []
    for i in items:
        s = i.get("slug")
        if not s or s in done:
            continue
        if has_field and i.get("similarIds"):
            continue
        out.append(i)
    return out, has_field


def main():
    a = sys.argv[1:]
    if not a:
        print(__doc__)
        sys.exit(2)
    items = load(a[0])
    if "--missing" in a:
        rows, has_field = missing(items)
        print(f"Прод отдаёт similarIds: {'да' if has_field else 'нет (функция ещё не выложена)'}")
        print(f"Без похожих: {len(rows)} из {len(items)}")
        for i in rows:
            print(f"{i['slug']}\t{(i.get('name') or '').strip()}\t{i.get('status')}")
        return
    if len(a) < 2:
        print(__doc__)
        sys.exit(2)
    picks = json.load(open(a[1]))
    by_slug = {i.get("slug"): i for i in items}
    bad = []
    for s, ps in picks.items():
        if not SLUG.match(s) or s not in by_slug:
            bad.append(f"{s}: нет в списке работ на людях")
        if not isinstance(ps, list) or len(ps) != 3 or len(set(ps)) != 3:
            bad.append(f"{s}: нужны ровно три разных слага")
            continue
        for p in ps:
            if not SLUG.match(p) or p not in by_slug:
                bad.append(f"{s}: «{p}» нет в списке работ на людях")
            if p == s:
                bad.append(f"{s}: похожей названа сама работа")
    if bad:
        print("Не записано:\n  " + "\n  ".join(bad), file=sys.stderr)
        sys.exit(1)

    suffix = "fill"
    if "--name" in a:
        suffix = re.sub(r"[^a-z0-9_]+", "_", a[a.index("--name") + 1].lower()).strip("_") or "fill"
    prefix = next_prefix()
    path = MIGRATIONS / f"{prefix}_figurine_similar_{suffix}.sql"
    rows = ",\n".join(f"    ('{s}', '{p[0]}', '{p[1]}', '{p[2]}')" for s, p in picks.items())
    path.write_text(f"""-- Похожие работы для работ, добавленных после первого выбора
-- (`20261018000001_figurine_similar_fill.sql`). Подобраны по фото: тип героя,
-- поза и снимок, затем название и состав. Первыми идут самые близкие.
--
-- Работы названы адресами (`slug`): номера в каждой базе свои. Незнакомый адрес
-- пропускается, `ON CONFLICT DO NOTHING` не затирает выбор, уже сделанный
-- автором в админке.
WITH picks (slug, p0, p1, p2) AS (
  VALUES
{rows}
)
INSERT INTO figurine_similar (figurine_id, similar_id, position)
SELECT f.id, t.id, k.pos
  FROM picks p
 CROSS JOIN LATERAL (VALUES (0, p.p0), (1, p.p1), (2, p.p2)) AS k (pos, target)
  JOIN figurines f ON f.slug = p.slug
  JOIN figurines t ON t.slug = k.target AND t.id <> f.id
ON CONFLICT DO NOTHING;
""")
    print(f"Записан {path.relative_to(ROOT)}\n")
    print("Для ручного выбора в админке (Реестр → работа → «Объект» → «Похожие работы»):")
    for s, ps in picks.items():
        print(f"\n{by_slug[s].get('name', s).strip()}")
        for n, p in enumerate(ps, 1):
            print(f"  {n}. {by_slug[p].get('name', p).strip()}")


if __name__ == "__main__":
    main()
