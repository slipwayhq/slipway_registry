use actix_web::{Error, HttpRequest, HttpResponse, get};
use prometheus::{Encoder, TextEncoder};

#[get("/metrics")]
pub(super) async fn metrics(req: HttpRequest) -> Result<HttpResponse, Error> {
    let peer_addr = req.peer_addr().map(|a| a.ip());
    let is_localhost = matches!(
        peer_addr,
        Some(ip) if ip.is_loopback() || ip.is_unspecified() // 127.0.0.1 or ::1 or 0.0.0.0
    );

    if !is_localhost {
        return Ok(HttpResponse::Forbidden().body("Forbidden"));
    }

    let encoder = TextEncoder::new();
    let metric_families = super::REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();

    Ok(HttpResponse::Ok()
        .content_type(encoder.format_type())
        .body(output))
}
