//! The card as the match sees it: frozen at the moment the match began.
//!
//! Not a reference to the row in the archive. The keeper edits cards; if a match
//! pointed at the living row, a rebalance would rewrite the history of every
//! match already played and every replay would start to lie.

use crate::damage::Channel;
use crate::spell::Casting;
use crate::unit::Stat;

/// One ability, frozen with the card.
///
/// The archive holds the full dictionary; the match only needs what `reduce`
/// reads. Unknown verbs sit here quietly — `legal_actions` never offers them,
/// so a card printed with a verb the engine does not yet run is not a trap.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilitySnapshot {
    pub id: String,
    pub verb: String,
    pub amount: i32,
    pub shape: String,
    pub range: u8,
    pub mana_cost: i32,
    pub cooldown: u8,
    pub trigger: String,
    /// Каким каналом бьёт чара. Лечению, проклятию и щиту канал не нужен, и
    /// умолчание тут не «обычный урон», а просто то, что читает `damage.rs`,
    /// когда его спрашивают.
    ///
    /// `default` не из вежливости к старым записям, а потому, что снимок доски
    /// лежит в базе: партия, начатая до чар, обязана дочитаться.
    #[serde(default = "bodily")]
    pub channel: Channel,
    /// Сколько ходов носителя держится всадник. Ноль — один ход.
    #[serde(default)]
    pub duration: u8,
    /// Сколько клеток вокруг захватывает опасная клетка. Ноль — только она сама.
    #[serde(default)]
    pub radius: u8,
    /// Тело, которое призывают. `None` — призывать нечего, и тогда движок этой
    /// чары не играет вовсе: призыв без тела не призыв.
    ///
    /// Заморожено вместе с умением — как всё остальное в партии, и по той же
    /// причине: карта призванного в архиве переживёт перебалансировку, а партия
    /// обязана переигрываться той, что была. В коробке, потому что иначе тип
    /// содержал бы сам себя; призванное не призывает, и это тоже сказано здесь.
    #[serde(default)]
    pub body: Option<Box<CardSnapshot>>,
    /// Какой показатель правит всадник. Пусто — сила: проклятие без уговора
    /// ослабляет удар, и это то, что читатель предполагает сам.
    #[serde(default)]
    pub stat: Option<Stat>,
}

fn bodily() -> Channel {
    Channel::Physical
}

impl AbilitySnapshot {
    /// An active heal of `amount` at `range`, for tests and the generator.
    pub fn heal(id: &str, amount: i32, range: u8) -> Self {
        Self {
            id: id.to_string(),
            verb: "heal".into(),
            amount,
            shape: "one".into(),
            range,
            mana_cost: 0,
            cooldown: 0,
            trigger: "active".into(),
            channel: Channel::Physical,
            duration: 0,
            radius: 0,
            stat: None,
            body: None,
        }
    }

    pub fn with_mana(mut self, mana: i32) -> Self {
        self.mana_cost = mana;
        self
    }

    pub fn with_cooldown(mut self, cooldown: u8) -> Self {
        self.cooldown = cooldown;
        self
    }

    pub fn on_self(mut self) -> Self {
        self.shape = "self".into();
        self
    }

    pub fn is_active_heal(&self) -> bool {
        self.verb == "heal" && self.trigger == "active" && self.amount > 0
    }

    /// Просят ли эту чару РУКОЙ. Остальные поводы случаются сами, и предлагать
    /// их в веер значило бы обещать выбор там, где его нет.
    ///
    /// `once` просится рукой наравне с `active` и отличается от него одним:
    /// спросить её можно единожды за партию.
    pub fn on_command(&self) -> bool {
        self.trigger == "active" || self.trigger == "once"
    }

    /// Уходит ли она после первого раза навсегда.
    pub fn spent_forever(&self) -> bool {
        self.trigger == "once"
    }

    /// A harming spell of `amount` at `range`, for tests and the generator.
    pub fn harm(id: &str, amount: i32, range: u8) -> Self {
        Self {
            id: id.to_string(),
            verb: "damage".into(),
            amount,
            shape: "one".into(),
            range,
            mana_cost: 0,
            cooldown: 0,
            trigger: "active".into(),
            channel: Channel::Magic,
            duration: 0,
            radius: 0,
            stat: None,
            body: None,
        }
    }

    /// A curse of `amount` on `stat` for `turns`, at `range`.
    pub fn curse(id: &str, amount: i32, stat: Stat, turns: u8, range: u8) -> Self {
        Self {
            verb: "curse".into(),
            duration: turns,
            stat: Some(stat),
            ..Self::harm(id, amount, range)
        }
    }

    pub fn with_verb(mut self, verb: &str) -> Self {
        self.verb = verb.to_string();
        self
    }

    pub fn with_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    /// Чем это умение становится, когда его просят, — и ничего, если движок его
    /// не играет.
    ///
    /// Про ПОВОД здесь не спрашивается: что чара делает — одно, а от чего она
    /// случается — другое, и реакция делает ровно то же, что приказ. Кого
    /// спрашивать позволено человеку, говорит `on_command`.
    ///
    /// Пригоршня чару не отсекает — считаются все восемь, — но запрещённые §4
    /// сочетания (массовые оцепенение, смута, покров) не играются вовсе.
    pub fn casting(&self) -> Option<Casting> {
        if self.amount <= 0 {
            return None;
        }
        let what = Casting::of_verb(&self.verb)?;
        // Запрещённое сочетание не играется вовсе — и это не жадность движка, а
        // §4: «✗ — запрещённые, не дорогие». Хранителю о нём говорит стол при
        // сохранении; здесь стоит второй замок, чтобы карта, сохранённая мимо
        // стола, не вышла на поле концом партии.
        if what.forbids(&self.shape, self.radius) {
            return None;
        }
        // Призыв без тела не призыв: предложить его значило бы обещать выход
        // того, чего нет. Хранитель, забывший назвать карту, увидит, что чара
        // не выходит, — и это честнее, чем пустое место на поле.
        if matches!(what, Casting::Summon) && self.body.is_none() {
            return None;
        }
        Some(what)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSnapshot {
    pub name: String,
    pub cost: i32,
    pub health: i32,
    pub power: i32,
    pub armor: i32,
    pub ward: i32,
    pub reach: u8,
    /// How many cells it walks in one move. Zero means it does not walk at all —
    /// a cauldron stands where it was put.
    pub step: u8,
    /// How much it mends in one act of mending. Zero — it does not mend.
    /// Interim: a heal ability on the card takes over when present.
    pub mend: i32,
    pub channel: Channel,
    /// Бьёт ли это тело вообще.
    ///
    /// Отдельным полем, а не четвёртым каналом: канала три, и четвёртым он был
    /// бы не «ещё один вид урона», а его отсутствие — то есть слово не из того
    /// словаря. Котёл, знамя, лекарь без оружия стоят на поле и не наносят
    /// ударов; до этого поля хранитель мог сказать «не бьёт» в форме, а карта
    /// всё равно выходила и била.
    #[serde(default = "strikes_by_default")]
    pub strikes: bool,
    /// Строение: стена, башня, ворота. Стоит и может быть сломано, но
    /// сторона, у которой остались одни строения, побеждена.
    ///
    /// Отдельным полем, а не выводом из «не ходит и не бьёт»: котёл и знамя
    /// тоже не ходят и не бьют, а сторона, у которой остался котёл, не
    /// проиграла. И башня бьёт, а взятому городу не нужно ломать её до
    /// последнего камня. Замер штурма (`BATTLE-GENRE-RESEARCH.md` § 7.5): без
    /// этого поля в 14 партиях из 16 город «стоял» одной стеной с одним
    /// здоровьем, когда защитников в нём уже не было.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub structure: bool,
    /// Стреляет: лук, праща, самострел. Бьёт над строением — через стену
    /// города. Маг (магический урон) делает это и без отметки.
    ///
    /// Отдельным полем, а не выводом из дальности: дальность 2 бывает и у
    /// копья, а копьём через стену не бьют. Пока различия не было, через
    /// стены города били все с дальностью от двух — и Бабушка с курицей тоже
    /// (`BATTLE-GENRE-RESEARCH.md` § 7.7).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shoots: bool,
    /// Abilities frozen with the card. Empty on every match begun before this
    /// field existed — `default` keeps their board caches readable.
    #[serde(default)]
    pub abilities: Vec<AbilitySnapshot>,
    /// Раса карты — её id в справочнике, как строка. Движку раса нужна только
    /// для одного вопроса: «есть ли у бьющего прибавка против этой расы» и
    /// «есть ли у цели защита от этой расы» (`RaceDamage`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,
    /// Урон расы против других рас. Заморожен вместе с картой, как и всё в
    /// снимке: правка таблицы рас не меняет уже начатых партий.
    #[serde(default, skip_serializing_if = "RaceDamage::is_empty")]
    pub race_damage: RaceDamage,
}

/// Урон по расам: «+`attack` к урону по картам рас `against`» и «−`defence`
/// урона от карт рас `from`». Одно число на список, а не число на каждую пару:
/// так его настраивает стол рас (число и галочки), и так его легко держать в
/// голове игроку.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaceDamage {
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub against: Vec<String>,
    #[serde(default)]
    pub defence: i32,
    #[serde(default)]
    pub from: Vec<String>,
}

impl RaceDamage {
    pub fn is_empty(&self) -> bool {
        (self.attack == 0 || self.against.is_empty()) && (self.defence == 0 || self.from.is_empty())
    }

    /// Прибавка к урону по карте расы `target`.
    pub fn attack_on(&self, target: Option<&str>) -> i32 {
        match target {
            Some(r) if self.against.iter().any(|a| a == r) => self.attack.max(0),
            _ => 0,
        }
    }

    /// Сколько урона снимается, когда бьёт карта расы `attacker`.
    pub fn defence_from(&self, attacker: Option<&str>) -> i32 {
        match attacker {
            Some(r) if self.from.iter().any(|a| a == r) => self.defence.max(0),
            _ => 0,
        }
    }
}

fn strikes_by_default() -> bool {
    true
}

impl CardSnapshot {
    /// A plain body: the two-line card the first slice needs.
    pub fn new(name: &str, cost: i32, health: i32, power: i32) -> Self {
        Self {
            name: name.to_string(),
            cost,
            health,
            power,
            armor: 0,
            ward: 0,
            reach: 1,
            step: 1,
            mend: 0,
            channel: Channel::Physical,
            strikes: true,
            structure: false,
            shoots: false,
            abilities: Vec::new(),
            race: None,
            race_damage: RaceDamage::default(),
        }
    }

    /// Раса и её урон — для проверок.
    pub fn with_race(mut self, race: &str, damage: RaceDamage) -> Self {
        self.race = Some(race.to_string());
        self.race_damage = damage;
        self
    }

    pub fn with_armor(mut self, armor: i32) -> Self {
        self.armor = armor;
        self
    }

    pub fn with_ward(mut self, ward: i32) -> Self {
        self.ward = ward;
        self
    }

    pub fn with_reach(mut self, reach: u8) -> Self {
        self.reach = reach;
        self
    }

    pub fn with_step(mut self, step: u8) -> Self {
        self.step = step;
        self
    }

    pub fn with_mend(mut self, mend: i32) -> Self {
        self.mend = mend;
        self
    }

    pub fn with_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    pub fn with_ability(mut self, ability: AbilitySnapshot) -> Self {
        self.abilities.push(ability);
        self
    }

    /// Тело, которое стоит на поле и не наносит ударов.
    pub fn without_a_blow(mut self) -> Self {
        self.strikes = false;
        self
    }
}
