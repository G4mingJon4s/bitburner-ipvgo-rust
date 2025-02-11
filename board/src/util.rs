use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl TryFrom<String> for Turn {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "white" => Ok(Turn::White),
            "black" => Ok(Turn::Black),
            "none" => Ok(Turn::None),
            _ => Err("Invalid string".to_string()),
        }
    }
}

impl Into<&'static str> for Turn {
    fn into(self) -> &'static str {
        match self {
            Turn::White => "White",
            Turn::Black => "Black",
            Turn::None => "None",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Move {
    Pos(usize),
    Coords((usize, usize)),
    Pass,
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Move::Pass => "Pass".to_string(),
            Move::Coords((x, y)) => format!("Move ({},{})", x, y),
            Move::Pos(p) => format!("Move {}", p),
        };

        write!(f, "{:width$}", s, width = f.width().unwrap_or(0))
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
