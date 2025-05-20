use actix_web::{App, HttpServer};
use once_cell::sync::Lazy;
use prometheus::{CounterVec, Opts, Registry};
use time::{OffsetDateTime, format_description};
use tracing::{Level, debug, info};
use tracing_subscriber::{FmtSubscriber, fmt::time::FormatTime};

mod components;
mod metrics;

// Static metrics registry
pub static REGISTRY: Lazy<Registry> = Lazy::new(|| {
    debug!("Initializing metrics registry");
    Registry::new()
});

pub static DOWNLOAD_COUNTER: Lazy<CounterVec> = Lazy::new(|| {
    debug!("Initializing download counter");
    let c = CounterVec::new(
        Opts::new("component_downloads", "Count of component downloads"),
        &["publisher", "name", "version"],
    )
    .unwrap();
    REGISTRY.register(Box::new(c.clone())).unwrap();
    c
});

pub static DOWNLOAD_ERRORS: Lazy<CounterVec> = Lazy::new(|| {
    debug!("Initializing download errors counter");
    let c = CounterVec::new(
        Opts::new(
            "component_download_errors",
            "Invalid or failed download requests",
        ),
        &["reason"],
    )
    .unwrap();
    REGISTRY.register(Box::new(c.clone())).unwrap();
    c
});

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
    App::new()
        .service(components::redirect_to_github)
        .service(metrics::metrics)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    configure_tracing(Some("debug".to_string()));
    info!("Tracing configured.");
    HttpServer::new(|| configure_app())
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
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

fn configure_tracing(log_level: Option<String>) {
    let log_level = match log_level.map(|level| level.to_lowercase()).as_deref() {
        Some("error") => Level::ERROR,
        Some("warn") => Level::WARN,
        Some("info") => Level::INFO,
        Some("debug") => Level::DEBUG,
        Some("trace") => Level::TRACE,
        Some(_) => panic!("invalid log level. must be one of [error, warn, info, debug, trace]."),
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_target(false)
        .with_timer(CustomTimer)
        .with_max_level(log_level)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::header, test};

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
