// implant check in

pub async fn beacon(
    data: web::Data<AppState>,
    beacon: web::Json<Beacon>
) -> impl Responder {
    let mut beacons = data.beacons.lock().unwrap();
    beacons.insert(beacon.id, beacon.into_inner());
    HttpResponse::Ok().body("Beacon received")
}
