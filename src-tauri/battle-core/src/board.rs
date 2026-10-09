//! The field: two halves facing each other, three or four cells wide and three
//! to five deep each (`Field`). Three by three is the default and what every
//! match before the dial was played on.
//!
//! One flat coordinate rather than "side plus local row": the keeper holds
//! y 0..depth-1, the player depth..2·depth-1, and distance is then a single
//! subtraction instead of a case analysis. It also makes the numbers the keeper already wrote come
//! out right — range 1 is the neighbouring cell and range 5 is the whole field,
//! corner to corner, with nothing in between needing to be renamed.

/// Which side of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Player,
    Keeper,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Player => Side::Keeper,
            Side::Keeper => Side::Player,
        }
    }
}

/// Самое широкое поле, какое бывает, — по нему считается хранилище доски.
pub const MAX_WIDTH: u8 = 4;
/// Самая глубокая половина.
pub const MAX_HALF: u8 = 5;
/// Сколько клеток держит хранилище доски: всё поле самой большой величины.
pub const MAX_CELLS: usize = (MAX_WIDTH as usize) * (MAX_HALF as usize) * 2;

/// Величина поля: сколько клеток поперёк и сколько вглубь у КАЖДОЙ половины.
///
/// Задаёт её этюд, а не дом: поле 4 × 5 — другая задача, а не другие правила.
/// Обе половины всегда одной величины — несимметричное поле решало бы партию
/// за игроков. Хранится в расстановке и в партии; записанное без неё поле —
/// 3 × 3, то есть то, на котором игралось всё до этой ручки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    /// Сколько клеток поперёк: 3 или 4.
    pub width: u8,
    /// Сколько клеток вглубь у одной половины: от 3 до 5.
    pub depth: u8,
}

impl Default for Field {
    fn default() -> Self {
        Field { width: 3, depth: 3 }
    }
}

impl Field {
    pub const WIDTH_MIN: u8 = 3;
    pub const WIDTH_MAX: u8 = MAX_WIDTH;
    pub const DEPTH_MIN: u8 = 3;
    pub const DEPTH_MAX: u8 = MAX_HALF;

    /// Поле той величины, что умеет движок: опечатка зажимается, а не роняет.
    pub fn normalized(self) -> Field {
        Field {
            width: self.width.clamp(Self::WIDTH_MIN, Self::WIDTH_MAX),
            depth: self.depth.clamp(Self::DEPTH_MIN, Self::DEPTH_MAX),
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Field::default()
    }

    /// Рядов от края до края, обе половины вместе.
    pub fn rows(&self) -> u8 {
        self.depth * 2
    }

    pub fn contains(&self, cell: Cell) -> bool {
        cell.x < self.width && cell.y < self.rows()
    }

    /// Клетка этого поля — или ничего, если такой на нём нет.
    pub fn cell(&self, x: u8, y: u8) -> Option<Cell> {
        let cell = Cell { y, x };
        self.contains(cell).then_some(cell)
    }

    /// Чья это половина.
    pub fn side_of(&self, cell: Cell) -> Side {
        if cell.y < self.depth { Side::Keeper } else { Side::Player }
    }

    /// Дальний ряд чужой половины — куда эта сторона идёт прорывом.
    pub fn goal_row(&self, side: Side) -> u8 {
        match side {
            Side::Player => 0,
            Side::Keeper => self.rows() - 1,
        }
    }

    /// Та же клетка на другой половине: поле зеркально по шву.
    pub fn mirror(&self, cell: Cell) -> Cell {
        Cell { x: cell.x, y: self.rows() - 1 - cell.y }
    }

    /// Каждая клетка поля в порядке обхода: ряд за рядом от дальнего края
    /// хранителя, слева направо внутри ряда.
    ///
    /// Нужна тем, кто говорит о КЛЕТКАХ, а не о телах, — опасной клетке и её
    /// радиусу. Порядок тот же, что у доски, и по той же причине: две клетки,
    /// одинаково подходящие, должны выбираться одинаково всегда, иначе
    /// переигрывание партии расходится.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        let width = self.width;
        (0..self.rows()).flat_map(move |y| (0..width).map(move |x| Cell { y, x }))
    }

    /// The eight cells a king could step to, those that exist on this field.
    pub fn neighbours(&self, cell: Cell) -> Vec<Cell> {
        let mut out = Vec::with_capacity(8);
        for dy in -1i16..=1 {
            for dx in -1i16..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let x = cell.x as i16 + dx;
                let y = cell.y as i16 + dy;
                if (0..self.width as i16).contains(&x) && (0..self.rows() as i16).contains(&y) {
                    out.push(Cell { y: y as u8, x: x as u8 });
                }
            }
        }
        out
    }
}

/// A place on the field. A value, not a thing: two cells with the same numbers
/// are the same cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub y: u8,
    pub x: u8,
}

impl Cell {
    /// Клетка, которая может быть на каком-нибудь поле. Есть ли она на ЭТОМ,
    /// спрашивают у `Field::cell`.
    pub fn new(x: u8, y: u8) -> Option<Cell> {
        (x < MAX_WIDTH && y < MAX_HALF * 2).then_some(Cell { y, x })
    }

    /// Distance the way a king moves: a diagonal costs the same as a straight
    /// step. Chosen over the Manhattan metric because a player should not have
    /// to do arithmetic to see whether a card reaches.
    pub fn distance(self, other: Cell) -> u8 {
        let dx = self.x.abs_diff(other.x);
        let dy = self.y.abs_diff(other.y);
        dx.max(dy)
    }

    /// Место в хранилище доски. Шаг — самая широкая ширина, а не ширина этого
    /// поля: так порядок индексов — это порядок обхода (ряд, затем столбец) на
    /// любом поле, и равные клетки выбираются одинаково на 3 × 3 и на 4 × 5.
    fn index(self) -> usize {
        self.y as usize * MAX_WIDTH as usize + self.x as usize
    }

    fn from_index(i: usize) -> Cell {
        Cell { y: (i / MAX_WIDTH as usize) as u8, x: (i % MAX_WIDTH as usize) as u8 }
    }
}

/// One taken cell, as the board is written down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spot {
    pub cell: Cell,
    pub unit: crate::unit::UnitId,
}

/// Who stands where. Holds identities, never bodies — a unit lives in one place
/// only, and the board keeps its address. Two copies of a unit would be two
/// truths, and one of them would eventually be wrong.
///
/// Written down as a **list of taken cells**, not as the flat array it is kept
/// in. The array would make a reader work out that index eleven means row three,
/// column two — which is a rule, and the whole arrangement of this project is
/// that a reader of the board knows no rules. The cost is a conversion on the
/// way in and out; the gain is that the record explains itself.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(into = "Vec<Spot>", from = "Vec<Spot>")]
pub struct Board {
    slots: [Option<crate::unit::UnitId>; MAX_CELLS],
}

impl From<Board> for Vec<Spot> {
    fn from(board: Board) -> Self {
        board.occupied().map(|(cell, unit)| Spot { cell, unit }).collect()
    }
}

impl From<Vec<Spot>> for Board {
    fn from(spots: Vec<Spot>) -> Self {
        let mut board = Board::default();
        for spot in spots {
            board.place(spot.cell, spot.unit);
        }
        board
    }
}

impl Default for Board {
    fn default() -> Self {
        Self { slots: [None; MAX_CELLS] }
    }
}

impl Board {
    pub fn at(&self, cell: Cell) -> Option<crate::unit::UnitId> {
        self.slots[cell.index()]
    }

    pub fn is_free(&self, cell: Cell) -> bool {
        self.slots[cell.index()].is_none()
    }

    pub fn place(&mut self, cell: Cell, unit: crate::unit::UnitId) {
        self.slots[cell.index()] = Some(unit);
    }

    pub fn clear(&mut self, cell: Cell) {
        self.slots[cell.index()] = None;
    }

    pub fn cell_of(&self, unit: crate::unit::UnitId) -> Option<Cell> {
        self.occupied().find(|(_, id)| *id == unit).map(|(c, _)| c)
    }

    /// Every taken cell, in the order that settles every tie in this engine:
    /// row by row from the keeper's far rank, left to right inside a row.
    ///
    /// Written down because "the nearest enemy" is meaningless when two are
    /// equally near, and an engine that picks arbitrarily there is not
    /// deterministic — which would cost the replay tests and the balance runs
    /// at once.
    pub fn occupied(&self) -> impl Iterator<Item = (Cell, crate::unit::UnitId)> + '_ {
        (0..MAX_CELLS).filter_map(move |i| self.slots[i].map(|id| (Cell::from_index(i), id)))
    }

    /// Where a body standing on `from` can walk in `step` steps.
    ///
    /// A breadth-first walk over free cells, not a distance check. The
    /// difference shows the moment `step` is above one: a plain distance test
    /// lets a body cross a rank of standing bodies as if they were not there,
    /// and then holding a line means nothing. Bodies are walked around, never
    /// through.
    ///
    /// Returned in the field's scan order, so a caller that takes the first
    /// suitable cell behaves the same way every time.
    pub fn reachable(&self, field: &Field, from: Cell, step: u8) -> Vec<Cell> {
        let mut depth = [u8::MAX; MAX_CELLS];
        depth[from.index()] = 0;
        let mut frontier = vec![from];

        for d in 1..=step {
            let mut next = Vec::new();
            for cell in frontier.drain(..) {
                for neighbour in field.neighbours(cell) {
                    let i = neighbour.index();
                    if depth[i] == u8::MAX && self.is_free(neighbour) {
                        depth[i] = d;
                        next.push(neighbour);
                    }
                }
            }
            frontier = next;
            if frontier.is_empty() {
                break;
            }
        }

        field
            .cells()
            .filter(|c| depth[c.index()] != u8::MAX && *c != from)
            .collect()
    }

    /// Free cells on one side, in the same scan order.
    pub fn free_cells<'a>(&'a self, field: &'a Field, side: Side) -> impl Iterator<Item = Cell> + 'a {
        field.cells().filter(move |c| field.side_of(*c) == side && self.is_free(*c))
    }
}
