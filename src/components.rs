use actix_web::{HttpResponse, Responder, get, http::header, web::Path};
use once_cell::sync::Lazy;
use regex::Regex;

pub static REGISTRY_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<publisher>\w+)\.(?P<name>\w+)\.(?P<version>.+)$").unwrap());

#[get("/components/{reference}.tar")]
pub(super) async fn redirect_to_github(filename: Path<String>) -> impl Responder {
    let Some(caps) = REGISTRY_REGEX.captures(&filename) else {
        super::DOWNLOAD_ERRORS
            .with_label_values(&["invalid_format"])
            .inc();
        return HttpResponse::BadRequest()
            .body("Invalid filename format. Expected format: <publisher>.<name>.<version>.tar");
    };

    let publisher = &caps["publisher"];
    let name = &caps["name"];
    let version = &caps["version"];

    super::DOWNLOAD_COUNTER
        .with_label_values(&[publisher, name, version])
        .inc();

    let publisher_hyphenated = publisher.replace('_', "-");

    let target = format!(
        "https://github.com/{publisher_hyphenated}/slipway_{name}/releases/download/{version}/{publisher}.{name}.{version}.tar",
    );

    println!("Redirecting \"{publisher}.{name}.{version}\" to: {target}");

    HttpResponse::Found()
        .insert_header((header::LOCATION, target))
        .finish()
}
