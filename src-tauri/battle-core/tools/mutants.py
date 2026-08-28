#!/usr/bin/env python3
"""Мутационный прогон по математике battle-core.

Ломает по одному месту в правилах и смотрит, падает ли хоть один тест.
Мутация, которая выжила, — это либо дыра в проверках, либо доказательство,
что правка ничего не меняет (например, перестановка двух сложений).

    python3 tools/mutants.py

Ничего не оставляет за собой: исходник восстанавливается после каждого шага.
"""

import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BACKUP = "/tmp/battle-core-src-backup"

# Две мутации сюда сознательно не входят — они проверялись и выжили, и это
# правильно, потому что обе ничего не меняют арифметически:
#
#   • перестановка "благословения бьющего" и "проклятия бьющего" — сложение
#     коммутативно;
#   • перестановка "уязвимости цели" и "защиты по каналу" — (a + V) − D равно
#     (a − D) + V, пока ни один из шагов не обрезает промежуточное значение.
#
# Их выживание — довод, что тесты проверяют поведение, а не порядок строк.
# Но второе верно только сегодня: как только между этими шагами появится
# отсечение, порядок станет наблюдаемым, и мутацию надо будет сюда внести.

# (имя, файл, что заменить, на что). Каждая пара должна встречаться ровно раз.
MUTATIONS = [
    ("благословение бьющего считается как проклятие", "src/damage.rs",
     "Some(a) => amount + a.status_sum(Stat::Power).max(0),",
     "Some(a) => amount + a.status_sum(Stat::Power).min(0),"),
    ("проклятие бьющего не применяется", "src/damage.rs",
     "Some(a) => amount + a.status_sum(Stat::Power).min(0),\n        None => amount,",
     "Some(_a) => amount,\n        None => amount,"),
    ("уязвимость цели не применяется", "src/damage.rs",
     "amount + ctx.target.status_sum(Stat::Vulnerable)", "amount"),
    ("броня и оберег поменяны местами", "src/damage.rs",
     "Channel::Physical => t.armor + t.status_sum(Stat::Armor),\n        Channel::Magic => t.ward + t.status_sum(Stat::Ward),",
     "Channel::Physical => t.ward + t.status_sum(Stat::Ward),\n        Channel::Magic => t.armor + t.status_sum(Stat::Armor),"),
    ("сглаз режется бронёй", "src/damage.rs",
     "Channel::Pure => 0,", "Channel::Pure => t.armor,"),
    ("состояния не влияют на защиту", "src/damage.rs",
     "Channel::Physical => t.armor + t.status_sum(Stat::Armor),", "Channel::Physical => t.armor,"),
    ("минимум 1 стал минимумом 0", "src/damage.rs",
     "fn step_floor(amount: i32, _ctx: &Ctx) -> i32 {\n    amount.max(1)",
     "fn step_floor(amount: i32, _ctx: &Ctx) -> i32 {\n    amount.max(0)"),
    ("минимум применяется до защиты", "src/damage.rs",
     "    (StepId::ChannelDefence, step_channel_defence),\n    (StepId::Floor, step_floor),",
     "    (StepId::Floor, step_floor),\n    (StepId::ChannelDefence, step_channel_defence),"),
    ("щит не тает", "src/damage.rs",
     "    target.shield -= res.to_shield;", "    target.shield -= 0;"),
    ("щит не поглощает", "src/damage.rs",
     "let to_shield = amount.min(target.shield.max(0));", "let to_shield = 0;"),
    ("невосприимчивость не действует", "src/damage.rs",
     "    if target.immune == Some(packet.channel) {", "    if false && target.immune == Some(packet.channel) {"),
    ("здоровье не ограничено нулём снизу", "src/damage.rs",
     "target.health.current = (target.health.current - res.to_health).max(0);",
     "target.health.current -= res.to_health;"),
    ("удар берёт не свой канал", "src/damage.rs",
     "DamagePacket::new(attacker.power, attacker.channel, Source::Attack)",
     "DamagePacket::new(attacker.power, Channel::Physical, Source::Attack)"),
    ("шипы отвечают на шипы", "src/damage.rs",
     "matches!(self, Source::Attack | Source::Ability)",
     "matches!(self, Source::Attack | Source::Ability | Source::Thorns)"),
    ("одноимённое состояние складывается", "src/unit.rs",
     "            existing.turns = existing.turns.max(status.turns);\n            return;",
     "            existing.turns = existing.turns.max(status.turns);\n            existing.amount += status.amount;\n            return;"),
    ("состояния суммируются без разбора оси", "src/unit.rs",
     "self.statuses.iter().filter(|s| s.stat == stat).map(|s| s.amount).sum()",
     "self.statuses.iter().map(|s| s.amount).sum()"),
    ("потолок состояний снят", "src/unit.rs",
     "        if self.statuses.len() >= STATUS_CAP {\n            self.statuses.remove(0);\n        }", ""),
    ("смерть не снимает состояния", "src/damage.rs",
     "        target.statuses.clear();", ""),
    ("лечение поднимает выше исходного", "src/heal.rs",
     "restored: amount.max(0).min(target.wound())", "restored: amount.max(0)"),
    ("лечение не отсекает отрицательное", "src/heal.rs",
     "restored: amount.max(0).min(target.wound())", "restored: amount.min(target.wound())"),
    ("расстояние стало манхэттенским", "src/board.rs", "        dx.max(dy)", "        dx + dy"),
    ("ход проходит сквозь тела", "src/board.rs",
     "                    if depth[i] == u8::MAX && self.is_free(neighbour) {",
     "                    if depth[i] == u8::MAX {"),
    ("рука держит партию, даже если её нечем сыграть", "src/state.rs",
     "let affordable = self.side_state(side).hand.iter().any(|c| c.cost <= MANA_CAP);",
     "let affordable = !self.side_state(side).hand.is_empty();"),
    ("выставить некуда — но сторона считается живой", "src/state.rs",
     "!(affordable && self.board.free_cells(side).next().is_some())",
     "!affordable"),
]


def restore():
    src = os.path.join(ROOT, "src")
    shutil.rmtree(src)
    shutil.copytree(BACKUP, src)
    # copytree сохраняет время файлов, и cargo решает, что пересобирать нечего,
    # — тест пойдёт со старым бинарником и соврёт. Без этой строки весь прогон
    # бессмыслен.
    for root, _, files in os.walk(src):
        for f in files:
            os.utime(os.path.join(root, f), None)


def main():
    if os.path.exists(BACKUP):
        shutil.rmtree(BACKUP)
    shutil.copytree(os.path.join(ROOT, "src"), BACKUP)

    survivors = []
    print(f"{'мутация':<48} исход")
    print("─" * 70)

    for name, rel, old, new in MUTATIONS:
        restore()
        path = os.path.join(ROOT, rel)
        text = open(path).read()
        if text.count(old) != 1:
            print(f"{name:<48} НЕ ПРИМЕНИЛАСЬ (вхождений {text.count(old)})")
            continue
        open(path, "w").write(text.replace(old, new))
        result = subprocess.run(["cargo", "test", "--quiet"], cwd=ROOT, capture_output=True, text=True)
        caught = result.returncode != 0
        print(f"{name:<48} {'поймана' if caught else 'ВЫЖИЛА'}")
        if not caught:
            survivors.append(name)

    restore()
    print(f"\nмутаций {len(MUTATIONS)} · выжило {len(survivors)}")
    for s in survivors:
        print("  ", s)
    return 1 if survivors else 0


if __name__ == "__main__":
    sys.exit(main())
