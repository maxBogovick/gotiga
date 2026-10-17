"""The card set made from the finished pictures in ~/Downloads/рамки.

One place for everything that differs per card, so the same set can be built on
the local database and, with an admin key, on the server: `build.py` reads
this file and nothing else.

Names are plain words (CLAUDE.md § 1.1, § 9.11). Effects say exactly what the
numbers and abilities do — nothing on a card is decoration.
"""

# Races. `hue` turns the cost medallion (violet at 270°) to the race colour.
RACES = [
    # slug, EN, RU, UK, note RU, cost-medallion hue
    ("people", "People", "Люди", "Люди", "Обычные жители: у них нет чар, зато есть выучка.", 45),
    ("gnomes", "Gnomes", "Гномы", "Гноми", "Низкие и крепкие, живут у камня и огня.", 110),
    ("witches", "Witches", "Ведьмы", "Відьми", "Колдуют проклятиями и ядом.", 320),
    ("mages", "Mages", "Маги", "Маги", "Колдуют по приказу и бьют издалека.", 210),
    ("undead", "Undead", "Нежить", "Нежить", "Умершие, которые продолжают действовать.", 270),
    ("water", "Water Folk", "Водяные", "Водяні", "Живут в воде, на берегу слабеют.", 170),
]

# Who each race is strong against (+1 to blows and spells). A ring: every race
# is strong against one and weak against one, so no race wins by itself.
RACE_RING = {
    "gnomes": "mages",
    "mages": "undead",
    "undead": "people",
    "people": "witches",
    "witches": "water",
    "water": "gnomes",
}

# Motions already in the local set (settings.battle_motions).
M_SWORD = "45e36212-6453-46a0-802d-58fc0a6338fb"
M_AXE = "8b53c732-860a-47af-9fa3-237d8f60fe3b"
M_AXE_CHOP = "50cfb01b-0ae5-433e-83ca-e7b019f0751e"
M_MACE = "c7295337-2d41-403d-a381-66342182621a"
M_LIGHTNING = "9f4af22a-9521-4fa8-8831-57f4d580c820"
M_RAVENS = "2e716234-4631-4009-8279-978c7b713e32"
M_POISON = "d9729383-0fd0-4c06-9c92-fec7f2849caa"
M_SHOT = "e9ff567d-9979-422d-a9b5-ea76cbbcd07d"
M_HEAL = "05c7b9de-f4b3-4ed2-a32a-7995020ac396"


def wear(blow, spell=None, mend=None):
    out = {"blow": blow}
    if spell:
        out["spell"] = spell
    if mend:
        out["mend"] = mend
    return out


def ab(id, en, ru, uk, verb, amount, shape="one", range=1, duration=0, trigger="active",
       mana=0, cooldown=0, channel="magic", stat="", radius=0):
    return {
        "id": id, "nameEn": en, "nameRu": ru, "nameUk": uk, "verb": verb, "channel": channel,
        "amount": amount, "shape": shape, "radius": radius, "range": range, "duration": duration,
        "trigger": trigger, "manaCost": mana, "cooldown": cooldown, "keywords": [], "stat": stat,
        "summon": "",
    }


# file: picture in ~/Downloads/рамки; work: slug of the work on the site.
# palette: (cost hue is the race's), health is always red, power always amber.
CARDS = [
    dict(slug="captain-salazar", file="salazar.png", work="art-doll-captainsalazar", race="undead", tier=3,
         en="Captain Salazar", ru="Капитан Салазар", uk="Капітан Салазар",
         effect=("Marks an enemy: for 2 turns it takes 2 more damage.",
                 "Метит врага: 2 хода тот получает на 2 урона больше.",
                 "Позначає ворога: 2 ходи той отримує на 2 шкоди більше."),
         cost=4, power=4, health=6, mana=10, armor=1, ward=2, channel="magic", reach=3,
         abilities=[ab("mark", "Hunter's Mark", "Метка охотника", "Мітка мисливця", "curse", 2, range=3,
                       duration=2, mana=4, cooldown=2, stat="vulnerable")],
         price_dust=40, motion=wear(M_LIGHTNING, M_LIGHTNING)),

    # ── Люди ──
    dict(slug="hunchback", file="ChatGPT Image 2 окт. 2026 г., 16_54_17.png", work="small-art-doll-quazimodo",
         race="people", tier=2, en="Hunchback", ru="Горбун", uk="Горбань",
         effect=("Stands firm: blows against him weigh 1 less.",
                 "Стоит крепко: удары по нему слабее на 1.",
                 "Стоїть міцно: удари по ньому слабші на 1."),
         cost=2, power=4, health=7, mana=0, armor=1, ward=0, channel="physical", reach=1,
         abilities=[], price_dust=20, motion=wear(M_SWORD)),
    dict(slug="piper", file="dudoxhnik.png", work="small-art-doll-pied-piper", race="people", tier=1,
         en="The Ratcatcher", ru="Крысолов", uk="Щуролов",
         effect=("Plays a tune: an ally nearby deals 1 more for 2 turns.",
                 "Играет мелодию: союзник рядом 2 хода бьёт на 1 сильнее.",
                 "Грає мелодію: союзник поруч 2 ходи б'є на 1 сильніше."),
         cost=1, power=3, health=5, mana=4, armor=0, ward=0, channel="physical", reach=1,
         abilities=[ab("tune", "Tune", "Мелодия", "Мелодія", "bless", 1, range=2, duration=2, mana=2,
                       cooldown=1, stat="power")],
         price_dust=10, motion=wear(M_SWORD)),
    dict(slug="bagpiper", file="volunka.png", work="small-art-doll-piper", race="people", tier=1,
         en="Highland Piper", ru="Горный волынщик", uk="Гірський волинщик",
         effect=("A loud drone: an enemy in range deals 1 less for 2 turns.",
                 "Громкий гул: враг в дальности 2 хода бьёт на 1 слабее.",
                 "Гучний гул: ворог у дальності 2 ходи б'є на 1 слабше."),
         cost=1, power=3, health=5, mana=4, armor=0, ward=0, channel="physical", reach=1,
         abilities=[ab("drone", "Drone", "Гул", "Гул", "curse", 1, range=2, duration=2, mana=2,
                       cooldown=1, stat="power")],
         price_dust=10, motion=wear(M_MACE)),
    dict(slug="falconer", file="green-indeec.png", work="small-art-doll-falconer", race="people", tier=2,
         en="Falconer", ru="Сокольничий", uk="Сокільничий",
         effect=("Shoots from a distance: hits up to 3 cells away.",
                 "Стреляет издалека: бьёт на 3 клетки.",
                 "Стріляє здалеку: б'є на 3 клітинки."),
         cost=1, power=4, health=5, mana=0, armor=0, ward=0, channel="physical", reach=3, shoots=True,
         abilities=[], price_dust=20, motion=wear(M_SHOT)),
    dict(slug="granny-with-a-sheep", file="grand_and_ship.png", work="small-doll-granny-with-her-sheep",
         race="people", tier=1, en="Old Shepherdess", ru="Старая пастушка", uk="Стара пастушка",
         effect=("Heals an ally nearby for 2.",
                 "Лечит союзника рядом на 2.",
                 "Лікує союзника поруч на 2."),
         cost=1, power=2, health=6, mana=4, armor=0, ward=0, channel="physical", reach=1,
         abilities=[ab("heal", "Heal", "Лечение", "Лікування", "heal", 2, range=2, mana=2, cooldown=1)],
         price_dust=10, motion=wear(M_MACE, mend=M_HEAL)),

    # ── Гномы ──
    dict(slug="forest-gnome", file="gnome_in_forest.png", work="small-doll-gnome", race="gnomes", tier=2,
         en="Woodsman", ru="Лесовик", uk="Лісовик",
         effect=("Sturdy: blows against him weigh 1 less.",
                 "Крепкий: удары по нему слабее на 1.",
                 "Міцний: удари по ньому слабші на 1."),
         cost=1, power=3, health=7, mana=0, armor=1, ward=0, channel="physical", reach=1,
         abilities=[], price_dust=20, motion=wear(M_MACE)),
    dict(slug="miner", file="gnome_in_miner.png", work="small-art-doll-gnome-2", race="gnomes", tier=2,
         en="Delver", ru="Рудокоп", uk="Рудокоп",
         effect=("Hits hard with a pick.",
                 "Бьёт киркой тяжело.",
                 "Б'є кіркою важко."),
         cost=1, power=5, health=6, mana=0, armor=0, ward=0, channel="physical", reach=1,
         abilities=[], price_dust=20, motion=wear(M_AXE)),
    dict(slug="gnome-axeman", file="gnome_war.png", work="small-art-doll-gnome-with-axe", race="gnomes", tier=3,
         en="Axebearer", ru="Секироносец", uk="Сокироносець",
         effect=("A heavy axe: strong blows, thick armor.",
                 "Тяжёлая секира: сильный удар и плотная броня.",
                 "Важка сокира: сильний удар і щільна броня."),
         cost=3, power=5, health=6, mana=0, armor=2, ward=0, channel="physical", reach=1,
         abilities=[], price_dust=40, motion=wear(M_AXE_CHOP)),
    dict(slug="gnome-with-a-pot", file="gnome_with_food.png", work="small-art-doll-gnome-with-gold",
         race="gnomes", tier=1, en="The Cook", ru="Кашевар", uk="Куховар",
         effect=("Warm food: heals an ally nearby for 1 each turn, for 3 turns.",
                 "Горячая еда: лечит союзника рядом на 1 каждый ход, 3 хода.",
                 "Гаряча їжа: лікує союзника поруч на 1 щоходу, 3 ходи."),
         cost=1, power=2, health=6, mana=4, armor=0, ward=0, channel="physical", reach=1,
         abilities=[ab("stew", "Stew", "Похлёбка", "Юшка", "hot", 1, range=2, duration=3, mana=2, cooldown=2)],
         price_dust=10, motion=wear(M_MACE, mend=M_HEAL)),

    # ── Ведьмы ──
    dict(slug="old-witch", file="witch-1.png", work="small-doll-witch", race="witches", tier=2,
         en="Grey Witch", ru="Седая ведьма", uk="Сива відьма",
         effect=("Weakens an enemy: it deals 2 less for 2 turns.",
                 "Ослабляет врага: 2 хода тот бьёт на 2 слабее.",
                 "Послаблює ворога: 2 ходи той б'є на 2 слабше."),
         cost=2, power=3, health=5, mana=6, armor=0, ward=1, channel="magic", reach=3,
         abilities=[ab("hex", "Hex", "Порча", "Псування", "curse", 2, range=3, duration=2, mana=3,
                       cooldown=2, stat="power")],
         price_dust=20, motion=wear(M_LIGHTNING, M_LIGHTNING)),
    dict(slug="witch-with-a-key", file="witch_2.png", work="small-doll-witch-in-black", race="witches", tier=3,
         en="Keyholder Witch", ru="Ведьма-ключница", uk="Відьма-ключниця",
         effect=("Poison: an enemy loses 2 health each turn for 3 turns.",
                 "Яд: враг теряет по 2 здоровья каждый ход, 3 хода.",
                 "Отрута: ворог втрачає по 2 здоров'я щоходу, 3 ходи."),
         cost=3, power=4, health=6, mana=8, armor=0, ward=1, channel="magic", reach=3,
         abilities=[ab("poison", "Poison", "Яд", "Отрута", "dot", 2, range=3, duration=3, mana=4, cooldown=2)],
         price_dust=40, motion=wear(M_POISON, M_POISON)),
    dict(slug="yaga-with-a-bird", file="bird_grand_kyky.png", work="small-baba-yaga-with-bird", race="witches",
         tier=3, en="Little Yaga", ru="Крошка Яга", uk="Крихітка Яга",
         effect=("Hides herself: for 2 turns she cannot be chosen as a target.",
                 "Прячется: на 2 хода её нельзя выбрать целью.",
                 "Ховається: на 2 ходи її не можна обрати ціллю."),
         cost=4, power=5, health=7, mana=6, armor=0, ward=1, channel="magic", reach=2,
         abilities=[ab("hide", "Hide", "Невидимость", "Невидимість", "veil", 0, shape="self", duration=2,
                       mana=3, cooldown=4)],
         price_dust=40, motion=wear(M_RAVENS, M_RAVENS)),

    # ── Маги ──
    dict(slug="wizard-with-a-raven", file="blue-wizard.png", work="tiny-ooak-wizard-art-doll-with-birds",
         race="mages", tier=4, en="Storm Mage", ru="Грозовой маг", uk="Грозовий маг",
         effect=("Lightning jumps between enemies: 3 damage to the first, then along 2 more.",
                 "Молния прыгает между врагами: 3 урона первому и ещё двум по цепочке.",
                 "Блискавка стрибає між ворогами: 3 шкоди першому і ще двом за ланцюжком."),
         cost=4, power=5, health=5, mana=12, armor=0, ward=2, channel="magic", reach=4,
         abilities=[ab("storm", "Chain Lightning", "Цепная молния", "Ланцюгова блискавка", "damage", 3,
                       shape="chain", radius=2, range=4, mana=5, cooldown=2)],
         price_dust=80, motion=wear(M_LIGHTNING, M_LIGHTNING)),
    dict(slug="old-wizard", file="wizard_with_stick.png", work="small-art-doll-wizard", race="mages", tier=3,
         en="Old Wizard", ru="Старый маг", uk="Старий маг",
         effect=("Strengthens an ally: it deals 2 more for 2 turns.",
                 "Усиливает союзника: 2 хода тот бьёт на 2 сильнее.",
                 "Підсилює союзника: 2 ходи той б'є на 2 сильніше."),
         cost=3, power=3, health=6, mana=8, armor=0, ward=2, channel="magic", reach=3,
         abilities=[ab("boon", "Blessing", "Благословение", "Благословення", "bless", 2, range=3, duration=2,
                       mana=3, cooldown=2, stat="power")],
         price_dust=40, motion=wear(M_LIGHTNING, M_LIGHTNING)),
    dict(slug="shaman", file="shaman.jpg", work="small-art-doll-shaman", race="mages", tier=3,
         en="Shaman", ru="Шаман", uk="Шаман",
         effect=("Heals an ally in range for 3.",
                 "Лечит союзника в дальности на 3.",
                 "Лікує союзника у дальності на 3."),
         cost=2, power=3, health=7, mana=8, armor=0, ward=1, channel="magic", reach=2,
         abilities=[ab("mend", "Mend", "Исцеление", "Зцілення", "heal", 3, range=2, mana=3, cooldown=1)],
         price_dust=40, motion=wear(M_MACE, mend=M_HEAL)),

    # ── Водяные ──
    dict(slug="steam-mermaid", file="baba-fish.png", work="ooak-steampunk-mermaid-art-doll", race="water",
         tier=2, en="Sea Maiden", ru="Морская дева", uk="Морська діва",
         effect=("Holds one enemy still for 1 turn.",
                 "Удерживает одного врага на месте 1 ход.",
                 "Утримує одного ворога на місці 1 хід."),
         cost=3, power=3, health=6, mana=6, armor=0, ward=1, channel="magic", reach=2,
         abilities=[ab("hold", "Hold", "Захват", "Захват", "control", 0, range=2, duration=1, mana=4,
                       cooldown=3)],
         price_dust=20, motion=wear(M_POISON, M_POISON)),
    dict(slug="old-mermaid", file="mermaid_old.png", work="ooak-old-mermaid-art-doll", race="water", tier=3,
         en="Silent Mermaid", ru="Безмолвная русалка", uk="Безмовна русалка",
         effect=("Silences one enemy for 2 turns: it cannot use abilities.",
                 "Лишает одного врага способностей на 2 хода.",
                 "Позбавляє одного ворога здібностей на 2 ходи."),
         cost=4, power=4, health=6, mana=8, armor=0, ward=2, channel="magic", reach=3,
         abilities=[ab("hush", "Hush", "Немота", "Німота", "silence", 0, range=3, duration=2, mana=4,
                       cooldown=3)],
         price_dust=40, motion=wear(M_LIGHTNING, M_LIGHTNING)),

    # ── Нежить ──
    dict(slug="old-vampire", file="vampir-funny.png", work="small-doll-vampire", race="undead", tier=2,
         en="Old Vampire", ru="Старый вампир", uk="Старий вампір",
         effect=("Every blow that lands heals him for 2.",
                 "Каждый удар, достигший цели, лечит его на 2.",
                 "Кожен удар, що влучив, лікує його на 2."),
         cost=1, power=4, health=5, mana=0, armor=0, ward=0, channel="physical", reach=1,
         abilities=[ab("sip", "A Sip", "Глоток", "Ковток", "heal", 2, shape="self", trigger="onHit",
                       channel="physical")],
         price_dust=20, motion=wear(M_RAVENS)),
    dict(slug="dark-vampire", file="vampir.png", work="small-authors-doll-tiny-vapmire", race="undead", tier=3,
         en="Dark Vampire", ru="Тёмный вампир", uk="Темний вампір",
         effect=("Bleeding: an enemy loses 2 health each turn for 2 turns.",
                 "Кровотечение: враг теряет по 2 здоровья каждый ход, 2 хода.",
                 "Кровотеча: ворог втрачає по 2 здоров'я щоходу, 2 ходи."),
         cost=3, power=5, health=6, mana=6, armor=0, ward=1, channel="physical", reach=1,
         abilities=[ab("bite", "Bite", "Укус", "Укус", "dot", 2, range=1, duration=2, mana=3, cooldown=2,
                       channel="physical")],
         price_dust=40, motion=wear(M_RAVENS)),
    dict(slug="red-vampire", file="vampir_2.png", work="small-doll-little-vampire", race="undead", tier=3,
         en="Crimson Vampire", ru="Багровый вампир", uk="Багряний вампір",
         effect=("Every blow that lands heals him for 2. Armor 1.",
                 "Каждый удар, достигший цели, лечит его на 2. Броня 1.",
                 "Кожен удар, що влучив, лікує його на 2. Броня 1."),
         cost=2, power=4, health=6, mana=0, armor=1, ward=0, channel="physical", reach=1,
         abilities=[ab("sip", "A Sip", "Глоток", "Ковток", "heal", 2, shape="self", trigger="onHit",
                       channel="physical")],
         price_dust=40, motion=wear(M_RAVENS)),
    dict(slug="the-creature", file="frankensh.png", work="small-art-doll-frankenstein-s-creature", race="undead",
         tier=3, en="The Stitched", ru="Сшитый", uk="Зшитий",
         effect=("Made of parts of other bodies: blows against him weigh 1 less.",
                 "Собрано из частей других тел: удары по нему слабее на 1.",
                 "Зібране з частин інших тіл: удари по ньому слабші на 1."),
         cost=3, power=5, health=7, mana=0, armor=1, ward=0, channel="physical", reach=1,
         abilities=[], price_dust=40, motion=wear(M_MACE)),
]

# Where each stat medallion sits. Mana is the fourth socket every picture has.
CORNERS = {"cost": "tl", "health": "tr", "power": "br", "mana": "bl"}
HEALTH_HUE = 0
POWER_HUE = 38
MANA_HUE = 0
