use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::header, web::Path};
use regex::Regex;

pub(crate) static REGISTRY_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^(?<publisher>\w+)\.(?<name>\w+)\.(?<version>.+)$").unwrap()
});

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

// Configure and return the app for testing and production use
pub fn configure_app() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().service(redirect_to_github)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| configure_app())
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_web::test]
    async fn test_redirect_valid_reference() {
        let app = test::init_service(configure_app()).await;
        let req = test::TestRequest::get()
            .uri("/components/test_publisher.component.1.0.0.tar")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 302);

        let location = resp
            .headers()
            .get(header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(
            location,
            "https://github.com/test-publisher/slipway_component/releases/download/1.0.0/test_publisher.component.1.0.0.tar"
        );
    }

    #[actix_web::test]
    async fn test_invalid_reference_format() {
        let app = test::init_service(configure_app()).await;
        let req = test::TestRequest::get()
            .uri("/components/invalid-format.tar")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 400);
    }
}
