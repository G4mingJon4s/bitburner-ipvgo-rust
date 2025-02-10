use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use board::{util::Move, Board, BoardData};
use threads::ThreadPool;

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

    pub fn get_current_evaluation(&self, pool: &ThreadPool) -> (Duration, Vec<(Move, f32)>) {
        let moves = self.board.valid_moves().collect::<Vec<_>>();
        let __relative = moves.len() as f32 / self.board.size.pow(2) as f32;
        let depth = self.max_depth;

        let start = Instant::now();

        let results = pool.execute::<(Move, Board, Arc<Evaluation>), (Move, f32)>(
            &self
                .board
                .valid_moves()
                .map(|(m, b)| (m, b, self.evaluation.clone()))
                .collect::<Vec<_>>(),
            move |(m, b, e)| {
                (
                    match *m {
                        Move::Pos(p) => Move::Coords(b.to_coords(p)),
                        v => v,
                    },
                    e.evaluate(b, depth),
                )
            },
        );

        let end = Instant::now();
        (end - start, results)
    }
}
