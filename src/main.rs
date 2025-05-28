use actix_web::{App, HttpResponse, HttpServer, Responder, get, http::header, web::Path};
use regex::Regex;
use time::{OffsetDateTime, format_description};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt::time::FormatTime, layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub(crate) static REGISTRY_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^(?<publisher>\w+)\.(?<name>\w+)\.(?<version>.+)$").unwrap()
});

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = configure_tracing() {
        eprintln!("Failed to configure tracing: {}", e);
        return Ok(());
    }

    HttpServer::new(|| configure_app())
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[get("/components/{reference}.tar")]
async fn redirect_to_github(filename: Path<String>) -> impl Responder {
    let Some(caps) = REGISTRY_REGEX.captures(&filename) else {
        info!(
            "Received invalid filename in component download request: {}",
            filename
        );
        return HttpResponse::BadRequest()
            .body("Invalid filename format. Expected format: <publisher>.<name>.<version>.tar");
    };

    let publisher = &caps["publisher"];
    let name = &caps["name"];
    let version = &caps["version"];

    let (namespace, _localname) = match name.split_once("__") {
        Some((ns, l)) => (ns, l),
        None => (name, name),
    };

    let publisher_hyphenated = publisher.replace('_', "-");

    let target = format!(
        "https://github.com/{publisher_hyphenated}/slipway_{namespace}/releases/download/{version}/{publisher}.{name}.{version}.tar",
    );

    info!(
        loki.labels = "type",
        type="component_download_request",
        publisher, name, version,
        "Component download request redirected to: {}", target
    );

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

struct CustomTimer;

impl FormatTime for CustomTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = OffsetDateTime::now_utc();
        let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .expect("Timestamp format should be valid");
        write!(w, "{}", now.format(&format).unwrap())
    }
}

fn configure_tracing() -> anyhow::Result<()> {
    let loki_layer = init_grafana_layer()?;

    match loki_layer {
        Some(_) => {
            println!("Initializing Grafana Cloud logging");
        }
        None => {
            println!("Grafana Cloud logging not configured");
        }
    }

    tracing_subscriber::registry()
        .with(loki_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_timer(CustomTimer),
        )
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    Ok(())
}

fn init_grafana_layer() -> anyhow::Result<Option<impl Layer<Registry>>> {
    let Ok(grafana_logging_url) = std::env::var("GRAFANA_CLOUD_LOGGING_BASE_URL") else {
        return Ok(None);
    };
    let Ok(grafana_logging_id) = std::env::var("GRAFANA_CLOUD_LOGGING_ID") else {
        return Ok(None);
    };
    let Ok(grafana_logging_api_key) = std::env::var("GRAFANA_CLOUD_LOGGING_API_KEY") else {
        return Ok(None);
    };

    let (loki_layer, loki_task) = tracing_loki::builder()
        .label("application", env!("CARGO_PKG_NAME"))?
        .label(
            "instance",
            std::env::var("FLY_MACHINE_ID").unwrap_or_else(|_| "unknown".to_string()),
        )?
        .label(
            "region",
            std::env::var("FLY_REGION").unwrap_or_else(|_| "unknown".to_string()),
        )?
        .http_header(
            "Authorization",
            format!("Bearer {grafana_logging_id}:{grafana_logging_api_key}"),
        )?
        .build_url(url::Url::parse(&grafana_logging_url).unwrap())?;

    tokio::spawn(loki_task);

    Ok(Some(loki_layer))
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
    async fn test_redirect_valid_reference_with_namespace() {
        let app = test::init_service(configure_app()).await;
        let req = test::TestRequest::get()
            .uri("/components/test_publisher.component__sub_component.1.0.0.tar")
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
            "https://github.com/test-publisher/slipway_component/releases/download/1.0.0/test_publisher.component__sub_component.1.0.0.tar"
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
