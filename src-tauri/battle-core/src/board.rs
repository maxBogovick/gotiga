//! The field: two grids of three by three, facing each other.
//!
//! One flat coordinate rather than "side plus local row": the keeper holds
//! y 0..2, the player y 3..5, and distance is then a single subtraction instead
//! of a case analysis. It also makes the numbers the keeper already wrote come
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

pub const WIDTH: u8 = 3;
pub const DEPTH: u8 = 6;
pub const CELLS: usize = (WIDTH * DEPTH) as usize;

/// A place on the field. A value, not a thing: two cells with the same numbers
/// are the same cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub y: u8,
    pub x: u8,
}

impl Cell {
    pub fn new(x: u8, y: u8) -> Option<Cell> {
        (x < WIDTH && y < DEPTH).then_some(Cell { y, x })
    }

    pub fn side(self) -> Side {
        if self.y < 3 { Side::Keeper } else { Side::Player }
    }

    /// Distance the way a king moves: a diagonal costs the same as a straight
    /// step. Chosen over the Manhattan metric because a player should not have
    /// to do arithmetic to see whether a card reaches.
    pub fn distance(self, other: Cell) -> u8 {
        let dx = self.x.abs_diff(other.x);
        let dy = self.y.abs_diff(other.y);
        dx.max(dy)
    }

    /// The eight cells a king could step to, those that exist.
    pub fn neighbours(self) -> Vec<Cell> {
        let mut out = Vec::with_capacity(8);
        for dy in -1i16..=1 {
            for dx in -1i16..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let x = self.x as i16 + dx;
                let y = self.y as i16 + dy;
                if (0..WIDTH as i16).contains(&x) && (0..DEPTH as i16).contains(&y) {
                    out.push(Cell { y: y as u8, x: x as u8 });
                }
            }
        }
        out
    }

    fn index(self) -> usize {
        (self.y * WIDTH + self.x) as usize
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
    slots: [Option<crate::unit::UnitId>; CELLS],
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
        Self { slots: [None; CELLS] }
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
        (0..CELLS).filter_map(move |i| {
            self.slots[i].map(|id| {
                (Cell { y: (i as u8) / WIDTH, x: (i as u8) % WIDTH }, id)
            })
        })
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
    pub fn reachable(&self, from: Cell, step: u8) -> Vec<Cell> {
        let mut depth = [u8::MAX; CELLS];
        depth[from.index()] = 0;
        let mut frontier = vec![from];

        for d in 1..=step {
            let mut next = Vec::new();
            for cell in frontier.drain(..) {
                for neighbour in cell.neighbours() {
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

        (0..CELLS)
            .filter(|i| depth[*i] != u8::MAX && *i != from.index())
            .map(|i| Cell { y: (i as u8) / WIDTH, x: (i as u8) % WIDTH })
            .collect()
    }

    /// Free cells on one side, in the same scan order.
    pub fn free_cells(&self, side: Side) -> impl Iterator<Item = Cell> + '_ {
        (0..CELLS).filter_map(move |i| {
            let cell = Cell { y: (i as u8) / WIDTH, x: (i as u8) % WIDTH };
            (cell.side() == side && self.slots[i].is_none()).then_some(cell)
        })
    }
}
