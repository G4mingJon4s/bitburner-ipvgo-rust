use std::{io::stdin, thread, time::Duration};

use clap::Parser;
use evaluation::{evaluation::Heuristic, session::Session};
use io::IO;
use threads::ThreadPool;

mod io;

const DEPTH: u8 = 4;
const THREADS: usize = 5;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = DEPTH)]
    depth: u8,
    #[arg(short, long, default_value_t = THREADS)]
    threads: usize,
    #[arg(short, long, default_value_t = false)]
    no_cache: bool,
    #[arg(short, long, default_value_t = false)]
    manual: bool,
}

fn main() {
    let args = Args::parse();
    let depth = args.depth;
    let threads = args.threads;
    let manual = args.manual;

    let sin = stdin();
    let pool = ThreadPool {
        max_threads: threads,
    };

    let state = IO::read_state(&sin);
    if let Err(e) = state {
        eprintln!("Error: {}", e);
        return;
    }

    let mut session = Session::new(&state.unwrap(), !args.no_cache, depth)
        .expect("Could not instantiate session");

    while !session.is_over() {
        if !manual {
            let (time, move_evaluation) = session.get_current_evaluation(&pool);
            IO::print_move_evalutations(move_evaluation, session.get_board().is_maximizing(), time);
        }

        let mv = IO::read_move(&sin);
        if let Err(e) = mv {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        let parsed_move = mv.unwrap();
        let new_board = session.make_move(parsed_move);
        if let Err(e) = new_board {
            eprintln!("Error: {}", e);
            thread::sleep(Duration::from_millis(2000));
            continue;
        }

        IO::print_result(parsed_move, session.get_board());
        IO::press_enter_continue(&sin);
    }

    println!("The game is over");
}
