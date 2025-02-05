use std::{collections::{HashSet, LinkedList}, iter};
use crate::board::util::*;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::fmt::Debug;

pub struct Board {
    pub white: Vec<u32>,
    pub black: Vec<u32>,
    pub dead: Vec<u32>,

    pub size: u8,
    pub turn: Turn,

    pub komi: f32,
    pub prev: LinkedList<PreviousData>,
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Board({:?},{},{},{},{})",
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
        self.white.iter().eq(other.white.iter()) &&
        self.black.iter().eq(other.black.iter()) &&
        self.dead.iter().eq(other.dead.iter())
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
        let mut queue: LinkedList<usize> = LinkedList::new();

        queue.push_front(pos);

        while let Some(cur) = queue.pop_front() {
            if tiles.contains(&cur) {
                continue;
            }
            tiles.insert(cur);

            for p in self.neighbors(cur) {
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
            tile: tile,
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

        Self {
            white,
            black,
            dead,
            size,
            turn,
            komi,
            prev: LinkedList::new(),
        }
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
            _ if self.dead[idx]  & mask != 0 => Tile::Dead,
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

    fn neighbors(&self, pos: usize) -> Vec<usize> {
        let (x, y) = self.to_coords(pos);
        vec![
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1), Some(y)),
            (Some(x), y.checked_sub(1)),
            (Some(x), y.checked_add(1)),
        ].iter().filter(|(x, y)|
            (x.is_some() && y.is_some()) &&
            (x.unwrap() < self.size as usize) &&
            (y.unwrap() < self.size as usize)
        ).map(|(x, y)| self.to_pos(x.unwrap(), y.unwrap())).collect::<Vec<_>>()
    }

    pub fn make_pass(&self) -> Board {
        let mut new_board = self.clone();

        let new_turn = match self.prev.front() {
            Some(p) if p.mv == Move::Pass => Turn::None,
            _ => self.turn.next(),
        };
        new_board.turn = new_turn;
        new_board.prev.push_front(PreviousData { board: self.get_hash(), mv: Move::Pass });
        new_board
    }

    pub fn make_move(&self, pos: usize) -> Result<Board, String> {
        if self.tile(pos) != Tile::Free {
            return Err("Tile is not free".to_string());
        }

        let tile: Tile = self.turn.try_into()?;
        let mut new_board = self.clone();

        new_board.set_tile(pos, tile);
        new_board.turn = self.turn.next();
        new_board.prev.push_front(PreviousData {
            board: self.get_hash(),
            mv: Move::Pos(pos)
        });

        let next_tile: Tile = new_board.turn.try_into()?;

        for aff in self.neighbors(pos) {
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
        iter::once((Move::Pass, self.make_pass())).chain(
            (0..((self.size as usize).pow(2))).filter_map(|p| {
                match self.make_move(p) {
                    Ok(board) => Some((Move::Pos(p), board)),
                    Err(_) => None,
                }
            })
        )
    }

    pub fn from(rep: &String, turn: Turn, komi: f32) -> Result<Self, String> {
        let data: Vec<&str> = rep.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        let size: u8 = data.len().try_into().map_err(|_| "Size is too big".to_string())?;

        if data.iter().any(|l| l.len() != size as usize) {
            return Err("Board is not a square".to_string());
        }

        let mut board = Self::new(size, turn, komi);

        for x in 0..(size as usize) {
            for (y, c) in data[x].chars().enumerate() {
                board.set_tile(board.to_pos(x, y), Tile::try_from(c)?);
            }
        }

        Ok(board)
    }

    pub fn get_board_state(&self) -> String {
        let size = self.size as usize;
        (0..size).map(|x| String::from_iter((0..size)
            .map(|y| self.tile(self.to_pos(x, y)).into())
            .collect::<Vec<char>>()
            .iter()
        )).collect::<Vec<_>>().join("\n")
    }
}