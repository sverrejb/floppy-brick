use rand::Rng;

pub type IVector = (i32, i32);

/// The layout of one tetromino
pub struct TetrominoLayout {
    /// All tetrominos consist of 4 blocks, so we use a fixed-size array.
    /// This is expressed in the tetromino coordinate system
    pub coords: [IVector; 4],
    /// OTOH, The number of _joints_ is variable..
    pub joints: Vec<(usize, usize)>,
}

#[derive(Clone, Copy, Debug)]
pub enum TetrominoKind {
    I,
    O,
    T,
    J,
    L,
    S,
    Z,
}

impl TetrominoKind {
    pub fn random() -> Self {
        match rand::thread_rng().gen_range(0..=6) {
            0 => Self::I,
            1 => Self::O,
            2 => Self::T,
            3 => Self::J,
            4 => Self::L,
            5 => Self::S,
            _ => Self::Z,
        }
    }

    pub fn layout(&self) -> TetrominoLayout {
        match self {
            Self::I => TetrominoLayout {
                coords: [(1, 1), (1, 0), (1, -1), (1, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::O => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (0, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3), (1, 0)],
            },
            Self::T => TetrominoLayout {
                coords: [(0, 0), (1, 0), (2, 0), (1, -1)],
                joints: vec![(0, 1), (1, 2), (1, 3)],
            },
            Self::J => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (0, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::L => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (2, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::S => TetrominoLayout {
                coords: [(0, -1), (1, -1), (1, 0), (2, 0)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::Z => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (2, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
        }
    }
}
