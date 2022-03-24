#![feature(decl_macro)]
#![feature(never_type)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rocket_contrib;
extern crate itertools;
extern crate rcir;
use rocket::response::Redirect;

mod schema;

use diesel::SqliteConnection;
use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{self, Form, FromRequest, Request};
use rocket::Rocket;
use rocket_contrib::{json::Json, templates::Template};

use schema::{Ballot, Item, NewUser, Vote};

#[database("sqlite_database")]
pub struct DbConn(SqliteConnection);

#[derive(Debug, Serialize)]
struct Context {
    winner: Vec<Item>,
    items: Vec<(Item, bool)>,
}

impl Context {
    pub fn new(conn: &DbConn) -> Context {
        Context {
            winner: Vote::run_election(conn).into_iter().collect(),
            items: Vec::new(), // not used if not logged in
        }
    }

    pub fn for_user(user: Auth, conn: &DbConn) -> Context {
        let winner = Vote::run_election(conn);
        Context { 
            winner: Vote::run_election(conn).into_iter().collect(),
            items: Item::for_user(user.0, conn),
        }
    }
}

#[derive(Debug)]
struct Auth(i32);

impl<'a, 'r> FromRequest<'a, 'r> for Auth {
    type Error = !;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Auth, !> {
        request
            .cookies()
            .get("user_id")
            .and_then(|cookie| cookie.value().parse().ok())
            .map(|id| Auth(id))
            .or_forward(())
    }
}

#[post("/login", data = "<input>")]
fn login(mut cookies: Cookies, input: Form<NewUser>, conn: DbConn) -> Redirect {
    let user = input.into_inner();
    if user.username.is_empty() {
        Redirect::to("/")
    } else {
        let u = user.login(&conn);
        cookies.add(Cookie::new("user_id", u.id.to_string()));
        //TODO set session
        Redirect::to("/")
    }
}


#[post("/vote", data = "<ballot>")]
fn vote(ballot: Json<Ballot>, user: Auth, conn: DbConn) -> &'static str {
    Vote::save_ballot(user.0,ballot.into_inner(), &conn);
    "voted"
}

#[get("/")]
fn votes(user: Auth, conn: DbConn) -> Template {
    Template::render(
        "vote",
        Context::for_user(user,&conn),
    )
}

#[get("/", rank = 2)]
fn index(conn: DbConn) -> Template {
    Template::render(
        "index",
        Context::new(&conn),
    )
}

fn rocket() -> (Rocket, Option<DbConn>) {
    let rocket = rocket::ignite()
        .attach(DbConn::fairing())
        .attach(Template::fairing())
        .mount("/", routes![index, login, votes, vote]);

    let conn = match cfg!(test) {
        true => DbConn::get_one(&rocket),
        false => None,
    };

    (rocket, conn)
}

fn main() {
    rocket().0.launch();
}