use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{collections::HashSet, iter};

use crate::util::{ChainData, Chains, Move, PreviousData, Tile, Turn};

pub mod util;

pub struct BoardData {
    pub komi: f32,
    pub turn: Turn,
    pub rep: String,
    pub size: u8,
}

impl BoardData {
    pub fn from(s: String) -> Result<Self, String> {
        let parts: Vec<_> = s.trim().split(";").map(|s| s.to_string()).collect();
        if parts.len() != 4 {
            return Err("Missing information".to_string());
        }

        let turn_char = parts[0]
            .chars()
            .nth(0)
            .ok_or("Invalid turn char".to_string())?;
        let turn = Turn::try_from(turn_char)?;
        let size = parts[1].parse::<u8>().map_err(|e| e.to_string())?;
        let rep = parts[2].clone();
        let komi = parts[3].parse::<f32>().map_err(|e| e.to_string())?;
        Ok(BoardData {
            komi,
            size,
            turn,
            rep,
        })
    }
}

pub struct Board {
    pub white: Vec<u32>,
    pub black: Vec<u32>,
    pub dead: Vec<u32>,

    pub size: u8,
    pub turn: Turn,

    pub komi: f32,
    pub prev: Vec<PreviousData>,

    neighbors: Vec<[usize; 4]>,
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Board({:?},{},{},{},{})",
            self.turn,
            self.size,
            self.white.as_ptr() as u32,
            self.black.as_ptr() as u32,
            self.dead.as_ptr() as u32,
        )
    }
}

impl Clone for Board {
    fn clone(&self) -> Self {
        Self {
            white: self.white.clone(),
            black: self.black.clone(),
            dead: self.dead.clone(),
            prev: self.prev.clone(),

            komi: self.komi,
            size: self.size,
            turn: self.turn,

            neighbors: self.neighbors.clone(),
        }
    }
}

impl Hash for Board {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.white.hash(state);
        self.black.hash(state);
        self.dead.hash(state);
    }
}

impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.white.iter().eq(other.white.iter())
            && self.black.iter().eq(other.black.iter())
            && self.dead.iter().eq(other.dead.iter())
    }

    fn ne(&self, other: &Self) -> bool {
        !(self == other)
    }
}

impl Into<String> for Board {
    fn into(self) -> String {
        self.get_board_state()
    }
}

impl Chains for Board {
    fn get_chain(&self, pos: usize) -> ChainData {
        let tile = self.tile(pos);

        let mut tiles: HashSet<usize> = HashSet::new();
        let mut adjacent: HashSet<usize> = HashSet::new();
        let mut liberties: HashSet<usize> = HashSet::new();
        let mut queue = VecDeque::from([pos]);

        while let Some(cur) = queue.pop_front() {
            if tiles.contains(&cur) {
                continue;
            }
            tiles.insert(cur);

            for p in self.neighbors[cur] {
                if p == usize::MAX {
                    continue;
                }
                let t = self.tile(p);
                if t == tile {
                    queue.push_back(p);
                    continue;
                }

                if t == Tile::Free && tile != Tile::Dead {
                    liberties.insert(p);
                }

                adjacent.insert(p);
            }
        }

        ChainData {
            tile,
            tiles: tiles.iter().map(|p| *p).collect::<Vec<_>>(),
            adjacent: adjacent.iter().map(|p| *p).collect::<Vec<_>>(),
            liberties: liberties.iter().map(|p| *p).collect::<Vec<_>>(),
        }
    }
}

impl Board {
    pub fn new(size: u8, turn: Turn, komi: f32) -> Self {
        let sets_of_32 = (size as usize).pow(2).div_ceil(32).max(1);

        let white = vec![0; sets_of_32];
        let black = vec![0; sets_of_32];
        let dead = vec![0; sets_of_32];

        let neighbors: Vec<[usize; 4]> = Vec::new();

        let mut board = Self {
            white,
            black,
            dead,
            size,
            turn,
            komi,
            prev: Vec::new(),

            neighbors,
        };

        board.neighbors = (0..((size as usize).pow(2)))
            .map(|pos| {
                let (x, y) = board.to_coords(pos);
                let mut nbrs = [0; 4];
                let mut count = 0;

                if x > 0 {
                    nbrs[count] = board.to_pos(x - 1, y);
                    count += 1;
                }
                if x + 1 < size as usize {
                    nbrs[count] = board.to_pos(x + 1, y);
                    count += 1;
                }
                if y > 0 {
                    nbrs[count] = board.to_pos(x, y - 1);
                    count += 1;
                }
                if y + 1 < size as usize {
                    nbrs[count] = board.to_pos(x, y + 1);
                    count += 1;
                }

                // Pad with a default value if fewer than 4 neighbors
                while count < 4 {
                    nbrs[count] = usize::MAX; // Or some other invalid value
                    count += 1;
                }

                nbrs
            })
            .collect();

        board
    }

    pub fn get_hash(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }

    fn to_index(pos: usize) -> (usize, usize) {
        (pos / 32, pos % 32)
    }

    pub fn tile(&self, pos: usize) -> Tile {
        let (idx, ofs) = Self::to_index(pos);
        let mask = 1 << ofs;

        match () {
            _ if self.white[idx] & mask != 0 => Tile::White,
            _ if self.black[idx] & mask != 0 => Tile::Black,
            _ if self.dead[idx] & mask != 0 => Tile::Dead,
            _ => Tile::Free,
        }
    }

    fn set_tile(&mut self, pos: usize, tile: Tile) {
        let (idx, ofs) = Self::to_index(pos);
        let mask = 1 << ofs;

        self.white[idx] &= !mask;
        self.black[idx] &= !mask;
        self.dead[idx] &= !mask;

        match tile {
            Tile::White => self.white[idx] |= mask,
            Tile::Black => self.black[idx] |= mask,
            Tile::Dead => self.dead[idx] |= mask,
            _ => (),
        }
    }

    pub fn to_pos(&self, x: usize, y: usize) -> usize {
        (x * self.size as usize) + y
    }

    pub fn to_coords(&self, pos: usize) -> (usize, usize) {
        (pos / self.size as usize, pos % self.size as usize)
    }

    fn pass(&self) -> Board {
        let mut new_board = self.clone();

        let new_turn = match self.prev.last() {
            Some(p) if p.mv == Move::Pass => Turn::None,
            _ => self.turn.next(),
        };
        new_board.turn = new_turn;
        new_board.prev.push(PreviousData {
            board: self.get_hash(),
            mv: Move::Pass,
        });
        new_board
    }

    pub fn make_move(&self, mv: Move) -> Result<Board, String> {
        if mv == Move::Pass {
            return Ok(self.pass());
        }
        let pos = match mv {
            Move::Coords((x, y)) => self.to_pos(x, y),
            Move::Pos(p) => p,
            _ => panic!("Not possible"),
        };

        if self.tile(pos) != Tile::Free {
            return Err("Tile is not free".to_string());
        }

        let tile: Tile = self.turn.try_into()?;
        let mut new_board = self.clone();

        new_board.set_tile(pos, tile);
        new_board.turn = self.turn.next();
        new_board.prev.push(PreviousData {
            board: self.get_hash(),
            mv: Move::Pos(pos),
        });

        let next_tile: Tile = new_board.turn.try_into()?;

        for aff in self.neighbors[pos] {
            if aff == usize::MAX {
                continue;
            }
            if new_board.tile(aff) != next_tile {
                continue;
            }

            let chain = new_board.get_chain(aff);
            if chain.liberties.len() != 0 {
                continue;
            }
            for p in chain.tiles {
                new_board.set_tile(p, Tile::Free);
            }
        }

        let chain = new_board.get_chain(pos);
        if chain.liberties.len() == 0 {
            return Err("You cannot suicide".to_string());
        }

        let cur_hash = new_board.get_hash();
        if new_board.prev.iter().any(|p| p.board == cur_hash) {
            return Err("Repeating move".to_string());
        }

        Ok(new_board)
    }

    pub fn valid_moves(&self) -> impl Iterator<Item = (Move, Board)> + '_ {
        iter::once((Move::Pass, self.pass())).chain((0..((self.size as usize).pow(2))).filter_map(
            |p| match self.make_move(Move::Pos(p)) {
                Ok(board) => Some((Move::Pos(p), board)),
                Err(_) => None,
            },
        ))
    }

    pub fn from(data: &BoardData) -> Result<Self, String> {
        let rep = data
            .rep
            .replace(" ", "")
            .replace("\r", "")
            .replace("\n", "")
            .replace(":", "");
        assert_eq!(data.size.pow(2) as usize, rep.len(), "Invalid rep shape");

        let mut board = Board::new(data.size, data.turn, data.komi);
        for (pos, c) in rep.char_indices() {
            board.set_tile(pos, Tile::try_from(c)?);
        }

        Ok(board)
    }

    pub fn get_board_state(&self) -> String {
        let size = self.size as usize;
        let capacity = size * size + (size - 1);
        let mut s = String::with_capacity(capacity);
        for x in 0..size {
            for y in 0..size {
                s.push(self.tile(self.to_pos(x, y)).into());
            }
            if x < size - 1 {
                s.push('\n');
            }
        }
        s
    }
}
