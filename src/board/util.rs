use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    White,
    Black,
    Dead,
    Free,
}

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c: char = (*self).into();
        write!(f, "{}", c)
    }
}

impl Into<char> for Tile {
    fn into(self) -> char {
        match self {
            Tile::White => 'O',
            Tile::Black => 'X',
            Tile::Dead => '#',
            Tile::Free => '.',
        }
    }
}

impl TryFrom<char> for Tile {
    type Error = String;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'O' => Ok(Tile::White),
            'X' => Ok(Tile::Black),
            '#' => Ok(Tile::Dead),
            '.' => Ok(Tile::Free),
            _ => Err("Invalid character".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    White,
    Black,
    None,
}

impl TryInto<Tile> for Turn {
    type Error = String;
    fn try_into(self) -> Result<Tile, Self::Error> {
        match self {
            Turn::Black => Ok(Tile::Black),
            Turn::White => Ok(Tile::White),
            Turn::None => Err("No tile for Turn::None".to_string()),
        }
    }
}

impl Turn {
    pub fn next(&self) -> Turn {
        match self {
            Turn::Black => Turn::White,
            Turn::White => Turn::Black,
            Turn::None => Turn::None,
        }
    }
}
impl TryFrom<char> for Turn {
    type Error = String;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'W' => Ok(Turn::White),
            'B' => Ok(Turn::Black),
            'N' => Ok(Turn::None),
            _ => Err("Invalid char".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Pos(usize),
    Pass,
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Move::Pass => write!(f, "Pass"),
            Move::Pos(p) => write!(f, "Move {}", *p),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreviousData {
    pub mv: Move,
    pub board: u64,
}

#[derive(Debug, Clone)]
pub struct ChainData {
    pub tile: Tile,
    pub tiles: Vec<usize>,
    pub adjacent: Vec<usize>,
    pub liberties: Vec<usize>,
}

pub trait Chains {
    fn get_chain(&self, pos: usize) -> ChainData;
}

pub fn extract_board_state(rep: &String) -> Result<(String, Turn, f32), String> {
    let lines = rep.lines().map(|l| l.trim()).collect::<Vec<_>>();
    let (&meta, board) = lines.split_first().ok_or("Invalid format".to_string())?;

    let (turn, komi) = meta
        .split_once(':')
        .ok_or("Metadata missing :".to_string())?;
    Ok((
        board.join("\n"),
        Turn::try_from(
            turn.chars()
                .nth(0)
                .ok_or("No chars on the left side".to_string())?,
        )?,
        komi.parse::<f32>()
            .map_err(|_| "komi is invalid".to_string())?,
    ))
}

pub fn extract_move(size: u8, rep: &String) -> Result<Move, String> {
    if rep.trim().to_lowercase() == "pass" {
        return Ok(Move::Pass);
    }
    let (x, y) = rep
        .trim()
        .split_once(' ')
        .ok_or("Please provide two values".to_string())?;

    Ok(Move::Pos(
        x.parse::<usize>().map_err(|_| "Invalid x".to_string())? * (size as usize)
            + y.parse::<usize>().map_err(|_| "Invalid y".to_string())?,
    ))
}
