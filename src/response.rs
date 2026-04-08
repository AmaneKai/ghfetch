use serde::Serialize;
use worker::Response;

pub fn json<T: Serialize>(data: &T, status: u16) -> Result<Response, worker::Error> {
    let mut r = Response::from_json(data)?.with_status(status);
    let h = r.headers_mut();
    
    h.set("Access-Control-Allow-Origin", "*")?;
    h.set("Access-Control-Allow-Methods", "GET, OPTIONS")?;
    h.set("Access-Control-Allow-Headers", "Content-Type")?;
    h.set("Access-Control-Max-Age", "86400")?;
    
    h.set("X-Content-Type-Options", "nosniff")?;
    h.set("X-Frame-Options", "DENY")?;
    h.set("X-XSS-Protection", "1; mode=block")?;
    h.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    
    h.set("Cache-Control", "public, max-age=300, stale-while-revalidate=60")?;
    h.set("Vary", "Accept-Encoding")?;
    h.set("Content-Type", "application/json")?;
    
    Ok(r)
}

pub fn success<T: Serialize>(data: T) -> Result<Response, worker::Error> {
    json(&serde_json::json!({ "ok": true, "data": data }), 200)
}

pub fn err(status: u16, msg: &str) -> Result<Response, worker::Error> {
    json(&serde_json::json!({ "ok": false, "error": msg }), status)
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
