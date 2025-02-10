use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use board::{util::Move, Board, BoardData};
use threads::{Pool, PoolHandle};

use crate::evaluation::{Evaluation, Heuristic};

pub struct Session {
    board: Board,
    evaluation: Arc<Evaluation>,
    max_depth: u8,
}

impl Session {
    pub fn new(data: &BoardData, use_cache: bool, max_depth: u8) -> Result<Self, String> {
        Ok(Self {
            board: Board::from(data)?,
            evaluation: Arc::new(Evaluation::new(use_cache)),
            max_depth,
        })
    }

    pub fn make_move(&mut self, mv: Move) -> Result<&Board, String> {
        let new_board = self.board.make_move(mv)?;
        self.board = new_board;
        Ok(&self.board)
    }

    pub fn get_board(&self) -> &Board {
        &self.board
    }

    pub fn is_over(&self) -> bool {
        self.board.is_terminal()
    }

    pub fn get_current_evaluation<P: Pool>(&self, pool: &P) -> (Duration, Vec<(Move, f32)>) {
        let start = Instant::now();
        let results = <P as Pool>::multiple(
            self.board
                .valid_moves()
                .map(|(m, b)| (m, b, self.evaluation.clone(), self.max_depth))
                .collect(),
            |(m, b, e, d)| (m, e.evaluate(&b, d)),
            pool.get_max_threads(),
        );

        let evaluations = PoolHandle::recv_all(results);
        let end = Instant::now();
        (end - start, evaluations)
    }
}
