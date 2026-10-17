"""Check that an etude is ABOUT its mechanic: without it the bots must lose.

A `mechanic` is one of
  {"no_abilities": ["old-witch", ...]}   those cards lose their abilities
  {"no_ground": ["altar", ...]}          those kinds of ground are removed
  {"rules": {"breakthrough": False}}     a rule is switched off
  {"no_race": True}                      race damage is switched off
and `variant` returns the same etude with it switched off. An etude passes when
the real one is won by the thinking bots and the variant is not.
"""
import copy


def variant(entry, mechanic):
    e = copy.deepcopy(entry)
    e["slug"] = entry["slug"] + "#without"
    setup = e["setup"]
    if "no_abilities" in mechanic:
        e["cardPatches"] = {slug: {"abilities": []} for slug in mechanic["no_abilities"]}
    if "patch" in mechanic:
        e["cardPatches"] = {**e.get("cardPatches", {}), **mechanic["patch"]}
    if "no_ground" in mechanic:
        setup["terrain"] = [t for t in setup.get("terrain", []) if t["ground"] not in mechanic["no_ground"]]
        if not setup["terrain"]:
            setup.pop("terrain")
    if "rules" in mechanic:
        setup["rules"] = {**(setup.get("rules") or DEFAULT_RULES), **mechanic["rules"]}
    if mechanic.get("no_race"):
        e["noRace"] = True
    return e


DEFAULT_RULES = {"secondSideCoin": 1, "openingAttacks": 1, "walkSpendsTurn": False, "retaliation": False,
                 "actsPerTurn": 255, "escalationFrom": 0, "idleToll": 0, "maxRounds": 12,
                 "keeperHoldsOnTime": False, "longShotPower": 25, "pointBlankPower": 50, "breakthrough": False}


def uses(real, mechanic):
    """The winning line must actually USE the mechanic, not just differ without it:
    abilities are cast (or healing is done), a breakthrough is what ends the game.
    Ground and race damage leave no trace in the record; for them the loss
    without the mechanic is the only evidence."""
    d3 = real["d3"]
    if "no_abilities" in mechanic:
        return d3.get("casts", 0) + d3.get("mends", 0) >= 2
    if "rules" in mechanic and mechanic["rules"].get("breakthrough") is False:
        return bool(d3.get("breached"))
    return True


def passes(real, without, mechanic=None):
    """real / without: the harness records {"d1","d2","d3"} with "o" in W L D T."""
    ok = (real["d2"]["o"] == "W" and real["d3"]["o"] == "W" and real["d1"]["o"] != "W"
          and without["d2"]["o"] != "W" and without["d3"]["o"] != "W")
    return ok and (mechanic is None or uses(real, mechanic))
