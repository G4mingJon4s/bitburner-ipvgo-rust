use std::{
    collections::HashMap,
    ops::AddAssign,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use board::{Board, Move, Turn};
use evaluation::{AnyEvaluationSession, EvaluationSession};

use crate::requests::SessionIdentifier;

static CURRENT_ID: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

#[derive(Clone)]
pub struct Session {
    pub session_id: usize,
    pub evaluation_cache: Option<(Duration, Vec<(Move, f32)>)>,
    pub evaluation_session: AnyEvaluationSession<Board>,
}

impl Session {
    pub fn board(&self) -> &Board {
        &self.evaluation_session.get_root()
    }
}

pub struct BoardData {
    pub rep: String,
    pub size: u8,
    pub turn: Turn,
    pub komi: f32,
}

impl Session {
    pub fn new(
        data: &BoardData,
        session_fn: impl Fn(Board) -> AnyEvaluationSession<Board>,
    ) -> Result<Self, String> {
        let mut handle = CURRENT_ID.lock().unwrap();
        handle.add_assign(1);
        let id = handle.clone();

        let board = Board::from_rep(data.rep.clone(), data.size, data.turn, data.komi)?;

        Ok(Self {
            session_id: id,
            evaluation_cache: None,
            evaluation_session: session_fn(board),
        })
    }
}

impl Session {
    pub fn apply_move(&mut self, mv: Move) -> Result<(), String> {
        self.evaluation_session.apply_move(mv)?;
        self.evaluation_cache = None;
        Ok(())
    }

    pub fn undo_move(&mut self) -> Result<(), String> {
        self.evaluation_session.undo_move()?;
        self.evaluation_cache = None;
        Ok(())
    }
}

pub struct SessionStore {
    pub sessions: Mutex<HashMap<usize, Session>>,
    pub session_fn: Box<dyn Send + Sync + 'static + Fn(Board) -> AnyEvaluationSession<Board>>,
}

impl SessionStore {
    pub fn new(
        session_fn: impl Send + Sync + 'static + Fn(Board) -> AnyEvaluationSession<Board>,
    ) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            session_fn: Box::new(session_fn),
        }
    }

    pub fn get_session(&self, id: &usize) -> Result<Session, String> {
        let handle = self.sessions.lock().unwrap();
        let value = handle.get(id);
        match value {
            Some(v) => Ok(v.clone()),
            None => Err(String::from("The specified session does not exist")),
        }
    }

    pub fn update_session(&self, id: usize, session: Session) {
        let mut handle = self.sessions.lock().unwrap();
        handle.insert(id, session);
    }

    pub fn create_new_session(&self, data: &BoardData) -> Result<SessionIdentifier, String> {
        let session = Session::new(data, self.session_fn.as_ref())?;
        let id = session.session_id;

        let mut handle = self.sessions.lock().unwrap();
        handle.insert(session.session_id, session);

        Ok(SessionIdentifier { session_id: id })
    }

    pub fn delete_session(&self, id: &usize) -> Result<(), String> {
        let mut handle = self.sessions.lock().unwrap();
        if let Some(_) = handle.get(&id) {
            handle.remove(&id);
            return Ok(());
        }

        Err(String::from("The specified session does not exist"))
    }
}
