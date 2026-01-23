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

mod config;
mod handlers;
mod state;
mod utils;

use axum::{Router, http::Method, routing::any};
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Arc::new(Config::from_env());

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build reqwest client");

    let state = AppState {
        client,
        config: config.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::HEAD,
            Method::OPTIONS,
        ])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    let app = Router::new()
        .route("/robots.txt", any(handlers::robots_txt_handler))
        .route("/", any(handlers::proxy_handler))
        .route("/{*path}", any(handlers::proxy_handler))
        .layer(cors)
        .with_state(state);

    let addr_str = format!("0.0.0.0:{}", config.port);
    let addr: SocketAddr = addr_str
        .parse()
        .expect("Invalid address/port configuration");

    tracing::info!("Proxy listening on http://{}", addr);
    if let Some(base) = &config.base_url {
        tracing::info!("Public Base URL configured: {}", base);
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
