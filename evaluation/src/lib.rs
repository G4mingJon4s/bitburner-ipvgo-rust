use std::fmt::Debug;

pub mod alphabeta;

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
    fn evaluate<T: Heuristic>(&self, root: &mut T) -> Vec<(T::Action, f32)>;
}
