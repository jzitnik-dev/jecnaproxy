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

use crate::{state::AppState, utils};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

const BANNER_HTML: &str = r#"<div style="width: 100vw; height: 100vh; position: fixed; z-index: 1000; background-color: black; color: white; display: flex; flex-direction: column; justify-content: center; align-items: center; text-align: center; gap: 5px;">
  <h1 style="font-size: 40px;">Toto není oficiální web SPŠE Ječná!</h1>
  <p style="font-size: 20px;">Oficiální web se nachází na <a style="font-size: 20px; color: white;" href="https://spsejecna.cz">spsejecna.cz</a>.</p>
  <script>
    setTimeout(() => {
      const { pathname, search, hash } = window.location;
      window.location.replace(
        "https://spsejecna.cz" + pathname + search + hash
      );
    }, 500);
  </script>
</div>"#;

/// The main proxy handler that intercepts all traffic.
///
/// It forwards requests to `https://www.spsejecna.cz`, rewriting headers and body content
/// to ensure the site functions correctly when accessed via this proxy.
pub async fn proxy_handler(State(state): State<AppState>, req: Request) -> Response {
    let client = &state.client;
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or("/");

    let target_url = format!("{}{}", state.config.mode.url(), path_query);
    tracing::info!("Proxying: {} -> {}", req.uri(), target_url);

    let proxy_origin =
        utils::determine_proxy_origin(state.config.base_url.as_deref(), req.headers());

    let is_secure = utils::is_secure_origin(&proxy_origin);

    let method = req.method().clone();
    let mut headers = req.headers().clone();

    utils::prepare_request_headers(&mut headers, &state);

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    // Send Upstream Request
    let request_builder = client
        .request(method, &target_url)
        .headers(headers)
        .body(body_bytes);

    match request_builder.send().await {
        Ok(resp) => {
            process_response(resp, &proxy_origin, is_secure, state.config.disable_warning, &state).await
        }
        Err(e) => {
            tracing::error!("Upstream request failed: {}", e);
            (StatusCode::BAD_GATEWAY, format!("Proxy Error: {}", e)).into_response()
        }
    }
}

/// Processes the upstream response
async fn process_response(
    resp: reqwest::Response,
    proxy_origin: &str,
    is_secure: bool,
    disable_warning: bool,
    state: &AppState
) -> Response {
    let status = resp.status();
    let mut headers = HeaderMap::new();

    for (key, value) in resp.headers() {
        if key == "set-cookie" {
            if let Ok(str_val) = value.to_str() {
                let new_val = utils::process_cookie(str_val, is_secure);
                if let Ok(v) = HeaderValue::from_str(&new_val) {
                    headers.append(key, v);
                }
            } else {
                headers.append(key, value.clone());
            }
        } else if key == "location" {
            if let Ok(str_val) = value.to_str() {
                let new_val = utils::rewrite_content_urls(str_val.to_string(), proxy_origin, &state);

                let new_val = if new_val.is_empty() {
                    "/".to_string()
                } else {
                    new_val
                };

                if let Ok(v) = HeaderValue::from_str(&new_val) {
                    headers.append(key, v);
                } else {
                    headers.append(key, value.clone());
                }
            } else {
                headers.append(key, value.clone());
            }
        } else {
            headers.append(key, value.clone());
        }
    }

    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let should_rewrite_body = content_type.contains("text/html")
        || content_type.contains("application/javascript")
        || content_type.contains("application/json")
        || content_type.contains("text/css");

    if should_rewrite_body {
        match resp.bytes().await {
            Ok(bytes) => {
                let body_str = String::from_utf8_lossy(&bytes).to_string();
                let mut new_body_str = utils::rewrite_content_urls(body_str, proxy_origin, &state);

                if content_type.contains("text/html") && !disable_warning {
                    inject_banner(&mut new_body_str);
                }

                // Remove headers that are invalid after modification
                headers.remove("content-length");
                headers.remove("transfer-encoding");
                headers.remove("content-encoding");

                let mut response = Response::new(Body::from(new_body_str));
                *response.status_mut() = status;
                *response.headers_mut() = headers;
                response
            }
            Err(e) => {
                tracing::error!("Failed to read response body: {}", e);
                (StatusCode::BAD_GATEWAY, "Failed to read body").into_response()
            }
        }
    } else {
        // Stream binary content directly
        let body = Body::from_stream(resp.bytes_stream());
        let mut response = Response::new(body);
        *response.status_mut() = status;
        *response.headers_mut() = headers;
        response
    }
}

fn inject_banner(body: &mut String) {
    let insert_pos = body.match_indices('<').find_map(|(idx, _)| {
        if body[idx..].len() >= 5 && body[idx + 1..idx + 5].eq_ignore_ascii_case("body") {
            body[idx..].find('>').map(|offset| idx + offset + 1)
        } else {
            None
        }
    });

    if let Some(pos) = insert_pos {
        body.insert_str(pos, BANNER_HTML);
    } else {
        body.insert_str(0, BANNER_HTML);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_banner_basic() {
        let mut html = "<html><body><h1>Hello</h1></body></html>".to_string();
        inject_banner(&mut html);
        assert!(html.contains("<body><div"));
        assert!(html.contains(BANNER_HTML));
    }

    #[test]
    fn test_inject_banner_attributes() {
        let mut html = "<html lang='en'><body class='foo'><h1>Hello</h1></body></html>".to_string();
        inject_banner(&mut html);
        assert!(html.contains("<body class='foo'><div"));
    }

    #[test]
    fn test_inject_banner_case() {
        let mut html = "<HTML><BODY><h1>Hello</h1></BODY></HTML>".to_string();
        inject_banner(&mut html);
        assert!(html.contains("<BODY><div"));
    }

    #[test]
    fn test_inject_banner_no_body() {
        let mut html = "<h1>Hello</h1>".to_string();
        inject_banner(&mut html);
        assert!(html.starts_with("<div"));
        assert!(html.contains("<h1>Hello</h1>"));
    }
}
