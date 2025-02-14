use std::{io::Stdin, time::Duration};

use board::{util::Move, Board, BoardData};

pub struct IO;
impl IO {
    pub fn read_state(stdin: &Stdin) -> Result<BoardData, String> {
        println!("Please input your board state (Turn;Size;Board:with:semicolons;Komi):");

        let mut s = String::new();
        stdin.read_line(&mut s).map_err(|e| e.to_string())?;
        println!("");

        BoardData::from(s)
    }

    pub fn read_move(stdin: &Stdin) -> Result<Move, String> {
        println!("Please input the next move (pass | x,y):");

        let mut s = String::new();
        stdin.read_line(&mut s).map_err(|e| e.to_string())?;
        println!("");

        if s.trim().to_lowercase() == "pass" {
            return Ok(Move::Pass);
        }

        let (x, y) = s.trim().split_once(',').ok_or("Missing ','".to_string())?;
        Ok(Move::Coords((
            x.parse().map_err(|_| "X is not a valid number")?,
            y.parse().map_err(|_| "X is not a valid number")?,
        )))
    }

    pub fn press_enter_continue(stdin: &Stdin) {
        println!("Press Enter to continue...");
        let mut s = String::new();
        stdin.read_line(&mut s).unwrap();
    }

    pub fn print_move_evalutations(
        root: &Board,
        moves: Vec<(Move, f32)>,
        maximizing: bool,
        time: Duration,
    ) {
        println!("Move evaluations ({} seconds):", time.as_secs());

        let mut sorted: Vec<_> = moves.iter().collect();
        sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
        if maximizing {
            sorted.reverse();
        }

        let width = (sorted.len() as f32).log10().floor() as usize + 1;
        for (i, (mv, eval)) in sorted.iter().enumerate() {
            println!(
                "{:width$}: {:12} | {:+05.1}",
                i,
                match mv {
                    Move::Pos(a) => Move::Coords(Board::to_coords(*a, root.size)),
                    a => *a,
                },
                eval
            );
        }
    }

    pub fn print_result(mv: Move, board: &Board) {
        println!("Evaluation: {}", mv);
        println!("{}", board.get_board_state());
    }
}
