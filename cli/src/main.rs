use std::{
    io::stdin,
    thread::{self, available_parallelism},
    time::Duration,
    usize,
};

use board::Board;
use clap::Parser;
use evaluation::{Evaluator, Heuristic};
use io::IO;

mod io;

const DEPTH: u8 = 4;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = DEPTH)]
    depth: u8,
    #[arg(short, long, default_value_t = 0_f32)]
    threads: f32,
    #[arg(short, long, default_value_t = false)]
    no_cache: bool,
}

fn main() {
    let args = Args::parse();
    let depth = args.depth;
    let threads = args.threads;

    let available_cores = available_parallelism().unwrap().get();

    let threads: usize = if threads.is_sign_negative() || threads == 0_f32 {
        available_cores / 2
    } else if threads.floor() == threads {
        threads.floor() as usize
    } else {
        (available_cores as f32 * threads)
            .clamp(1_f32, available_cores as f32)
            .floor() as usize
    };

    println!("Running with {} threads", threads);
    println!("Calculating with a depth of {}", depth);

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()
        .expect("Thread pool error");

    let sin = stdin();
    let evaluator = Evaluator::new(!args.no_cache);

    let state = IO::read_state(&sin);
    if let Err(e) = state {
        eprintln!("Error: {}", e);
        return;
    }

    let mut board = Board::from(&state.unwrap()).expect("Could not instantiate session");

    while !board.is_terminal() {
        let (time, move_evaluation) = board.evaluate(&evaluator, depth);
        IO::print_move_evalutations(&board, move_evaluation, board.is_maximizing(), time);

        let mv = IO::read_move(&sin);
        if let Err(e) = mv {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        let parsed_move = mv.unwrap();
        let new_board = board.make_move_mut(parsed_move);
        if let Err(e) = new_board {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        IO::print_result(parsed_move, &board);
        IO::press_enter_continue(&sin);
    }

    println!("The game is over");
}
