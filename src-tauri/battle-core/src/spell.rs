//! Чары: то же, что удар, но не удар.
//!
//! Здесь ГРАММАТИКА — таблица §4 `TASKS-BATTLE-ENGINE.md`, переписанная так,
//! чтобы её читал компилятор. Что делает глагол, кого он берёт в цель и чем
//! считается — сказано по одному разу на глагол и больше нигде.
//!
//! Считают чары уже написанным: урон — тем же конвейером (`Source::Ability` и
//! канал с умения — всё, чем он отличается от удара), проклятие и
//! благословение кладут `Status`, который конвейер читает пятью своими шагами,
//! щит ложится в `shield`, а всё, что правит не числа, а возможности, —
//! удержанием (`Hold`). Ни одного нового понятия под двадцатью глаголами нет, и
//! это главное, ради чего список закрыт.
//!
//! Почему грамматика здесь, а не ветками внутри `reduce`: на вопрос «кого можно
//! взять в цель» отвечают ДВА места — `legal_actions`, предлагая, и `reduce`,
//! принимая. Разойдись они на полслова, и клиент получит законное действие,
//! которое сервер отвергнет, — единственный договор, на котором держится сцена,
//! ничего не знающая о правилах.

use crate::card::AbilitySnapshot;
use crate::event::Event;
use crate::unit::{Hold, HoldKind, Stat, Unit, UnitId};

/// Чем умение становится, когда его просят.
///
/// Закрытый список, и закрыт он по той же причине, по которой закрыт список
/// глаголов: новая карта — это новое СОЧЕТАНИЕ, а не новый глагол. Лечение
/// здесь отсутствует нарочно: оно играется `Action::Mend` и игралось им до
/// чар, а второй путь к тому же событию — это две пригоршни правил, которые
/// однажды разойдутся.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Casting {
    /// `damage` — урон на расстоянии.
    Harm,
    /// `dot` — порча: урон в конце каждого своего хода.
    Fester,
    /// `hot` — заживление: здоровье в конце каждого своего хода.
    Knit,
    /// `shield` — щит, который тает, принимая.
    Shield,
    /// `bless` — всадник в помощь.
    Bless,
    /// `curse` — всадник во вред.
    Curse,
    /// `control` — оцепенение: тело не делает ничего.
    Bind,
    /// `silence` — немота: не наводит чар, но бьёт.
    Hush,
    /// `disarm` — разоружение: не бьёт, но наводит.
    Disarm,
    /// `charm` — смута: чужое тело стоит за наводящего.
    Sway,
    /// `veil` — покров: тело нельзя выбрать целью.
    Veil,
    /// `guard` — стража: удары по соседям приходят ей.
    Guard,
    /// `immune` — оберег канала: канал не чувствуется вовсе.
    Numb,
    /// `thorns` — шипы: ударившему прилетает.
    Thorns,
    /// `move` — толчок, притяжение, свой шаг.
    Shove,
    /// `cleanse` — снять проклятия со своего.
    Cleanse,
    /// `dispel` — снять благословения с чужого.
    Dispel,
    /// `mana` — добрать ману.
    Coin,
    /// `sacrifice` — отдать своё тело.
    Offer,
    /// `zone` — клетка поля опасна.
    Zone,
    /// `summon` — призвать тело.
    Summon,
}

/// На кого наводится чара.
///
/// Выведено из глагола, а не названо на карте: проклятие, которое можно навести
/// на своего, — это не настройка, а опечатка, и «одно умение, которое лечит или
/// калечит по выбору» было бы двумя умениями.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Aim {
    /// Чужое тело.
    Foe,
    /// Своё, кроме себя самого, — ровно как у лечения формы `one`.
    Ally,
    /// Только сам носитель. Форма `self`.
    Bearer,
    /// Любое стоящее тело: своё, чужое и своё собственное. Одна чара на всех —
    /// у толчка, потому что «толкнуть», «притянуть» и «шагнуть» это один жест,
    /// разный только тем, кто на другом его конце.
    Any,
    /// Клетка поля, а не тело. `free` — только пустая: призванному телу нужно
    /// место, а опасной клетке всё равно, стоит на ней кто-нибудь или нет.
    Spot { free: bool },
}

/// Пригоршня: сколько тел берёт чара, когда цель уже выбрана.
///
/// Читается ОТДЕЛЬНО от прицела, и это существенно. Прицел (`Aim`) говорит,
/// КОГО чара берёт — своего, чужого, себя; пригоршня говорит, СКОЛЬКО их. Одно
/// понятие на двоих означало бы, что «круг» у проклятия и «круг» у щита — две
/// разные вещи, и обе пришлось бы держать в голове.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Одно тело — то, в которое ткнули.
    One,
    /// Цель и соседние с ней.
    Adjacent,
    /// Цепь: от цели к ближайшему следующему, столько-то звеньев.
    Chain(u8),
    /// Насквозь: цель и всё, что стоит за ней, если смотреть от наводящего.
    Line,
    /// Квадрат вокруг цели: `radius` шагов короля.
    Radius(u8),
    /// Вся сторона.
    Side,
}

/// Пригоршня по слову с карты. Незнакомое — одно тело: пригоршня, которой
/// движок не знает, не должна делать чару шире, чем она есть.
pub fn reach_of(shape: &str, radius: u8) -> Reach {
    match shape {
        "adjacent" => Reach::Adjacent,
        "chain" => Reach::Chain(radius.max(1)),
        "line" => Reach::Line,
        "radius" => Reach::Radius(radius.max(1)),
        "side" => Reach::Side,
        _ => Reach::One,
    }
}

impl Casting {
    /// Запрещено ли это сочетание глагола и пригоршни.
    ///
    /// §4: «✗ — ЗАПРЕЩЁННЫЕ сочетания, не „дорогие“. Массовый контроль и
    /// массовое подчинение — это конец партии одной картой, сколько бы он ни
    /// стоил». Здесь это записано ровно так: не ценой, которую можно заплатить,
    /// а отказом, который нельзя обойти.
    ///
    /// Смута — только по одному. Оцепенение и покров — до круга в один шаг;
    /// сторона и круг пошире запрещены: сторона, которой не дали шевельнуться,
    /// не играет, а смотрит.
    pub fn forbids(self, shape: &str, radius: u8) -> bool {
        match self {
            Casting::Sway => !matches!(shape, "one" | "self"),
            Casting::Bind | Casting::Veil => {
                shape == "side" || (shape == "radius" && radius >= 2)
            }
            _ => false,
        }
    }

    /// Берёт ли эта чара пригоршню вообще.
    ///
    /// Три не берут, и у каждой своя причина. Опасная клетка и призыв считают
    /// КЛЕТКИ, и ширину им задаёт свой `radius`. Толчок двигает тело — толкать
    /// ряд разом значит решать, в каком порядке они друг о друга спотыкаются, а
    /// это не правило, а разбирательство. Мана и жертва — про сторону и про
    /// одно тело, и «мана всей стороне» не число, а другая игра.
    pub fn spreads(self) -> bool {
        !matches!(
            self,
            Casting::Zone
                | Casting::Summon
                | Casting::Shove
                | Casting::Coin
                | Casting::Offer
        )
    }

    /// Глагол карты — в то, что движок играет. Незнакомое и неигранное — ничего,
    /// и `legal_actions` такую чару не предложит: карта с глаголом, которого
    /// движок не знает, не ловушка.
    pub fn of_verb(verb: &str) -> Option<Casting> {
        Some(match verb {
            "damage" => Casting::Harm,
            "dot" => Casting::Fester,
            "hot" => Casting::Knit,
            "shield" => Casting::Shield,
            "bless" => Casting::Bless,
            "curse" => Casting::Curse,
            "control" => Casting::Bind,
            "silence" => Casting::Hush,
            "disarm" => Casting::Disarm,
            "charm" => Casting::Sway,
            "veil" => Casting::Veil,
            "guard" => Casting::Guard,
            "immune" => Casting::Numb,
            "thorns" => Casting::Thorns,
            "move" => Casting::Shove,
            "cleanse" => Casting::Cleanse,
            "dispel" => Casting::Dispel,
            "mana" => Casting::Coin,
            "sacrifice" => Casting::Offer,
            "zone" => Casting::Zone,
            "summon" => Casting::Summon,
            // Лечение играется `Action::Mend` и игралось им до чар.
            _ => return None,
        })
    }

    /// Кого эта чара берёт в цель.
    ///
    /// Форма сужает там, где сужать есть что: `self` у чары, которая умеет и
    /// союзника, — это «только себе». У остальных форма ничего не решает: щит
    /// на врага и смуту на своего не заказывают.
    pub fn aim(self, shape: &str) -> Aim {
        // «Только о себе» — это форма `self` и никакая другая: пригоршня
        // расширяет число целей, а не переносит чару на другое плечо.
        let own = shape == "self";
        match self {
            // По чужому.
            Casting::Harm
            | Casting::Fester
            | Casting::Curse
            | Casting::Bind
            | Casting::Hush
            | Casting::Disarm
            | Casting::Sway
            | Casting::Dispel => Aim::Foe,
            // По своему — или по себе, если так написано.
            Casting::Knit | Casting::Shield | Casting::Bless | Casting::Veil | Casting::Cleanse => {
                if own { Aim::Bearer } else { Aim::Ally }
            }
            // Отдают СВОЁ тело: чаще себя, но и соседа.
            Casting::Offer => {
                if own { Aim::Bearer } else { Aim::Ally }
            }
            // Только о себе: шипы (§4 — «только self»), стража, оберег канала,
            // и мана, у которой тела нет вовсе и целью стоит сам носитель.
            Casting::Thorns | Casting::Guard | Casting::Numb | Casting::Coin => Aim::Bearer,
            // Толчок и притяжение — один жест, разный только тем, кто на
            // другом его конце. Свой шаг — тот же глагол формы `self`, и цель
            // у него не тело, а КЛЕТКА: у «шагнуть» без клетки нет ответа на
            // вопрос «куда».
            Casting::Shove => {
                if own { Aim::Spot { free: true } } else { Aim::Any }
            }
            // Клетка, а не тело.
            Casting::Zone => Aim::Spot { free: false },
            Casting::Summon => Aim::Spot { free: true },
        }
    }

    /// Наносит ли эта чара урон ПРЯМО СЕЙЧАС.
    ///
    /// Спрашивается там, где правило говорит про УДАРЫ (запрет первого круга),
    /// — чтобы в нём не оказалось дыры размером с дальнобойную чару. Порча и
    /// опасная клетка сюда не входят: запрет держит удар, а не обещание удара,
    /// и в круге, в котором их навели, они ещё никого не тронули.
    pub fn wounds(self) -> bool {
        matches!(self, Casting::Harm)
    }

    /// Кладёт ли она то, от чего защищает отдых (§5.3).
    pub fn is_control(self) -> bool {
        matches!(self, Casting::Bind | Casting::Sway)
    }

    /// Каким удержанием она ложится, если ложится им.
    ///
    /// Канал берётся с умения: оберег наводят не «от всего», а от чарного или
    /// телесного, и это ровно то поле, которое у умения уже есть.
    pub fn hold(self, a: &AbilitySnapshot) -> Option<HoldKind> {
        Some(match self {
            Casting::Fester => HoldKind::Festering,
            Casting::Knit => HoldKind::Knitting,
            Casting::Bind => HoldKind::Bound,
            Casting::Hush => HoldKind::Hushed,
            Casting::Disarm => HoldKind::Disarmed,
            Casting::Sway => HoldKind::Swayed,
            Casting::Veil => HoldKind::Veiled,
            Casting::Guard => HoldKind::Guarding,
            Casting::Numb => HoldKind::Numb(a.channel),
            Casting::Thorns => HoldKind::Thorned,
            _ => return None,
        })
    }
}

/// Во что превращается написанное на умении, когда оно ложится всадником.
///
/// Знак считается ЗДЕСЬ и только здесь. На карте `amount` всегда положительное
/// — хранитель пишет «на два», а не «минус два», — а куда это два пойдёт,
/// решают глагол и показатель: у силы, брони и оберега вред это минус, у
/// уязвимости вред это ПЛЮС, потому что уязвимость прибавляется к входящему.
/// Два места, считающие этот знак порознь, однажды проклянут бронёй вверх.
///
/// `name` — ключ умения, а не его название. Имя всадника есть правило сложения
/// (одноимённый освежает срок, а не удваивает число), и ключ для этого ровно
/// то, что нужно: своё у каждого умения и одинаковое у двух наведений одного.
/// Слово читателю подставляет комната, по тому же ключу, — как она подставляет
/// название карты по её слагу.
pub fn rider(a: &AbilitySnapshot, what: Casting, name: &str) -> Option<crate::unit::Status> {
    let stat = a.stat.unwrap_or(Stat::Power);
    let harm = match what {
        Casting::Curse => true,
        Casting::Bless => false,
        // Остальные всадника не кладут.
        _ => return None,
    };
    // Уязвимость считается наоборот: она прибавляется к входящему.
    let against = (stat == Stat::Vulnerable) != harm;
    let amount = if against { -a.amount.abs() } else { a.amount.abs() };
    Some(crate::unit::Status::new(name, stat, amount, turns(a)))
}

/// Сколько ходов держится наложенное.
///
/// Ноль — это ОДИН ход, а не всадник, сошедший раньше, чем его заметили: число
/// на карте пишет хранитель, и срок у наложенного есть всегда.
pub fn turns(a: &AbilitySnapshot) -> u8 {
    a.duration.max(1)
}

/// Положить всадника и сказать, что случилось.
///
/// Отдельно от счёта, как `apply_mend` отдельно от `resolve_mend`: всё, что
/// считает, обязано быть можно посчитать тысячу раз, и только это меняет тело.
pub fn lay_rider(
    by: Option<UnitId>,
    target: &mut Unit,
    status: crate::unit::Status,
    what: Casting,
) -> Vec<Event> {
    target.apply_status(status.clone());
    vec![Event::Rider {
        target: target.id,
        by,
        status,
        ill: matches!(what, Casting::Curse),
    }]
}

/// Наложить удержание и сказать, что случилось.
pub fn lay_hold(by: Option<UnitId>, target: &mut Unit, hold: Hold) -> Vec<Event> {
    let said = Event::Held {
        target: target.id,
        by,
        name: hold.name.clone(),
        kind: hold.kind,
        amount: hold.amount,
        turns: hold.turns,
    };
    // Оберег канала живёт сроком в удержании, а читается полем: конвейер
    // спрашивает `immune`, и спрашивать его списком значило бы читать одно и то
    // же двумя способами. Пишут поле два места — это и снятие по сроку.
    if let HoldKind::Numb(channel) = hold.kind {
        target.immune = Some(channel);
    }
    target.lay_hold(hold);
    vec![said]
}

/// Надеть щит. Складывается с уже стоящим: щит — запас, а не свойство.
pub fn raise_shield(by: Option<UnitId>, target: &mut Unit, amount: i32) -> Vec<Event> {
    let added = amount.max(0);
    if added == 0 {
        return Vec::new();
    }
    target.shield += added;
    vec![Event::Shielded {
        target: target.id,
        by,
        amount: added,
    }]
}

/// Снять с тела всадников в одну сторону: проклятия или благословения.
///
/// Сторону называет `ill`, а не знак числа: у уязвимости вред — это плюс, и
/// очищение, читающее знак, сняло бы с союзника ровно то, ради чего его звали.
/// Считается тот же признак и тем же способом, что в `rider`.
pub fn lift_riders(by: Option<UnitId>, target: &mut Unit, ill: bool, upto: usize) -> Vec<Event> {
    let mut lifted = Vec::new();
    target.statuses.retain(|s| {
        if lifted.len() >= upto {
            return true;
        }
        let against = (s.stat == Stat::Vulnerable) != (s.amount < 0);
        if against == ill {
            lifted.push(s.name.clone());
            false
        } else {
            true
        }
    });
    if lifted.is_empty() {
        return Vec::new();
    }
    vec![Event::Lifted {
        target: target.id,
        by,
        ill,
        count: lifted.len() as u8,
    }]
}
