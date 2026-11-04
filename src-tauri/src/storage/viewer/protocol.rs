use super::{Result, failure};
use crate::storage::{StorageState, service::StorageService};
use std::io::{Read, Seek, SeekFrom};
use tauri::{
    Manager,
    http::{Request, Response},
};

pub fn allowed_origin(origin: &str) -> bool {
    matches!(
        origin,
        "http://tauri.localhost" | "https://tauri.localhost" | "tauri://localhost"
    ) || (cfg!(debug_assertions) && origin == "http://127.0.0.1:1420")
}
fn empty(status: u16) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Cache-Control", "no-store")
        .header("X-Content-Type-Options", "nosniff")
        .body(Vec::new())
        .unwrap()
}
pub(crate) fn byte_range(value: Option<&str>, length: u64) -> Result<Option<(u64, u64)>> {
    let invalid = || failure("media_range", "The requested media range is invalid.");
    let Some(value) = value else { return Ok(None) };
    let (start, end) = value
        .strip_prefix("bytes=")
        .ok_or_else(invalid)?
        .split_once('-')
        .ok_or_else(invalid)?;
    if value.contains(',') || length == 0 {
        return Err(invalid());
    }
    let (start, end) = if start.is_empty() {
        let suffix = end.parse::<u64>().map_err(|_| invalid())?;
        if suffix == 0 {
            return Err(invalid());
        }
        (length.saturating_sub(suffix), length - 1)
    } else {
        let start = start.parse::<u64>().map_err(|_| invalid())?;
        let end = if end.is_empty() {
            length - 1
        } else {
            end.parse::<u64>().map_err(|_| invalid())?.min(length - 1)
        };
        (start, end)
    };
    if start > end || start >= length {
        return Err(invalid());
    }
    Ok(Some((
        start,
        end.min(start.saturating_add(2 * 1024 * 1024 - 1)),
    )))
}
impl StorageService {
    pub(crate) fn media_response(&self, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
        if request.method() != "GET" && request.method() != "HEAD" {
            return empty(405);
        }
        let origin = request
            .headers()
            .get("Origin")
            .and_then(|v| v.to_str().ok());
        if origin.is_some_and(|o| !allowed_origin(o)) {
            return empty(403);
        }
        let token = request.uri().path().strip_prefix('/').unwrap_or("");
        if uuid::Uuid::parse_str(token).is_err() || request.uri().query().is_some() {
            return empty(403);
        }
        let Ok(mut resolved) = self.granted_media(token) else {
            return empty(404);
        };
        if resolved.metadata.kind == "document" {
            return empty(403);
        }
        let length = resolved.metadata.size_bytes;
        let range = match byte_range(
            request.headers().get("Range").and_then(|v| v.to_str().ok()),
            length,
        ) {
            Ok(value) => value,
            Err(_) => {
                return Response::builder()
                    .status(416)
                    .header("Content-Range", format!("bytes */{length}"))
                    .body(Vec::new())
                    .unwrap();
            }
        };
        let (start, end) = range.unwrap_or((0, length.saturating_sub(1)));
        // WebView media requests use byte ranges. Never buffer a large entire video.
        if range.is_none() && length > 16 * 1024 * 1024 {
            return empty(413);
        }
        let count = if length == 0 { 0 } else { end - start + 1 };
        let mut bytes = Vec::new();
        if request.method() != "HEAD" {
            if resolved.file.seek(SeekFrom::Start(start)).is_err()
                || resolved.file.take(count).read_to_end(&mut bytes).is_err()
                || bytes.len() as u64 != count
            {
                return empty(404);
            }
        }
        let mut response = Response::builder()
            .status(if range.is_some() { 206 } else { 200 })
            .header("Content-Type", resolved.metadata.mime)
            .header("Content-Length", count.to_string())
            .header("Accept-Ranges", "bytes")
            .header("Cache-Control", "no-store")
            .header("X-Content-Type-Options", "nosniff")
            .header("Content-Security-Policy", "default-src 'none'; sandbox")
            .header("Cross-Origin-Resource-Policy", "cross-origin");
        if let Some(origin) = origin {
            response = response
                .header("Access-Control-Allow-Origin", origin)
                .header("Vary", "Origin");
        }
        if range.is_some() {
            response = response.header("Content-Range", format!("bytes {start}-{end}/{length}"));
        }
        response.body(bytes).unwrap_or_else(|_| empty(500))
    }
}
pub fn register(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.register_asynchronous_uri_scheme_protocol(
        "chatvault-media",
        |context, request, responder| {
            if context.webview_label() != "main" {
                responder.respond(empty(403));
                return;
            }
            let state = context.app_handle().state::<StorageState>().inner().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let response = state
                    .run(false, |service| Ok(service.media_response(request)))
                    .unwrap_or_else(|_| empty(503));
                responder.respond(response);
            });
        },
    )
}
