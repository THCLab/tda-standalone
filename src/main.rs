mod db;
mod models;

use crate::db::TDADatabase;
use uuid::Uuid;

use crate::models::{Event, Message, Token};
use actix_web::http::StatusCode;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

use std::sync::Mutex;

// Server methods.
pub async fn handle_pair(db: web::Data<Mutex<TDADatabase>>) -> impl Responder {
    // Generate new token.
    let new_uuid = Uuid::new_v4();
    let data = db.lock().unwrap();

    // Add new token to db.
    data.add_token(Token {
        uuid: new_uuid.to_string(),
    });
    web::Json(Token {
        uuid: new_uuid.to_string(),
    })
}

async fn handle_message(
    form: web::Json<Event>,
    db: web::Data<Mutex<TDADatabase>>,
) -> impl Responder {
    let client_token = Token {
        uuid: form.uuid.clone(),
    };
    // Authentication.
    let data = db.lock().unwrap();
    if data.check_token(client_token) {
        println!("Token authenticated.");

        return HttpResponse::new(StatusCode::CONTINUE);
    } else {
        return HttpResponse::new(StatusCode::UNAUTHORIZED);
    }
}

// Client methods
fn pair() {
    todo!();
}

fn send_message(recipant: String) {
    todo!();
}
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let data = TDADatabase::new();
    let data = web::Data::new(Mutex::new(data));

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/pair", web::get().to(handle_pair))
            .route("/message", web::post().to(handle_message))
    })
    .bind("127.0.0.1:3030")?
    .run()
    .await?;

    Ok(())
}

mod tests {
    use crate::db::*;
    use crate::models::*;
    #[test]
    fn add_and_get_token() {
        let data = TDADatabase::new();
        data.add_token(Token {
            uuid: "12345".to_string(),
        });
        let t = data.get_token("12345".to_string());
        //println!("Token: {:?}", t.unwrap().uuid);

        assert_eq!("12345".to_string(), t.unwrap().uuid);
    }

    #[test]
    fn check_is_in_db() {
        let data = TDADatabase::new();
        data.add_token(Token {
            uuid: "12345".to_string(),
        });

        let is_12345_there = data.check_token(Token {
            uuid: "12345".to_string(),
        });
        assert!(is_12345_there, true);

        let is_3214_there = data.check_token(Token {
            uuid: "3214".to_string(),
        });
        assert!(is_3214_there, false);
    }
}
