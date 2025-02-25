use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};
use std::usize;

use evaluation::{Evaluator, Heuristic};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tile {
    White,
    Black,
    Dead,
    Free,
}
impl Tile {
    pub fn to_char(&self) -> char {
        match self {
            Tile::White => 'O',
            Tile::Black => 'X',
            Tile::Dead => '#',
            Tile::Free => '.',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'O' => Some(Tile::White),
            'X' => Some(Tile::Black),
            '#' => Some(Tile::Dead),
            '.' => Some(Tile::Free),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Turn {
    White,
    Black,
    None,
}
impl Turn {
    pub fn to_str(&self) -> &'static str {
        match self {
            Turn::White => "White",
            Turn::Black => "Black",
            Turn::None => "None",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "white" => Some(Turn::White),
            "black" => Some(Turn::Black),
            "none" => Some(Turn::None),
            _ => None,
        }
    }
}

impl Turn {
    pub fn next(self) -> Turn {
        match self {
            Turn::White => Turn::Black,
            Turn::Black => Turn::White,
            Turn::None => Turn::None,
        }
    }

    pub fn get_placing_color(self) -> Option<Tile> {
        match self {
            Turn::Black => Some(Tile::Black),
            Turn::White => Some(Tile::White),
            Turn::None => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Move {
    Place(usize),
    Coords((usize, usize)),
    Pass,
}

#[derive(Clone, Debug)]
pub struct Chain {
    pub id: usize,
    pub tile: Tile,
    pub positions: HashSet<usize>,
    pub liberties: HashSet<usize>,
    pub adjacent: HashSet<usize>,
}

#[derive(Clone, Debug)]
pub enum Mod {
    Assignment((usize, usize)),
    Addition(usize),
    Change((usize, Chain)),
}

#[derive(Clone, Debug)]
pub struct MoveChange {
    pub action: Move,
    pub previous_turn: Turn,
    pub board_hash: u64,

    pub mods: Vec<Mod>,
}

pub struct Board {
    pub size: u8,
    pub komi: f32,
    pub turn: Turn,
    pub pos_to_chain: Vec<Option<usize>>,
    pub chains: Vec<Option<Chain>>,
    pub history: Vec<MoveChange>,
}

impl Hash for Board {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for p in 0..self.pos_to_chain.len() {
            let t = self.get_tile(p);
            t.hash(state);
        }
    }
}

impl Clone for Board {
    fn clone(&self) -> Self {
        Self {
            size: self.size,
            komi: self.komi,
            turn: self.turn,
            chains: self.chains.clone(),
            history: self.history.clone(),
            pos_to_chain: self.pos_to_chain.clone(),
        }
    }
}

impl Board {
    pub fn new(size: u8, starting_turn: Turn, komi: f32) -> Self {
        let total = (size as usize).pow(2);
        Self {
            size,
            komi,
            turn: starting_turn,
            pos_to_chain: vec![None; total],
            chains: Vec::new(),
            history: Vec::new(),
        }
    }

    pub fn to_coords(&self, pos: usize) -> (usize, usize) {
        (pos / self.size as usize, pos % self.size as usize)
    }

    pub fn to_pos(&self, x: usize, y: usize) -> usize {
        x * self.size as usize + y
    }

    fn neighbors(&self, pos: usize) -> Vec<usize> {
        let (x, y) = self.to_coords(pos);
        let mut nbrs = Vec::new();
        if x > 0 {
            nbrs.push(self.to_pos(x - 1, y));
        }
        if x + 1 < self.size as usize {
            nbrs.push(self.to_pos(x + 1, y));
        }
        if y > 0 {
            nbrs.push(self.to_pos(x, y - 1));
        }
        if y + 1 < self.size as usize {
            nbrs.push(self.to_pos(x, y + 1));
        }
        nbrs
    }

    pub fn compute_board_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_tile(&self, pos: usize) -> Tile {
        match self.pos_to_chain[pos] {
            None => Tile::Dead,
            Some(id) => self.chains[id].as_ref().unwrap().tile,
        }
    }

    fn floodfill<F: Fn(usize) -> Tile, N: Fn(usize) -> Vec<usize>>(
        tile: F,
        neighbors: N,
        pos: usize,
        id: usize,
    ) -> Chain {
        let c = tile(pos);

        let mut positions: HashSet<usize> = HashSet::new();
        let mut adjacent: HashSet<usize> = HashSet::new();
        let mut liberties: HashSet<usize> = HashSet::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(pos);
        while !queue.is_empty() {
            let cur = queue.pop_front().unwrap();
            if positions.contains(&cur) {
                continue;
            }
            positions.insert(cur);
            for n in neighbors(cur) {
                let t = tile(n);
                if t == c {
                    queue.push_back(n);
                    continue;
                }
                if t == Tile::Free {
                    liberties.insert(n);
                }
                adjacent.insert(n);
            }
        }

        Chain {
            id,
            tile: c,
            positions,
            adjacent,
            liberties,
        }
    }

    pub fn from_rep(rep: String, size: u8, starting_turn: Turn, komi: f32) -> Result<Self, String> {
        if rep.len() != (size as usize).pow(2) {
            return Err("Invalid shape".to_string());
        }

        let mut board = Self::new(size, starting_turn, komi);

        let mut seen: HashSet<usize> = HashSet::new();
        let mut rep_tiles: Vec<Tile> = Vec::with_capacity((size as usize).pow(2));
        for t in rep.chars() {
            let tile = Tile::from_char(t).ok_or_else(|| "Invalid char".to_string())?;
            rep_tiles.push(tile);
        }

        for (p, c) in rep_tiles.iter().enumerate() {
            if seen.contains(&p) {
                continue;
            }

            if *c == Tile::Dead {
                continue;
            }

            let id = board.chains.len();
            let new_chain = Board::floodfill(|p| rep_tiles[p], |p| board.neighbors(p), p, id);

            seen.extend(new_chain.positions.iter());
            for p in new_chain.positions.iter() {
                board.pos_to_chain[*p] = Some(id);
            }

            board.chains.push(Some(new_chain))
        }

        Ok(board)
    }

    pub fn get_rep(&self) -> String {
        (0..(self.size as usize).pow(2))
            .map(|p| self.get_tile(p).to_char())
            .collect()
    }

    fn rollback_change(&mut self, change: MoveChange) {
        self.turn = change.previous_turn;

        for m in change.mods.into_iter().rev() {
            match m {
                Mod::Addition(a) => self.chains.truncate(a),
                Mod::Assignment((p, o)) => self.pos_to_chain[p] = Some(o),
                Mod::Change((a, c)) => self.chains[a] = Some(c),
            }
        }

        debug_assert_eq!(change.board_hash, self.compute_board_hash());
    }

    fn get_chain(&self, pos: usize) -> Option<(usize, &Chain)> {
        if let Some(id) = self.pos_to_chain[pos] {
            return Some((id, self.chains[id].as_ref().unwrap()));
        }
        None
    }

    fn get_chain_mut(&mut self, pos: usize) -> Option<(usize, &mut Chain)> {
        if let Some(id) = self.pos_to_chain[pos] {
            return Some((id, self.chains[id].as_mut().unwrap()));
        }
        None
    }

    pub fn apply_move(&mut self, mut action: Move) -> Result<(), String> {
        if self.turn == Turn::None {
            return Err(format!("Game is over ({:?})", action));
        }

        let mut change = MoveChange {
            action,
            previous_turn: self.turn,
            board_hash: self.compute_board_hash(),
            mods: Vec::new(),
        };

        if let Move::Coords((x, y)) = action {
            action = Move::Place(self.to_pos(x, y));
        }

        if let Move::Place(pos) = action {
            if self.get_tile(pos) != Tile::Free {
                return Err(format!("Tile is occupied ({:?})", action));
            }

            let neighbors = self
                .neighbors(pos)
                .into_iter()
                .filter(|&p| self.get_tile(p) != Tile::Dead)
                .collect::<Vec<_>>();

            let friendly_color = self.turn.get_placing_color().unwrap();
            let opponent_color = self.turn.next().get_placing_color().unwrap();

            let initial_free_neighbors = neighbors
                .iter()
                .filter(|&&t| self.get_tile(t) == Tile::Free)
                .copied()
                .collect::<Vec<_>>();
            for &neighbor in neighbors.iter() {
                if self.get_tile(neighbor) != opponent_color {
                    continue;
                }

                let (chain_id, chain) = self.get_chain_mut(neighbor).unwrap();
                change.mods.push(Mod::Change((chain_id, chain.clone())));

                chain.liberties.remove(&pos);
                if chain.liberties.len() > 0 {
                    continue;
                }

                chain.tile = Tile::Free;
                let adjacents = chain.adjacent.iter().copied().collect::<Vec<_>>();
                for adj in adjacents {
                    if self.pos_to_chain[adj].is_none() {
                        continue;
                    }
                    if adj == pos {
                        continue;
                    }

                    let free_neighbors = self
                        .neighbors(adj)
                        .into_iter()
                        .filter(|&p| self.get_tile(p) == Tile::Free)
                        .collect::<Vec<_>>();

                    let (id, adj_chain) = self.get_chain_mut(adj).unwrap();

                    if id == chain_id {
                        continue;
                    }

                    debug_assert!(
                        adj_chain.tile != Tile::Free,
                        "No liberties but adj chain is free ({:?} @{:?})",
                        action,
                        adj
                    );

                    change.mods.push(Mod::Change((id, adj_chain.clone())));
                    adj_chain.liberties.extend(free_neighbors.iter());
                }
            }

            let mut friendly_chains: HashSet<usize> = HashSet::new();
            friendly_chains.extend(
                neighbors
                    .iter()
                    .filter_map(|&n| self.pos_to_chain[n])
                    .filter(|&id| self.chains[id].as_ref().unwrap().tile == friendly_color),
            );
            let free_neighbors = neighbors
                .iter()
                .filter(|&&p| self.get_tile(p) == Tile::Free)
                .copied()
                .collect::<Vec<_>>();
            let non_friendly_neighbors = neighbors
                .iter()
                .filter(|&&p| self.get_tile(p) != friendly_color)
                .copied()
                .collect::<Vec<_>>();

            let pos_id = self.pos_to_chain[pos].unwrap();
            change.mods.push(Mod::Assignment((pos, pos_id)));

            match friendly_chains.iter().collect::<Vec<_>>().as_slice() {
                [] => {
                    let new_id = self.chains.len();
                    let mut new_chain = Chain {
                        id: new_id,
                        tile: friendly_color,
                        positions: HashSet::new(),
                        adjacent: HashSet::new(),
                        liberties: HashSet::new(),
                    };

                    new_chain.positions.insert(pos);
                    new_chain.adjacent.extend(neighbors.iter());
                    new_chain.liberties.extend(free_neighbors.iter());

                    change.mods.push(Mod::Addition(new_id));
                    self.pos_to_chain[pos] = Some(new_id);
                    self.chains.push(Some(new_chain));
                }
                [&one] => {
                    let chain = self.chains[one].as_mut().unwrap();
                    change.mods.push(Mod::Change((one, chain.clone())));

                    chain.liberties.remove(&pos);
                    chain.adjacent.remove(&pos);
                    chain.positions.insert(pos);
                    self.pos_to_chain[pos] = Some(chain.id);
                    chain.adjacent.extend(non_friendly_neighbors.iter());
                    chain.liberties.extend(free_neighbors.iter());
                }
                many => {
                    let mut positions: HashSet<usize> = HashSet::new();
                    let mut adjacents: HashSet<usize> = HashSet::new();
                    let mut liberties: HashSet<usize> = HashSet::new();

                    for &&other in &many[1..] {
                        let chain = self.chains[other].as_ref().unwrap();
                        positions.extend(chain.positions.iter());
                        adjacents.extend(chain.adjacent.iter());
                        liberties.extend(chain.liberties.iter());

                        change.mods.push(Mod::Change((other, chain.clone())));
                        self.chains[other] = None;
                    }

                    adjacents.extend(non_friendly_neighbors.iter());
                    liberties.extend(free_neighbors.iter());

                    let survivor = self.chains[*many[0]].as_mut().unwrap();
                    change.mods.push(Mod::Change((*many[0], survivor.clone())));

                    for &p in positions.iter() {
                        change
                            .mods
                            .push(Mod::Assignment((p, self.pos_to_chain[p].unwrap())));
                        self.pos_to_chain[p] = Some(survivor.id);
                    }

                    survivor.positions.extend(positions.iter());
                    survivor.adjacent.extend(adjacents.iter());
                    survivor.liberties.extend(liberties.iter());

                    survivor.positions.insert(pos);
                    survivor.adjacent.remove(&pos);
                    survivor.liberties.remove(&pos);
                    self.pos_to_chain[pos] = Some(survivor.id);
                }
            }

            let prev_pos_chain = self.chains[pos_id].as_ref().unwrap();
            change
                .mods
                .push(Mod::Change((pos_id, prev_pos_chain.clone())));

            if initial_free_neighbors.len() >= 2 {
                let flood_filled = neighbors
                    .iter()
                    .map(|&n| {
                        Board::floodfill(|t| self.get_tile(t), |n| self.neighbors(n), n, usize::MAX)
                    })
                    .collect::<Vec<_>>();

                self.chains[pos_id] = None;

                let mut filtered: Vec<Chain> = Vec::new();
                for new_chain in flood_filled {
                    // Case 1: There is already a free chain from capturing opponent chains
                    if self.chains.iter().any(|c| {
                        c.is_some()
                            && c.as_ref()
                                .unwrap()
                                .positions
                                .is_subset(&new_chain.positions)
                    }) {
                        continue;
                    }

                    // Case 2: The new chain covers multiple neighbors
                    if filtered
                        .iter()
                        .any(|c| c.positions.is_subset(&new_chain.positions))
                    {
                        continue;
                    }

                    filtered.push(new_chain);
                }

                for mut new_chain in filtered {
                    let id = self.chains.len();
                    new_chain.id = id;
                    for &p in new_chain.positions.iter() {
                        change
                            .mods
                            .push(Mod::Assignment((p, self.pos_to_chain[p].unwrap())));
                        self.pos_to_chain[p] = Some(id);
                    }
                    change.mods.push(Mod::Addition(id));
                    self.chains.push(Some(new_chain));
                }
            } else if initial_free_neighbors.len() == 1 {
                let new_chain = Board::floodfill(
                    |t| self.get_tile(t),
                    |n| self.neighbors(n),
                    initial_free_neighbors[0],
                    pos_id,
                );

                self.chains[pos_id] = Some(new_chain);
            } else {
                self.chains[pos_id] = None;
            }
        }

        if action == Move::Pass
            && self.history.len() > 0
            && self.history.iter().last().unwrap().action == Move::Pass
        {
            self.turn = Turn::None;
        } else {
            self.turn = self.turn.next();
        }

        let hash = self.compute_board_hash();
        if self.history.len() > 0
            && self
                .history
                .iter()
                .any(|c| c.action != Move::Pass && c.board_hash == hash)
        {
            self.rollback_change(change);
            return Err("Repetition".to_string());
        }
        self.history.push(change);

        Ok(())
    }

    pub fn undo_move(&mut self) -> Result<(), String> {
        if let Some(change) = self.history.pop() {
            self.rollback_change(change);
            Ok(())
        } else {
            Err("No move to undo".to_string())
        }
    }
}

impl Heuristic for Board {
    type Action = Move;

    fn calculate_heuristic(&self) -> f32 {
        let mut score = -self.komi;

        for c in self.chains.iter().filter_map(|a| a.as_ref()) {
            if c.tile == Tile::Free {
                let tile = c.adjacent.iter().find_map(|&a| match self.get_tile(a) {
                    Tile::Dead => None,
                    Tile::Free => None,
                    a => Some(a),
                });
                if tile.is_some()
                    && c.adjacent.iter().all(|&a| {
                        let t = self.get_tile(a);
                        t == Tile::Dead || t == tile.unwrap()
                    })
                {
                    match tile.unwrap() {
                        Tile::Black => score += c.positions.len() as f32,
                        Tile::White => score -= c.positions.len() as f32,
                        _ => panic!("not possible"),
                    }
                }
                continue;
            }

            match c.tile {
                Tile::Black => score += c.positions.len() as f32,
                Tile::White => score -= c.positions.len() as f32,
                _ => panic!("not possible"),
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

    fn get_hash(&self) -> u64 {
        self.compute_board_hash()
    }

    fn moves(&self) -> impl Iterator<Item = Self::Action> {
        let mut possible_moves = vec![Move::Pass];

        let friendly_color = self.turn.get_placing_color().unwrap();

        for chain in self.chains.iter().filter_map(|a| a.as_ref()) {
            if chain.tile != Tile::Free {
                continue;
            }
            if chain.positions.len() >= 2 {
                possible_moves.extend(chain.positions.iter().map(|&p| Move::Place(p)));
                continue;
            }

            let &pos = chain.positions.iter().nth(0).unwrap();
            let can_place = self
                .neighbors(pos)
                .iter()
                .filter(|&&n| self.pos_to_chain[n].is_some())
                .any(|&n| {
                    let (_, n_chain) = self.get_chain(n).unwrap();
                    if n_chain.tile == friendly_color && n_chain.liberties.len() >= 2 {
                        return true;
                    }
                    n_chain.tile != friendly_color
                        && n_chain.liberties.len() == 1
                        && n_chain.liberties.contains(&pos)
                });
            if can_place {
                possible_moves.push(Move::Place(pos));
            }
        }

        possible_moves.into_iter()
    }

    fn play(&mut self, mv: Self::Action) -> Result<(), String> {
        self.apply_move(mv)
    }

    fn undo(&mut self) -> Result<(), String> {
        self.undo_move()
    }

    fn evaluate(&self, e: &Evaluator, depth: u8) -> (Duration, Vec<(Self::Action, f32)>) {
        let start = Instant::now();
        let moves = self.moves().collect::<Vec<_>>();

        let results = e.evaluate_all(moves, depth, |&mv| {
            let mut copy = self.clone();
            match copy.apply_move(mv) {
                Ok(_) => Some(copy),
                Err(_) => None,
            }
        });

        let end = Instant::now();
        (end - start, results)
    }
}
