use std::fmt::Debug;

pub mod alphabeta;
pub mod montecarlo;

pub trait Heuristic: Send + Sync + Clone {
    type Action: Debug + Copy + Send + Sync + PartialEq;

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

pub trait EvaluationSession<T: Heuristic>: Clone {
    fn get_root(&self) -> &T;
    fn evaluate(&mut self) -> Result<Vec<(T::Action, f32)>, String>;
    fn is_multi_threaded(&self) -> bool;

    fn apply_move(&mut self, mv: T::Action) -> Result<(), String>;
    fn undo_move(&mut self) -> Result<(), String>;
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

#[derive(Clone)]
pub enum AnyEvaluationSession<T: Heuristic> {
    AlphaBeta(alphabeta::AlphaBetaSession<T>),
    MonteCarlo(montecarlo::MonteCarloSession<T>),
}

impl<T: Heuristic> EvaluationSession<T> for AnyEvaluationSession<T> {
    fn apply_move(&mut self, mv: T::Action) -> Result<(), String> {
        match self {
            AnyEvaluationSession::AlphaBeta(ref mut a) => a.apply_move(mv),
            AnyEvaluationSession::MonteCarlo(ref mut m) => m.apply_move(mv),
        }
    }

    fn undo_move(&mut self) -> Result<(), String> {
        match self {
            AnyEvaluationSession::AlphaBeta(ref mut a) => a.undo_move(),
            AnyEvaluationSession::MonteCarlo(ref mut m) => m.undo_move(),
        }
    }

    fn is_multi_threaded(&self) -> bool {
        match self {
            AnyEvaluationSession::AlphaBeta(ref a) => a.is_multi_threaded(),
            AnyEvaluationSession::MonteCarlo(ref m) => m.is_multi_threaded(),
        }
    }

    fn evaluate(&mut self) -> Result<Vec<(T::Action, f32)>, String> {
        match self {
            AnyEvaluationSession::AlphaBeta(ref mut a) => a.evaluate(),
            AnyEvaluationSession::MonteCarlo(ref mut m) => m.evaluate(),
        }
    }

    fn get_root(&self) -> &T {
        match self {
            AnyEvaluationSession::AlphaBeta(ref a) => a.get_root(),
            AnyEvaluationSession::MonteCarlo(ref m) => m.get_root(),
        }
    }
}
