use board::{
    util::{Move, Turn},
    BoardData,
};
use evaluation::{evaluation::Heuristic, session::Session};
use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SessionIdentifier {
    pub session_id: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SessionCreateData {
    pub turn: Turn,
    pub size: u8,
    pub rep: String,
    pub komi: f32,
}

impl Into<BoardData> for SessionCreateData {
    fn into(self) -> BoardData {
        BoardData {
            turn: self.turn,
            komi: self.komi,
            rep: self.rep,
            size: self.size,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SessionStateData {
    pub turn: Turn,
    pub size: u8,
    pub rep: String,

    pub komi: f32,
    pub current_score: f32,
}

impl SessionStateData {
    pub fn new(session: &Session) -> Self {
        let board = session.get_board();
        Self {
            size: board.size,
            turn: board.turn,
            komi: board.komi,
            rep: board.get_board_state(),
            current_score: board.calculate(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SessionMoveRequest {
    pub mv: Move,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SessionMoveResponse {
    pub mv: Move,
    pub state: SessionStateData,
}

impl SessionMoveResponse {
    pub fn new(mv: Move, state: SessionStateData) -> Self {
        Self { mv, state }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SessionListData {
    pub sessions: Vec<usize>,
}
