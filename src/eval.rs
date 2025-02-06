use crate::{board::*, util::*};
use std::collections::HashSet;

pub trait Heuristic
where
    Self: Sized,
{
    fn calculate(&self) -> f32;
    fn is_terminal(&self) -> bool;
    fn is_maximizing(&self) -> bool;
    fn get_children(&self) -> impl Iterator<Item = Self>;
}

pub struct Evaluation;
impl Evaluation {
    pub fn evaluate<T: Heuristic>(root: &T, depth: u8) -> f32 {
        Evaluation::alpha_beta(root, depth, f32::NEG_INFINITY, f32::INFINITY)
    }

    fn alpha_beta<T: Heuristic>(root: &T, depth: u8, mut alpha: f32, mut beta: f32) -> f32 {
        if depth == 0 || root.is_terminal() {
            return root.calculate();
        }

        let mut value = if root.is_maximizing() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };
        for child in root.get_children() {
            let eval = Evaluation::alpha_beta(&child, depth - 1, alpha, beta);

            if root.is_maximizing() {
                value = value.max(eval);
                alpha = alpha.max(value);
            } else {
                value = value.min(eval);
                beta = beta.min(value);
            };

            if alpha >= beta {
                break;
            }
        }

        value
    }
}

impl Heuristic for Board {
    fn calculate(&self) -> f32 {
        let mut score = self.komi * -1.0;

        let mut seen: HashSet<usize> = HashSet::new();
        for pos in 0..(self.size as usize).pow(2) {
            if seen.contains(&pos) {
                continue;
            }
            let chain = self.get_chain(pos);
            chain.tiles.iter().for_each(|&t| {
                seen.insert(t);
            });

            if chain.tile == Tile::White {
                score -= chain.tiles.len() as f32;
                continue;
            }
            if chain.tile == Tile::Black {
                score += chain.tiles.len() as f32;
                continue;
            }
            if chain.tile == Tile::Dead {
                continue;
            }

            let adj_type = chain.adjacent.iter().find_map(|&t| match self.tile(t) {
                Tile::Free | Tile::Dead => None,
                tile => Some(tile),
            });

            match adj_type {
                Some(typ)
                    if chain.adjacent.iter().all(|&t| {
                        let tile = self.tile(t);
                        tile == Tile::Dead || tile == typ
                    }) =>
                {
                    if typ == Tile::White {
                        score -= chain.tiles.len() as f32;
                    } else {
                        score += chain.tiles.len() as f32;
                    }
                }
                _ => (),
            }
        }

        score
    }

    fn is_terminal(&self) -> bool {
        self.turn == Turn::None
    }

    fn is_maximizing(&self) -> bool {
        self.turn == Turn::Black
    }

    fn get_children(&self) -> impl Iterator<Item = Self> {
        self.valid_moves().map(|(_, b)| b)
    }
}
