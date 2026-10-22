# Исправления текстов прода

Ошибки в английских текстах работ, подписях фото и заметках рас, найденные при
переводе (выгрузка `tools/i18n/source/`, 20.10.2026). Переводчикам ru/uk:
переводите **исправленный** текст (поле `to`), а не то, что лежит в `source/`.

| Файл | Что | Ключ |
|---|---|---|
| `figurines_en.json` | английские колонки `figurines` | слаг → поле → `{from, to}` |
| `figurine_images_en.json` | `images.alt_text` | id фото → `{from, to}` |
| `races.json` | `battle_races.note_en`, `note_uk`; удаление дубля | слаг → колонка → `{from, to}` |

Миграция: `python3 tools/i18n/make_fixes.py` →
`src-tauri/server/migrations/20261022000001_fix_texts.sql`. Значение меняется,
только пока в базе лежит ровно `from`: правку автора, сделанную после выгрузки,
миграция не трогает. `updated_at` не меняется. Повторный прогон ничего не меняет.
Новые исправления — новым файлом (`--stamp`), применённый файл не правят.

## Три работы с противоречивыми текстами — что на фото

- **`small-authors-doll-man-with-the-dog`** — на всех 17 фото рядом со стариком
  сидит лохматая тёмная **собака**: висячие уши, пушистый хвост, сидит по-собачьи.
  Название и краткий текст правы, в полном описании «rat companion» и «the rat» —
  ошибка. Исправлено на «faithful dog» / «the dog». В кратком тексте «Eldery» →
  «Elderly».
- **`small-art-doll-gnome-with-axe`** — **гном**: красный колпак, длинная белая
  борода, топор. «figure of troll» в кратком тексте → «figure of a gnome».
- **`small-doll-gnome-with-fancy-bag`** — правы оба текста наполовину: за спиной
  у гнома **пёстрый тканый рюкзак, полный хвороста** (вязанка сучьев торчит из
  рюкзака), в руке посох. Краткий текст: «Small doll with bag of brushwood» →
  «Small doll Gnome with a colorful backpack full of brushwood». Полное описание:
  «filled with mysterious treasures» → «filled with brushwood». Там же
  «Approximately10-11» → «Approximately 10-11».

Проверены по фото и оставлены как есть: `small-doll-pirate-with-parrot` (в
описании «tiny bird» — на фото зелёный попугай, это не ошибка);
`small-doll-granny-with-bread` (в названии хлеб, в описании пирог — на фото
круглая выпечка с решёткой, годится и то и другое).

## Прочие исправления в английском (опечатки)

Названия:
- `small-doll-with-with-nest`: «Small doll With with nest» → «Small doll Witch with nest»
- `art-doll-captainsalazar`: «CaptainSalazar» → «Captain Salazar»
- `small-authors-doll-tiny-vapmire`: «Vapmire» → «Vampire»
- `small-art-doll-quazimodo`: «Quazimodo» → «Quasimodo» (и в кратком тексте)
- `witch-with-scull`: «Witch with scull » → «Witch with skull» (и в кратком тексте)

Краткие тексты:
- `small-doll-gnome-woman`, `small-art-doll-gnome-2`: двойной пробел
- `small-doll-witch-2`: «hadcraft» → «handcrafted»
- `small-doll-sweet-baba-yaga`: «dresed» → «dressed»
- `miniature-book-fairy-tales`: «Miniuature» → «Miniature»
- `small-doll-granny-with-her-sheep`: «. sculpted» → «. Sculpted»
- `small-doll-witch-in-black`: «polymer clay ,» → «polymer clay,»
- `small-art-doll-piper`: «scottish closing» → «Scottish clothing»
- `small-art-doll-pied-piper`: «one f a kind» → «one of a kind»
- `small-authors-doll-tiny-vapmire`: «pained» → «painted»
- `small-doll-gnome-with-bag`: «bag.Sculpted» → «bag. Sculpted»
- `small-doll-indian-man`: «Heigh is» → «Height is»
- `small-doll-little-vampire`: «acrylics ad dressed» → «acrylics and dressed»
- `small-doll-granny-with-chicken`: «3 chicken» → «3 chickens»
- `small-doll-granny`: «bag of potato» → «bag of potatoes»

Полные описания:
- `small-art-doll-tailor`: два двойных пробела
- `small-doll-witch-2`: «details , while» → «details, while»
- `small-doll-with-with-nest`: «a enchanting» → «an enchanting»
- `small-doll-granny-with-bread`: «Its a unique» → «It's a unique»

Материал: `granny-with-cats` «acryilcs» → «acrylics».

Размеры в текстах сверены с полем `dimensions` и между собой — расхождений нет.
Стиль автора («Small authors doll», «custom doll» у `small-art-doll-lady-agatha`)
не правился.

## Подписи фото

219 подписей лежали в английском поле на русском и с машинными оборотами
(«изготовленная на заказ» — неверный перевод OOAK: работы не делаются на заказ).
Заменены на короткие английские по образцу «Small art doll Grandma at the kitchen
table with a rolling pin, dough and tiny pastries, handmade one-of-a-kind, photo 1»:
что на фото, «handmade one-of-a-kind», номер фото сохранён. Подпись одна на работу
(20 работ), отличаются номером. Переводы es/de/it подписей уже были без «на заказ»
и не менялись.

## Расы

- `note_en` и `note_uk` у шести рас были русскими. Написаны английские и
  украинские (украинские — с ’, «чари» как в `uk.ts`):
  witches «They cast curses and poison.» / «Чаклують прокляттями й отрутою.»;
  water «They live in water and grow weaker on the shore.» / «Живуть у воді, на
  березі слабшають.»; gnomes «Short and sturdy, they live by stone and fire.» /
  «Низькі й міцні, живуть біля каменю й вогню.»; people «Ordinary folk: no spells,
  but they have training.» / «Звичайні мешканці: чар у них немає, зате є вишкіл.»;
  mages «They cast spells on command and strike from afar.» / «Чаклують за наказом
  і б’ють здалеку.»; undead «The dead who keep on acting.» / «Померлі, які
  продовжують діяти.»
- Раса `witch` дублирует `witches` (0 карт). Миграция удаляет её, только если на
  неё не ссылается ни карта, ни раса студии, ни таблица урона другой расы **и**
  автор её не одевал (нет загруженных картинок в рамках по уровням, нет значка).
  **На проде у `witch` есть рамки по уровням (≈14 КБ, картинки со склада), у
  `witches` их нет**, поэтому там миграция её не удалит: решать автору —
  перенести рамки на `witches` или удалить `witch` вручную.

## Переводы es/de/it

В `content/{es,de,it}/figurines.json` те же три места исправлены по английскому:
крыса → собака (`small-authors-doll-man-with-the-dog`), тролль → гном в кратком
тексте (`small-art-doll-gnome-with-axe`), «вязанка хвороста» → «пёстрый рюкзак,
полный хвороста» в кратком тексте и «таинственные сокровища» → «хворост» в
описании (`small-doll-gnome-with-fancy-bag`). Опечатки английского переводчики
уже не переносили. Файлы `20261020000002/3` перегенерированы.
