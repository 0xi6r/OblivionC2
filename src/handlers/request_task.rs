async fn get_task(
    data: web::Data<AppState>,
    id: web::Path<Uuid>
) -> impl Responder {
    let tasks = data.tasks.lock().unwrap();
    if let Some(task) = tasks.get(&id.into_inner()) {
        HttpResponse::Ok().json(task)
    } else {
        HttpRequest::Ok().body("No task")
    }
}
