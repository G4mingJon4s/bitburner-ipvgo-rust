use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use board::{
    util::{Chains, Move, Tile, Turn},
    Board,
};

pub trait Heuristic: Send + Sync {
    fn calculate_heuristic(&self) -> f32;
    fn is_terminal(&self) -> bool;
    fn is_maximizing(&self) -> bool;
    fn hash(&self) -> u64;
    fn children(&self) -> impl Iterator<Item = Self>;
    fn evaluate(&self, e: &Evaluator, depth: u8) -> (Duration, Vec<(Move, f32)>);
}

#[derive(Clone, Copy, Debug)]
pub enum Bound {
    Exact,
    LowerBound,
    UpperBound,
}

#[derive(Clone, Copy, Debug)]
pub struct TranspositionEntry {
    pub depth: u8,
    pub value: f32,
    pub bound: Bound,
}

#[derive(Default)]
pub struct TranspositionTable {
    entries: HashMap<u64, TranspositionEntry>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: u64, depth: u8) -> Option<TranspositionEntry> {
        self.entries.get(&key).and_then(|entry| {
            if entry.depth >= depth {
                Some(*entry)
            } else {
                None
            }
        })
    }

    pub fn insert(&mut self, key: u64, entry: TranspositionEntry) {
        self.entries.insert(key, entry);
    }
}

pub type SharedTable = Arc<Mutex<TranspositionTable>>;

pub struct Evaluator {
    table: Option<SharedTable>,
}

impl Evaluator {
    pub fn new(use_cache: bool) -> Self {
        let table = if use_cache {
            Some(Arc::new(Mutex::new(TranspositionTable::new())))
        } else {
            None
        };
        Self { table }
    }

    pub fn evaluate<T: Heuristic>(&self, root: &T, depth: u8) -> f32 {
        self.alpha_beta(root, depth, f32::NEG_INFINITY, f32::INFINITY)
    }

    pub fn evaluate_all<T: Heuristic + Send + Sync>(&self, roots: Vec<T>, depth: u8) -> Vec<f32> {
        let indexed = roots.iter().enumerate().collect::<Vec<_>>();
        let mut unordered = indexed
            .par_iter()
            .map(|(i, b)| (*i, self.evaluate(*b, depth)))
            .collect::<Vec<_>>();
        unordered.sort_by_key(|a| a.0);
        unordered.iter().map(|(_, e)| *e).collect::<Vec<_>>()
    }

    fn alpha_beta<T: Heuristic>(&self, node: &T, depth: u8, mut alpha: f32, mut beta: f32) -> f32 {
        let key = node.hash();

        if let Some(ref table) = self.table {
            if let Ok(table) = table.lock() {
                if let Some(entry) = table.get(key, depth) {
                    match entry.bound {
                        Bound::Exact => return entry.value,
                        Bound::LowerBound => alpha = alpha.max(entry.value),
                        Bound::UpperBound => beta = beta.max(entry.value),
                    }
                    if alpha >= beta {
                        return entry.value;
                    }
                }
            }
        }

        if depth == 0 || node.is_terminal() {
            return node.calculate_heuristic();
        }

        let original_alpha = alpha;
        let mut best_value = if node.is_maximizing() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };

        for child in node.children() {
            let value = self.alpha_beta(&child, depth - 1, alpha, beta);
            if node.is_maximizing() {
                best_value = best_value.max(value);
                alpha = alpha.max(best_value);
            } else {
                best_value = best_value.min(value);
                beta = beta.min(best_value);
            }
            if alpha >= beta {
                break;
            }
        }

        let bound = if best_value <= original_alpha {
            Bound::UpperBound
        } else if best_value >= beta {
            Bound::LowerBound
        } else {
            Bound::Exact
        };

        if let Some(ref table) = self.table {
            if let Ok(mut table) = table.lock() {
                table.insert(
                    key,
                    TranspositionEntry {
                        depth,
                        value: best_value,
                        bound,
                    },
                );
            }
        }

        best_value
    }
}

impl Heuristic for Board {
    fn calculate_heuristic(&self) -> f32 {
        let mut score = -self.komi;
        let mut seen = HashSet::new();

        for pos in 0..(self.size as usize).pow(2) {
            if seen.contains(&pos) {
                continue;
            }
            let chain = self.get_chain(pos);
            for &tile in &chain.tiles {
                seen.insert(tile);
            }

            match chain.tile {
                Tile::White => score -= chain.tiles.len() as f32,
                Tile::Black => score += chain.tiles.len() as f32,
                Tile::Dead => {} // dead stones are not scored
                _ => {
                    if let Some(adj_tile) = chain
                        .adjacent
                        .iter()
                        .filter_map(|&p| {
                            let t = self.tile(p);
                            if t != Tile::Free && t != Tile::Dead {
                                Some(t)
                            } else {
                                None
                            }
                        })
                        .next()
                    {
                        if chain.adjacent.iter().all(|&p| {
                            let t = self.tile(p);
                            t == Tile::Dead || t == adj_tile
                        }) {
                            if adj_tile == Tile::White {
                                score -= chain.tiles.len() as f32;
                            } else {
                                score += chain.tiles.len() as f32;
                            }
                        }
                    }
                }
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

    fn hash(&self) -> u64 {
        self.get_hash()
    }

    fn children(&self) -> impl Iterator<Item = Board> {
        self.valid_moves().map(|(_, b)| b)
    }

    fn evaluate(&self, e: &Evaluator, depth: u8) -> (Duration, Vec<(Move, f32)>) {
        let start = Instant::now();
        let moves = self.valid_moves().collect::<Vec<_>>();

        let roots = moves.iter().map(|(_, b)| b.clone()).collect::<Vec<_>>();
        let results = e.evaluate_all(roots, depth);
        let data = moves
            .iter()
            .zip(results.iter())
            .map(|(m, e)| (m.0, *e))
            .collect::<Vec<_>>();

        let end = Instant::now();
        (end - start, data)
    }
}
