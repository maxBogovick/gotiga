//! The frame: a field, turns, an action economy, and an ending.
//!
//! Everything here is written once and is not meant to be rewritten. Later
//! stages add vocabulary — more verbs, shapes, triggers — by widening what an
//! `Action` may be and what `reduce` dispatches to, never by reshaping the loop
//! below. If a later stage has to change the signature of `reduce`, the journal
//! format or the shape of a legal action, the stages were cut wrongly.

use crate::board::{Board, Cell, Field, Side};
use crate::card::{AbilitySnapshot as CardAbilitySnap, CardSnapshot};
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
    /// Прорыв: тело, которое дошло до дальнего ряда чужой половины и
    /// простояло там ход противника, выигрывает партию.
    ///
    /// Вторая цель рядом с «уничтожить всех» — ту обороняющийся вправе не
    /// преследовать, и пятая часть партий против умелой руки не кончалась
    /// никогда (§19, §20). Эту пересидеть нельзя: кто стоит, тот пропускает.
    ///
    /// «Простояло ход», а не «дошло»: от переднего ряда до чужого края три
    /// клетки, и тело с шагом 3 доходило бы в первый же ход, не дав ответить.
    ///
    /// Засчитывается хозяину и только пока тело ходит за него: иначе смута,
    /// наведённая на тело у собственного края, выигрывала бы партию наводящему.
    ///
    /// Умолчание сериализации — выключено: партии и этюды, записанные до этой
    /// ручки, играются тем, чем игрались.
    ///
    /// Умолчание дома — тоже выключено, и это выбрано замером
    /// (`examples/proryv.rs`, `TASKS-BATTLE-ENGINE.md` §25):
    ///
    /// ```text
    /// рукой с перебором           лимитом  ничьих  1-й ход  не кончилось за 60
    ///   без прорыва                 23.6 %   2.0 %   56.8 %   6.7 %
    ///   прорыв                      12.0 %   0.8 %   53.6 %   7.5 %
    /// ```
    ///
    /// Счётчиком решено вдвое реже, но партии, не кончающиеся никогда, остались:
    /// на поле шириной 3 один страж в средней колонке перекрывает весь ряд, а
    /// шаг вбок от платы за простой освобождает. И при руке в одну карту
    /// первый ход берёт 65 %. Ручка остаётся этюдам.
    #[serde(default)]
    pub breakthrough: bool,
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
            breakthrough: false,
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
    /// Местность: клетки с правилом. Лежит в расстановке, потому что
    /// расстановка замораживается вместе с партией, — и пересмотр, и свёртка
    /// журнала видят то же поле, на котором играли.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain: Vec<Tile>,
    /// Величина поля. Не названная — 3 × 3 на половину, как было всегда, и
    /// тогда не пишется вовсе: замороженные партии не меняются ни на байт.
    #[serde(default, skip_serializing_if = "Field::is_default")]
    pub field: Field,
}

/// Что за земля под клеткой. Три рода, и список закрыт по той же причине, по
/// которой закрыт словарь глаголов: новая местность — новое сочетание этих,
/// а не четвёртое правило в коде.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ground {
    /// Стена: на неё не встают и через неё не проходят. Удары и чары через
    /// неё летят — поле 3 × 6 слишком мало для линий видимости, и правило,
    /// которое надо проверять глазом по клеткам, здесь не читается.
    Wall,
    /// Укрытие: удар издали (дальше соседней клетки) по стоящему здесь доходит
    /// вполсилы. Не «не достаёт вовсе»: недосягаемая клетка — это та самая
    /// безопасная клетка, на которой стоят до конца партии (§20), и её здесь
    /// убирали.
    Cover,
    /// Болото: войти можно, пройти насквозь — нет. Вошедший останавливается,
    /// стоящий в нём выходит на одну клетку.
    Mire,
    /// Овраг: на него не встают. Тело с шагом 1 его не пересечёт, с шагом 2 и
    /// больше — перепрыгнет. Сброшенное в овраг толчком тело гибнет: это то
    /// место на поле, где позиция решает сама, без чисел на карте.
    Ravine,
    /// Яма: кто оказался в ней по ходу партии — шагом, толчком, из руки,
    /// призывом, — получает `PIT_HARM` урона от земли и дальше в этот ход не
    /// идёт. Стоять в ней можно; ранит она один раз, при входе.
    Pit,
    /// Холм: стрелок (дальность больше одной клетки), стоящий здесь, бьёт на
    /// клетку дальше. Ближнему бою холм не даёт ничего — с холма не дотянешься
    /// рукой до соседа через клетку.
    Hill,
    /// Родник: тело, стоящее здесь, в начале своего хода восстанавливает
    /// `SPRING_MEND` здоровья. Повод держать клетку, а не уходить от боя.
    Spring,
}

/// Сколько ранит яма при входе.
pub const PIT_HARM: i32 = 2;

/// Сколько возвращает родник в начале своего хода.
pub const SPRING_MEND: i32 = 1;

/// Одна клетка местности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    pub cell: Cell,
    pub ground: Ground,
}

/// Сколько силы, в сотых, доходит до тела в укрытии из-за соседней клетки.
pub const COVER_KEPT: i32 = 50;

/// Опасная клетка поля.
///
/// Живёт в состоянии, а не на теле, потому что она и есть не тело: ведьмин
/// котёл остаётся стоять там, где стоял, даже когда ведьму убили, и уходит по
/// своему сроку. Чей срок — сказано `side`: тикает зона ходами того, кто её
/// поставил, ровно как всадник тикает ходами носителя.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zone {
    /// Ключ умения: одноимённая зона обновляет срок, а не копится второй.
    pub name: String,
    pub side: Side,
    pub by: Option<UnitId>,
    pub cells: Vec<Cell>,
    pub amount: i32,
    pub channel: crate::damage::Channel,
    pub turns: u8,
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
    /// Опасные клетки. Пусто на всякой партии, начатой до них.
    #[serde(default)]
    pub zones: Vec<Zone>,
    /// Тела, которые закончили ход своей стороны на её ряду прорыва. Решается
    /// в начале следующего хода той же стороны: кто там всё ещё стоит, тот и
    /// выиграл. Обеих сторон в одном списке: записывается одна, а читается
    /// другая, и два поля пришлось бы держать в согласии вручную.
    #[serde(default)]
    pub poised: Vec<UnitId>,
    /// Местность поля. Пусто на всякой партии, начатой до неё.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain: Vec<Tile>,
    /// Величина поля. Партия, начатая до неё, играется на 3 × 3.
    #[serde(default, skip_serializing_if = "Field::is_default")]
    pub field: Field,
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
    /// Навести чару. `ability` — КЛЮЧ умения (`ability_key`), а не его номер в
    /// списке: номер значит одно и то же только пока карту не правили, а карта
    /// в журнале переживает и правку, и перебалансировку. Тот же ключ носит
    /// откат и тот же — всадник.
    Cast {
        caster: UnitId,
        ability: String,
        /// Тело или КЛЕТКА. Двумя словами, а не номером тела: опасная клетка
        /// ставится там, где никто не стоит, и призванному телу нужно пустое
        /// место — обе чары в «номер тела» не записываются вовсе.
        target: Mark,
    },
    EndTurn,
}

/// Куда наведена чара.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
pub enum Mark {
    Unit(UnitId),
    Spot(Cell),
}

impl Mark {
    pub fn unit(self) -> Option<UnitId> {
        match self {
            Mark::Unit(id) => Some(id),
            Mark::Spot(_) => None,
        }
    }

    pub fn spot(self) -> Option<Cell> {
        match self {
            Mark::Spot(cell) => Some(cell),
            Mark::Unit(_) => None,
        }
    }
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
    /// У этого тела нет такого умения — или есть, но движок его не играет.
    /// Одно слово на оба случая нарочно: для спрашивающего это одно и то же, а
    /// разница между ними — дело свода, а не партии.
    NoSuchAbility,
    /// Чара ещё не вернулась.
    AbilityAsleep,
    /// Наводить чару на себя, когда она не про себя, — и наоборот. Сюда же
    /// клетка вместо тела и тело вместо клетки: для спрашивающего это одно и то
    /// же — «не туда».
    NotThatAim,
    /// Тело не делает ничего: оцепенение.
    Bound,
    /// Тело не наводит чар: немота.
    Hushed,
    /// Тело не бьёт сейчас: разоружено.
    Disarmed,
    /// Целью не выбирают: покров. По площади — достаётся.
    Veiled,
    /// Только что вышло из-под оцепенения или смуты (§5.3).
    Rested,
    /// Сюда встать нельзя: занято или такой клетки нет.
    NoRoom,
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
            zones: Vec::new(),
            poised: Vec::new(),
            terrain: Vec::new(),
            field: setup.field.normalized(),
        };
        // The coin is laid before the first turn, so that turn's rise adds to it.
        st.keeper.mana_max = rules.second_side_coin.max(0);

        for (card, cell) in setup.player_board {
            st.raise(&card, cell, Side::Player);
        }
        for (card, cell) in setup.keeper_board {
            st.raise(&card, cell, Side::Keeper);
        }

        // Местность ложится после тел. Клетка вне поля, вторая запись о той же
        // клетке и стена под стоящим телом отбрасываются молча: расстановку
        // проверяет сервер, а движок только не даёт ей сломать партию.
        for tile in setup.terrain {
            let inside = st.field.contains(tile.cell);
            let repeated = st.terrain.iter().any(|t| t.cell == tile.cell);
            let walled_in = matches!(tile.ground, Ground::Wall | Ground::Ravine)
                && !st.board.is_free(tile.cell);
            if inside && !repeated && !walled_in {
                st.terrain.push(tile);
            }
        }

        // Bodies standing before the first turn are not newly played and may
        // swing at once; only what is played from a hand waits a turn.
        for u in st.units.iter_mut() {
            u.acted = false;
        }
        // Расставленные дышат с самого начала: аура — это состояние поля, а не
        // событие, и первый же удар обязан считаться с ней.
        st.breathe();
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
                // За КОГО оно стоит, а не чьё оно: смута разводит эти два
                // вопроса, и бой спрашивает первый. Второй спрашивают там, где
                // речь о хозяйстве, — `is_spent` и счёт здоровья на исходе.
                u.side() == side && !u.health.is_dead()
            })
            .collect()
    }

    /// Чьи тела ещё стоят — по хозяину, а не по тому, за кого они бьют.
    pub fn owned_standing(&self, side: Side) -> Vec<UnitId> {
        self.board
            .occupied()
            .map(|(_, id)| id)
            .filter(|id| {
                let u = &self.units[*id as usize];
                u.owner == side && !u.health.is_dead()
            })
            .collect()
    }

    /// Что за земля под клеткой.
    pub fn ground(&self, cell: Cell) -> Option<Ground> {
        self.terrain.iter().find(|t| t.cell == cell).map(|t| t.ground)
    }

    /// Можно ли сюда встать: клетка пуста, и это не стена и не овраг.
    pub fn open(&self, cell: Cell) -> bool {
        self.board.is_free(cell) && !self.unstandable(cell)
    }

    /// Стена или овраг: на такую землю не встают.
    fn unstandable(&self, cell: Cell) -> bool {
        matches!(self.ground(cell), Some(Ground::Wall | Ground::Ravine))
    }

    /// Свободные клетки стороны, куда можно выставить тело, — в порядке обхода.
    pub fn free_cells(&self, side: Side) -> Vec<Cell> {
        self.board.free_cells(&self.field, side).filter(|c| !self.unstandable(*c)).collect()
    }

    /// Дальность удара с учётом земли: стрелок на холме бьёт на клетку дальше.
    pub fn reach_of(&self, unit: &Unit) -> u8 {
        let on_hill = self
            .board
            .cell_of(unit.id)
            .is_some_and(|c| self.ground(c) == Some(Ground::Hill));
        if on_hill && unit.reach > 1 { unit.reach + 1 } else { unit.reach }
    }

    /// Тело оказалось на клетке по ходу партии: яма ранит его. Одно место на
    /// все пути на клетку — шаг, толчок, свой прыжок, выход из руки, призыв:
    /// забыть яму в одной ветке из пяти ничего не стоит.
    fn arrive(&mut self, id: UnitId, events: &mut Vec<Event>) {
        let Some(cell) = self.board.cell_of(id) else { return };
        if self.ground(cell) != Some(Ground::Pit) {
            return;
        }
        let body = self.units[id as usize].clone();
        let res = crate::damage::resolve(
            None,
            &body,
            crate::damage::DamagePacket::new(PIT_HARM, crate::damage::Channel::Physical, crate::damage::Source::Zone),
        );
        events.extend(apply(&mut self.units[id as usize], &res));
    }

    /// Куда тело дойдёт с этой клетки — с местностью.
    ///
    /// Тот же обход в ширину, что у `Board::reachable`, и при пустой местности
    /// ответ у них один до клетки: стена не пускает; болото и яма пускают, но
    /// дальше из них в этот ход не идут; стоящий в болоте выходит на одну
    /// клетку; овраг перепрыгивают, если шага хватает на клетку за ним.
    pub fn walkable(&self, from: Cell, step: u8) -> Vec<Cell> {
        if self.terrain.is_empty() {
            return self.board.reachable(&self.field, from, step);
        }
        let step = if self.ground(from) == Some(Ground::Mire) { step.min(1) } else { step };
        let mut seen: Vec<Cell> = vec![from];
        let mut frontier = vec![from];
        for _ in 0..step {
            let mut next = Vec::new();
            for cell in frontier.drain(..) {
                // Болото и яма держат вошедшего: дальше из них в этот ход нет.
                if cell != from && matches!(self.ground(cell), Some(Ground::Mire | Ground::Pit)) {
                    continue;
                }
                for n in self.field.neighbours(cell) {
                    // Через овраг идут как через клетку — встать на нём нельзя,
                    // но перепрыгнуть можно, если шага хватает на клетку за ним.
                    let passable = self.board.is_free(n) && self.ground(n) != Some(Ground::Wall);
                    if !seen.contains(&n) && passable {
                        seen.push(n);
                        next.push(n);
                    }
                }
            }
            frontier = next;
            if frontier.is_empty() {
                break;
            }
        }
        self.field.cells()
            .filter(|c| *c != from && seen.contains(c) && !self.unstandable(*c))
            .collect()
    }

    /// Тела стороны, стоящие на её ряду прорыва и ходящие за неё.
    ///
    /// Хозяин и тот, за кого тело ходит, должны совпасть: уведённое смутой
    /// тело прорыва не делает ни хозяину (он им не ходит), ни уведшему (иначе
    /// смута на тело у собственного края выигрывала бы партию).
    pub fn on_goal(&self, side: Side) -> Vec<UnitId> {
        self.owned_standing(side)
            .into_iter()
            .filter(|id| {
                let u = &self.units[*id as usize];
                u.side() == side
                    && self.board.cell_of(*id).is_some_and(|c| c.y == self.field.goal_row(side))
            })
            .collect()
    }

    /// Тело стороны, которое закончило её прошлый ход на ряду прорыва и стоит
    /// там до сих пор.
    pub fn breaker(&self, side: Side) -> Option<UnitId> {
        let now = self.on_goal(side);
        self.poised.iter().copied().find(|id| now.contains(id))
    }

    /// Health left standing on one side — what decides a match that ran out of
    /// rounds.
    pub fn standing_health(&self, side: Side) -> i32 {
        // По хозяину: смута временна, а материал — чей он есть.
        self.owned_standing(side)
            .iter()
            .map(|id| self.units[*id as usize].health.current)
            .sum()
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
            // Просыпается то, что ходит ЗА эту сторону: уведённое смутой тело
            // ходит с тем, кто его увёл, — иначе оно стояло бы у него в руках
            // без права шевельнуться.
            if u.side() == side {
                u.acted = false;
                u.moved = false;
                u.tick_ability_cds();
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
        from.distance(to) <= self.reach_of(attacker)
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

    /// Стоит ли цель в укрытии, а бьют её не с соседней клетки.
    pub fn covered(&self, attacker: &Unit, target: &Unit) -> bool {
        let (Some(from), Some(at)) = (self.board.cell_of(attacker.id), self.board.cell_of(target.id)) else {
            return false;
        };
        self.ground(at) == Some(Ground::Cover) && from.distance(at) > 1
    }

    pub fn blow(&self, attacker: &Unit, target: &Unit) -> crate::damage::Resolution {
        let late = self.escalation();
        let far = self.long_shot(attacker, target);
        let covered = self.covered(attacker, target);
        if !self.engaged(attacker) && late == 0 && !far && !covered {
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
        if covered {
            let next = kept * COVER_KEPT / 100;
            steps.push((crate::damage::StepId::Cover, kept, next));
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

    /// Кто ответит на то, что только что случилось.
    ///
    /// Читается по СЛЕДАМ — по событиям, которые действие уже положило, — а не
    /// по тому, что действие собиралось сделать. Разница видна на страже:
    /// удар был нацелен в соседа, а пришёл ей, и отвечать на него ей же.
    ///
    /// Порядок поводов выбран, а не сложился: сперва ударивший («когда я бью»),
    /// потом раненые («когда меня ранят»), потом павшие («предсмертный»). Он
    /// же порядок чтения вслух — и он же порядок, в котором это случилось бы за
    /// столом.
    fn answer(&mut self, by: Option<UnitId>, since: usize, events: &mut Vec<Event>) {
        let mut hurt: Vec<UnitId> = Vec::new();
        let mut fallen: Vec<UnitId> = Vec::new();
        for e in events.iter().skip(since) {
            match e {
                Event::Damaged { target, source, .. } if source.is_felt() => {
                    if !hurt.contains(target) {
                        hurt.push(*target);
                    }
                }
                Event::Died { target } => {
                    if !fallen.contains(target) {
                        fallen.push(*target);
                    }
                }
                _ => {}
            }
        }
        if let Some(striker) = by
            && !hurt.is_empty()
        {
            self.reacts("onHit", striker, hurt.first().copied(), events);
        }
        for id in hurt {
            if !self.units[id as usize].health.is_dead() {
                self.reacts("onDamaged", id, by, events);
            }
        }
        // Предсмертный дар срабатывает, пока тело ещё стоит на доске: у чары
        // должно быть место, откуда она идёт. Убирает павших один общий проход
        // в конце действия — и потому здесь их убирать не надо.
        for id in fallen {
            self.reacts("onDeath", id, by, events);
        }
    }

    /// Убрать с доски всех, кто пал за это действие.
    ///
    /// Один проход в конце, а не `clear` в каждой ветке: павших за одно
    /// действие бывает шестеро — от круга, от шипов, от сдачи, от предсмертного
    /// дара, — и ветка, забывшая убрать своего, оставляет на поле тело, которое
    /// и не живо, и не убрано.
    fn bury_the_fallen(&mut self) {
        let dead: Vec<Cell> = self
            .board
            .occupied()
            .filter(|(_, id)| self.units[*id as usize].health.is_dead())
            .map(|(cell, _)| cell)
            .collect();
        for cell in dead {
            self.board.clear(cell);
        }
    }

    /// Чем дышит поле: пересчитать все ауры заново.
    ///
    /// Пересчёт целиком, а не правка по месту, и это главное решение здесь.
    /// Аура не событие, а СОСТОЯНИЕ: она есть, пока стоит тот, кто ею дышит, и
    /// пропадает в тот же миг, как он пал, ушёл смутой на другую сторону или
    /// его сдвинули на клетку дальше. Пытаться поддерживать это правками —
    /// значит перечислить все способы, которыми доска меняется, и забыть
    /// седьмой. Пересчёт стоит один обход поля и не может разойтись с доской,
    /// потому что читает её же.
    ///
    /// Аурой бывают только благословение и проклятие. Остальные глаголы — это
    /// события: «призвать, пока стою» или «толкнуть, пока стою» не значат
    /// ничего, и стол отказывает в таком сочетании словами.
    fn breathe(&mut self) {
        for u in self.units.iter_mut() {
            u.aura.clear();
        }
        for bearer in self.board.occupied().map(|(_, id)| id).collect::<Vec<_>>() {
            if self.units[bearer as usize].health.is_dead() {
                continue;
            }
            let body = self.units[bearer as usize].clone();
            let Some(from) = self.board.cell_of(bearer) else { continue };
            for (i, a) in body.card.abilities.iter().enumerate() {
                if a.trigger != "aura" || a.amount <= 0 {
                    continue;
                }
                let Some(what) = crate::spell::Casting::of_verb(&a.verb) else { continue };
                if !matches!(what, crate::spell::Casting::Bless | crate::spell::Casting::Curse) {
                    continue;
                }
                let key = crate::unit::ability_key(a, i);
                let Some(mut status) = crate::spell::rider(a, what, &key) else { continue };
                // У ауры нет срока: она держится собой. Число здесь — только
                // чтобы всадник был правильно устроен; тикать его никто не будет.
                status.turns = 1;
                // Дышит она ОТ СЕБЯ: цели у ауры нет, и потому нет и вопроса,
                // куда наведено, — она берёт то, что стоит вокруг носителя.
                // Своих или чужих, решает глагол, и решает он так же, как у
                // приказа: благословение своим, проклятие чужим.
                let ill = matches!(what, crate::spell::Casting::Curse);
                let reach = crate::spell::reach_of(&a.shape, a.radius);
                for id in self.around(reach, from, body.side(), ill, a.range) {
                    self.units[id as usize].aura.push(status.clone());
                }
            }
        }
    }

    /// Кого накрывает то, что дышит ОТ НОСИТЕЛЯ: у ауры цели нет, и пригоршня
    /// считается от его собственной клетки.
    ///
    /// `foes` говорит, чью сторону собирать. `range` держит ауру в её
    /// дальности: «пока стою» не значит «на всё поле», если на карте написано
    /// иначе; форма `self` — только сам носитель.
    fn around(
        &self,
        reach: crate::spell::Reach,
        from: Cell,
        side: Side,
        foes: bool,
        range: u8,
    ) -> Vec<UnitId> {
        let want = if foes { side.other() } else { side };
        self.board
            .occupied()
            .filter(|(cell, id)| {
                let u = &self.units[*id as usize];
                if u.health.is_dead() || u.side() != want {
                    return false;
                }
                if cell.distance(from) > range {
                    return false;
                }
                match reach {
                    crate::spell::Reach::One => *id == self.board.at(from).unwrap_or(*id),
                    crate::spell::Reach::Adjacent => cell.distance(from) <= 1,
                    crate::spell::Reach::Radius(r) => cell.distance(from) <= r,
                    crate::spell::Reach::Chain(n) => cell.distance(from) <= n,
                    crate::spell::Reach::Line => cell.x == from.x || cell.y == from.y,
                    crate::spell::Reach::Side => true,
                }
            })
            .map(|(_, id)| id)
            .collect()
    }

    /// Ответить на повод: сработать всем умениям этого тела, заведённым на него.
    ///
    /// `other` — тот, кто на другом конце повода: кого ударили, кто ударил, кто
    /// свалил. Цель реакция выбирает САМА, и в этом её отличие от приказа:
    /// поводу нечего спрашивать у человека. Правило одно на все поводы —
    /// **берётся тот, о ком повод; а если повод ни о ком (вышел на поле, начался
    /// ход), то ближайший** из тех, кого этот глагол берёт. Ближайший, а не
    /// первый попавшийся: два одинаково близких разрешаются обходом доски, и
    /// тогда партия переигрывается.
    ///
    /// Реакция не тратит ни ход, ни ману — она и есть то, за что платят ценой
    /// карты, — но откат соблюдает: «раз в два хода» значит раз в два хода,
    /// кем бы повод ни был подан.
    ///
    /// Реакция НЕ вызывает реакций. Это записано здесь тем, что `work` никого
    /// не зовёт обратно, и это не осторожность, а условие: цепь из «ранили —
    /// отвечаю — ранил — отвечают» не имеет дна, и одна карта уронила бы доску.
    fn reacts(
        &mut self,
        trigger: &str,
        bearer: UnitId,
        other: Option<UnitId>,
        events: &mut Vec<Event>,
    ) {
        let Some(body) = self.units.get(bearer as usize).cloned() else { return };
        if body.health.is_dead() && trigger != "onDeath" {
            return;
        }
        let Some(from) = self.board.cell_of(bearer) else { return };
        let side = body.side();

        for (i, a) in body.card.abilities.iter().enumerate() {
            if a.trigger != trigger || a.amount <= 0 {
                continue;
            }
            // Лечение не в словаре чар (`Casting::of_verb` его не знает: рукой
            // оно играется `Action::Mend`), но и случиться само оно обязано:
            // у вампира напечатано «восстанавливает 2 здоровья после каждого
            // удара», и пока здесь стояло `continue`, этот повод молча не
            // делал ничего. Хода и маны не тратит, как всякая реакция.
            if a.verb == "heal" {
                let key = crate::unit::ability_key(a, i);
                self.react_heal(bearer, a, &key, events);
                continue;
            }
            let Some(what) = a.casting() else { continue };
            let key = crate::unit::ability_key(a, i);
            if self.units[bearer as usize].ability_cd(&key) > 0 {
                continue;
            }

            // Куда. Клеточные чары реакцией наводятся себе под ноги: выбирать
            // клетку некому, а «где-нибудь» — не место.
            let aim = what.aim(&a.shape);
            let target = match aim {
                crate::spell::Aim::Spot { .. } => None,
                crate::spell::Aim::Bearer => Some(bearer),
                _ => {
                    let wants_foe = matches!(aim, crate::spell::Aim::Foe);
                    let named = other.filter(|id| {
                        let u = &self.units[*id as usize];
                        !u.health.is_dead()
                            && (u.side() != side) == wants_foe
                            && self
                                .board
                                .cell_of(*id)
                                .is_some_and(|c| c.distance(from) <= a.range)
                    });
                    named.or_else(|| {
                        // Повод ни о ком — берём ближайшего, кого этот глагол
                        // берёт. Дальше своей дальности реакция не достаёт.
                        let want = if wants_foe { side.other() } else { side };
                        self.board
                            .occupied()
                            .filter(|(cell, id)| {
                                let u = &self.units[*id as usize];
                                !u.health.is_dead()
                                    && u.side() == want
                                    && (*id != bearer || !wants_foe)
                                    && cell.distance(from) <= a.range
                            })
                            .min_by_key(|(cell, _)| cell.distance(from))
                            .map(|(_, id)| id)
                    })
                }
            };
            let spot = match target {
                Some(id) => match self.board.cell_of(id) {
                    Some(cell) => cell,
                    None => continue,
                },
                None if matches!(aim, crate::spell::Aim::Spot { .. }) => from,
                None => continue,
            };

            let mut swept: Vec<UnitId> = match (target, what.spreads()) {
                (Some(id), true) => {
                    let mut all = self.touched(crate::spell::reach_of(&a.shape, a.radius), from, spot);
                    all.retain(|x| *x != id);
                    let mut out = vec![id];
                    out.append(&mut all);
                    out
                }
                (Some(id), false) => vec![id],
                (None, _) => Vec::new(),
            };
            swept.retain(|id| !self.units[*id as usize].health.is_dead());

            let mine = swept
                .first()
                .map(|id| self.units[*id as usize].side() == side)
                .unwrap_or(false);
            let span = crate::spell::turns(a) + if mine { 1 } else { 0 };
            if a.cooldown > 0 {
                self.units[bearer as usize].start_ability_cd(&key, a.cooldown);
            }
            // Отказ реакции — не ошибка партии: призыв без места просто не
            // случается. Человек его и не просил.
            let _ = self.work(&body, a, what, &key, &swept, from, spot, span, side, events);
        }
    }

    /// Лечение по поводу: глоток по удару, исцеление при выходе, дар при гибели.
    ///
    /// Цель — носитель (форма `self`) или ближайший РАНЕНЫЙ свой в пределах
    /// дальности умения, не считая носителя: «затянуть чужую рану» значит
    /// «чужую», и самолечение — отдельная форма. Лечить некого — реакция не
    /// случается и откат не тратит. Количество считает тот же `resolve_mend`,
    /// что и рука: выше полного здоровья не лечат.
    fn react_heal(&mut self, bearer: UnitId, a: &CardAbilitySnap, key: &str, events: &mut Vec<Event>) {
        if a.amount <= 0 || self.units[bearer as usize].ability_cd(key) > 0 {
            return;
        }
        let Some(from) = self.board.cell_of(bearer) else { return };
        let side = self.units[bearer as usize].side();
        let target = if a.shape == "self" {
            Some(bearer)
        } else {
            self.board
                .occupied()
                .filter(|(cell, id)| {
                    let u = &self.units[*id as usize];
                    *id != bearer
                        && !u.health.is_dead()
                        && u.side() == side
                        && u.wound() > 0
                        && cell.distance(from) <= a.range
                })
                .min_by_key(|(cell, _)| cell.distance(from))
                .map(|(_, id)| id)
        };
        let Some(target) = target else { return };
        let mending = crate::heal::resolve_mend(&self.units[target as usize], a.amount);
        if mending.restored == 0 {
            return;
        }
        if a.cooldown > 0 {
            self.units[bearer as usize].start_ability_cd(key, a.cooldown);
        }
        events.extend(crate::heal::apply_mend(Some(bearer), &mut self.units[target as usize], &mending));
    }

    /// ЧТО делает чара, когда уже решено, кто её наводит и в кого.
    ///
    /// Вынесено из ветки приказа, потому что приказ у чары не единственный
    /// повод: аура, предсмертный дар и ответ на удар делают ТО ЖЕ САМОЕ, и
    /// второе место, где это написано, разошлось бы с первым на первой же
    /// правке. Здесь — только действие; кто, кого и почём решено выше.
    #[allow(clippy::too_many_arguments)]
    fn work(
        &mut self,
        c: &Unit,
        a: &CardAbilitySnap,
        what: crate::spell::Casting,
        key: &str,
        swept: &[UnitId],
        from: Cell,
        spot: Cell,
        span: u8,
        side: Side,
        events: &mut Vec<Event>,
    ) -> Result<(), Illegal> {
        let swept = swept.to_vec();
        let t_id = swept.first().copied();
        match what {
            crate::spell::Casting::Harm => {
                for (i, id) in swept.clone().into_iter().enumerate() {
                    let aimed_at_him = i == 0;
                    // Стража принимает НАЦЕЛЕННОЕ, а не всё, что сыплется
                    // кругом: иначе один страж собирал бы на себя весь круг
                    // и стоил бы вчетверо против написанного.
                    let hit = if aimed_at_him { self.shielded_by_guard(id) } else { id };
                    let victim = self.units[hit as usize].clone();
                    let res = crate::damage::resolve(
                        Some(&c),
                        &victim,
                        crate::damage::DamagePacket::new(
                            a.amount,
                            a.channel,
                            // Задетые кругом чувствуют ПЛЕСК: шипы на него
                            // не отвечают (`provokes_thorns`), и это не
                            // мелочь — иначе круг по пятерым в шипах убивал
                            // бы ведьму об её же чару.
                            if aimed_at_him {
                                crate::damage::Source::Ability
                            } else {
                                crate::damage::Source::Splash
                            },
                        ),
                    );
                    events.extend(apply(&mut self.units[hit as usize], &res));
                    // Сдачи у чары нет, и это правило, а не упущение: сдача
                    // в движке — ответный УДАР, а мечом на проклятие с
                    // четырёх клеток не отвечают. Шипы — другое дело: это
                    // возмездие, и оно достаёт всякого, кто тронул.
                    if aimed_at_him {
                        self.prick(hit, c.id, events);
                    }
                }
            }
            crate::spell::Casting::Shield => {
                for id in &swept {
                    events.extend(crate::spell::raise_shield(
                        Some(c.id),
                        &mut self.units[*id as usize],
                        a.amount,
                    ));
                }
            }
            crate::spell::Casting::Curse | crate::spell::Casting::Bless => {
                // Имя всадника — ключ умения: по нему одноимённый освежает
                // срок вместо того, чтобы удвоить число.
                if let Some(mut status) = crate::spell::rider(&a, what, &key) {
                    status.turns = span;
                    for id in &swept {
                        events.extend(crate::spell::lay_rider(
                            Some(c.id),
                            &mut self.units[*id as usize],
                            status.clone(),
                            what,
                        ));
                    }
                }
            }
            crate::spell::Casting::Cleanse | crate::spell::Casting::Dispel => {
                // Снимает то, что чужой руке мешает, и только его: очищение
                // — проклятия со своего, развеивание — благословения с
                // чужого. `amount` — сколько разом, и ноль значит одно.
                let ill = matches!(what, crate::spell::Casting::Cleanse);
                for id in &swept {
                    events.extend(crate::spell::lift_riders(
                        Some(c.id),
                        &mut self.units[*id as usize],
                        ill,
                        a.amount.max(1) as usize,
                    ));
                }
            }
            crate::spell::Casting::Coin => {
                // Мана — у СТОРОНЫ, и добрать её выше собственного потолка
                // нельзя: это возврат, а не второй источник.
                let s = self.side_state_mut(side);
                let before = s.mana;
                s.mana = (s.mana + a.amount).min(s.mana_max);
                let got = s.mana - before;
                if got > 0 {
                    events.push(Event::Mana { side, amount: got });
                }
            }
            crate::spell::Casting::Offer => {
                // Жертва: тело уходит с поля, сторона получает ману. Гибель
                // настоящая — со снятием всадников и с событием, — потому
                // что «ушло с поля» и «погибло» в этом движке одно и то же.
                let given = t_id.unwrap();
                self.units[given as usize].health.current = 0;
                self.units[given as usize].statuses.clear();
                self.units[given as usize].holds.clear();
                if let Some(cell) = self.board.cell_of(given) {
                    self.board.clear(cell);
                }
                events.push(Event::Died { target: given });
                let s = self.side_state_mut(side);
                let before = s.mana;
                s.mana = (s.mana + a.amount).min(MANA_CAP);
                let got = s.mana - before;
                if got > 0 {
                    events.push(Event::Mana { side, amount: got });
                }
            }
            crate::spell::Casting::Shove => {
                if let Some(id) = t_id {
                    // Толчок от себя, притяжение к себе: направление — это
                    // и есть разница между ними, и берётся оно из того, кто
                    // кому свой, а не из поля на карте, которого нет.
                    let away = self.units[id as usize].side() != side;
                    let (to, fell) = self.slide(from, spot, a.amount, away);
                    if let Some(chasm) = fell {
                        // Падение — та же гибель, что у жертвы: со снятием
                        // всадников и с событием. `Fell` идёт первым, чтобы сцена
                        // показала, КУДА, а не только что тела больше нет.
                        self.units[id as usize].health.current = 0;
                        self.units[id as usize].statuses.clear();
                        self.units[id as usize].holds.clear();
                        self.board.clear(spot);
                        events.push(Event::Fell { unit: id, from: spot, to: chasm });
                        events.push(Event::Died { target: id });
                    } else if to != spot {
                        self.board.clear(spot);
                        self.board.place(to, id);
                        events.push(Event::Moved { unit: id, from: spot, to });
                        self.arrive(id, events);
                    }
                } else {
                    // Свой шаг: тело прыгает на пустую клетку в пределах
                    // чары. Не `reachable`: это чара, а не ходьба, и стоящие
                    // на пути тела ей не помеха.
                    self.board.clear(from);
                    self.board.place(spot, c.id);
                    events.push(Event::Moved { unit: c.id, from, to: spot });
                    self.arrive(c.id, events);
                }
            }
            crate::spell::Casting::Zone => {
                // Клетка и её соседи по радиусу. Радиус — то самое поле,
                // которое у формы уже есть, и здесь оно значит то же, что
                // везде: сколько шагов короля вокруг.
                let cells: Vec<Cell> = if a.radius > 0 {
                    let mut out: Vec<Cell> = self.field.cells()
                        .filter(|cell| cell.distance(spot) <= a.radius)
                        .collect();
                    out.sort();
                    out
                } else {
                    vec![spot]
                };
                let zone = Zone {
                    name: key.to_string(),
                    side,
                    by: Some(c.id),
                    cells: cells.clone(),
                    amount: a.amount,
                    channel: a.channel,
                    turns: crate::spell::turns(&a) + 1,
                };
                // Одноимённая зона обновляет срок, а не встаёт второй:
                // то же правило наложения, что у всадника (§5.1).
                if let Some(old) = self.zones.iter_mut().find(|z| z.name == zone.name) {
                    *old = zone.clone();
                } else {
                    self.zones.push(zone.clone());
                }
                events.push(Event::Zoned {
                    by: Some(c.id),
                    side,
                    cells,
                    amount: zone.amount,
                    turns: zone.turns,
                });
            }
            crate::spell::Casting::Summon => {
                // Тело призванного заморожено вместе с умением — как всё
                // остальное в партии. Призванное не призывает: иначе карта
                // призывает карту, которая призывает карту.
                let body = a.body.clone().ok_or(Illegal::NoSuchAbility)?;
                let id = self.raise(&body, spot, side);
                events.push(Event::Played {
                    side,
                    unit: id,
                    cell: spot,
                    cost: 0,
                });
                self.arrive(id, events);
            }
            _ => {
                // Всё остальное — удержание: порча, заживление, оцепенение,
                // немота, разоружение, смута, покров, стража, оберег канала,
                // шипы. Род один раз назван в грамматике, и здесь его
                // только кладут.
                let kind = what
                    .hold(&a)
                    .expect("глагол без толкования сюда не доходит");
                for id in &swept {
                    let hold = crate::unit::Hold::new(&key, kind, a.amount, span);
                    events.extend(crate::spell::lay_hold(
                        Some(c.id),
                        &mut self.units[*id as usize],
                        hold,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Кого зацепит чара, наведённая ОТСЮДА в ЭТУ клетку.
    ///
    /// Два правила, и оба записаны здесь, потому что оба обязаны быть одни на
    /// весь движок.
    ///
    /// Первое: пригоршня расширяет ЧИСЛО целей, а не круг тех, кого чара
    /// берёт. Сглаз, брошенный в чужого, не перекидывается на своих, сколько бы
    /// их ни стояло рядом, — иначе «круг» у проклятия и «круг» у щита были бы
    /// двумя разными вещами, и обе пришлось бы держать в голове.
    ///
    /// Второе: покров здесь НЕ спрашивается. «Нельзя выбрать целью, но по
    /// площади достаёт» (§4) — ровно это и значит, что скрытого нет в списке
    /// законного, но в круге он есть.
    ///
    /// Порядок — всегда обход доски, кроме цепи, которая идёт своими прыжками:
    /// два тела, одинаково подходящие, обязаны выбираться одинаково всегда,
    /// иначе переигрывание партии расходится.
    fn touched(
        &self,
        reach: crate::spell::Reach,
        from: Cell,
        at: Cell,
    ) -> Vec<UnitId> {
        let standing: Vec<(Cell, UnitId)> = self
            .board
            .occupied()
            .filter(|(_, id)| !self.units[*id as usize].health.is_dead())
            .collect();
        // Кого берёт чара — говорит то тело, в которое ткнули: чара по чужому
        // цепляет чужих, чара по своему — своих.
        let aimed_side = self.board.at(at).map(|id| self.units[id as usize].side());
        let same = |id: UnitId| match aimed_side {
            Some(s) => self.units[id as usize].side() == s,
            // Ткнули в пустую клетку: такая чара тел не собирает вовсе.
            None => false,
        };

        match reach {
            crate::spell::Reach::One => self.board.at(at).into_iter().collect(),
            crate::spell::Reach::Adjacent => standing
                .iter()
                .filter(|(cell, id)| cell.distance(at) <= 1 && same(*id))
                .map(|(_, id)| *id)
                .collect(),
            crate::spell::Reach::Radius(r) => standing
                .iter()
                .filter(|(cell, id)| cell.distance(at) <= r && same(*id))
                .map(|(_, id)| *id)
                .collect(),
            crate::spell::Reach::Side => standing
                .iter()
                .filter(|(_, id)| same(*id))
                .map(|(_, id)| *id)
                .collect(),
            crate::spell::Reach::Line => {
                // Насквозь: цель и всё, что стоит ЗА ней, если смотреть от
                // наводящего. Луч начинается у цели, а не у наводящего, и
                // потому определён всегда — даже когда цель стоит не по прямой
                // от него и «направление на неё» пришлось бы округлять.
                let dx = (at.x as i16 - from.x as i16).signum();
                let dy = (at.y as i16 - from.y as i16).signum();
                let mut out = Vec::new();
                let mut cell = Some(at);
                while let Some(c) = cell {
                    if let Some(id) = self.board.at(c)
                        && same(id)
                        && !self.units[id as usize].health.is_dead()
                    {
                        out.push(id);
                    }
                    let nx = c.x as i16 + dx;
                    let ny = c.y as i16 + dy;
                    cell = if (dx == 0 && dy == 0) || nx < 0 || ny < 0 {
                        None
                    } else {
                        Cell::new(nx as u8, ny as u8)
                    };
                }
                out
            }
            crate::spell::Reach::Chain(links) => {
                // Цепь прыгает от цели к ближайшему ещё не задетому. Равные
                // разрешаются обходом доски — тем же, что и всё остальное.
                let mut out = Vec::new();
                let Some(first) = self.board.at(at) else {
                    return out;
                };
                out.push(first);
                let mut here = at;
                for _ in 1..links.max(1) {
                    let next = standing
                        .iter()
                        .filter(|(_, id)| same(*id) && !out.contains(id))
                        .min_by_key(|(cell, _)| cell.distance(here));
                    let Some((cell, id)) = next else { break };
                    out.push(*id);
                    here = *cell;
                }
                out
            }
        }
    }

    /// Кто на самом деле примет удар, нацеленный в это тело.
    ///
    /// Стража принимает на себя то, что летит в СОСЕДА, пока у неё есть чем
    /// принимать (`amount` — сколько ударов ещё). Считается здесь, а не в
    /// конвейере урона: конвейер отвечает на «сколько», а это вопрос «кому», и
    /// ответ на него обязан быть виден в событии — иначе число снимается у
    /// того, в кого не целились, без всякой видимой причины.
    fn shielded_by_guard(&mut self, aimed: UnitId) -> UnitId {
        let Some(cell) = self.board.cell_of(aimed) else {
            return aimed;
        };
        let side = self.units[aimed as usize].side();
        let guard = self
            .board
            .occupied()
            .filter(|(c, id)| *id != aimed && c.distance(cell) <= 1)
            .map(|(_, id)| id)
            .find(|id| {
                let u = &self.units[*id as usize];
                u.side() == side
                    && !u.health.is_dead()
                    && u.hold_amount(crate::unit::HoldKind::Guarding) > 0
            });
        let Some(g) = guard else { return aimed };
        if let Some(hold) = self.units[g as usize]
            .holds
            .iter_mut()
            .find(|h| h.kind.same_as(crate::unit::HoldKind::Guarding))
        {
            hold.amount -= 1;
        }
        g
    }

    /// Шипы: тому, кто тронул, прилетает.
    ///
    /// Отличается от сдачи двумя вещами, и обе намеренны. Число своё, с умения,
    /// а не сила тела — поэтому в конвейер бьющим уходит `None`, иначе
    /// благословение силы на носителе множило бы шипы. И дальности у шипов нет:
    /// сдача — ответный удар и потому меряется досягаемостью, а возмездие
    /// достаёт всякого, кто тронул, хоть с другого конца поля.
    fn prick(&mut self, victim: UnitId, striker: UnitId, events: &mut Vec<Event>) {
        let back = self.units[victim as usize].hold_amount(crate::unit::HoldKind::Thorned);
        if back <= 0
            || victim == striker
            || self.units[striker as usize].health.is_dead()
            || self.units[victim as usize].health.is_dead()
        {
            return;
        }
        let hurt = self.units[striker as usize].clone();
        let mut res = crate::damage::resolve(
            None,
            &hurt,
            crate::damage::DamagePacket::new(
                back,
                crate::damage::Channel::Physical,
                crate::damage::Source::Thorns,
            ),
        );
        // Число у шипов своё, а автор у них есть: без имени на доске из шести
        // рядов это здоровье, убывшее само по себе.
        res.by = Some(victim);
        events.extend(apply(&mut self.units[striker as usize], &res));
    }

    /// Куда отъедет тело, которое толкнули от наводящего (или притянули к нему).
    ///
    /// Останавливается перед первым занятым — и перед краем поля. Толкнуть в
    /// стену значит не сдвинуть: продавливать чужие тела было бы вторым
    /// правилом ходьбы, а она уже написана и обходит стоящих, а не проходит
    /// сквозь них.
    fn slide(&self, from: Cell, spot: Cell, amount: i32, away: bool) -> (Cell, Option<Cell>) {
        let sign = if away { 1 } else { -1 };
        let dx = (spot.x as i16 - from.x as i16).signum() * sign;
        let dy = (spot.y as i16 - from.y as i16).signum() * sign;
        if dx == 0 && dy == 0 {
            return (spot, None);
        }
        let mut at = spot;
        for _ in 0..amount.max(0) {
            let nx = at.x as i16 + dx;
            let ny = at.y as i16 + dy;
            if nx < 0 || ny < 0 {
                break;
            }
            let Some(next) = Cell::new(nx as u8, ny as u8) else {
                break;
            };
            // Овраг на пути толчка — падение. Не «остановился у края»: край
            // оврага и есть то, ради чего его ставят рядом с толкающим.
            if self.ground(next) == Some(Ground::Ravine) && self.board.is_free(next) {
                return (at, Some(next));
            }
            if !self.open(next) {
                break;
            }
            // Притягивают ДО вплотную, а не мимо: по-королевски шагая наискось,
            // тело обошло бы наводящего по кругу, не приближаясь, — и «притянул»
            // значило бы «покрутил вокруг себя».
            if !away && next.distance(from) >= at.distance(from) {
                break;
            }
            at = next;
            // В болоте и в яме и толкнутое останавливается: они держат
            // всякого, кто в них оказался, а не только того, кто вошёл сам.
            if matches!(self.ground(at), Some(Ground::Mire | Ground::Pit)) {
                break;
            }
        }
        (at, None)
    }

    /// Бьёт ли это тело дальше своей дальности.
    fn long_shot(&self, attacker: &Unit, target: &Unit) -> bool {
        if attacker.reach <= 1 || self.rules.long_shot_power == 0 {
            return false;
        }
        match (self.board.cell_of(attacker.id), self.board.cell_of(target.id)) {
            (Some(from), Some(to)) => from.distance(to) > self.reach_of(attacker),
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

    /// A side is out when nothing of it stands. The hand is not counted.
    ///
    /// Held cards used to keep a bare board alive: a card that mana would one
    /// day reach meant the match went on. It was the softer rule and it read as
    /// a broken one — the board was empty, the match was plainly lost, and the
    /// room said nothing while both sides passed the turn back and forth.
    ///
    /// The keeper chose the hard rule: mana is a thing to PLAN, and holding a
    /// body you cannot afford while the field is taken from you is exactly the
    /// mistake the match is meant to punish. A challenge is not made
    /// unwinnable by it — it is made a challenge.
    ///
    /// The one thing this rule cannot see is a side that never had a body at
    /// all: it would lose on its first end of turn having never played. That is
    /// refused where a deck is assembled (`check_deck`), not excused here — an
    /// exception in the engine would be a rule that means one thing in round one
    /// and another in round two.
    fn is_spent(&self, side: Side) -> bool {
        // По ХОЗЯИНУ. Сторона, у которой увели последнее тело, не проиграла:
        // §4 запрещает массовое подчинение как «конец партии одной картой», и
        // одиночное не должно кончать её тихо. Смута вернётся — поражение нет.
        self.owned_standing(side).is_empty()
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
            // Клетка, которой нет на ЭТОМ поле, — занятая: встать туда нельзя
            // так же, как на стену.
            if !st.field.contains(*cell) {
                return Err(Illegal::CellTaken);
            }
            if st.field.side_of(*cell) != side {
                return Err(Illegal::NotYourHalf);
            }
            if !st.open(*cell) {
                return Err(Illegal::CellTaken);
            }
            if st.side_state(side).mana < card.cost {
                return Err(Illegal::NotEnoughMana);
            }

            st.side_state_mut(side).hand.remove(*hand_index);
            st.side_state_mut(side).mana -= card.cost;
            let id = st.raise(&card, *cell, side);
            events.push(Event::Played { side, unit: id, cell: *cell, cost: card.cost });
            st.arrive(id, &mut events);
            let since = events.len();
            st.reacts("onPlay", id, None, &mut events);
            st.answer(Some(id), since, &mut events);
        }

        Action::Move { unit, to } => {
            let u = st.unit(*unit).ok_or(Illegal::NoSuchUnit)?.clone();
            if u.side() != side {
                return Err(Illegal::NotYourUnit);
            }
            if u.health.is_dead() {
                return Err(Illegal::UnitIsDown);
            }
            if u.bound() {
                return Err(Illegal::Bound);
            }
            if u.acted {
                return Err(Illegal::AlreadyActed);
            }
            let from = st.board.cell_of(u.id).ok_or(Illegal::UnitIsDown)?;
            if !st.walkable(from, u.step).contains(to) {
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
            st.arrive(u.id, &mut events);
        }

        Action::Mend { healer, target } => {
            let h = st.unit(*healer).ok_or(Illegal::NoSuchUnit)?.clone();
            if h.side() != side {
                return Err(Illegal::NotYourUnit);
            }
            if h.health.is_dead() {
                return Err(Illegal::UnitIsDown);
            }
            if h.bound() {
                return Err(Illegal::Bound);
            }
            // Лечение даром — чара, и немота держит его вместе со всеми
            // остальными; лечение телом (`mend` на карте) она не держит: это
            // руки, а не слово.
            if h.hushed() && h.ready_heal(st.side_state(side).mana).is_some() && h.mend == 0 {
                return Err(Illegal::Hushed);
            }
            if h.acted {
                return Err(Illegal::AlreadyActed);
            }

            let mana = st.side_state(side).mana;
            let ability = h.ready_heal(mana);
            let (amount, reach, mana_cost, allow_self, ability_id, cooldown) =
                if let Some(ref a) = ability {
                    (a.amount, a.range, a.mana_cost, a.shape == "self", a.id.clone(), a.cooldown)
                } else if h.mend > 0 {
                    (h.mend, h.reach, 0, false, String::new(), 0)
                } else {
                    return Err(Illegal::DoesNotMend);
                };

            if mana_cost > mana {
                return Err(Illegal::NotEnoughMana);
            }

            if allow_self {
                if healer != target {
                    // This ability only tends its bearer; another ally is not a
                    // target it knows. Same refuse word as "do not tend yourself"
                    // on a body mend — the list never offers the wrong one.
                    return Err(Illegal::TargetIsAlly);
                }
            } else if healer == target {
                // A mender tends others; tending itself is a different verb
                // (shape `self` on a heal ability).
                return Err(Illegal::TargetIsAlly);
            }

            let t = st.unit(*target).ok_or(Illegal::NoSuchUnit)?.clone();
            if t.side() != side {
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
            if from.distance(to) > reach {
                return Err(Illegal::OutOfReach);
            }

            let mending = crate::heal::resolve_mend(&t, amount);
            st.units[h.id as usize].acted = true;
            if mana_cost > 0 {
                st.side_state_mut(side).mana -= mana_cost;
            }
            if !ability_id.is_empty() {
                st.units[h.id as usize].start_ability_cd(&ability_id, cooldown);
            }
            events.extend(crate::heal::apply_mend(
                Some(h.id),
                &mut st.units[t.id as usize],
                &mending,
            ));
        }

        Action::Attack { attacker, target } => {
            let a = st.unit(*attacker).ok_or(Illegal::NoSuchUnit)?.clone();
            if a.side() != side {
                return Err(Illegal::NotYourUnit);
            }
            if a.health.is_dead() {
                return Err(Illegal::UnitIsDown);
            }
            if a.bound() {
                return Err(Illegal::Bound);
            }
            if a.disarmed() {
                return Err(Illegal::Disarmed);
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
            if t.side() == side {
                return Err(Illegal::TargetIsAlly);
            }
            if t.health.is_dead() {
                return Err(Illegal::TargetIsDown);
            }
            // Покров: целью не выбирают.
            if t.veiled() {
                return Err(Illegal::Veiled);
            }

            let from = st.board.cell_of(a.id).ok_or(Illegal::UnitIsDown)?;
            let to = st.board.cell_of(t.id).ok_or(Illegal::TargetIsDown)?;
            if !st.can_reach(&a, from, to) {
                return Err(Illegal::OutOfReach);
            }

            // Стража принимает на себя то, что летит в соседа. Счёт удара при
            // этом считается ПО НЕЙ: броня у неё своя, и «сколько» зависит от
            // того, кто принял, а не от того, в кого целились.
            let hit = st.shielded_by_guard(t.id);
            let victim = st.units[hit as usize].clone();
            let res = st.blow(&a, &victim);
            st.units[a.id as usize].acted = true;
            if st.round == 1 && side == Side::Player {
                st.opening_attacks_used = st.opening_attacks_used.saturating_add(1);
            }
            let since = events.len();
            events.extend(apply(&mut st.units[hit as usize], &res));
            // Шипы отвечают раньше сдачи: сдача — это удар, и мёртвый его не
            // наносит, а возмездие срабатывает от самого прикосновения.
            st.prick(hit, a.id, &mut events);
            st.answer(Some(a.id), since, &mut events);
            let t = st.units[hit as usize].clone();
            let to = st.board.cell_of(hit).unwrap_or(to);

            // Сдача. Только если ударенный жив, достаёт до обидчика и ещё не
            // отвечал в этот ход. Отвечает `Recoil`, поэтому ответ на ответ
            // невозможен по построению — не по договорённости.
            if st.rules.retaliation
                && !st.units[a.id as usize].health.is_dead()
                && !st.units[t.id as usize].health.is_dead()
                && !st.units[t.id as usize].retaliated
                && !st.units[t.id as usize].disarmed()
                && !st.units[t.id as usize].bound()
                && st.units[t.id as usize].strikes
                && to.distance(from) <= st.reach_of(&st.units[t.id as usize])
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
                let since = events.len();
                events.extend(apply(&mut st.units[a.id as usize], &back));
                st.answer(Some(t.id), since, &mut events);
            }
        }

        Action::Cast {
            caster,
            ability,
            target,
        } => {
            // Проверки те же и в том же порядке, что у удара и у лечения.
            // Выписаны подряд, а не вызовом общей: порядок отказов — это то, что
            // человек ЧИТАЕТ, и спрятанный в помощник он перестаёт быть виден.
            let c = st.unit(*caster).ok_or(Illegal::NoSuchUnit)?.clone();
            if c.side() != side {
                return Err(Illegal::NotYourUnit);
            }
            if c.health.is_dead() {
                return Err(Illegal::UnitIsDown);
            }
            if c.bound() {
                return Err(Illegal::Bound);
            }
            if c.hushed() {
                return Err(Illegal::Hushed);
            }
            if c.acted {
                return Err(Illegal::AlreadyActed);
            }

            let a = c.ability_by_key(ability).ok_or(Illegal::NoSuchAbility)?;
            let what = a.casting().ok_or(Illegal::NoSuchAbility)?;
            if c.ability_cd(ability) > 0 {
                return Err(Illegal::AbilityAsleep);
            }
            let mana = st.side_state(side).mana;
            if a.mana_cost > mana {
                return Err(Illegal::NotEnoughMana);
            }
            if what.wounds() && st.holds_at_the_opening() {
                return Err(Illegal::HeldAtTheOpening);
            }

            let from = st.board.cell_of(c.id).ok_or(Illegal::UnitIsDown)?;
            let aim = what.aim(&a.shape);

            // Куда наведено — и достаёт ли туда чара. Клетка и тело
            // проверяются здесь ОДИНАКОВО, потому что дальность у них одна.
            let spot = match target {
                Mark::Unit(id) => st.board.cell_of(*id),
                Mark::Spot(cell) => Some(*cell),
            }
            .ok_or(Illegal::UnitIsDown)?;
            if from.distance(spot) > a.range {
                return Err(Illegal::OutOfReach);
            }

            // Кого взяли. У клеточных чар тела нет вовсе, и это не «цель не
            // найдена», а другой род цели.
            let mut aimed: Option<Unit> = None;
            match aim {
                crate::spell::Aim::Spot { free } => {
                    if target.spot().is_none() {
                        return Err(Illegal::NotThatAim);
                    }
                    if !st.field.contains(spot) {
                        return Err(Illegal::NoRoom);
                    }
                    if free && !st.open(spot) {
                        return Err(Illegal::NoRoom);
                    }
                }
                _ => {
                    let id = target.unit().ok_or(Illegal::NotThatAim)?;
                    let t = st.unit(id).ok_or(Illegal::NoSuchUnit)?.clone();
                    if t.health.is_dead() {
                        return Err(Illegal::TargetIsDown);
                    }
                    match aim {
                        crate::spell::Aim::Bearer if id != *caster => {
                            return Err(Illegal::NotThatAim);
                        }
                        crate::spell::Aim::Ally if id == *caster => {
                            return Err(Illegal::NotThatAim);
                        }
                        crate::spell::Aim::Ally | crate::spell::Aim::Bearer
                            if t.side() != side =>
                        {
                            return Err(Illegal::TargetIsEnemy);
                        }
                        crate::spell::Aim::Foe if t.side() == side => {
                            return Err(Illegal::TargetIsAlly);
                        }
                        crate::spell::Aim::Any if id == *caster => {
                            // Толкнуть себя некуда: у толчка направление берётся
                            // от наводящего, и от себя до себя его нет.
                            return Err(Illegal::NotThatAim);
                        }
                        _ => {}
                    }
                    // Покров: целью не выбирают. По площади — достаётся, и
                    // потому проверка стоит здесь, а не в конвейере урона.
                    if t.side() != side && t.veiled() {
                        return Err(Illegal::Veiled);
                    }
                    // Отдых после оцепенения и смуты (§5.3).
                    if what.is_control() && t.held(crate::unit::HoldKind::Rested) {
                        return Err(Illegal::Rested);
                    }
                    aimed = Some(t);
                }
            }

            // Плата одна на все двадцать глаголов, и берётся она ДО того, как
            // хоть что-то случилось: чара, которая иногда не тратит ход, — это
            // двадцать развилок вместо одной строки.
            st.units[c.id as usize].acted = true;
            if a.mana_cost > 0 {
                st.side_state_mut(side).mana -= a.mana_cost;
            }
            // «Однажды» — это откат навсегда: тот же вопрос «вернулось ли»
            // и тот же ответ, только окончательный.
            st.units[c.id as usize].start_ability_cd(
                ability,
                if a.spent_forever() { crate::unit::Unit::FOREVER } else { a.cooldown },
            );
            if what.wounds() && st.round == 1 && side == Side::Player {
                st.opening_attacks_used = st.opening_attacks_used.saturating_add(1);
            }

            let key = ability.clone();
            let t_id = aimed.as_ref().map(|t| t.id);
            // Кого зацепило. У чары, которая пригоршни не берёт, это ровно то
            // тело, в которое ткнули; у остальных — то, что насчитала §4.
            // Первое в списке всегда ОНО ЖЕ: на него приходится и стража, и
            // шипы, и `Source::Ability`, а на остальных — `Splash`.
            let mut swept: Vec<UnitId> = match (t_id, what.spreads()) {
                (Some(id), true) => {
                    let mut all =
                        st.touched(crate::spell::reach_of(&a.shape, a.radius), from, spot);
                    all.retain(|x| *x != id);
                    let mut out = vec![id];
                    out.append(&mut all);
                    out
                }
                (Some(id), false) => vec![id],
                (None, _) => Vec::new(),
            };
            swept.retain(|id| !st.units[*id as usize].health.is_dead());
            // Срок наложенного на СВОЮ сторону берёт лишний ход, и это не
            // подгонка числа, а §5.4: «2 хода» значит два собственных хода
            // цели ВСЕГДА. Ход, в котором чара легла, у своего тела уже идёт и
            // кончится раньше, чем оно успеет им воспользоваться, — на чужом
            // теле такого хода нет, и прибавки нет тоже.
            let mine = t_id
                .map(|id| {
                    if matches!(what, crate::spell::Casting::Sway) {
                        // Смута переводит тело на сторону наводящего — то есть
                        // на ту, что сейчас ходит.
                        true
                    } else {
                        st.units[id as usize].side() == side
                    }
                })
                .unwrap_or(false);
            let span = crate::spell::turns(&a) + if mine { 1 } else { 0 };

            let since = events.len();
            st.work(&c, &a, what, &key, &swept, from, spot, span, side, &mut events)?;
            st.answer(Some(c.id), since, &mut events);
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
            // ── Всё, что считается ходами, считается ЗДЕСЬ ────────────────
            //
            // Один конец хода — один порядок, и порядок этот выбран, а не
            // сложился: сперва горит поле, потом тлеют раны, потом заживает,
            // и только после этого сроки становятся короче. Иначе порча на один
            // ход, наложенная в этот же ход, сходила бы, ни разу не тронув, — то
            // самое, из-за чего срок и тикает в конце хода НОСИТЕЛЯ, а не в
            // начале: «2 хода» обязано значить два собственных хода цели (§5.4).

            // Опасные клетки жгут того, кто на них стоял свой ход. Своих тоже:
            // котёл не разбирает, кто над ним наклонился.
            let mut burning: Vec<(UnitId, i32, crate::damage::Channel, Option<UnitId>)> =
                Vec::new();
            for z in &st.zones {
                for cell in &z.cells {
                    let Some(id) = st.board.at(*cell) else { continue };
                    if st.units[id as usize].side() != side {
                        continue;
                    }
                    burning.push((id, z.amount, z.channel, z.by));
                }
            }
            for (id, amount, channel, by) in burning {
                let body = st.units[id as usize].clone();
                let mut res = crate::damage::resolve(
                    None,
                    &body,
                    crate::damage::DamagePacket::new(amount, channel, crate::damage::Source::Zone),
                );
                // Автор у клетки есть, а числа его нет: жжёт она сама, и сила
                // ведьмы на это не влияет — потому конвейеру бьющий не назван.
                res.by = by;
                events.extend(apply(&mut st.units[id as usize], &res));
            }

            // Порча и заживление: то, что тлеет на самом теле.
            for id in st.standing(side) {
                let fester = st.units[id as usize]
                    .hold_amount(crate::unit::HoldKind::Festering);
                if fester > 0 {
                    let body = st.units[id as usize].clone();
                    let res = crate::damage::resolve(
                        None,
                        &body,
                        crate::damage::DamagePacket::new(
                            fester,
                            crate::damage::Channel::Pure,
                            crate::damage::Source::Dot,
                        ),
                    );
                    events.extend(apply(&mut st.units[id as usize], &res));
                }
                let knit = st.units[id as usize].hold_amount(crate::unit::HoldKind::Knitting);
                if knit > 0 && !st.units[id as usize].health.is_dead() {
                    let mending = crate::heal::resolve_mend(&st.units[id as usize], knit);
                    events.extend(crate::heal::apply_mend(
                        None,
                        &mut st.units[id as usize],
                        &mending,
                    ));
                }
            }

            // И только теперь сроки. Стоящие берутся заново: кто-то из них
            // только что пал, и укорачивать сроки павшему нечего.
            for id in st.standing(side) {
                st.units[id as usize].tick_statuses();
                st.units[id as usize].tick_holds();
            }
            // Зона тикает ходами того, кто её поставил, — ровно как всадник
            // тикает ходами носителя.
            for z in st.zones.iter_mut() {
                if z.side == side {
                    z.turns = z.turns.saturating_sub(1);
                }
            }
            st.zones.retain(|z| z.turns > 0);
            // Кто закончил ход на чужом краю — запоминается после платы и
            // сроков: павший от них края не держит.
            if st.rules.breakthrough {
                let mine = |id: &UnitId| st.units[*id as usize].owner == side;
                let kept: Vec<UnitId> = st.poised.iter().copied().filter(|id| !mine(id)).collect();
                let mut poised = kept;
                poised.extend(st.on_goal(side));
                st.poised = poised;
            }
            events.push(Event::TurnEnded { side, round: st.round });
            st.active = side.other();
            if st.active == Side::Player {
                st.round += 1;
            }
            // Прорыв решается раньше счётчика: простоявший ход противника
            // заработал победу, и истёкший лимит её не отнимает.
            if st.rules.breakthrough {
                let next = st.active;
                if let Some(unit) = st.breaker(next) {
                    let outcome = match next {
                        Side::Player => Outcome::Player,
                        Side::Keeper => Outcome::Keeper,
                    };
                    st.outcome = Some(outcome);
                    events.push(Event::Breached { unit, side: next });
                    events.push(Event::Finished { outcome });
                    return Ok((st, events));
                }
            }
            if st.round > st.rules.max_rounds {
                st.settle_on_time(&mut events);
                return Ok((st, events));
            }
            st.open_turn();
            // Родник поит стоящих на нём — до поводов начала хода: вода в
            // начале хода, а не после того, как кто-то уже успел ответить.
            for id in st.standing(st.active) {
                let on_spring = st.board.cell_of(id).is_some_and(|c| st.ground(c) == Some(Ground::Spring));
                if on_spring {
                    let mending = crate::heal::resolve_mend(&st.units[id as usize], SPRING_MEND);
                    events.extend(crate::heal::apply_mend(None, &mut st.units[id as usize], &mending));
                }
            }
            // Начало своего хода — повод. Стоящие берутся после того, как ход
            // уже перешёл: «свой ход» у тела то, в котором оно ходит.
            for id in st.standing(st.active) {
                let since = events.len();
                st.reacts("turnStart", id, None, &mut events);
                st.answer(Some(id), since, &mut events);
            }
        }
    }

    // Три вещи в конце ВСЯКОГО действия, и все три — в одном месте, потому что
    // забыть их в одной ветке из шести ничего не стоит: убрать павших, дать
    // полю вздохнуть заново (аура держится тем, кто стоит, а стоят уже другие)
    // и посмотреть, не кончилась ли партия.
    st.bury_the_fallen();
    st.breathe();
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
        for cell in state.free_cells(side) {
            out.push(Action::Play { hand_index: i, cell });
        }
    }

    for unit in state.standing(side) {
        let u = &state.units[unit as usize];
        if u.acted || u.bound() || u.step == 0 || (!state.rules.walk_spends_turn && u.moved) {
            continue;
        }
        let Some(from) = state.board.cell_of(unit) else { continue };
        for to in state.walkable(from, u.step) {
            out.push(Action::Move { unit, to });
        }
    }

    for healer in state.standing(side) {
        let h = &state.units[healer as usize];
        if h.acted || h.bound() {
            continue;
        }
        let mana = state.side_state(side).mana;
        // Немота держит лечение даром и не держит лечение руками.
        if h.hushed() && h.ready_heal(mana).is_some() && h.mend == 0 {
            continue;
        }
        let ability = h.ready_heal(mana);
        let (amount, reach, allow_self) = if let Some(ref a) = ability {
            (a.amount, a.range, a.shape == "self")
        } else if h.mend > 0 {
            (h.mend, h.reach, false)
        } else {
            continue;
        };
        if amount <= 0 {
            continue;
        }
        let Some(from) = state.board.cell_of(healer) else { continue };

        if allow_self {
            if state.units[healer as usize].wound() > 0 {
                out.push(Action::Mend {
                    healer,
                    target: healer,
                });
            }
            continue;
        }

        for target in state.standing(side) {
            if target == healer || state.units[target as usize].wound() == 0 {
                continue;
            }
            let Some(to) = state.board.cell_of(target) else { continue };
            if from.distance(to) <= reach {
                out.push(Action::Mend { healer, target });
            }
        }
    }

    for attacker in state.standing(side) {
        let a = &state.units[attacker as usize];
        // Offered and refused would break the one contract the client relies on.
        if a.acted || !a.strikes || a.bound() || a.disarmed() || state.holds_at_the_opening() {
            continue;
        }
        let Some(from) = state.board.cell_of(attacker) else { continue };
        for target in state.standing(side.other()) {
            // Покров: целью не выбирают. По площади достаётся — и достанется,
            // когда площадь появится; ткнуть в него нельзя уже сейчас.
            if state.units[target as usize].veiled() {
                continue;
            }
            let Some(to) = state.board.cell_of(target) else { continue };
            if state.can_reach(a, from, to) {
                out.push(Action::Attack { attacker, target });
            }
        }
    }

    // Чары ПОСЛЕ ударов, и это не вкус: равные ветки перебора разрешаются
    // порядком обхода, и список, в который новое вписано посередине, переиграл
    // бы все прежние замеры молча.
    for caster in state.standing(side) {
        let c = &state.units[caster as usize];
        if c.acted || c.bound() || c.hushed() {
            continue;
        }
        let Some(from) = state.board.cell_of(caster) else { continue };
        let mana = state.side_state(side).mana;
        for (key, a, what) in c.casts_ready(mana) {
            // Чара, наносящая урон, — удар в смысле запрета первого круга.
            // Иначе в правиле дыра ровно размером с дальнобойную чару: сторона,
            // которой не дали ударить, «наводит» то же число с того же места.
            if what.wounds() && state.holds_at_the_opening() {
                continue;
            }
            let mut offer = |target: Mark| {
                out.push(Action::Cast {
                    caster,
                    ability: key.clone(),
                    target,
                });
            };
            let reaches = |cell: Cell| from.distance(cell) <= a.range;
            let body_at = |target: UnitId| -> Option<Cell> { state.board.cell_of(target) };

            match what.aim(&a.shape) {
                crate::spell::Aim::Bearer => offer(Mark::Unit(caster)),
                crate::spell::Aim::Ally => {
                    for target in state.standing(side) {
                        if target == caster {
                            continue;
                        }
                        if body_at(target).is_some_and(&reaches) {
                            offer(Mark::Unit(target));
                        }
                    }
                }
                crate::spell::Aim::Foe => {
                    for target in state.standing(side.other()) {
                        // Покров: целью не выбирают. Тело при этом на доске
                        // стоит и по площади достаётся — просто ткнуть в него
                        // нельзя.
                        if state.units[target as usize].veiled() {
                            continue;
                        }
                        // Отдых: под оцепенение и смуту снова нельзя (§5.3).
                        if what.is_control()
                            && state.units[target as usize].held(crate::unit::HoldKind::Rested)
                        {
                            continue;
                        }
                        if body_at(target).is_some_and(&reaches) {
                            offer(Mark::Unit(target));
                        }
                    }
                }
                crate::spell::Aim::Any => {
                    // Толчок и притяжение: всякое стоящее тело, кроме себя.
                    for target in state
                        .standing(side)
                        .into_iter()
                        .chain(state.standing(side.other()))
                    {
                        if target == caster {
                            continue;
                        }
                        if state.units[target as usize].side() != side
                            && state.units[target as usize].veiled()
                        {
                            continue;
                        }
                        if body_at(target).is_some_and(&reaches) {
                            offer(Mark::Unit(target));
                        }
                    }
                }
                crate::spell::Aim::Spot { free } => {
                    for cell in state.field.cells() {
                        if !reaches(cell) {
                            continue;
                        }
                        if free && !state.open(cell) {
                            continue;
                        }
                        // Своим шагом на своё же место не ходят.
                        if free && cell == from {
                            continue;
                        }
                        offer(Mark::Spot(cell));
                    }
                }
            }
        }
    }

    out.push(Action::EndTurn);
    out
}
