use active_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::{serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/*
    defining core types
    */
    
#[derive(serialize, Deserialize, Clone)]
struct Beacon {
    id: Uuid,
    hostname: String,
    last_seen: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    command: String,
}

#[derive(Serialize, Deseriaize, Clone)]
struct ResultData{
    output: String,
}

fn main() {
    println!("Hello, world!");
}
