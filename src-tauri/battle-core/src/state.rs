//! The frame: a field, turns, an action economy, and an ending.
//!
//! Everything here is written once and is not meant to be rewritten. Later
//! stages add vocabulary — more verbs, shapes, triggers — by widening what an
//! `Action` may be and what `reduce` dispatches to, never by reshaping the loop
//! below. If a later stage has to change the signature of `reduce`, the journal
//! format or the shape of a legal action, the stages were cut wrongly.

use crate::board::{Board, Cell, Side};
use crate::card::CardSnapshot;
use crate::damage::{apply, strike};
use crate::event::{Event, Outcome};
use crate::unit::{Unit, UnitId};

/// Mana grows by one each of your own turns and stops here.
pub const MANA_CAP: i32 = 10;

/// Two decks of healing and thorns can fail to kill each other forever. After
/// this many rounds the match is decided on the health left standing.
pub const MAX_ROUNDS: u8 = 12;

/// Mana must be able to climb to its cap inside a match, or `is_spent` above
/// would be answering a question the clock has already settled. Written as an
/// assertion rather than a note because a note does not fail the build.
const _: () = assert!(MAX_ROUNDS as i32 >= MANA_CAP, "мана не успевает дорасти до потолка");

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SideState {
    pub hand: Vec<CardSnapshot>,
    pub mana: i32,
    pub mana_max: i32,
}

/// How a match starts: who stands where, and what is held back.
/// The handful of rules that have a dial on them.
///
/// Exists because the first measurement said the side moving first wins 77.8%
/// of mirrored matches — a number no amount of tuning card values can fix, and
/// one that has to be answered by a rule. Which rule is a question for the
/// runner, not for taste, so both candidates live here and are measured.
///
/// A match records the version of the rules it was played under, so this is
/// also where a future variation goes without rewriting played matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rules {
    /// Mana the side moving second starts with, before its first turn adds one.
    /// The oldest answer in the genre: compensation, not a handicap.
    pub second_side_coin: i32,
    /// How many blows the side moving first may land during the opening round.
    /// `u8::MAX` — as many as it likes.
    ///
    /// A dial rather than a switch, because the switch was measured and it
    /// overcorrected: forbidding the opening round entirely moved the advantage
    /// from 78% for the first side to 70% for the second. The advantage is worth
    /// about one blow, so the answer has to be able to cost about one blow.
    pub opening_attacks: u8,
    /// Тратит ли шаг ход тела целиком.
    ///
    /// `true` — как было: тело либо идёт, либо бьёт. §12.2 замерил, чем это
    /// кончается: ближний бой проиграл стрелкам 0 партий из 6 при любой
    /// скорости, потому что пока он идёт, он не делает ничего.
    /// `false` — тело может пройти и ударить в один ход, по разу за ход.
    #[serde(default = "walk_spends_turn_default")]
    pub walk_spends_turn: bool,
    /// Отвечает ли ударенное тело ударом, если достаёт. Раз за ход противника.
    ///
    /// Ответ на измеренное: у карт, равных по очкам, 98.8 % партий решает то,
    /// кто ударил первым. Сдача отнимает у первого удара часть его цены — но
    /// заодно наказывает того, кто подошёл, и это надо мерить, а не решать.
    #[serde(default)]
    pub retaliation: bool,
    /// Сколько действий сторона совершает за один свой ход. `u8::MAX` — сколько
    /// угодно, то есть каждое тело по разу, как было.
    #[serde(default = "acts_per_turn_default")]
    pub acts_per_turn: u8,
    /// С какого круга удары начинают расти, по единице за круг. Ноль — не
    /// растут вовсе.
    ///
    /// Ответ на измеренное: против руки с перебором половина партий
    /// доигрывалась до лимита кругов и решалась остатком здоровья. Умелая рука
    /// не идёт в невыгодный размен — и никто не умирает. Растущий удар делает
    /// размен со временем выгодным для обоих, то есть возвращает партии
    /// развязку, не отнимая ни у кого выбора.
    #[serde(default)]
    pub escalation_from: u8,
    /// Сколько здоровья теряет тело, простоявшее свой ход без дела. Ноль — не
    /// теряет.
    ///
    /// Целится ровно в то, что мерилось: умелая рука не идёт в невыгодный
    /// размен и просто СТОИТ. Растущий удар делает размен со временем выгоднее,
    /// но не мешает стоять; плата за бездействие мешает.
    ///
    /// Тело, выставленное с руки в этот ход, платы не платит: оно и так не
    /// могло действовать (`acted` у него поднят при выходе на поле).
    #[serde(default)]
    pub idle_toll: i32,
    /// Сколько кругов идёт партия, прежде чем её решит остаток здоровья.
    ///
    /// Ручка, а не константа, потому что вопрос «а если счётчик просто
    /// отодвинуть — кончится ли партия сама?» иначе не проверить, а он решает,
    /// чинить ли счётчик или то, ради чего он поставлен.
    #[serde(default = "max_rounds_default")]
    pub max_rounds: u8,
    /// Какую долю силы сохраняет стрелок, бьющий ДАЛЬШЕ своей дальности.
    /// Ноль — не достаёт вовсе, как было.
    ///
    /// Замер (§19) показал, что пятая часть партий не кончается никогда: тела
    /// становятся туда, куда до них не дотянуться, и стоят. Дальность у нас —
    /// жёсткая отсечка, и за ней поле безопасно.
    ///
    /// В «Героях меча и магии III» отсечки нет: стрелок достаёт до любой точки
    /// поля, а за удобной дистанцией бьёт вполовину. Безопасных клеток там нет,
    /// и стоять негде. Ближнего боя это не касается — он и там бьёт только
    /// вплотную.
    #[serde(default)]
    pub long_shot_power: u8,
    /// Какую долю своей силы, в сотых, сохраняет стрелок, к которому подошли
    /// вплотную. 100 — никакого штрафа.
    ///
    /// Ручка, а не выключатель, по той же причине, что и `opening_attacks`:
    /// «стрелок в упор не стреляет вовсе» — это одно число из ста возможных, и
    /// выбирать его надо прогоном, а не на слух.
    #[serde(default = "point_blank_default")]
    pub point_blank_power: u8,
}

fn point_blank_default() -> u8 {
    100
}

fn max_rounds_default() -> u8 {
    MAX_ROUNDS
}

fn walk_spends_turn_default() -> bool {
    true
}

fn acts_per_turn_default() -> u8 {
    u8::MAX
}

impl Default for Rules {
    /// Chosen by measurement, not by taste.
    ///
    /// # Шаг не тратит ход
    ///
    /// Пока тело за свой ход могло либо пройти, либо ударить, подойти значило
    /// проиграть: подошедший тратил ход на шаг и получал удар, не ответив. У
    /// двух карт равной силы это решало партию целиком.
    ///
    /// Мерилось так: одна и та же пара карт, обе расстановки, много партий.
    /// Доля побед у той стороны, чья очередь наступала раньше. 50 % значит
    /// «решает карта», 100 % — «решает очередь».
    ///
    /// ```text
    ///   шаг тратит ход целиком        98 %   решала очередь
    ///   шаг не тратит ход             43 %   решает карта      <- это
    ///   сдача (ответный удар)         65 %
    ///   два действия за ход           88 %
    /// ```
    ///
    /// Проверено и отвергнуто заодно: **сдача** лечит вполовину и поверх шага
    /// не добавляет ничего; **одно действие на весь ход** отдаёт лимиту кругов
    /// 69 % партий вместо нынешнего одного; **два действия за ход** возвращают
    /// и перевес первого хода, и власть очереди.
    ///
    /// # Монета второй стороне
    ///
    /// Шаг, переставший тратить ход, отменил прежнюю калибровку: с монетой в
    /// две маны перевес уезжал ко второй стороне. Пересчитано на новой
    /// экономике, и выбрано снова по устойчивости, а не по близости к
    /// половине, — тем же правилом, каким выбирали в прошлый раз.
    ///
    /// ```text
    /// доля побед первого хода, по глубине руки 1..4
    ///   монета 1 · один удар   52.8  50.5  53.1  54.2   разброс 3.8  <- это
    ///   монета 3 · два удара   55.2  49.5  50.1  49.2   разброс 6.0
    ///   монета 0 · один удар   49.0  52.0  54.0  60.4   разброс 11.4
    /// ```
    ///
    /// Что стало на общем прогоне (2000 зеркальных партий): первый ход 50.4 %,
    /// ничьих 0.1 % против прежних 1.1 %, решено лимитом 0.6 % против 2.4 %,
    /// круга 6.2 против 6.9.
    ///
    /// # Плата за бездействие
    ///
    /// Всё выше мерилось ЖАДНОЙ рукой бота — до того, как появилась вторая. С
    /// перебором вскрылось худшее: **половина партий доигрывалась до лимита
    /// кругов**, ничьих 11 %. Умелая рука не идёт в невыгодный размен и просто
    /// стоит; никто не умирает, и партию решает счётчик.
    ///
    /// Мерились две ручки. Растущий к концу удар (`escalation_from`) снял часть
    /// и заметно увёл честность; плата за простой (`idle_toll`) целится прямо в
    /// причину и держит честность в коридоре.
    ///
    /// ```text
    /// рукой с перебором, 400 партий
    ///                     1-й ход  кругов  ничьих  лимитом
    ///   как было           45.8 %   10.3   11.2 %   51.8 %
    ///   плата 1            54.8 %    9.7    1.8 %   35.5 %   <- это
    ///   рост с 6-го круга  55.0 %    8.8    8.3 %   25.0 %
    ///
    /// жадной рукой, 800 партий — не сломалось ли то, что работало
    ///   как было           44.1 %    6.3    0.2 %    1.2 %
    ///   плата 1            43.6 %    6.2    0.2 %    0.9 %
    /// ```
    ///
    /// Ничьи почти исчезли, длина попала в проектные 8–12, жадная рука правки
    /// не заметила, а выбор по-прежнему значит всё: думающий обыгрывает
    /// случайного 99–100 % при любой из этих настроек.
    ///
    /// **Вылечено не до конца, и это надо знать:** треть партий против умелой
    /// руки всё ещё решается счётчиком. Плата мешает стоять, но не мешает
    /// держаться вне досягаемости — остальное вопрос геометрии и лимита кругов,
    /// и мерить его надо отдельно.
    ///
    /// Честность при этом переехала с 45.8 на 54.8 — из «второй стороне» в
    /// «первой». В коридоре 45–55 держится, поэтому монету не трогаем: правило
    /// открытия калибруется под темп, а темп ещё будет меняться.
    ///
    /// # У стрелка нет отсечки: безопасных клеток на поле не осталось
    ///
    /// §19 показал, что счётчик кругов не тесен, а подпирает: пятая часть
    /// партий не кончалась НИКОГДА, сколько лимит ни отодвигай, и в трёх
    /// четвертях кругов не было ни одного удара. Причина — жёсткая отсечка по
    /// дальности: за ней поле безопасно, и на него становятся и стоят.
    ///
    /// Взято из «Героев меча и магии III», где отсечки нет: стрелок достаёт до
    /// любой точки поля, а за удобной дистанцией бьёт вполовину. Ближнего боя
    /// это не касается — он и там бьёт только вплотную.
    ///
    /// ```text
    /// рукой с перебором, 300 партий, лимит отодвинут до 120 кругов
    ///                   не кончилось  кругов  1-й ход
    ///   отсечка (как было)    25.0 %    36.6   53.0 %
    ///   за далью 25 %         16.0 %    26.6   57.7 %   <- это
    /// ```
    ///
    /// Сила за далью выбрана замером: 25 % убирает больше безнадёжных партий,
    /// чем 50, 75 или 100. Слабый дальний выстрел заставляет сближаться, но не
    /// делает стрельбу выгоднее ближнего боя.
    ///
    /// Побочно вылечилось §12.2 — и в обе стороны. С платой за простой (§18)
    /// неподвижные стрелки бледнели сами: 18 побед ближнего боя из 18, то есть
    /// перекос уже в другую сторону. Дальний выстрел вернул их к живому: 9 на 9.
    ///
    /// **Чем заплачено:** перевес первого хода 55.7 → 58.0 %, то есть вышел за
    /// коридор 45–55. Монету перевыбирать надо будет — но один раз и после
    /// того, как решится остаток ниже, а не сейчас.
    ///
    /// **Вылечено не до конца:** 16 % партий всё ещё не кончаются. Остаток —
    /// ближний бой против ближнего боя, который просто уходит: платы за простой
    /// он не платит (он ходит), и заставить его драться нечем. Это уже вопрос
    /// цели партии, а не дальности.
    ///
    /// # Стрелок в упор бьёт вполсилы
    ///
    /// Шаг с ударом стрелков не вылечил: 2 партии из 18 у ближнего боя против
    /// прежних 0. Настоящая беда оказалась не «стрелки всегда выигрывают», а
    /// «один и тот же бюджет, потраченный по-разному, даёт от 0 до 72 % побед,
    /// и весы об этом не знают». Четыре способа потратить 12 очков, доля побед
    /// ближнего боя:
    ///
    /// ```text
    /// сила в упор   §12.2   крепкий/  крепкий/  сильный/  живучий/
    ///               ближ.   неподв.   подвиж.   живучий   сильный
    ///     100 %      2/16     66 %      72 %      10 %       0 %
    ///      70 %      2/16     71 %      76 %      44 %      16 %
    ///      50 %     11/7      71 %      76 %      44 %      30 %   <- это
    ///      25 %     11/7      87 %      97 %      70 %      67 %
    /// ```
    ///
    /// Ниже 65 крайний случай §12.2 переворачивается; ниже 50 ближний бой сам
    /// становится непобедим. Половина — середина этой полки.
    ///
    /// # Чем за это заплачено
    ///
    /// Штраф сам по себе, любой силы, сдвигает перевес ко второй стороне:
    /// 55 % → 45 % на общем прогоне, и монету обратно не докупить — при монете
    /// 0 выходит 74 %, а половины маны не бывает. Вернуть половину можно только
    /// двумя-тремя ударами в первом круге, но тогда возвращается и власть
    /// очереди (43 % → 69 %), то есть отменяется всё, что дала правка выше.
    ///
    /// Взято меньшее зло: перекос примерно 45/55 во вторую сторону. Он терпим
    /// ровно потому, что **первым всегда ходит гость**, а второй стороной
    /// играет бот хранителя: перекос достаётся дому, а не другому человеку.
    /// Появится игра человека с человеком — число придётся перевыбрать, и
    /// понадобится ручка тоньше, чем целая мана.
    ///
    /// ```text
    /// доля побед первого хода при штрафе в половину, по глубине руки 1..4
    ///   монета 1 · один удар   44.8  41.6  44.1  47.9   разброс 6.2
    /// ```
    fn default() -> Self {
        Self {
            second_side_coin: 1,
            opening_attacks: 1,
            walk_spends_turn: false,
            retaliation: false,
            acts_per_turn: u8::MAX,
            point_blank_power: 50,
            escalation_from: 0,
            idle_toll: 1,
            long_shot_power: 25,
            max_rounds: MAX_ROUNDS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub player_board: Vec<(CardSnapshot, Cell)>,
    pub player_hand: Vec<CardSnapshot>,
    pub keeper_board: Vec<(CardSnapshot, Cell)>,
    pub keeper_hand: Vec<CardSnapshot>,
}

/// The whole of a match at one moment. The only thing `reduce` takes and the
/// only thing it returns — nothing outside it may affect the result, which is
/// the entire definition of determinism here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchState {
    /// Every body ever raised, alive or fallen. A fallen one keeps its place in
    /// the list so its identity stays valid in the journal for ever.
    pub units: Vec<Unit>,
    pub board: Board,
    pub player: SideState,
    pub keeper: SideState,
    pub round: u8,
    pub active: Side,
    pub outcome: Option<Outcome>,
    pub rules: Rules,
    /// Blows the first side has already landed in the opening round.
    pub opening_attacks_used: u8,
    /// Действий, уже совершённых стороной за этот ход. Читается только когда
    /// `acts_per_turn` не бесконечен.
    #[serde(default)]
    pub acts_this_turn: u8,
}

/// What was asked for. May be refused; refusal is an ordinary answer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
pub enum Action {
    Play { hand_index: usize, cell: Cell },
    Move { unit: UnitId, to: Cell },
    Mend { healer: UnitId, target: UnitId },
    Attack { attacker: UnitId, target: UnitId },
    EndTurn,
}

/// Why it cannot be done. A closed list, so it can be shown to a person in two
/// languages instead of being swallowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Illegal {
    MatchOver,
    NoSuchCard,
    NotYourHalf,
    CellTaken,
    NotEnoughMana,
    NoSuchUnit,
    NotYourUnit,
    UnitIsDown,
    AlreadyActed,
    /// The opening round, and this side moves first: it holds its hand.
    HeldAtTheOpening,
    TargetIsAlly,
    TargetIsDown,
    OutOfReach,
    /// The cell exists and is free, but no walk of this length arrives at it —
    /// either it is too far, or standing bodies are in the way.
    NoWayThere,
    /// This body does not mend at all.
    DoesNotMend,
    /// Это тело не наносит ударов вовсе — котёл, знамя, безоружный лекарь.
    DoesNotStrike,
    /// Mending an unwounded ally would spend a turn on nothing. Refused rather
    /// than allowed and wasted — a legal action that achieves nothing is a trap
    /// for a person and noise for the bot.
    NothingToMend,
    TargetIsEnemy,
}

impl MatchState {
    pub fn begin(setup: Setup) -> MatchState {
        Self::begin_with(setup, Rules::default())
    }

    pub fn begin_with(setup: Setup, rules: Rules) -> MatchState {
        let mut st = MatchState {
            units: Vec::new(),
            board: Board::default(),
            player: SideState { hand: setup.player_hand, ..Default::default() },
            keeper: SideState { hand: setup.keeper_hand, ..Default::default() },
            round: 1,
            active: Side::Player,
            outcome: None,
            rules,
            opening_attacks_used: 0,
            acts_this_turn: 0,
        };
        // The coin is laid before the first turn, so that turn's rise adds to it.
        st.keeper.mana_max = rules.second_side_coin.max(0);

        for (card, cell) in setup.player_board {
            st.raise(&card, cell, Side::Player);
        }
        for (card, cell) in setup.keeper_board {
            st.raise(&card, cell, Side::Keeper);
        }

        // Bodies standing before the first turn are not newly played and may
        // swing at once; only what is played from a hand waits a turn.
        for u in st.units.iter_mut() {
            u.acted = false;
        }
        st.open_turn();
        st
    }

    fn raise(&mut self, card: &CardSnapshot, cell: Cell, owner: Side) -> UnitId {
        let id = self.units.len() as UnitId;
        self.units.push(Unit::from_card(id, card, owner));
        self.board.place(cell, id);
        id
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(id as usize)
    }

    pub fn side_state(&self, side: Side) -> &SideState {
        match side {
            Side::Player => &self.player,
            Side::Keeper => &self.keeper,
        }
    }

    fn side_state_mut(&mut self, side: Side) -> &mut SideState {
        match side {
            Side::Player => &mut self.player,
            Side::Keeper => &mut self.keeper,
        }
    }

    /// Bodies of one side still standing, in the field's scan order.
    pub fn standing(&self, side: Side) -> Vec<UnitId> {
        self.board
            .occupied()
            .map(|(_, id)| id)
            .filter(|id| {
                let u = &self.units[*id as usize];
                u.owner == side && !u.health.is_dead()
            })
            .collect()
    }

    /// Health left standing on one side — what decides a match that ran out of
    /// rounds.
    pub fn standing_health(&self, side: Side) -> i32 {
        self.standing(side).iter().map(|id| self.units[*id as usize].health.current).sum()
    }

    /// Start of the active side's turn: mana rises, bodies are ready again.
    fn open_turn(&mut self) {
        let side = self.active;
        {
            let s = self.side_state_mut(side);
            s.mana_max = (s.mana_max + 1).min(MANA_CAP);
            s.mana = s.mana_max;
        }
        self.acts_this_turn = 0;
        for u in self.units.iter_mut() {
            // Сдача обнуляется у всех: она тратится в чужой ход, а не в свой.
            u.retaliated = false;
            if u.owner == side {
                u.acted = false;
                u.moved = false;
            }
        }
    }

    /// Исчерпала ли сторона отпущенные ей на ход действия.
    pub fn out_of_acts(&self) -> bool {
        self.acts_this_turn >= self.rules.acts_per_turn
    }

    /// Достаёт ли это тело до той клетки — с учётом дальнего выстрела.
    ///
    /// Ближний бой (дальность 1) достаёт только до соседней клетки: стрелять
    /// ему нечем, и штраф за дальний выстрел к нему не относится.
    pub fn can_reach(&self, attacker: &Unit, from: Cell, to: Cell) -> bool {
        from.distance(to) <= attacker.reach
            || (attacker.reach > 1 && self.rules.long_shot_power > 0)
    }

    /// Стрелок ли это, к которому подошли вплотную.
    ///
    /// Стрелок — тело, бьющее дальше соседней клетки. У ближнего боя штрафу
    /// взяться неоткуда: он и так бьёт только вплотную.
    pub fn engaged(&self, unit: &Unit) -> bool {
        if unit.reach <= 1 {
            return false;
        }
        let Some(from) = self.board.cell_of(unit.id) else { return false };
        self.standing(unit.owner.other())
            .iter()
            .filter_map(|id| self.board.cell_of(*id))
            .any(|other| from.distance(other) == 1)
    }

    /// Обычный удар — со штрафом за упор, если он положен.
    ///
    /// Единственное место, где считается сила удара в партии. Бот зовёт его же:
    /// бот, оценивающий удар не по тем числам, которыми удар потом наносится,
    /// выбирает не тот ход, и это не видно ни в одном тесте.
    /// На сколько удары выросли к нынешнему кругу.
    pub fn escalation(&self) -> i32 {
        let from = self.rules.escalation_from;
        if from == 0 || self.round < from {
            return 0;
        }
        (self.round - from + 1) as i32
    }

    pub fn blow(&self, attacker: &Unit, target: &Unit) -> crate::damage::Resolution {
        let late = self.escalation();
        let far = self.long_shot(attacker, target);
        if !self.engaged(attacker) && late == 0 && !far {
            return strike(attacker, target);
        }
        // Оба штрафа перемножаются, когда положены оба: стрелок, которого
        // достали вплотную и который всё равно бьёт вдаль, платит и за то, и за
        // другое. Складывать их значило бы решить за игру, какой из двух помех
        // «главнее».
        let mut kept = attacker.power;
        let mut steps: Vec<(crate::damage::StepId, i32, i32)> = Vec::new();
        if far {
            let next = kept * self.rules.long_shot_power as i32 / 100;
            steps.push((crate::damage::StepId::LongShot, kept, next));
            kept = next;
        }
        if self.engaged(attacker) {
            let next = kept * self.rules.point_blank_power as i32 / 100;
            steps.push((crate::damage::StepId::PointBlank, kept, next));
            kept = next;
        }
        let mut res = crate::damage::resolve(
            Some(attacker),
            target,
            crate::damage::DamagePacket::new(
                kept + late,
                attacker.channel,
                crate::damage::Source::Attack,
            ),
        );
        // Помехи и прибавка — первыми строками следа, в том порядке, в каком
        // считались: без этого стрелок наносил бы три вместо восьми без всякого
        // объяснения, а разбор урона затем и существует.
        if late > 0 {
            res.trail.insert(
                0,
                crate::damage::Breakdown {
                    step: crate::damage::StepId::Escalation,
                    from: kept,
                    to: kept + late,
                },
            );
        }
        for (i, (step, from, to)) in steps.into_iter().enumerate() {
            res.trail.insert(i, crate::damage::Breakdown { step, from, to });
        }
        // Помеха записывается в след первой строкой. Без неё стрелок наносил бы
        // три вместо шести, и на вопрос «почему три» ответа бы не было —
        // а весь разбор урона затем и существует, чтобы ответ был всегда.
        res
    }

    /// Бьёт ли это тело дальше своей дальности.
    fn long_shot(&self, attacker: &Unit, target: &Unit) -> bool {
        if attacker.reach <= 1 || self.rules.long_shot_power == 0 {
            return false;
        }
        match (self.board.cell_of(attacker.id), self.board.cell_of(target.id)) {
            (Some(from), Some(to)) => from.distance(to) > attacker.reach,
            _ => false,
        }
    }

    /// Whether the side to move has used up the blows it is allowed this
    /// opening round. Only the side moving first is ever held.
    pub fn holds_at_the_opening(&self) -> bool {
        self.round == 1
            && self.active == Side::Player
            && self.opening_attacks_used >= self.rules.opening_attacks
    }

    /// A side is out when nothing of it stands and nothing can be put where it
    /// stood.
    ///
    /// Not "the hand is empty". A card costing more than mana will ever reach is
    /// the same as no card at all, and asking only whether the hand held
    /// something left the board bare for twelve rounds while both sides passed
    /// the turn back and forth: the match was long over and only the counter
    /// disagreed.
    ///
    /// The ceiling is the cap and not "what is reachable before time runs out",
    /// because mana rises by one every turn and therefore stands at the cap long
    /// before the rounds do — the clock never gets to decide this. The assertion
    /// below that constant keeps that true rather than hoped.
    fn is_spent(&self, side: Side) -> bool {
        if !self.standing(side).is_empty() {
            return false;
        }
        let affordable = self.side_state(side).hand.iter().any(|c| c.cost <= MANA_CAP);
        // Both halves of `Play`: something to lay down, and somewhere to lay it.
        !(affordable && self.board.free_cells(side).next().is_some())
    }

    fn settle(&mut self, events: &mut Vec<Event>) {
        if self.outcome.is_some() {
            return;
        }
        let player_out = self.is_spent(Side::Player);
        let keeper_out = self.is_spent(Side::Keeper);
        let outcome = match (player_out, keeper_out) {
            (true, true) => Some(Outcome::Draw),
            (true, false) => Some(Outcome::Keeper),
            (false, true) => Some(Outcome::Player),
            (false, false) => None,
        };
        if let Some(o) = outcome {
            self.outcome = Some(o);
            events.push(Event::Finished { outcome: o });
        }
    }

    fn settle_on_time(&mut self, events: &mut Vec<Event>) {
        let p = self.standing_health(Side::Player);
        let k = self.standing_health(Side::Keeper);
        let o = if p > k {
            Outcome::Player
        } else if k > p {
            Outcome::Keeper
        } else {
            Outcome::Draw
        };
        self.outcome = Some(o);
        events.push(Event::Finished { outcome: o });
    }
}

/// The one function that changes anything.
///
/// Takes a state and an intention, returns the next state and what happened.
/// No clock, no database, no chance — the same pair of arguments always gives
/// the same pair of answers, which is what lets a recorded match be replayed to
/// check a rules change and ten thousand matches be run to measure balance.
pub fn reduce(state: &MatchState, action: &Action) -> Result<(MatchState, Vec<Event>), Illegal> {
    if state.outcome.is_some() {
        return Err(Illegal::MatchOver);
    }

    let mut st = state.clone();
    let mut events = Vec::new();
    let side = st.active;

    // Потолок действий за ход. Умолчание — без потолка, и тогда эта строка
    // не срабатывает никогда.
    if !matches!(action, Action::EndTurn) && st.out_of_acts() {
        return Err(Illegal::AlreadyActed);
    }
    if !matches!(action, Action::EndTurn) {
        st.acts_this_turn = st.acts_this_turn.saturating_add(1);
    }

    match action {
        Action::Play { hand_index, cell } => {
            let card = st
                .side_state(side)
                .hand
                .get(*hand_index)
                .cloned()
                .ok_or(Illegal::NoSuchCard)?;
            if cell.side() != side {
                return Err(Illegal::NotYourHalf);
            }
            if !st.board.is_free(*cell) {
                return Err(Illegal::CellTaken);
            }
            if st.side_state(side).mana < card.cost {
                return Err(Illegal::NotEnoughMana);
            }

            st.side_state_mut(side).hand.remove(*hand_index);
            st.side_state_mut(side).mana -= card.cost;
            let id = st.raise(&card, *cell, side);
            events.push(Event::Played { side, unit: id, cell: *cell, cost: card.cost });
        }

        Action::Move { unit, to } => {
            let u = st.unit(*unit).ok_or(Illegal::NoSuchUnit)?.clone();
            if u.owner != side {
                return Err(Illegal::NotYourUnit);
            }
            if u.health.is_dead() {
                return Err(Illegal::UnitIsDown);
            }
            if u.acted {
                return Err(Illegal::AlreadyActed);
            }
            let from = st.board.cell_of(u.id).ok_or(Illegal::UnitIsDown)?;
            if !st.board.reachable(from, u.step).contains(to) {
                return Err(Illegal::NoWayThere);
            }

            if !st.rules.walk_spends_turn && u.moved {
                // Пройти можно раз за ход. Иначе тело гуляет по доске бесплатно.
                return Err(Illegal::AlreadyActed);
            }

            st.board.clear(from);
            st.board.place(*to, u.id);
            // Тратит ли шаг ход целиком — правило, а не устройство доски.
            // Умолчание прежнее: тело либо идёт, либо бьёт.
            if st.rules.walk_spends_turn {
                st.units[u.id as usize].acted = true;
            } else {
                st.units[u.id as usize].moved = true;
            }
            events.push(Event::Moved { unit: u.id, from, to: *to });
        }

        Action::Mend { healer, target } => {
            let h = st.unit(*healer).ok_or(Illegal::NoSuchUnit)?.clone();
            if h.owner != side {
                return Err(Illegal::NotYourUnit);
            }
            if h.health.is_dead() {
                return Err(Illegal::UnitIsDown);
            }
            if h.acted {
                return Err(Illegal::AlreadyActed);
            }
            if h.mend <= 0 {
                return Err(Illegal::DoesNotMend);
            }
            if healer == target {
                // A mender tends others; tending itself is a different verb.
                return Err(Illegal::TargetIsAlly);
            }

            let t = st.unit(*target).ok_or(Illegal::NoSuchUnit)?.clone();
            if t.owner != side {
                return Err(Illegal::TargetIsEnemy);
            }
            if t.health.is_dead() {
                return Err(Illegal::TargetIsDown);
            }
            if t.wound() == 0 {
                return Err(Illegal::NothingToMend);
            }

            let from = st.board.cell_of(h.id).ok_or(Illegal::UnitIsDown)?;
            let to = st.board.cell_of(t.id).ok_or(Illegal::TargetIsDown)?;
            if from.distance(to) > h.reach {
                return Err(Illegal::OutOfReach);
            }

            let mending = crate::heal::resolve_mend(&t, h.mend);
            st.units[h.id as usize].acted = true;
            events.extend(crate::heal::apply_mend(Some(h.id), &mut st.units[t.id as usize], &mending));
        }

        Action::Attack { attacker, target } => {
            let a = st.unit(*attacker).ok_or(Illegal::NoSuchUnit)?.clone();
            if a.owner != side {
                return Err(Illegal::NotYourUnit);
            }
            if a.health.is_dead() {
                return Err(Illegal::UnitIsDown);
            }
            if a.acted {
                return Err(Illegal::AlreadyActed);
            }
            if !a.strikes {
                return Err(Illegal::DoesNotStrike);
            }
            if st.holds_at_the_opening() {
                return Err(Illegal::HeldAtTheOpening);
            }

            let t = st.unit(*target).ok_or(Illegal::NoSuchUnit)?.clone();
            if t.owner == side {
                return Err(Illegal::TargetIsAlly);
            }
            if t.health.is_dead() {
                return Err(Illegal::TargetIsDown);
            }

            let from = st.board.cell_of(a.id).ok_or(Illegal::UnitIsDown)?;
            let to = st.board.cell_of(t.id).ok_or(Illegal::TargetIsDown)?;
            if !st.can_reach(&a, from, to) {
                return Err(Illegal::OutOfReach);
            }

            let res = st.blow(&a, &t);
            st.units[a.id as usize].acted = true;
            if st.round == 1 && side == Side::Player {
                st.opening_attacks_used = st.opening_attacks_used.saturating_add(1);
            }
            events.extend(apply(&mut st.units[t.id as usize], &res));

            // Сдача. Только если ударенный жив, достаёт до обидчика и ещё не
            // отвечал в этот ход. Отвечает `Recoil`, поэтому ответ на ответ
            // невозможен по построению — не по договорённости.
            if st.rules.retaliation
                && !st.units[t.id as usize].health.is_dead()
                && !st.units[t.id as usize].retaliated
                && to.distance(from) <= st.units[t.id as usize].reach
            {
                let defender = st.units[t.id as usize].clone();
                let back = crate::damage::resolve(
                    Some(&defender),
                    &st.units[a.id as usize],
                    crate::damage::DamagePacket::new(
                        defender.printed_power(),
                        defender.channel,
                        crate::damage::Source::Recoil,
                    ),
                );
                st.units[t.id as usize].retaliated = true;
                events.extend(apply(&mut st.units[a.id as usize], &back));
                if st.units[a.id as usize].health.is_dead() {
                    st.board.clear(from);
                }
            }

            if st.units[t.id as usize].health.is_dead() {
                // The body leaves the field; its identity stays in the list, so
                // the journal can still name it a year from now.
                st.board.clear(to);
            }
        }

        Action::EndTurn => {
            // Плата за бездействие берётся ДО того, как ход перейдёт: платит
            // тот, кто простоял, и платит в свой ход, а не в чужой.
            if st.rules.idle_toll > 0 {
                for id in st.standing(side) {
                    let u = &st.units[id as usize];
                    if u.acted || u.moved {
                        continue;
                    }
                    let res = crate::damage::resolve(
                        None,
                        u,
                        crate::damage::DamagePacket::new(
                            st.rules.idle_toll,
                            crate::damage::Channel::Pure,
                            // Не удар и не способность: ни шипы этого не
                            // чувствуют, ни «когда меня ранят».
                            crate::damage::Source::Dot,
                        ),
                    );
                    events.extend(apply(&mut st.units[id as usize], &res));
                    if st.units[id as usize].health.is_dead()
                        && let Some(cell) = st.board.cell_of(id)
                    {
                        st.board.clear(cell);
                    }
                }
            }
            events.push(Event::TurnEnded { side, round: st.round });
            st.active = side.other();
            if st.active == Side::Player {
                st.round += 1;
            }
            if st.round > st.rules.max_rounds {
                st.settle_on_time(&mut events);
                return Ok((st, events));
            }
            st.open_turn();
        }
    }

    st.settle(&mut events);
    Ok((st, events))
}

/// Everything the active side may do right now, in the field's scan order.
///
/// Computed here so the client never computes it. A browser that knows no rule
/// at all can still light the right cells and grey out the right buttons — and
/// there is exactly one place where reach, mana and legality are decided.
pub fn legal_actions(state: &MatchState) -> Vec<Action> {
    if state.outcome.is_some() {
        return Vec::new();
    }
    let side = state.active;
    let mut out = Vec::new();
    if state.out_of_acts() {
        return vec![Action::EndTurn];
    }

    for (i, card) in state.side_state(side).hand.iter().enumerate() {
        if card.cost > state.side_state(side).mana {
            continue;
        }
        for cell in state.board.free_cells(side) {
            out.push(Action::Play { hand_index: i, cell });
        }
    }

    for unit in state.standing(side) {
        let u = &state.units[unit as usize];
        if u.acted || u.step == 0 || (!state.rules.walk_spends_turn && u.moved) {
            continue;
        }
        let Some(from) = state.board.cell_of(unit) else { continue };
        for to in state.board.reachable(from, u.step) {
            out.push(Action::Move { unit, to });
        }
    }

    for healer in state.standing(side) {
        let h = &state.units[healer as usize];
        if h.acted || h.mend <= 0 {
            continue;
        }
        let Some(from) = state.board.cell_of(healer) else { continue };
        for target in state.standing(side) {
            if target == healer || state.units[target as usize].wound() == 0 {
                continue;
            }
            let Some(to) = state.board.cell_of(target) else { continue };
            if from.distance(to) <= h.reach {
                out.push(Action::Mend { healer, target });
            }
        }
    }

    for attacker in state.standing(side) {
        let a = &state.units[attacker as usize];
        // Offered and refused would break the one contract the client relies on.
        if a.acted || !a.strikes || state.holds_at_the_opening() {
            continue;
        }
        let Some(from) = state.board.cell_of(attacker) else { continue };
        for target in state.standing(side.other()) {
            let Some(to) = state.board.cell_of(target) else { continue };
            if state.can_reach(a, from, to) {
                out.push(Action::Attack { attacker, target });
            }
        }
    }

    out.push(Action::EndTurn);
    out
}
