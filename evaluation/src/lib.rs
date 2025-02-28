use std::fmt::Debug;

pub mod alphabeta;
pub mod montecarlo;

pub trait Heuristic: Send + Sync + Clone {
    type Action: Debug + Copy + Send + Sync;

    fn calculate_heuristic(&self) -> f32;
    fn is_terminal(&self) -> bool;
    fn is_maximizing(&self) -> bool;
    fn get_hash(&self) -> u64;
    fn moves(&self) -> impl Iterator<Item = Self::Action>;
    fn play(&mut self, mv: Self::Action) -> Result<(), String>;
    fn undo(&mut self) -> Result<(), String>;
}

pub trait Evaluator {
    fn evaluate<T: Heuristic>(&self, root: &mut T) -> Result<Vec<(T::Action, f32)>, String>;
    fn is_multi_threaded(&self) -> bool;
}

pub enum AnyEvaluator {
    AlphaBeta(alphabeta::AlphaBeta),
    MonteCarlo(montecarlo::MonteCarlo),
}

impl Evaluator for AnyEvaluator {
    fn evaluate<T: Heuristic>(&self, root: &mut T) -> Result<Vec<(T::Action, f32)>, String> {
        match self {
            &AnyEvaluator::AlphaBeta(ref a) => a.evaluate(root),
            &AnyEvaluator::MonteCarlo(ref m) => m.evaluate(root),
        }
    }

    fn is_multi_threaded(&self) -> bool {
        match self {
            &AnyEvaluator::AlphaBeta(ref a) => a.is_multi_threaded(),
            &AnyEvaluator::MonteCarlo(ref m) => m.is_multi_threaded(),
        }
    }
}
