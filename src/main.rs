#[macro_use] extern crate rocket;

use rocket::data::{Data, ToByteUnit};
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::response::content;
use rocket::{Build, Rocket, State};
use serde::{Serialize, Deserialize};
use sqlx::SqlitePool;
use tokio::io::AsyncReadExt;

//create a paste struct
#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct Paste {
    id: String,
    content: String
}

async fn run_migrations(rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
    // make a connection to the db
    let pool = SqlitePool::connect("sqlite://pastes.db").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    
    // pool is past to all state handlers 
    Ok(rocket.manage(pool))
}

// create a dynamic route
#[get("/pastes/<id>")]
async fn show_paste(id: String, db: &State<SqlitePool>) -> Option<content::RawText<String>> {
    // retrive the content from the db
    let paste = 
        sqlx::query_as::<_, Paste>("SELECT * FROM pastes WHERE id = ?")
            .bind(&id)
            .fetch_one(db.inner())
            .await
            .ok()?;

    Some(content::RawText(paste.content))
}

// create a post request
#[post("/upload", data = "<paste>")]
async fn upload(paste: Data<'_>, db: &State<SqlitePool>) -> Result<String, Status> {
    // return an id with 8 unique characters
    let id = nanoid::nanoid!(8);
    
    let mut content = String::new();
    let mut stream = paste.open(32.kibibytes());

    stream.read_to_string(&mut content).await.map_err(|_| Status::BadRequest)?;  

    // execute query
    sqlx::query("INSERT INTO pastes (id, content) VALUES (?, ?)")
        .bind(&id)
        .bind(&content)
        .execute(db.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(format!("http://127.0.0.1:8000/pastes/{}", id))
}

// launch rocket
#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
        .mount("/", routes![show_paste, upload])
}