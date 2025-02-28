use std::{
    io::stdin,
    thread::{self},
    time::{Duration, Instant},
};

use board::Board;
use evaluation::{AnyEvaluator, Evaluator, Heuristic};
use io::{Action, IO};
use rayon::ThreadPoolBuilder;

mod io;

fn main() -> Result<(), String> {
    let sin = stdin();
    let evaluator: AnyEvaluator = IO::read_algorithm(&sin)?;

    if evaluator.is_multi_threaded() {
        let threads = IO::read_threads(&sin)?;
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap();
    }

    let (rep, size, turn, komi) = IO::read_state(&sin)?;

    let mut board = Board::from_rep(rep, size, turn, komi)?;

    while !board.is_terminal() {
        IO::print_result(&board);

        let start = Instant::now();
        let move_evaluation = evaluator.evaluate(&mut board)?;
        let end = Instant::now();

        IO::print_move_evalutations(&board, move_evaluation, board.is_maximizing(), end - start);

        let action = IO::read_action(&sin, &board);
        if let Err(e) = action {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        match action.unwrap() {
            Action::Mv(mv) => {
                if let Err(e) = board.apply_move(mv) {
                    eprintln!("Error: {}", e);
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }
            }
            Action::Undo => {
                if let Err(e) = board.undo_move() {
                    eprintln!("Error: {}", e);
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }
            }
        }

        IO::press_enter_continue(&sin);
    }

    println!("The game is over");

    Ok(())
}
