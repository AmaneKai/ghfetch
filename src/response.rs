use serde::Serialize;
use worker::Response;

pub fn json<T: Serialize>(
    data: &T,
    status: u16,
    extra_headers: Option<Vec<(&str, String)>>,
) -> Result<Response, worker::Error> {
    let mut r = Response::from_json(data)?.with_status(status);
    let h = r.headers_mut();

    // CORS
    h.set("Access-Control-Allow-Origin", "*")?;
    h.set("Access-Control-Allow-Methods", "GET, OPTIONS")?;
    h.set("Access-Control-Allow-Headers", "Content-Type")?;
    h.set("Access-Control-Max-Age", "86400")?;

    // Security
    h.set("X-Content-Type-Options", "nosniff")?;
    h.set("X-Frame-Options", "DENY")?;
    h.set("X-XSS-Protection", "1; mode=block")?;
    h.set("Referrer-Policy", "strict-origin-when-cross-origin")?;

    // Cache
    h.set("Cache-Control", "public, max-age=300, stale-while-revalidate=60")?;
    h.set("Vary", "Accept-Encoding")?;
    h.set("Content-Type", "application/json")?;

    // Extra headers (rate limit info etc)
    if let Some(headers) = extra_headers {
        for (k, v) in headers {
            h.set(k, &v)?;
        }
    }

    Ok(r)
}

pub fn success<T: Serialize>(
    data: T,
    remaining: u32,
    reset: u64,
    cached: bool,
) -> Result<Response, worker::Error> {
    let headers = vec![
        ("X-RateLimit-Remaining", remaining.to_string()),
        ("X-RateLimit-Reset", reset.to_string()),
        ("X-Cache", if cached { "HIT".to_string() } else { "MISS".to_string() }),
    ];
    json(
        &serde_json::json!({ "ok": true, "data": data }),
        200,
        Some(headers),
    )
}

pub fn err(
    status: u16,
    msg: &str,
    rate_info: Option<(u32, u64)>,
) -> Result<Response, worker::Error> {
    let headers = rate_info.map(|(remaining, reset)| {
        vec![
            ("X-RateLimit-Remaining", remaining.to_string()),
            ("X-RateLimit-Reset", reset.to_string()),
        ]
    });
    json(
        &serde_json::json!({ "ok": false, "error": msg }),
        status,
        headers,
    )
}

pub fn health(version: &str) -> Result<Response, worker::Error> {
    json(
        &serde_json::json!({
            "status": "ok",
            "version": version
        }),
        200,
        None,
    )
}

pub fn root(version: &str) -> Result<Response, worker::Error> {
    json(
        &serde_json::json!({
            "name": "ghfetch",
            "version": version,
            "description": "GitHub stats API built with Rust on Cloudflare Workers",
            "endpoints": {
                "stats": "/v1/stats?username=<github-username>",
                "health": "/health"
            },
            "source": "https://github.com/AmaneKai/ghfetch"
        }),
        200,
        None,
    )
}

pub fn cors_preflight() -> Result<Response, worker::Error> {
    let mut r = Response::empty()?;
    let h = r.headers_mut();
    h.set("Access-Control-Allow-Origin", "*")?;
    h.set("Access-Control-Allow-Methods", "GET, OPTIONS")?;
    h.set("Access-Control-Allow-Headers", "Content-Type")?;
    h.set("Access-Control-Max-Age", "86400")?;
    Ok(r)
}
