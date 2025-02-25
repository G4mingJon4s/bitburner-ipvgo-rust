use std::{io::Stdin, time::Duration};

use board::{Board, Move, Turn};

pub enum Action {
    Undo,
    Mv(Move),
}

pub struct IO;
impl IO {
    pub fn read_state(stdin: &Stdin) -> Result<(String, u8, Turn, f32), String> {
        println!("Please input the starting state (rep;size;turn;komi):");

        let mut s = String::new();
        stdin.read_line(&mut s).map_err(|e| e.to_string())?;
        println!("");

        let splits = s
            .trim()
            .to_lowercase()
            .split(";")
            .map(|a| a.to_string())
            .collect::<Vec<_>>();
        if splits.len() != 4 {
            return Err("Please input 4 things separated by semicolons".to_string());
        }

        let rep = splits[0].trim().to_string();
        let size = splits[1]
            .parse::<u8>()
            .map_err(|_| "Size is not a number".to_string())?;
        let turn = Turn::from_str(splits[2].as_str()).ok_or("Turn is not valid".to_string())?;
        let komi = splits[3]
            .trim()
            .parse::<f32>()
            .map_err(|_| "Komi is not a number".to_string())?;

        Ok((rep, size, turn, komi))
    }

    pub fn read_action(stdin: &Stdin, board: &Board) -> Result<Action, String> {
        println!("Please input the next action (pass | x,y | undo):");

        let mut s = String::new();
        stdin.read_line(&mut s).map_err(|e| e.to_string())?;
        println!("");

        if s.trim().to_lowercase() == "pass" {
            return Ok(Action::Mv(Move::Pass));
        }

        if s.trim().to_lowercase() == "undo" {
            return Ok(Action::Undo);
        }

        let (x, y) = s.trim().split_once(',').ok_or("Missing ','".to_string())?;
        Ok(Action::Mv(Move::Place(board.to_pos(
            x.parse().map_err(|_| "X is not a valid number")?,
            y.parse().map_err(|_| "Y is not a valid number")?,
        ))))
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
                    Move::Coords((x, y)) => format!("Place {}, {}", x, y),
                    Move::Place(a) => {
                        let coords = root.to_coords(*a);
                        format!("Place {}, {}", coords.0, coords.1)
                    }
                    Move::Pass => "Pass".to_string(),
                },
                eval
            );
        }
    }

    pub fn print_result(board: &Board) {
        println!(
            "{}",
            board
                .get_rep()
                .char_indices()
                .fold(String::new(), |mut acc, (i, c)| {
                    if i > 0 && (i % board.size as usize) == 0 {
                        acc.push('\n');
                    }
                    acc.push(c);
                    acc
                })
        );
    }
}
