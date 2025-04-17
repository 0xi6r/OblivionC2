async fn post_result(
    data: web::Data<AppState>,
    id: web::Path<Uuid>,
    result: web::Json<ResultData>
) -> impl Responder {
    println!("[+] Result from {}: {}", id, result.output);
    HttpResponse::Ok().body("Result received")
}
