use std::{
    collections::HashMap,
    ops::AddAssign,
    sync::{LazyLock, Mutex},
};

use board::BoardData;
use evaluation::session::Session;

use crate::requests::SessionIdentifier;

static CURRENT_ID: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
static USE_CACHE: bool = true;
static MAX_DEPTH: u8 = 5;

#[derive(Clone)]
pub struct SessionData {
    pub session_id: usize,
    pub game_session: Session,
}

impl SessionData {
    pub fn new(data: &BoardData) -> Result<Self, String> {
        let mut handle = CURRENT_ID.lock().unwrap();
        handle.add_assign(1);
        let id = handle.clone();

        let session = Session::new(data, USE_CACHE, MAX_DEPTH)?;

        Ok(Self {
            session_id: id,
            game_session: session,
        })
    }
}

pub struct SessionStore {
    pub sessions: Mutex<HashMap<usize, SessionData>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_session(&self, id: &usize) -> Result<SessionData, String> {
        let handle = self.sessions.lock().unwrap();
        let value = handle.get(id);
        match value {
            Some(v) => Ok(v.clone()),
            None => Err(String::from("The specified session does not exist")),
        }
    }

    pub fn create_new_session(&self, data: &BoardData) -> Result<SessionIdentifier, String> {
        let session_data = SessionData::new(data)?;
        let id = session_data.session_id;

        let mut handle = self.sessions.lock().unwrap();
        handle.insert(session_data.session_id, session_data);

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
