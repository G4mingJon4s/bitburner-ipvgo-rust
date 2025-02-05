use std::io::Stdin;

use crate::{board::*, util::*};

pub struct IO;
impl IO {
    pub fn read_state(stdin: &Stdin) -> Result<(Vec<String>, Turn, f32), String> {
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        println!("Please input your board state (Turn;Board:with:semicolons;Komi):");

        let mut s = String::new();
        stdin.read_line(&mut s).map_err(|e| e.to_string())?;
        println!("");

        let parts: Vec<_> = s.trim().split(";").map(|s| s.to_string()).collect();
        if parts.len() != 3 {
            return Err("Missing information".to_string());
        }

        let turn_char = parts[0]
            .chars()
            .nth(0)
            .ok_or("Invalid turn char".to_string())?;
        let turn = Turn::try_from(turn_char)?;
        let rep = parts[1]
            .split(":")
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let komi = parts[2].parse::<f32>().map_err(|e| e.to_string())?;
        Ok((rep, turn, komi))
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

    pub fn print_result(mv: Move, board: &Board) {
        println!("Evaluation: {}", mv);
        println!("{}", board.get_board_state());
    }
}
