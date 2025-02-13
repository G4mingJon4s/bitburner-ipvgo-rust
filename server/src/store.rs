use std::{
    collections::HashMap,
    ops::AddAssign,
    sync::{LazyLock, Mutex},
};

use board::{Board, BoardData};

use crate::requests::SessionIdentifier;

static CURRENT_ID: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

#[derive(Clone)]
pub struct Session {
    pub session_id: usize,
    pub board: Board,
}

impl Session {
    pub fn new(data: &BoardData) -> Result<Self, String> {
        let mut handle = CURRENT_ID.lock().unwrap();
        handle.add_assign(1);
        let id = handle.clone();

        let board = Board::from(data)?;

        Ok(Self {
            session_id: id,
            board,
        })
    }
}

pub struct SessionStore {
    pub sessions: Mutex<HashMap<usize, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
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

    pub fn create_new_session(&self, data: &BoardData) -> Result<SessionIdentifier, String> {
        let session = Session::new(data)?;
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
