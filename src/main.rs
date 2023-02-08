mod hello;

#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate rocket;

use rgb::RGB8;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Header, Method, ContentType, Status},
    serde::json::Json,
    Request, Response, State,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    name: String,
    color: RGB8,
}

struct UserRecord {
    name: Option<String>,
    r: Option<i64>,
    g: Option<i64>,
    b: Option<i64>,
}

impl From<UserRecord> for User {
    fn from(value: UserRecord) -> Self {
        Self {
            name: value.name.unwrap(),
            color: RGB8 {
                r: value.r.unwrap() as u8,
                g: value.g.unwrap() as u8,
                b: value.b.unwrap() as u8,
            }
        }
    }
}

#[get("/user", format = "json")]
async fn get_users(pool: &State<SqlitePool>) -> Json<Vec<User>> {
    let users: Vec<User> =
        sqlx::query_as!(UserRecord,
                        "SELECT name, r, g, b
                         FROM users
                         INNER JOIN colors ON colors.colorid = users.color")
        .fetch_all(&**pool)
        .await
        .expect("Failed to fetch users")
        .into_iter()
        .map(User::from)
        .collect();

    Json::from(users)
}

#[get("/user/<id>", format = "json")]
async fn get_user(id: u32, pool: &State<SqlitePool>) -> Json<User> {
    let user: User =
        sqlx::query_as!(UserRecord,
                        "SELECT name, r, g, b
                         FROM users
                         INNER JOIN colors ON colors.colorid = users.color
                         WHERE userid = ?",
                        id)
        .fetch_one(&**pool)
        .await
        .expect("Failed to fetch user")
        .into();

    Json::from(user)
}

#[post("/user", format = "json", data = "<user>")]
async fn create_user(pool: &State<SqlitePool>, user: Json<User>) {
    match sqlx::query!(
        "INSERT INTO colors(r, g, b) VALUES(?, ?, ?);",
        user.color.r,
        user.color.g,
        user.color.b
    )
    .execute(&**pool)
    .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("{:?}; color already exists", e),
    };

    let colorid = sqlx::query!(
        "SELECT colorid FROM colors WHERE r = ? AND g = ? AND b = ?;",
        user.color.r,
        user.color.g,
        user.color.b
    )
    .fetch_one(&**pool)
    .await
    .expect("No such color (this should never happen)")
    .colorid
    .unwrap();

    sqlx::query!(
        "INSERT INTO users(name, color) VALUES(?, ?);",
        user.name,
        colorid
    )
    .execute(&**pool)
    .await
    .expect("Failed to add user");
}

#[delete("/user/<id>")]
async fn delete_user(id: u32, pool: &State<SqlitePool>) {
    let query = sqlx::query!("DELETE FROM users WHERE userid = ?", id)
        .execute(&**pool)
        .await
        .expect("Failed to delete user");

    if query.rows_affected() == 0 {
        eprintln!("No rows affected");
        panic!()
    }
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }
    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, DELETE, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));

        if request.method() == Method::Options {
            let body = "";
            response.set_header(ContentType::Plain);
            response.set_sized_body(body.len(), std::io::Cursor::new(body));
            response.set_status(Status::Ok);
        }
    }
}

#[launch]
async fn rocket() -> _ {
    let database_url = dotenv!("DATABASE_URL");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    rocket::build()
        .mount("/hello", routes![hello::index, hello::delay, hello::hello])
        .mount("/api/v1", routes![create_user, get_user, get_users, delete_user])
        .manage(pool)
        .attach(CORS)
}
