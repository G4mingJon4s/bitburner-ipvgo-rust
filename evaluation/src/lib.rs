use board::{Board, Move, Tile, Turn};
use rayon::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub trait Heuristic: Send + Sync {
    type Action: Debug + Copy;

    fn calculate_heuristic(&self) -> f32;
    fn is_terminal(&self) -> bool;
    fn is_maximizing(&self) -> bool;
    fn hash(&self) -> u64;
    fn moves(&self) -> impl Iterator<Item = Self::Action>;
    fn play(&mut self, mv: Self::Action) -> Result<(), String>;
    fn undo(&mut self) -> Result<(), String>;
    fn evaluate(&self, e: &Evaluator, depth: u8) -> (Duration, Vec<(Self::Action, f32)>);
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
    capacity: usize,
    entries: HashMap<u64, TranspositionEntry>,
    inserted: VecDeque<u64>,
}

impl TranspositionTable {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            inserted: VecDeque::with_capacity(capacity),
        }
    }

    pub fn capacity_from_ram(ram: usize) -> usize {
        ram / (32 + size_of::<TranspositionEntry>())
    }

    pub fn get(&mut self, key: u64, depth: u8) -> Option<TranspositionEntry> {
        if *self.inserted.front().unwrap_or(&u64::MAX) == key {
            self.inserted.pop_front();
            self.inserted.push_back(key);
        }
        self.entries.get(&key).and_then(|entry| {
            if entry.depth >= depth {
                Some(*entry)
            } else {
                None
            }
        })
    }

    pub fn insert(&mut self, key: u64, entry: TranspositionEntry) {
        if self.entries.len() >= self.capacity {
            let removal = self.inserted.pop_front().unwrap();
            self.entries.remove(&removal);
        }

        self.entries.insert(key, entry);
        self.inserted.push_back(key);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

pub type SharedTable = Arc<Mutex<TranspositionTable>>;

pub struct Evaluator {
    table: Option<SharedTable>,
}

impl Evaluator {
    pub fn new(use_cache: bool, capacity: usize) -> Self {
        let table = if use_cache {
            Some(Arc::new(Mutex::new(TranspositionTable::new(capacity))))
        } else {
            None
        };
        Self { table }
    }

    pub fn stored_states(&self) -> usize {
        self.table.clone().map_or(0, |t| t.lock().unwrap().len())
    }

    pub fn evaluate<T: Heuristic>(&self, root: &mut T, depth: u8) -> f32 {
        self.alpha_beta(root, depth, f32::NEG_INFINITY, f32::INFINITY)
    }

    pub fn evaluate_all<
        A: Send + Sync,
        T: Heuristic + Send + Sync,
        F: Send + Sync + Fn(&A) -> Option<T>,
    >(
        &self,
        roots: Vec<A>,
        depth: u8,
        key: F,
    ) -> Vec<(A, f32)> {
        let mut unordered = roots
            .into_par_iter()
            .filter_map(|a| {
                let mut root = key(&a)?;
                let result = self.evaluate(&mut root, depth);
                Some((a, result))
            })
            .collect::<Vec<_>>();
        unordered.sort_by(|a, b| b.1.total_cmp(&a.1));
        unordered
    }

    fn alpha_beta<T: Heuristic>(
        &self,
        node: &mut T,
        depth: u8,
        mut alpha: f32,
        mut beta: f32,
    ) -> f32 {
        let key = node.hash();

        if let Some(ref table) = self.table {
            if let Ok(mut table) = table.lock() {
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

        let moves = node.moves().collect::<Vec<_>>();
        for mv in moves {
            if node.play(mv).is_err() {
                continue;
            }

            let value = self.alpha_beta(node, depth - 1, alpha, beta);
            node.undo().unwrap();
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
    type Action = Move;

    fn calculate_heuristic(&self) -> f32 {
        let mut score = -self.komi;

        for c in self.chains.iter().filter_map(|a| a.as_ref()) {
            if c.tile == Tile::Free {
                let tile = c.adjacent.iter().find_map(|&a| match self.get_tile(a) {
                    Tile::Dead => None,
                    Tile::Free => None,
                    a => Some(a),
                });
                if tile.is_some()
                    && c.adjacent.iter().all(|&a| {
                        let t = self.get_tile(a);
                        t == Tile::Dead || t == tile.unwrap()
                    })
                {
                    match tile.unwrap() {
                        Tile::Black => score += c.positions.len() as f32,
                        Tile::White => score -= c.positions.len() as f32,
                        _ => panic!("not possible"),
                    }
                }
                continue;
            }

            match c.tile {
                Tile::Black => score += c.positions.len() as f32,
                Tile::White => score -= c.positions.len() as f32,
                _ => panic!("not possible"),
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
        self.compute_board_hash()
    }

    fn moves(&self) -> impl Iterator<Item = Self::Action> {
        self.valid_moves()
    }

    fn play(&mut self, mv: Self::Action) -> Result<(), String> {
        self.apply_move(mv)
    }

    fn undo(&mut self) -> Result<(), String> {
        self.undo_move()
    }

    fn evaluate(&self, e: &Evaluator, depth: u8) -> (Duration, Vec<(Self::Action, f32)>) {
        let start = Instant::now();
        let moves = self.valid_moves().collect::<Vec<_>>();

        let results = e.evaluate_all(moves, depth, |&mv| {
            let mut copy = self.clone();
            match copy.apply_move(mv) {
                Ok(_) => Some(copy),
                Err(_) => None,
            }
        });

        let end = Instant::now();
        (end - start, results)
    }
}
