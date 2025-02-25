use std::{
    env,
    io::stdin,
    thread::{self, available_parallelism},
    time::{Duration, Instant},
    usize,
};

use board::Board;
use clap::Parser;
use evaluation::{
    alphabeta::{AlphaBeta, CacheOption},
    Evaluator, Heuristic,
};
use io::{Action, IO};

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

fn main() -> Result<(), String> {
    env::set_var("RUST_BACKTRACE", "1");
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
    let evaluator = AlphaBeta::new(
        depth,
        match args.no_cache {
            true => CacheOption::Disable,
            false => CacheOption::Capacity(800_000_000),
        },
    );
    let (rep, size, turn, komi) = IO::read_state(&sin)?;

    let mut board = Board::from_rep(rep, size, turn, komi)?;

    while !board.is_terminal() {
        IO::print_result(&board);

        let start = Instant::now();
        let move_evaluation = evaluator.evaluate(&mut board);
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

    println!(
        "The game is over. Total cached states: {}",
        evaluator.stored_states()
    );

    Ok(())
}
