use core::f32;
use std::time::{Duration, Instant};

use rand::{
    rng,
    seq::{IndexedRandom, IteratorRandom},
};

use crate::{EvaluationSession, Evaluator, Heuristic};

const UCB1: f32 = 1.1;

#[derive(Clone)]
struct Node<T: Heuristic> {
    pub children: Option<Vec<(T::Action, Node<T>)>>,
    pub maximizing: bool,
    pub total: f32,
    pub visits: usize,
}

impl<T: Heuristic> Node<T> {
    pub fn new(maximizing: bool) -> Self {
        Self {
            children: None,
            maximizing,
            total: 0.0,
            visits: 0,
        }
    }

    pub fn expand(&mut self, game: &mut T) {
        if self.children.is_some() || game.is_terminal() {
            return;
        }

        let moves = game.moves().collect::<Vec<_>>();
        let mut children: Vec<(T::Action, Node<T>)> = Vec::new();
        for mv in moves {
            let result = game.play(mv);

            if result.is_err() {
                continue;
            }

            children.push((mv, Self::new(game.is_maximizing())));
            game.undo().unwrap();
        }

        self.children = Some(children);
    }

    pub fn ucb1(&self, parent_visits: usize) -> f32 {
        let exploration = (2.0 * (parent_visits as f32).ln() / self.visits as f32).sqrt() * UCB1;
        let signed_score = if self.maximizing {
            self.total * -1.0
        } else {
            self.total
        };
        let exploitation = signed_score / self.visits as f32;
        let exploitation = 1.0 / (1.0 + (-0.3 * exploitation).exp());

        if exploration.is_infinite() || exploitation.is_infinite() {
            return f32::MAX;
        }

        exploitation + exploration
    }

    pub fn simulate(game: &mut T) -> f32 {
        if game.is_terminal() {
            return game.calculate_heuristic();
        }

        let moves = game.moves().collect::<Vec<_>>();
        loop {
            let &chosen = moves.choose(&mut rng()).unwrap();
            let result = game.play(chosen);

            if result.is_ok() {
                break;
            }
        }

        let value = Self::simulate(game);
        game.undo().unwrap();

        value
    }

    pub fn max_child(&mut self) -> (T::Action, &mut Node<T>) {
        let mut cur_value = f32::MIN;
        let mut cur_max: Vec<(T::Action, &mut Node<T>)> = Vec::new();

        let children = self.children.as_mut().unwrap();
        for (mv, node) in children.iter_mut() {
            let value = node.ucb1(self.visits);

            if cur_value > value {
                continue;
            }

            if cur_value < value {
                cur_value = value;
                cur_max.clear();
            }

            cur_max.push((*mv, node));
        }

        cur_max.into_iter().choose(&mut rng()).unwrap()
    }

    pub fn backpropagate(&mut self, game: &mut T) -> f32 {
        if game.is_terminal() {
            let value = game.calculate_heuristic();

            self.total += value;
            self.visits += 1;

            return value;
        }

        if self.visits > 0 && self.children.is_none() {
            self.expand(game);
        }

        if self.children.is_some() {
            let (mv, child) = self.max_child();

            game.play(mv).unwrap();
            let value = child.backpropagate(game);
            game.undo().unwrap();

            self.total += value;
            self.visits += 1;

            return value;
        }

        let value = Self::simulate(game);
        self.total += value;
        self.visits += 1;

        value
    }
}

pub struct MonteCarlo {
    pub time: Duration,
}

impl MonteCarlo {
    pub fn new(time: Duration) -> Self {
        Self { time }
    }
}

impl Evaluator for MonteCarlo {
    fn evaluate<T: Heuristic>(&self, game: &mut T) -> Result<Vec<(T::Action, f32)>, String> {
        let mut root: Node<T> = Node::new(game.is_maximizing());

        let start = Instant::now();
        while Instant::now() - start < self.time {
            root.backpropagate(game);
        }

        Ok(root
            .children
            .unwrap()
            .into_iter()
            .map(|(m, n)| {
                let sign = if root.maximizing { 1.0 } else { -1.0 };
                (m, sign * n.visits as f32)
            })
            .collect())
    }

    fn is_multi_threaded(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct MonteCarloSession<T: Heuristic> {
    node: Node<T>,

    pub root: T,
    pub time: Duration,
}

impl<T: Heuristic> MonteCarloSession<T> {
    pub fn new(root: T, time: Duration) -> Self {
        Self {
            time,
            node: Node::new(root.is_maximizing()),
            root,
        }
    }
}

impl<T: Heuristic> EvaluationSession<T> for MonteCarloSession<T> {
    fn is_multi_threaded(&self) -> bool {
        false
    }

    fn apply_move(&mut self, mv: T::Action) -> Result<(), String> {
        self.root.play(mv)?;

        if self.node.children.is_none() {
            self.node = Node::new(self.root.is_maximizing());
            return Ok(());
        }

        let children = self.node.children.take().unwrap();

        let new_node = children
            .into_iter()
            .find(|a| a.0 == mv)
            .ok_or("move not in children".to_string())?;
        self.node = new_node.1;

        Ok(())
    }

    fn undo_move(&mut self) -> Result<(), String> {
        self.root.undo()?;
        self.node = Node::new(self.root.is_maximizing());

        Ok(())
    }

    fn evaluate(&mut self) -> Result<Vec<(<T as Heuristic>::Action, f32)>, String> {
        let start = Instant::now();
        while Instant::now() - start < self.time {
            self.node.backpropagate(&mut self.root);
        }

        Ok(self
            .node
            .children
            .as_ref()
            .unwrap()
            .into_iter()
            .map(|(m, n)| {
                let sign = if self.node.maximizing { 1.0 } else { -1.0 };
                (*m, sign * n.visits as f32)
            })
            .collect())
    }

    fn get_root(&self) -> &T {
        &self.root
    }
}
