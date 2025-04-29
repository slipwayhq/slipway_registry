use std::sync::LazyLock;

use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::header, web::Path};
use regex::Regex;

pub(crate) static REGISTRY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<publisher>\w+)\.(?<name>\w+)\.(?<version>.+)$").unwrap());

#[get("/components/{reference}.tar")]
async fn redirect_to_github(filename: Path<String>) -> impl Responder {
    let Some(caps) = REGISTRY_REGEX.captures(&filename) else {
        return HttpResponse::BadRequest()
            .body("Invalid filename format. Expected format: <publisher>.<name>.<version>.tar");
    };

    let publisher = &caps["publisher"];
    let name = &caps["name"];
    let version = &caps["version"];

    let publisher_hyphenated = publisher.replace('_', "-");

    let target = format!(
        "https://github.com/{publisher_hyphenated}/slipway_{name}/releases/download/{version}/{publisher}.{name}.{version}.tar",
    );

    println!("Redirecting \"{publisher}.{name}.{version}\" to: {target}");

    HttpResponse::Found()
        .insert_header((header::LOCATION, target))
        .finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(redirect_to_github))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
