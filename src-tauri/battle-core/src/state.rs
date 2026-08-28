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
}

impl Default for Rules {
    /// Chosen by measurement, not by taste.
    ///
    /// Left alone, the side moving first won 83.5% of mirrored matches. Two
    /// candidates reached the fair corridor, and the deciding test was not which
    /// sat closest to even but which stayed there: across hands one to four
    /// cards deep, this pair moved 2.9 points, while the simpler "no blows at
    /// all in the opening round" moved 13.5 and fell out of the corridor at a
    /// shallow hand. A rule that is only fair for one deck shape is not a rule.
    ///
    /// ```text
    /// share of wins for the side moving first, by hand depth 1..4
    ///   as it was            80.4  83.5  83.5  82.5     spread  3.1
    ///   one blow · coin 2    53.5  51.5  50.5  53.3     spread  2.9  <- this
    ///   no blows at all      41.2  48.8  53.0  54.8     spread 13.5
    /// ```
    fn default() -> Self {
        Self { second_side_coin: 2, opening_attacks: 1 }
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
        for u in self.units.iter_mut() {
            if u.owner == side {
                u.acted = false;
            }
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

            st.board.clear(from);
            st.board.place(*to, u.id);
            // Walking is what this body does this turn. A step and a blow in one
            // turn would be a different action economy, not a new verb — and the
            // economy is the part the frame is not supposed to keep changing.
            st.units[u.id as usize].acted = true;
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
            events.extend(crate::heal::apply_mend(&mut st.units[t.id as usize], &mending));
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
            if from.distance(to) > a.reach {
                return Err(Illegal::OutOfReach);
            }

            let res = strike(&a, &t);
            st.units[a.id as usize].acted = true;
            if st.round == 1 && side == Side::Player {
                st.opening_attacks_used = st.opening_attacks_used.saturating_add(1);
            }
            events.extend(apply(&mut st.units[t.id as usize], &res));

            if st.units[t.id as usize].health.is_dead() {
                // The body leaves the field; its identity stays in the list, so
                // the journal can still name it a year from now.
                st.board.clear(to);
            }
        }

        Action::EndTurn => {
            events.push(Event::TurnEnded { side, round: st.round });
            st.active = side.other();
            if st.active == Side::Player {
                st.round += 1;
            }
            if st.round > MAX_ROUNDS {
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
        if u.acted || u.step == 0 {
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
        if a.acted || state.holds_at_the_opening() {
            continue;
        }
        let Some(from) = state.board.cell_of(attacker) else { continue };
        for target in state.standing(side.other()) {
            let Some(to) = state.board.cell_of(target) else { continue };
            if from.distance(to) <= a.reach {
                out.push(Action::Attack { attacker, target });
            }
        }
    }

    out.push(Action::EndTurn);
    out
}
