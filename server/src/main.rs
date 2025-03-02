use std::{env::args, time::Duration};

use board::Move;
use evaluation::{
    alphabeta::{AlphaBeta, CacheOption},
    montecarlo::MonteCarlo,
    AnyEvaluator, Evaluator,
};
use requests::{
    SessionBoardState, SessionCreateData, SessionEvaluationData, SessionIdentifier,
    SessionListData, SessionMoveRequest, SessionMoveResponse, SessionUndoResponse,
};
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Header, Method, Status},
    response::content::RawHtml,
    serde::json::Json,
    tokio::{task::spawn_blocking, time::Instant},
    Request, Response, State,
};
use store::SessionStore;

#[macro_use]
extern crate rocket;

mod requests;
mod store;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "CORS Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        if req.method() == Method::Options {
            res.set_status(Status::NoContent);
        }
        res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        res.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE",
        ));
        res.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        ));
    }
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml("<h1>Hello World!</h1>")
}

#[get("/session/<id>/state")]
fn get_session_state(
    id: usize,
    store: &State<SessionStore>,
) -> Result<Json<SessionBoardState>, Status> {
    let session = store.get_session(&id).map_err(|_| Status::NotFound)?;
    Ok(Json(SessionBoardState::new(&session.board)))
}

#[put("/session/<id>/move", format = "json", data = "<data>")]
fn put_session_move(
    id: usize,
    data: Json<SessionMoveRequest>,
    store: &State<SessionStore>,
) -> Result<Json<SessionMoveResponse>, Status> {
    let mut session = store.get_session(&id).map_err(|_| Status::NotFound)?;
    let mv = data.into_inner().mv;

    session.apply_move(mv).map_err(|e| {
        println!("Move provided is not valid: {}", e);
        Status::NotAcceptable
    })?;
    store.update_session(id, session.clone());

    Ok(Json(SessionMoveResponse::new(
        mv,
        SessionBoardState::new(&session.board),
    )))
}

#[put("/session/<id>/undo")]
fn put_session_undo(
    id: usize,
    store: &State<SessionStore>,
) -> Result<Json<SessionUndoResponse>, Status> {
    let mut session = store.get_session(&id).map_err(|_| Status::NotFound)?;

    session.undo_move().map_err(|e| {
        println!("Undo is not valid: {}", e);
        Status::NotAcceptable
    })?;
    store.update_session(id, session.clone());

    Ok(Json(SessionUndoResponse {
        state: SessionBoardState::new(&session.board),
    }))
}

#[get("/session/<id>/evaluation")]
async fn get_session_evaluation(
    id: usize,
    store: &State<SessionStore>,
) -> Result<Json<SessionEvaluationData>, Status> {
    let mut session = store.get_session(&id).map_err(|_| Status::NotFound)?;

    if let Some(cache) = session.evaluation_cache {
        return Ok(Json(SessionEvaluationData {
            time: cache.0,
            moves: cache.1,
        }));
    }

    let e = store.evaluator.clone();
    let mut board = session.board.clone();

    let start = Instant::now();
    let result = spawn_blocking(move || {
        let e = e.lock().unwrap();
        e.evaluate(&mut board)
    })
    .await
    .map_err(|_| Status::InternalServerError)?;
    let end = Instant::now();

    let duration = end - start;
    let moves = result
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|m| {
            (
                match m.0 {
                    Move::Place(p) => Move::Coords(session.board.to_coords(p)),
                    a => a,
                },
                m.1,
            )
        })
        .collect::<Vec<_>>();
    session.evaluation_cache = Some((duration, moves.clone()));

    Ok(Json(SessionEvaluationData {
        time: duration,
        moves,
    }))
}

#[get("/session/<id>/error")]
fn get_session_error(id: usize, store: &State<SessionStore>) -> Result<String, Status> {
    let session = store.get_session(&id).map_err(|_| Status::NotFound)?;
    let mut out = String::new();

    let board = session.board;
    out += format!("Requested error information:\n").as_str();
    out += board
        .get_rep()
        .char_indices()
        .fold(String::new(), |mut a, (i, c)| {
            if i > 0 && (i % board.size as usize) == 0 {
                a.push('\n');
            }
            a.push(c);
            a
        })
        .as_str();
    out.push('\n');
    out.push('\n');
    for (i, c) in board.chains.iter().enumerate() {
        out += format!(" #{i}: {:?}\n", c).as_str();
    }

    out.push('\n');
    for h in board.history.iter() {
        out += format!("{:?}", h.action).as_str();
    }

    out.push('\n');
    for (p, id) in board.pos_to_chain.iter().enumerate() {
        out += format!("P{}: {:?}\n", p, id).as_str();
    }

    Ok(out)
}

#[post("/session", format = "json", data = "<data>")]
fn post_session(
    data: Json<SessionCreateData>,
    store: &State<SessionStore>,
) -> Result<Json<SessionIdentifier>, Status> {
    let creation_data = data.into_inner();
    let created = store
        .create_new_session(&creation_data.into())
        .map_err(|_| Status::BadRequest)?;
    Ok(Json(created))
}

#[get("/session")]
fn get_session_list(store: &State<SessionStore>) -> Json<SessionListData> {
    let handle = store.sessions.lock().unwrap();
    let sessions = handle.keys().map(|k| k.clone()).collect::<Vec<_>>();

    Json(SessionListData { sessions })
}

#[delete("/session/<id>")]
fn delete_session(id: usize, store: &State<SessionStore>) -> Status {
    match store.delete_session(&id) {
        Ok(_) => Status::Ok,
        Err(_) => Status::NotFound,
    }
}

#[catch(404)]
fn not_found() -> RawHtml<&'static str> {
    RawHtml("<h1>Not found!</h1>")
}

#[launch]
fn rocket() -> _ {
    let arg_list = args().collect::<Vec<_>>();
    if arg_list.len() != 2 {
        panic!("No algorithm provided. Got {:?}", arg_list);
    }

    let ev: AnyEvaluator = match arg_list[1].to_lowercase().trim() {
        "alpha-beta" => {
            AnyEvaluator::AlphaBeta(AlphaBeta::new(6, CacheOption::Capacity(300_000_000)))
        }
        "monte-carlo" => AnyEvaluator::MonteCarlo(MonteCarlo::new(Duration::from_secs(4))),
        any => panic!("Invalid algorithm '{}'", any),
    };

    if ev.is_multi_threaded() {
        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build_global()
            .unwrap();
    }

    rocket::build()
        .manage(SessionStore::new(ev))
        .attach(CORS)
        .register("/", catchers![not_found])
        .mount(
            "/",
            routes![
                index,
                post_session,
                delete_session,
                get_session_list,
                get_session_state,
                get_session_evaluation,
                get_session_error,
                put_session_move,
                put_session_undo,
            ],
        )
}
