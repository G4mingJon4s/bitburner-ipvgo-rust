use requests::{
    SessionBoardState, SessionCreateData, SessionIdentifier, SessionListData, SessionMoveRequest,
    SessionMoveResponse,
};
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Header, Method, Status},
    response::content::RawHtml,
    serde::json::Json,
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
    let session = store.get_session(&id);
    match session {
        Ok(s) => Ok(Json(SessionBoardState::new(&s.board))),
        Err(_) => Err(Status::NotFound),
    }
}

#[put("/session/<id>/move", format = "json", data = "<data>")]
fn put_session_move(
    id: usize,
    data: Json<SessionMoveRequest>,
    store: &State<SessionStore>,
) -> Result<Json<SessionMoveResponse>, Status> {
    let session = store.get_session(&id);
    let mv = data.into_inner().mv;

    match session {
        Ok(mut s) => {
            let result = s.board.make_move_mut(mv);
            match result {
                Ok(_) => Ok(Json(SessionMoveResponse::new(
                    mv,
                    SessionBoardState::new(&s.board),
                ))),
                Err(_) => return Err(Status::NotAcceptable),
            }
        }
        Err(_) => Err(Status::NotFound),
    }
}

#[post("/session", format = "json", data = "<data>")]
fn post_session(
    data: Json<SessionCreateData>,
    store: &State<SessionStore>,
) -> Result<Json<SessionIdentifier>, Status> {
    let creation_data = data.into_inner();
    let created = store.create_new_session(&creation_data.into());
    match created {
        Ok(id) => Ok(Json(id)),
        Err(_) => Err(Status::BadRequest),
    }
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

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(SessionStore::new())
        .attach(CORS)
        .mount(
            "/",
            routes![
                index,
                post_session,
                delete_session,
                get_session_list,
                get_session_state,
                put_session_move
            ],
        )
}
