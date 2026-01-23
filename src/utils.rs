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
use reqwest::Url;

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
    let mut has_secure = false;
    let mut parts: Vec<String> = Vec::new();

    for raw in cookie.split(';') {
        let part = raw.trim();
        let lower = part.to_lowercase();

        match lower.as_str() {
            p if p.starts_with("domain=") => {}
            p if p.starts_with("path=") => parts.push(part.to_string()),
            p if p.starts_with("samesite=") => {}
            "secure" => {
                has_secure = true;
                if is_secure_context {
                    parts.push("Secure".to_string());
                }
            }
            "httponly" => {
                parts.push("HttpOnly".to_string());
            }
            _ => parts.push(part.to_string()),
        }
    }

    if is_secure_context {
        if !has_secure {
            parts.push("Secure".to_string());
        }
        parts.push("SameSite=None".to_string());
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
        headers.insert(
            "origin",
            HeaderValue::from_str(&state.config.mode.url()).unwrap(),
        );
    }

    if headers.contains_key("referer") {
        let base_url = Url::parse(&state.config.mode.url()).unwrap();

        let mut referer_url = Url::parse(headers["referer"].to_str().unwrap()).unwrap();

        referer_url.set_scheme(base_url.scheme()).unwrap();
        referer_url.set_host(base_url.host_str()).unwrap();
        referer_url.set_port(base_url.port()).unwrap();

        headers.insert(
            "referer",
            HeaderValue::from_str(referer_url.as_str()).unwrap(),
        );
    }

    tracing::info!(?headers);
}
