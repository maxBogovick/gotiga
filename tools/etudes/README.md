# Этюды

Позиции с решением. Данные — `etudes.py` (расстановка обеих сторон слагами карт,
местность, правила, тексты на трёх языках, награды). Порядок на карте — `sort_order`.

## Проверка ботами

Жадный бот (глубина 1) должен этюд проигрывать, думающий (2–3) выигрывать: значит,
решение есть и оно не первая мысль. Первый этюд-обучение может выигрываться и
жадным.

```bash
export ADMIN_KEY=$(grep -E '^ADMIN_API_KEY=' src-tauri/server/.env | cut -d= -f2-)   # ключ ЛОКАЛЬНОЙ базы
python3 tools/etudes/check.py                       # все этюды
python3 tools/etudes/search.py healer 300 1         # найти новые позиции по теме (melee range healer curse ring terrain altar breakthrough)
```

Проверяльщик: `src-tauri/server/examples/etude_check.rs`
(`cargo build --release --example etude_check`). Это ориентир против бота, а не
доказательство, что человек найдёт решение: этюды стоит пройти руками.

**Правка чисел карт может сломать этюд.** После любой правки показателей перегоните проверку.

## На сервер

Сначала карты (`tools/card_set`), потом этюды. Через панель: карточка «Этюды»
(`./deploy/start.sh`): «Собрать пакет этюдов» → «Проверить сервер» → «Залить» →
«Проверить результат». Из терминала:

```bash
python3 tools/etudes/export_package.py
python3 deploy/etude-set/load.py --base https://ritunia.com/api/v1 --dry-run
ADMIN_KEY='ключ' python3 deploy/etude-set/load.py --base https://ritunia.com/api/v1 [--update]
```

Что уже залито, пропускается (`--update` перезаписывает позицию и тексты). Ничего не
удаляется. Локально старые этюды можно скрыть: `tools/etudes/build.py --base … --hide-others`
(на проде не нужен: там старых нет).
