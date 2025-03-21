use rayon::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use crate::{EvaluationSession, Evaluator, Heuristic};

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

pub enum CacheOption {
    Capacity(usize),
    Disable,
}

#[derive(Clone)]
pub struct AlphaBeta {
    depth: u8,
    table: Option<Arc<Mutex<TranspositionTable>>>,
}

impl AlphaBeta {
    pub fn new(depth: u8, cache: CacheOption) -> Self {
        let table = match cache {
            CacheOption::Capacity(a) => Some(Arc::new(Mutex::new(TranspositionTable::new(a)))),
            CacheOption::Disable => None,
        };
        Self { depth, table }
    }

    pub fn stored_states(&self) -> usize {
        self.table.clone().map_or(0, |t| t.lock().unwrap().len())
    }

    fn alpha_beta<T: Heuristic>(
        &self,
        node: &mut T,
        depth: u8,
        mut alpha: f32,
        mut beta: f32,
    ) -> f32 {
        let key = node.get_hash();

        if let Some(entry) = self
            .table
            .as_ref()
            .map(|t| t.lock().unwrap().get(key, depth))
            .flatten()
        {
            match entry.bound {
                Bound::Exact => return entry.value,
                Bound::LowerBound => alpha = alpha.max(entry.value),
                Bound::UpperBound => beta = beta.max(entry.value),
            }
            if alpha >= beta {
                return entry.value;
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

        if let Some(mut table) = self.table.as_ref().map(|t| t.lock().unwrap()) {
            table.insert(
                key,
                TranspositionEntry {
                    depth,
                    value: best_value,
                    bound,
                },
            );
        }

        best_value
    }
}

impl Evaluator for AlphaBeta {
    fn evaluate<T: Heuristic>(&self, root: &mut T) -> Result<Vec<(T::Action, f32)>, String> {
        let moves = root.moves().collect::<Vec<_>>();
        Ok(moves
            .into_par_iter()
            .filter_map(|m| {
                let mut copy = root.clone();
                copy.play(m).ok()?;
                let eval = self.alpha_beta(&mut copy, self.depth, f32::MIN, f32::MAX);
                Some((m, eval))
            })
            .collect())
    }

    fn is_multi_threaded(&self) -> bool {
        true
    }
}

#[derive(Clone)]
pub struct AlphaBetaSession<T: Heuristic> {
    pub root: T,
    evaluator: AlphaBeta,
}

impl<T: Heuristic> AlphaBetaSession<T> {
    pub fn new(root: T, depth: u8, cache: CacheOption) -> Self {
        Self {
            root,
            evaluator: AlphaBeta::new(depth, cache),
        }
    }
}

impl<T: Heuristic> EvaluationSession<T> for AlphaBetaSession<T> {
    fn apply_move(&mut self, mv: <T as Heuristic>::Action) -> Result<(), String> {
        self.root.play(mv)
    }

    fn undo_move(&mut self) -> Result<(), String> {
        self.root.undo()
    }

    fn evaluate(&mut self) -> Result<Vec<(<T as Heuristic>::Action, f32)>, String> {
        self.evaluator.evaluate(&mut self.root)
    }

    fn is_multi_threaded(&self) -> bool {
        self.evaluator.is_multi_threaded()
    }

    fn get_root(&self) -> &T {
        &self.root
    }
}
