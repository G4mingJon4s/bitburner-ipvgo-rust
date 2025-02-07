use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::{
    board::Board,
    util::{Chains, Tile, Turn},
};

pub trait Heuristic
where
    Self: Sized,
{
    fn calculate(&self) -> f32;
    fn is_terminal(&self) -> bool;
    fn is_maximizing(&self) -> bool;
    fn calculate_hash(&self) -> u64;
    fn get_children(&self) -> impl Iterator<Item = Self>;
}

pub struct TranspositionTable {
    map: HashMap<u64, (u8, f32)>,
}
impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn get(&self, key: &u64) -> Option<&(u8, f32)> {
        self.map.get(key)
    }

    pub fn get_eval(&self, key: &u64, depth: &u8) -> Option<f32> {
        let entry = self.map.get(key);
        entry.map(|t| match t.0 {
            value if value >= *depth => Some(t.1),
            _ => None,
        })?
    }

    pub fn set(&mut self, key: u64, value: (u8, f32)) {
        self.map.insert(key, value);
    }

    pub fn set_eval(&mut self, key: u64, value: (u8, f32)) {
        if let Some((d, _)) = self.get(&key) {
            if *d >= value.0 {
                return;
            }
        }

        self.set(key, value);
    }
}

pub type Table = Arc<Mutex<TranspositionTable>>;

#[derive(Clone)]
pub struct Evaluation {
    table: Table,
    use_cache: bool,
}

impl Evaluation {
    pub fn new(use_cache: bool) -> Self {
        Self {
            table: Arc::new(Mutex::new(TranspositionTable::new())),
            use_cache,
        }
    }

    pub fn evaluate<T: Heuristic>(&self, root: &T, depth: u8) -> f32 {
        self.alpha_beta(root, depth, f32::NEG_INFINITY, f32::INFINITY)
    }

    fn alpha_beta<T: Heuristic>(&self, root: &T, depth: u8, mut alpha: f32, mut beta: f32) -> f32 {
        if self.use_cache {
            let handle = self.table.lock().unwrap();
            if let Some(eval) = handle.get_eval(&root.calculate_hash(), &depth) {
                return eval;
            }
        }
        if depth == 0 || root.is_terminal() {
            let result = root.calculate();
            if self.use_cache {
                let mut handle = self.table.lock().unwrap();
                handle.set_eval(root.calculate_hash(), (depth, result));
            }
            return result;
        }

        let mut value = if root.is_maximizing() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
        for child in root.get_children() {
            let eval = self.alpha_beta(&child, depth - 1, alpha, beta);

            if root.is_maximizing() {
                value = value.max(eval);
                alpha = alpha.max(value);
            } else {
                value = value.min(eval);
                beta = beta.min(value);
            };

            if alpha >= beta {
                break;
            }
        }

        if self.use_cache {
            let mut handle = self.table.lock().unwrap();
            handle.set_eval(root.calculate_hash(), (depth, value));
        }
        value
    }
}

impl Heuristic for Board {
    fn calculate(&self) -> f32 {
        let mut score = self.komi * -1.0;

        let mut seen: HashSet<usize> = HashSet::new();
        for pos in 0..(self.size as usize).pow(2) {
            if seen.contains(&pos) {
                continue;
            }
            let chain = self.get_chain(pos);
            chain.tiles.iter().for_each(|&t| {
                seen.insert(t);
            });

            if chain.tile == Tile::White {
                score -= chain.tiles.len() as f32;
                continue;
            }
            if chain.tile == Tile::Black {
                score += chain.tiles.len() as f32;
                continue;
            }
            if chain.tile == Tile::Dead {
                continue;
            }

            let adj_type = chain.adjacent.iter().find_map(|&t| match self.tile(t) {
                Tile::Free | Tile::Dead => None,
                tile => Some(tile),
            });

            match adj_type {
                Some(typ)
                    if chain.adjacent.iter().all(|&t| {
                        let tile = self.tile(t);
                        tile == Tile::Dead || tile == typ
                    }) =>
                {
                    if typ == Tile::White {
                        score -= chain.tiles.len() as f32;
                    } else {
                        score += chain.tiles.len() as f32;
                    }
                }
                _ => (),
            }
        }

        score
    }

    fn is_terminal(&self) -> bool {
        self.turn == Turn::None
    }

    fn is_maximizing(&self) -> bool {
        self.turn == Turn::Black
    }

    fn calculate_hash(&self) -> u64 {
        self.get_hash()
    }

    fn get_children(&self) -> impl Iterator<Item = Self> {
        self.valid_moves().map(|(_, b)| b)
    }
}
