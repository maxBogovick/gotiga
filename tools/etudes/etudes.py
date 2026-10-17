"""The etudes of the game: small positions with a solution.

Every etude lists both sides by card slug. Coordinates: x 0..width-1, y 0 is the
far edge of the keeper (the house) and the player stands at the bottom rows.
On the default 3 x 3 half-field the keeper uses y 0..2 and the player y 3..5.

`check.py` plays each position with bots (greedy depth 1, thinking depth 2-3):
an etude is good when the greedy bot mostly loses and a thinking one wins.
"""


def P(card, x, y):
    return {"card": card, "x": x, "y": y}


def T(x, y, ground):
    return {"cell": {"x": x, "y": y}, "ground": ground}


ETUDES = [
    dict(
        slug="first-meeting", mechanic=None, sort_order=10,
        title=('First Battle', 'Первый бой', 'Перший бій'), note=("Two against two. Defeat both enemies. A blow reaches the next cell; the forest gnome's armor takes 1 off every blow.", 'Двое против двоих. Победите обоих противников. Удар достаёт до соседней клетки, броня лесного гнома снимает 1 с каждого удара.', 'Двоє проти двох. Переможіть обох противників. Удар сягає сусідньої клітинки, броня лісового гнома знімає 1 з кожного удару.'),
        bot_depth=1, reward_dust=20, reward_finish_dust=5,
        setup=dict(
            playerBoard=[P("hunchback", 1, 3), P("piper", 0, 4)], playerHand=[],
            keeperBoard=[P("forest-gnome", 1, 2), P("gnome-with-a-pot", 2, 1)], keeperHand=[],
        )),
    dict(
        slug="blessing", mechanic={"no_abilities": ["piper"]}, sort_order=20,
        title=('The Tune', 'Мелодия', 'Мелодія'), note=("Two against three, and the enemy is stronger. The piper's tune adds 1 to an ally's blow for two turns: spend it on the one who fights.", 'Двое против троих, и враг сильнее. Мелодия дудочника на два хода прибавляет 1 к удару союзника: потратьте её на того, кто бьёт.', "Двоє проти трьох, і ворог сильніший. Мелодія сопілкаря на два ходи додає 1 до удару союзника: витратьте її на того, хто б'є."),
        bot_depth=1, reward_dust=30, reward_finish_dust=5,
        setup=dict(
            playerBoard=[P("piper", 1, 4), P("miner", 2, 4)], playerHand=[],
            keeperBoard=[P("the-creature", 0, 0), P("miner", 0, 2), P("forest-gnome", 2, 0)], keeperHand=[],
        )),
    dict(
        slug="ravines", mechanic={"no_ground": ["ravine"]}, sort_order=30,
        title=('Ravines', 'Овраги', 'Яри'), note=('A ravine cannot be stood on, and a body with step 1 cannot cross it. Four ravines leave the enemy a few ways in: meet him there.', 'На овраг нельзя встать, и тело с шагом 1 его не пересечёт. Четыре оврага оставляют врагу несколько путей: встретьте его там.', 'На яр не можна стати, і тіло з кроком 1 його не перетне. Чотири яри залишають ворогові кілька шляхів: зустріньте його там.'),
        bot_depth=1, reward_dust=35, reward_finish_dust=5,
        setup=dict(
            playerBoard=[P("bagpiper", 0, 5), P("forest-gnome", 1, 5), P("granny-with-a-sheep", 2, 5)], playerHand=[],
            keeperBoard=[P("dark-vampire", 1, 1), P("hunchback", 1, 2), P("gnome-axeman", 2, 0)], keeperHand=[],
            terrain=[T(0, 1, "ravine"), T(1, 3, "ravine"), T(2, 3, "ravine"), T(2, 2, "ravine")],
        )),
    dict(
        slug="high-ground", mechanic={"no_ground": ["hill"]}, sort_order=40,
        title=('High Ground', 'Холм', 'Пагорб'), note=('A shooter standing on a hill reaches one cell further. The falconer reaches three cells; on the hill he outranges everyone.', 'Стрелок, стоящий на холме, бьёт на клетку дальше. Сокольничий бьёт на три клетки, а на холме достаёт дальше всех.', "Стрілець, що стоїть на пагорбі, б'є на клітинку далі. Сокільничий б'є на три клітинки, а на пагорбі сягає далі за всіх."),
        bot_depth=1, reward_dust=40, reward_finish_dust=5,
        setup=dict(
            playerBoard=[P("gnome-with-a-pot", 1, 4), P("falconer", 2, 5)], playerHand=[],
            keeperBoard=[P("dark-vampire", 1, 0), P("hunchback", 2, 0), P("forest-gnome", 0, 0)], keeperHand=[],
            terrain=[T(2, 2, "hill"), T(0, 3, "hill"), T(0, 2, "hill"), T(2, 4, "hill")],
        )),
    dict(
        slug="mire", mechanic={"no_ground": ["mire"]}, sort_order=50,
        title=('The Mire', 'Топь', 'Болото'), note=('A body that enters a mire stops there and leaves one cell at a time. Do not enter it yourself, and make the enemy cross it under fire.', 'Тело, вошедшее в топь, останавливается и выходит по одной клетке. Не входите в неё сами и заставьте врага пересечь её под огнём.', 'Тіло, що ввійшло в болото, зупиняється і виходить по одній клітинці. Не входьте в нього самі й змусьте ворога перетнути його під вогнем.'),
        bot_depth=1, reward_dust=45, reward_finish_dust=5,
        setup=dict(
            playerBoard=[P("falconer", 0, 5), P("forest-gnome", 2, 3)], playerHand=[],
            keeperBoard=[P("gnome-axeman", 0, 0), P("red-vampire", 1, 1), P("forest-gnome", 2, 1)], keeperHand=[],
            terrain=[T(1, 3, "mire"), T(2, 2, "mire")],
        )),
    dict(
        slug="the-healer", mechanic={"no_abilities": ["shaman"]}, sort_order=60,
        title=('The Healer', 'Лекарь', 'Лікар'), note=('The shaman heals an ally within two cells for 3 each turn. The vampire heals himself with every blow: keep the shaman behind him.', 'Шаман каждый ход лечит союзника в двух клетках на 3. Вампир лечит себя каждым ударом: держите шамана за его спиной.', 'Шаман щоходу лікує союзника у двох клітинках на 3. Вампір лікує себе кожним ударом: тримайте шамана за його спиною.'),
        bot_depth=1, reward_dust=50, reward_finish_dust=5,
        setup=dict(
            playerBoard=[P("shaman", 2, 5), P("red-vampire", 1, 4)], playerHand=[],
            keeperBoard=[P("forest-gnome", 0, 2), P("witch-with-a-key", 0, 1), P("red-vampire", 1, 1)], keeperHand=[],
        )),
    dict(
        slug="song-for-three", mechanic={"no_abilities": ["piper"]}, sort_order=70,
        title=('Song for Three', 'Мелодия на троих', 'Мелодія на трьох'), note=("Three weak against three strong. The piper's tune lifts one ally at a time: choose whom to lift, and when.", 'Трое слабых против троих сильных. Мелодия дудочника поднимает одного союзника за раз: выберите, кого и когда.', 'Троє слабких проти трьох сильних. Мелодія сопілкаря піднімає одного союзника за раз: виберіть, кого і коли.'),
        bot_depth=1, reward_dust=60, reward_finish_dust=8,
        setup=dict(
            playerBoard=[P("bagpiper", 1, 4), P("old-witch", 1, 3), P("piper", 0, 4)], playerHand=[],
            keeperBoard=[P("gnome-axeman", 0, 0), P("dark-vampire", 2, 0), P("the-creature", 1, 1)], keeperHand=[],
        )),
    dict(
        slug="circle-of-races", mechanic={"no_race": True}, sort_order=80,
        title=('The Circle of Races', 'Круг рас', 'Коло рас'), note=('Gnomes are strong against mages: every blow of a gnome on a mage weighs 1 more. The mages shoot from afar; your task is to get close.', 'Гномы сильны против магов: каждый удар гнома по магу сильнее на 1. Маги бьют издалека; ваша задача подойти вплотную.', "Гноми сильні проти магів: кожен удар гнома по магу сильніший на 1. Маги б'ють здалеку; ваше завдання підійти впритул."),
        bot_depth=1, reward_dust=70, reward_finish_dust=8,
        setup=dict(
            playerBoard=[P("gnome-axeman", 1, 5), P("forest-gnome", 2, 5), P("miner", 0, 4)], playerHand=[],
            keeperBoard=[P("shaman", 1, 1), P("old-wizard", 0, 0), P("wizard-with-a-raven", 0, 2)], keeperHand=[],
        )),
    dict(
        slug="altar", mechanic={"no_ground": ["altar"]}, sort_order=90,
        title=('The Altar', 'Алтарь', 'Вівтар'), note=('The side that stands on an altar gets 1 extra mana at the start of its turn. With more mana you put a card from your hand into the fight earlier.', 'Сторона, у которой на алтаре стоит тело, получает в начале хода 1 лишней маны. С большей маной вы выставите карту из руки раньше.', 'Сторона, у якої на вівтарі стоїть тіло, отримує на початку ходу 1 зайву ману. З більшою маною ви виставите карту з руки раніше.'),
        bot_depth=1, reward_dust=90, reward_finish_dust=10,
        setup=dict(
            playerBoard=[P("miner", 1, 4), P("yaga-with-a-bird", 2, 4)], playerHand=["piper", "wizard-with-a-raven"],
            keeperBoard=[P("steam-mermaid", 1, 1), P("witch-with-a-key", 0, 2), P("the-creature", 2, 1)], keeperHand=["granny-with-a-sheep", "wizard-with-a-raven"],
            terrain=[T(1, 3, "altar"), T(2, 2, "altar")],
        )),
    dict(
        slug="breakthrough", mechanic={"rules": {"breakthrough": False}}, sort_order=100,
        title=('Breakthrough', 'Прорыв', 'Прорив'), note=("A second way to win: a body that reaches the far row of the enemy half and stands there through the enemy's turn wins at once. The ravine slows both sides.", 'Вторая цель: тело, дошедшее до дальнего ряда чужой половины и простоявшее там ход противника, побеждает сразу. Овраг замедляет обе стороны.', 'Друга мета: тіло, що дійшло до дальнього ряду чужої половини і простояло там хід противника, перемагає одразу. Яр сповільнює обидві сторони.'),
        bot_depth=1, reward_dust=100, reward_finish_dust=12,
        setup=dict(
            playerBoard=[P("gnome-with-a-pot", 1, 5), P("piper", 1, 4), P("forest-gnome", 1, 3)], playerHand=[],
            keeperBoard=[P("granny-with-a-sheep", 2, 2), P("shaman", 0, 2), P("red-vampire", 0, 0), P("forest-gnome", 2, 1)], keeperHand=[],
            terrain=[T(1, 1, "ravine")],
            rules={"secondSideCoin": 1, "openingAttacks": 1, "walkSpendsTurn": False, "retaliation": False, "actsPerTurn": 255, "escalationFrom": 0, "idleToll": 0, "maxRounds": 12, "keeperHoldsOnTime": False, "longShotPower": 25, "pointBlankPower": 50, "breakthrough": True},
        )),
]
