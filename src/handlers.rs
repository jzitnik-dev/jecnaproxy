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

/// The main proxy handler that intercepts all traffic.
///
/// It forwards requests to `https://www.spsejecna.cz`, rewriting headers and body content
/// to ensure the site functions correctly when accessed via this proxy.
pub async fn proxy_handler(State(state): State<AppState>, req: Request) -> Response {
    let client = state.client;
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or("/");

    let target_url = format!("https://www.spsejecna.cz{}", path_query);
    tracing::info!("Proxying: {} -> {}", req.uri(), target_url);

    let proxy_origin =
        utils::determine_proxy_origin(state.config.base_url.as_deref(), req.headers());

    let is_secure = utils::is_secure_origin(&proxy_origin);

    let method = req.method().clone();
    let mut headers = req.headers().clone();

    utils::prepare_request_headers(&mut headers);

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
        Ok(resp) => process_response(resp, &proxy_origin, is_secure).await,
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
                let new_val = utils::rewrite_content_urls(str_val.to_string(), proxy_origin);

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
                let new_body_str = utils::rewrite_content_urls(body_str, proxy_origin);

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
