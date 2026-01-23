/*
 * Copyright (C) 2025 Jakub Žitník
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 */

use axum::http::{HeaderMap, HeaderValue};

use crate::state::AppState;

/// Determines the public origin of the proxy for the current request.
///
/// Priority:
/// 1. `BASE_URL` from environment configuration.
/// 2. `Host` header from the incoming request.
/// 3. Fallback to `http://localhost:3000`.
pub fn determine_proxy_origin(base_url: Option<&str>, headers: &HeaderMap) -> String {
    if let Some(base) = base_url {
        return base.trim_end_matches('/').to_string();
    }

    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:3000");

    // If no BASE_URL is set we are probably running locally or behind a simple proxy
    // that forwards the Host header. We assume HTTP.
    format!("http://{}", host)
}

/// Rewrites a content string (HTML, JSON, etc.) to point to the proxy instead of the upstream.
pub fn rewrite_content_urls(content: String, proxy_origin: &str, state: &AppState) -> String {
    let urls = state.config.mode.get_all_variants();
    let mut result = content;
    for url in urls {
        result = result.replace(&url, proxy_origin);
    }
    result
}

/// Processes a `Set-Cookie` header value
pub fn process_cookie(cookie: &str, is_secure_context: bool) -> String {
    let mut parts: Vec<String> = cookie
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| {
            let lower = s.to_lowercase();
            !lower.starts_with("domain=")
                && !lower.starts_with("samesite=")
                && (!lower.starts_with("secure") || is_secure_context)
        })
        .collect();

    if is_secure_context {
        parts.push("SameSite=None".to_string());

        if !parts.iter().any(|p| p.to_lowercase() == "secure") {
            parts.push("Secure".to_string());
        }
    } else {
        parts.push("SameSite=Lax".to_string());
    }

    parts.join("; ")
}

/// Checks if the proxy origin is considered "secure" (HTTPS or localhost).
pub fn is_secure_origin(origin: &str) -> bool {
    origin.starts_with("https://")
        || origin.contains("://localhost")
        || origin.contains("://127.0.0.1")
}

/// Rewrites request headers before sending to the upstream server.
pub fn prepare_request_headers(headers: &mut HeaderMap, state: &AppState) {
    headers.remove("host");
    headers.remove("content-length");
    headers.remove("accept-encoding");

    if headers.contains_key("origin") {
        headers.insert("origin", HeaderValue::from_str(&state.config.mode.url()).unwrap());
    }

    if headers.contains_key("referer") {
        headers.insert(
            "referer",
            HeaderValue::from_str(&format!("{}/", state.config.mode.url())).unwrap(),
        );
    }
}
