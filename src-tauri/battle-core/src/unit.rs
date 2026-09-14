//! The body: what a card becomes once it stands on the board.
//!
//! A card is a template and never changes. A unit is one copy of it in one
//! match, with its own wounds, its own riders and its own place. Two units of
//! the same card in one match is an ordinary thing, so they cannot be the same
//! type — merging them is the single most expensive mistake available here.

/// Identity of one unit inside one match. Not a card id: the same card played
/// twice yields two units.
pub type UnitId = u32;

/// What a rider modifies. A closed list — a new card brings a new combination,
/// never a new stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stat {
    /// Strength of what this unit deals.
    Power,
    /// Flat reduction of bodily damage.
    Armor,
    /// Flat reduction of charmed damage.
    Ward,
    /// Added to *everything* incoming, which is why it is priced above the
    /// others: it stacks with every ally's damage, not only with its own caster's.
    Vulnerable,
}

/// One rider on a unit: a blessing or a curse, with a name and a term.
///
/// The name is not decoration. It is what the stacking rule reads: a second
/// "Дым из печи" refreshes the term of the first instead of doubling its
/// magnitude — the difference between a game whose numbers can be budgeted and
/// one whose numbers avalanche.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub name: String,
    pub stat: Stat,
    /// Signed. A blessing is positive, a curse negative; `Vulnerable` is
    /// positive because it *adds* to incoming damage.
    pub amount: i32,
    /// Turns of the *bearer* remaining. Ticked by whoever owns the turn loop,
    /// which does not exist yet in this slice.
    pub turns: u8,
}

impl Status {
    pub fn new(name: &str, stat: Stat, amount: i32, turns: u8) -> Self {
        Self { name: name.to_string(), stat, amount, turns }
    }
}

/// Что наложено на тело со СРОКОМ и правит не числа, а возможности.
///
/// Второй список рядом с `Status`, и это не небрежность. Всадник правит ЧИСЛА,
/// и читает его конвейер урона — по показателю, складывая. Удержание правит то,
/// что тело МОЖЕТ: ходить, бить, наводить, быть выбранным целью, — и складывать
/// его не с чем: «оцепенение плюс оцепенение» не число. Один список на оба
/// пришлось бы спрашивать двумя разными вопросами в каждом месте, где его
/// спрашивают, а спрашивают его из конвейера, из списка законного и из свёртки.
///
/// Сроки у обоих одни и те же и тикают в одном месте — это и есть то общее,
/// что у них правда есть.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HoldKind {
    /// Оцепенение: тело не делает ничего.
    Bound,
    /// Немота: не наводит чар, но бьёт.
    Hushed,
    /// Разоружение: не бьёт, но наводит.
    Disarmed,
    /// Смута: стоит за того, кто её навёл. Не «переходит на сторону»: своим
    /// оно быть не перестало, и свой хозяин его не теряет (`is_spent`).
    Swayed,
    /// Покров: тело нельзя ВЫБРАТЬ целью. По площади достаётся.
    Veiled,
    /// Стража: удары, нацеленные в соседей, приходят ему. `amount` — сколько
    /// ещё примет.
    Guarding,
    /// Оберег канала: этот канал не чувствуется вовсе.
    Numb(crate::damage::Channel),
    /// Шипы: тому, кто ударил, прилетает `amount`.
    Thorned,
    /// Порча: `amount` урона в конце своего хода.
    Festering,
    /// Заживление: `amount` здоровья в конце своего хода.
    Knitting,
    /// Отдых после оцепенения и смуты: под них снова нельзя.
    ///
    /// Правило дома, а не осторожность движка (§5.3): без него две ведьмы
    /// держат одно тело до конца партии, и человек просто смотрит.
    Rested,
}

impl HoldKind {
    /// Одного ли рода два удержания. Оберег канала — одного рода с любым
    /// оберегом канала: два разных канала на одном теле были бы неуязвимостью,
    /// собранной из двух карт в обход цены.
    pub fn same_as(self, other: HoldKind) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    /// Кладётся ли оно тому, у кого есть отдых.
    pub fn is_control(self) -> bool {
        matches!(self, HoldKind::Bound | HoldKind::Swayed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hold {
    /// Ключ умения, как и у всадника: по нему одноимённое освежает срок.
    pub name: String,
    pub kind: HoldKind,
    pub amount: i32,
    pub turns: u8,
}

impl Hold {
    pub fn new(name: &str, kind: HoldKind, amount: i32, turns: u8) -> Self {
        Self { name: name.to_string(), kind, amount, turns }
    }
}

/// Сколько удержаний тело несёт разом. То же число и по той же причине, что у
/// всадников: дальше карта перестаёт читаться с одного взгляда.
pub const HOLD_CAP: usize = 5;

/// Health is a number with a ceiling, and nothing else.
///
/// Mitigation deliberately does not live here. Armour is read by the damage
/// pipeline, by the card in the archive and by the points calculator; buried
/// inside a wrapper around health, every one of those would have to unwrap it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn full(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0
    }
}

fn strikes_by_default() -> bool {
    true
}

/// How many riders one unit may carry at once. Not a balance rule — a reading
/// rule. Past five the card can no longer be understood at a glance.
pub const STATUS_CAP: usize = 5;

/// Turns left before one ability may be asked again.
///
/// Kept as a list rather than a map: order must be deterministic, and this
/// crate refuses `HashMap` for that reason alone.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilityCooldown {
    pub id: String,
    pub left: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub id: UnitId,
    /// Whose body this is. Sides, not players: the keeper's side is a bot, and
    /// the rules must not care which.
    pub owner: crate::board::Side,
    /// How far its ordinary blow carries, in king's steps.
    pub reach: u8,
    /// How many cells it walks in one move.
    pub step: u8,
    /// How much it mends in one act of mending. Zero — it does not mend.
    pub mend: i32,
    /// Which defence answers its ordinary blow.
    pub channel: crate::damage::Channel,
    /// Бьёт ли это тело вообще. Читается с карты и в партии не меняется.
    #[serde(default = "strikes_by_default")]
    pub strikes: bool,
    /// Whether it has already struck, mended — or walked, when the rules say a
    /// walk spends the whole turn. One such act per body per turn is the whole
    /// of the action economy.
    pub acted: bool,
    /// Whether it has already walked this turn. Read when the rules let a body
    /// walk without spending its whole turn — which is the default; otherwise
    /// `acted` says it.
    /// Kept apart from `acted` so a body that walked can still be asked whether
    /// it may strike, which is the entire question the dial exists to answer.
    #[serde(default)]
    pub moved: bool,
    /// Whether it has already struck back during the enemy's turn. Retaliation
    /// is once per turn: without the cap, focusing one body would cost the
    /// attacker more than it costs the defender, and holding still would beat
    /// attacking at every board.
    #[serde(default)]
    pub retaliated: bool,
    /// The card this body was raised from, frozen. Kept because the fields
    /// below are the *current* numbers — wounded, blessed, cursed — and the
    /// card is what was printed. The name lives here and nowhere else.
    pub card: crate::card::CardSnapshot,
    pub health: Health,
    pub power: i32,
    pub armor: i32,
    pub ward: i32,
    /// Absorbs any channel and melts as it does. Placed before health and
    /// *after* the floor, so even the guaranteed single point of damage is
    /// caught by a shield rather than leaking through it.
    pub shield: i32,
    pub statuses: Vec<Status>,
    /// The one channel this unit does not feel at all. Rare, tier 5, one turn.
    pub immune: Option<crate::damage::Channel>,
    /// Cooldowns of abilities this body has already used. Empty on matches
    /// begun before abilities arrived.
    #[serde(default)]
    pub ability_cds: Vec<AbilityCooldown>,
    /// Что наложено на него со сроком и правит его возможности.
    #[serde(default)]
    pub holds: Vec<Hold>,
    /// Чем на него ДЫШИТ поле: всадники от аур, пока те стоят рядом.
    ///
    /// Отдельным списком, а не вперемешку с наложенным, и это не опрятность.
    /// Наложенное держится СРОКОМ: легло, тикает, сошло. Аура не держится
    /// ничем — она есть, пока на поле стоит тот, кто ею дышит, и исчезает в тот
    /// же миг, как он пал. Один список пришлось бы после каждого действия
    /// разбирать на «это тикает» и «это пересчитывается», а разбор по имени —
    /// ровно тот приём, из-за которого однажды теряют половину списка.
    ///
    /// Пересчитывается целиком (`breathe`) и не тикает никогда.
    #[serde(default)]
    pub aura: Vec<Status>,
}

impl Unit {
    pub fn new(id: UnitId, health: i32, power: i32) -> Self {
        Self {
            id,
            owner: crate::board::Side::Player,
            reach: 1,
            step: 1,
            mend: 0,
            channel: crate::damage::Channel::Physical,
            strikes: true,
            acted: false,
            moved: false,
            retaliated: false,
            card: crate::card::CardSnapshot::new("", 0, health, power),
            health: Health::full(health),
            power,
            armor: 0,
            ward: 0,
            shield: 0,
            statuses: Vec::new(),
            immune: None,
            ability_cds: Vec::new(),
            holds: Vec::new(),
            aura: Vec::new(),
        }
    }

    /// Raise a body from a frozen card. The only way a unit is made in a match.
    pub fn from_card(id: UnitId, card: &crate::card::CardSnapshot, owner: crate::board::Side) -> Self {
        Self {
            id,
            owner,
            reach: card.reach,
            step: card.step,
            mend: card.mend,
            channel: card.channel,
            strikes: card.strikes,
            // A body placed this turn does not swing this turn. Without it,
            // mana buys damage outright and holding the field means nothing.
            acted: true,
            moved: false,
            retaliated: false,
            card: card.clone(),
            health: Health::full(card.health),
            power: card.power,
            armor: card.armor,
            ward: card.ward,
            shield: 0,
            statuses: Vec::new(),
            immune: None,
            ability_cds: Vec::new(),
            holds: Vec::new(),
            aura: Vec::new(),
        }
    }

    /// Turns left before this ability may be asked again. Zero — free to use.
    pub fn ability_cd(&self, id: &str) -> u8 {
        self.ability_cds
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.left)
            .unwrap_or(0)
    }

    /// Столько ходов отката значит НАВСЕГДА: умение, которое просят один раз за
    /// партию. Числом, а не вторым списком, потому что спрашивают его тем же
    /// вопросом — «вернулось ли» — и ответ на него один: нет.
    pub const FOREVER: u8 = u8::MAX;

    /// Start a cooldown after an ability is used. Zero turns is a no-op: the
    /// ability may be asked again next turn (the body's `acted` still holds).
    pub fn start_ability_cd(&mut self, id: &str, turns: u8) {
        if turns == 0 || id.is_empty() {
            return;
        }
        if let Some(existing) = self.ability_cds.iter_mut().find(|c| c.id == id) {
            existing.left = turns;
            return;
        }
        self.ability_cds.push(AbilityCooldown {
            id: id.to_string(),
            left: turns,
        });
    }

    /// Tick every cooldown by one turn of this body. Called at the opening of
    /// its side's turn — the same moment `acted` clears.
    pub fn tick_ability_cds(&mut self) {
        for cd in self.ability_cds.iter_mut() {
            // Навсегда — это навсегда: такой откат не убывает.
            if cd.left == Self::FOREVER {
                continue;
            }
            cd.left = cd.left.saturating_sub(1);
        }
        self.ability_cds.retain(|c| c.left > 0);
    }

    /// The active heal this body may cast right now, if any — and if the side
    /// has the mana. Body `mend` is the fallback when no ability is ready.
    pub fn ready_heal<'a>(
        &'a self,
        mana: i32,
    ) -> Option<crate::card::AbilitySnapshot> {
        for (i, a) in self.card.abilities.iter().enumerate() {
            if !a.is_active_heal() {
                continue;
            }
            if a.shape != "one" && a.shape != "self" {
                // Other shapes arrive with the rest of the ability engine.
                continue;
            }
            let key = ability_key(a, i);
            if self.ability_cd(&key) > 0 {
                continue;
            }
            if a.mana_cost > mana {
                continue;
            }
            let mut ready = a.clone();
            // Empty ids must still key a cooldown: the archive allows them, and
            // two empty ids on one card would otherwise share a single timer.
            if ready.id.is_empty() {
                ready.id = key;
            }
            return Some(ready);
        }
        None
    }

    /// Whether this body has any way to mend — ability or printed field.
    pub fn can_mend(&self, mana: i32) -> bool {
        self.ready_heal(mana).is_some() || self.mend > 0
    }

    /// Чары, которые это тело может навести ПРЯМО СЕЙЧАС: ключ, само умение и
    /// чем оно является.
    ///
    /// Списком, а не «первой готовой», и этим оно отличается от `ready_heal`.
    /// Лечение у тела одно по построению — лечат одним способом, — а чар у
    /// карты может быть три, и человек выбирает между ними: ровно для этого
    /// выбора `legal_actions` обязан предложить все, а не ту, что стоит в списке
    /// первой. Невидимый выбор — это не правило, а потерянная карта.
    ///
    /// Ключ тот же, что у отката (`ability_key`): пустой `id` всё равно свой
    /// отсчёт, и он же — имя всадника, по которому комната подставит слово.
    pub fn casts_ready(&self, mana: i32) -> Vec<(String, crate::card::AbilitySnapshot, crate::spell::Casting)> {
        let mut out = Vec::new();
        for (i, a) in self.card.abilities.iter().enumerate() {
            // Только то, что просят рукой: аура и ответы на поводы случаются
            // сами, и в веере им места нет.
            if !a.on_command() {
                continue;
            }
            let Some(what) = a.casting() else { continue };
            let key = ability_key(a, i);
            if self.ability_cd(&key) > 0 {
                continue;
            }
            if a.mana_cost > mana {
                continue;
            }
            out.push((key, a.clone(), what));
        }
        out
    }

    /// Умение по его ключу — вместе с ключом, потому что спрашивающий прислал
    /// ключ, а не номер, и номер ему ничего не скажет.
    pub fn ability_by_key(&self, key: &str) -> Option<crate::card::AbilitySnapshot> {
        self.card
            .abilities
            .iter()
            .enumerate()
            .find(|(i, a)| ability_key(a, *i) == key)
            .map(|(_, a)| a.clone())
    }

    // ── Удержания ───────────────────────────────────────────────────────────

    /// Удержание этого рода, если оно есть.
    pub fn hold(&self, kind: HoldKind) -> Option<&Hold> {
        self.holds.iter().find(|h| h.kind.same_as(kind))
    }

    pub fn held(&self, kind: HoldKind) -> bool {
        self.hold(kind).is_some()
    }

    /// Сколько несёт удержание этого рода. Нет его — ноль.
    pub fn hold_amount(&self, kind: HoldKind) -> i32 {
        self.hold(kind).map(|h| h.amount).unwrap_or(0)
    }

    /// Наложить удержание.
    ///
    /// Правила те же, что у всадника, и это не совпадение, а §5.1: одноимённое
    /// освежает срок и не складывает число, разноимённое встаёт рядом, на
    /// переполнении уходит самое старое. Одного РОДА, но разных имён — два
    /// разных удержания: «оцепенение» от двух ведьм это два срока, и сходят
    /// они порознь; спрашивают же удержание по роду, так что телу от этого ни
    /// холодно ни жарко, а списку — честно.
    pub fn lay_hold(&mut self, hold: Hold) {
        if let Some(existing) = self.holds.iter_mut().find(|h| h.name == hold.name) {
            existing.turns = existing.turns.max(hold.turns);
            existing.amount = existing.amount.max(hold.amount);
            return;
        }
        if self.holds.len() >= HOLD_CAP {
            self.holds.remove(0);
        }
        self.holds.push(hold);
    }

    /// Снять удержания этого рода. Сколько сняли.
    pub fn lift_holds(&mut self, kind: HoldKind) -> usize {
        let before = self.holds.len();
        self.holds.retain(|h| !h.kind.same_as(kind));
        before - self.holds.len()
    }

    /// Не делает ничего вовсе.
    pub fn bound(&self) -> bool {
        self.held(HoldKind::Bound)
    }

    /// Не наводит чар.
    pub fn hushed(&self) -> bool {
        self.held(HoldKind::Hushed)
    }

    /// Не бьёт — сейчас. Отдельно от `strikes`, которое про карту и не меняется.
    pub fn disarmed(&self) -> bool {
        self.held(HoldKind::Disarmed)
    }

    /// Нельзя ВЫБРАТЬ целью. По площади достаётся, и это весь смысл покрова.
    pub fn veiled(&self) -> bool {
        self.held(HoldKind::Veiled)
    }

    pub fn swayed(&self) -> bool {
        self.held(HoldKind::Swayed)
    }

    /// За кого это тело стоит СЕЙЧАС.
    ///
    /// Не `owner`: смута разводит «чьё тело» и «за кого оно бьёт», и это
    /// разные вопросы. Бой спрашивает этот, а хозяйство — `owner`: сторона, у
    /// которой увели последнее тело, не проиграла (§4: массовое подчинение
    /// запрещено как «конец партии одной картой», и одиночное не должно
    /// кончать партию тихо).
    pub fn side(&self) -> crate::board::Side {
        if self.swayed() { self.owner.other() } else { self.owner }
    }

    /// Сбросить по ходу у каждого всадника и снять сошедших.
    ///
    /// Зовётся в КОНЦЕ хода носителя, а не в начале. Начало выглядело
    /// естественнее и было поломкой: проклятие на один ход, наведённое в свой
    /// ход, снималось бы у противника до того, как он что-нибудь сделает, —
    /// то есть не значило бы ничего ни разу.
    pub fn tick_statuses(&mut self) {
        for s in self.statuses.iter_mut() {
            s.turns = s.turns.saturating_sub(1);
        }
        self.statuses.retain(|s| s.turns > 0);
    }

    /// То же самое у удержаний, и в тот же миг: срок у них один и тот же —
    /// ходы носителя.
    ///
    /// Отдых кладётся ЗДЕСЬ, а не там, где оцепенение накладывали: тот, кто
    /// накладывал, не знает, когда оно сойдёт, а правило говорит «вышедшая
    /// из-под контроля карта», то есть про выход, а не про вход.
    pub fn tick_holds(&mut self) {
        for h in self.holds.iter_mut() {
            h.turns = h.turns.saturating_sub(1);
        }
        let freed = self
            .holds
            .iter()
            .any(|h| h.turns == 0 && h.kind.is_control());
        self.holds.retain(|h| h.turns > 0);
        if freed {
            self.lay_hold(Hold::new("", HoldKind::Rested, 0, 1));
        }
        // Оберег канала живёт сроком, а читается полем: конвейер спрашивает
        // `immune`, и спрашивать его список было бы вторым чтением одного и
        // того же. Пишет оба одно место — вот это.
        if !self.held(HoldKind::Numb(crate::damage::Channel::Physical)) {
            self.immune = None;
        }
    }

    pub fn with_owner(mut self, owner: crate::board::Side) -> Self {
        self.owner = owner;
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

    /// How much of this body is missing. What mending can actually put back.
    pub fn wound(&self) -> i32 {
        self.health.max - self.health.current
    }

    /// What the card says, as opposed to what the body currently is.
    pub fn name(&self) -> &str {
        &self.card.name
    }

    pub fn with_armor(mut self, armor: i32) -> Self {
        self.armor = armor;
        self
    }

    pub fn with_ward(mut self, ward: i32) -> Self {
        self.ward = ward;
        self
    }

    pub fn with_shield(mut self, shield: i32) -> Self {
        self.shield = shield;
        self
    }

    /// Sum of every rider touching one stat. Riders of different names add up;
    /// that is the whole of rule two.
    pub fn status_sum(&self, stat: Stat) -> i32 {
        // Аура складывается наравне с наложенным: для конвейера урона разницы
        // между «прокляли» и «стоит рядом с тем, кто проклинает» нет никакой —
        // разница в том, как долго это держится, а не в том, что это делает.
        self.statuses
            .iter()
            .chain(self.aura.iter())
            .filter(|s| s.stat == stat)
            .map(|s| s.amount)
            .sum()
    }

    /// Lay a rider on this unit.
    ///
    /// Same name: the term is refreshed and the magnitude left alone. Different
    /// name: it joins the others. At the cap the oldest rider is displaced —
    /// the list is kept in the order laid, so the oldest is the first.
    ///
    /// This is the operation a chain of wrappers cannot perform without being
    /// taken apart and rebuilt, and it is required by the rules on page one.
    pub fn apply_status(&mut self, status: Status) {
        if let Some(existing) = self.statuses.iter_mut().find(|s| s.name == status.name) {
            existing.turns = existing.turns.max(status.turns);
            return;
        }
        if self.statuses.len() >= STATUS_CAP {
            self.statuses.remove(0);
        }
        self.statuses.push(status);
    }

    /// Lift every rider of one name — what `cleanse` and `dispel` are made of.
    /// Returns how many were lifted.
    pub fn clear_status(&mut self, name: &str) -> usize {
        let before = self.statuses.len();
        self.statuses.retain(|s| s.name != name);
        before - self.statuses.len()
    }

    /// Strength as a card would print it after riders — for showing, never for
    /// striking. The pipeline applies blessings and curses itself, and adding
    /// them here too would count them twice. Two places that compute the same
    /// number is the mistake this whole crate is arranged to avoid.
    pub fn printed_power(&self) -> i32 {
        (self.power + self.status_sum(Stat::Power)).max(0)
    }
}

/// Stable key for a cooldown slot. Prefers the ability's own id; falls back to
/// its place on the card so an empty id is still one timer, not a shared one.
pub fn ability_key(a: &crate::card::AbilitySnapshot, index: usize) -> String {
    if a.id.is_empty() {
        format!("#{index}")
    } else {
        a.id.clone()
    }
}
