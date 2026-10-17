"""Meetings: the house stands, the guest brings his own cards.

Unlike an etude there is no fixed solution: the guest chooses the cards and
places them before the fight. A good meeting rewards the choice — a team built
AGAINST the house wins clearly more often than a random one, and a team built
the other way round loses (`check_meetings.py`). The race ring is the hint the
note gives: bring those the house's race is weak against.
"""
from etudes import P, T

MEETINGS = [
    dict(
        slug="crypt-of-captain-salazar", sort_order=210, counter=["Mages"], wrong=["People"],
        title=("The Crypt of Captain Salazar", "Склеп капитана Салазара", "Склеп капітана Салазара"),
        note=("The captain and his undead hold the crypt; the ground in front of them takes power from whoever stands on it. Undead are strong against people and weak against mages. Choose who goes down.",
              "Капитан и его нежить держат склеп, а земля перед ними отнимает силу у тех, кто на ней стоит. Нежить сильна против людей и слаба против магов. Выберите, кого вести вниз.",
              "Капітан і його нежить тримають склеп, а земля перед ними забирає силу у тих, хто на ній стоїть. Нежить сильна проти людей і слабка проти магів. Виберіть, кого вести вниз."),
        bot_depth=2, reward_dust=170, reward_finish_dust=25,
        setup=dict(
            playerBoard=[], playerHand=[],
            keeperBoard=[P("captain-salazar", 1, 0), P("dark-vampire", 0, 1), P("the-creature", 2, 1)],
            keeperHand=["old-vampire", "red-vampire"],
            terrain=[T(0, 2, "grave"), T(2, 2, "grave")])),
    dict(
        slug="storm-over-the-pass", sort_order=200, counter=["Gnomes"], wrong=["Undead"],
        title=("Storm over the Pass", "Гроза над перевалом", "Гроза над перевалом"),
        note=("Three mages on the heights shoot over the pass. Mages are strong against undead and weak against gnomes: whoever comes close under the hills wins.",
              "Три мага на высотах бьют через перевал. Маги сильны против нежити и слабы против гномов: выигрывает тот, кто подойдёт вплотную под холмами.",
              "Три маги на висотах б'ють через перевал. Маги сильні проти нежиті й слабкі проти гномів: виграє той, хто підійде впритул під пагорбами."),
        bot_depth=2, reward_dust=150, reward_finish_dust=20,
        setup=dict(
            playerBoard=[], playerHand=[],
            keeperBoard=[P("wizard-with-a-raven", 1, 0), P("old-wizard", 0, 1), P("shaman", 2, 1)],
            keeperHand=["old-wizard"],
            terrain=[T(0, 1, "hill"), T(2, 1, "hill"), T(1, 2, "cover")])),
    dict(
        slug="witch-bog", sort_order=220, counter=["People"], wrong=["Water Folk"],
        title=("The Witch Bog", "Болото ведьм", "Болото відьом"),
        note=("Three witches wait behind a mire. A body that enters the mire stops there. Witches are strong against water folk and weak against people.",
              "Три ведьмы ждут за топью. Тело, вошедшее в топь, останавливается. Ведьмы сильны против водяных и слабы против людей.",
              "Три відьми чекають за болотом. Тіло, що ввійшло в болото, зупиняється. Відьми сильні проти водяних і слабкі проти людей."),
        bot_depth=2, reward_dust=190, reward_finish_dust=30,
        setup=dict(
            playerBoard=[], playerHand=[],
            keeperBoard=[P("witch-with-a-key", 1, 0), P("old-witch", 0, 1), P("yaga-with-a-bird", 2, 1)],
            keeperHand=[],
            terrain=[T(0, 2, "mire"), T(1, 2, "mire"), T(2, 2, "mire")])),
    dict(
        slug="sunken-city", sort_order=230, counter=["Witches"], wrong=["Gnomes"],
        title=("The Sunken City", "Затонувший город", "Затонуле місто"),
        note=("Mermaids silence and hold whoever comes by the channels. Water folk are strong against gnomes and weak against witches.",
              "Русалки лишают голоса и удерживают тех, кто идёт по протокам. Водяные сильны против гномов и слабы против ведьм.",
              "Русалки позбавляють голосу і утримують тих, хто йде протоками. Водяні сильні проти гномів і слабкі проти відьом."),
        bot_depth=2, reward_dust=210, reward_finish_dust=35,
        setup=dict(
            playerBoard=[], playerHand=[],
            keeperBoard=[P("old-mermaid", 1, 0), P("steam-mermaid", 0, 1), P("old-wizard", 2, 1)],
            keeperHand=["granny-with-a-sheep", "shaman"],
            terrain=[T(0, 2, "ravine"), T(2, 2, "ravine")])),
]

for _m in MEETINGS:
    _m["player_side"] = "deck"
