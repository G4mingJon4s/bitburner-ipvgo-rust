use board::{util::Move, Board};
use evaluation::Heuristic;
use requests::{
    SessionBoardState, SessionCreateData, SessionEvaluationData, SessionIdentifier,
    SessionListData, SessionMoveRequest, SessionMoveResponse,
};
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Header, Method, Status},
    response::content::RawHtml,
    serde::json::Json,
    tokio::task::spawn_blocking,
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

    session.make_move(mv).map_err(|_| Status::NotAcceptable)?;
    store.update_session(id, session.clone());

    Ok(Json(SessionMoveResponse::new(
        mv,
        SessionBoardState::new(&session.board),
    )))
}

#[get("/session/<id>/evaluation")]
async fn get_session_evaluation(
    id: usize,
    store: &State<SessionStore>,
) -> Result<Json<SessionEvaluationData>, Status> {
    let mut session = store.get_session(&id).map_err(|_| Status::NotFound)?;

    let e = store.evaluator.clone();
    let depth = store.depth;

    let board = session.board.clone();
    let result = match session.evaluation_cache {
        Some(cache) => cache,
        None => spawn_blocking(move || {
            let e = e.lock().unwrap();
            board.evaluate(&e, depth)
        })
        .await
        .map_err(|_| Status::InternalServerError)?,
    };

    let duration = result.0;
    let moves = result.1;
    session.evaluation_cache = Some((duration, moves.clone()));

    let board = session.board.clone();
    let mapped_eval = moves.iter().map(|(m, e)| {
        (
            match *m {
                Move::Pos(a) => Move::Coords(Board::to_coords(a, board.size)),
                a => a,
            },
            (&board).make_move(*m),
            *e,
        )
    });

    let mut eval: Vec<(Move, Board, f32)> = Vec::new();
    for (m, b, e) in mapped_eval {
        eval.push((m, b.map_err(|_| Status::InternalServerError)?, e));
    }

    Ok(Json(SessionEvaluationData::new((duration, eval))))
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
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()
        .unwrap();

    rocket::build()
        .manage(SessionStore::new())
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
                put_session_move,
            ],
        )
}
