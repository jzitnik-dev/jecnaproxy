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

use std::env;

/// Configuration for the Proxy Server.
#[derive(Debug, Clone)]
pub struct Config {
    /// The port to listen on.
    pub port: u16,
    /// The base URL of this proxy
    /// If `None`, it is determined dynamically from the `Host` header.
    pub base_url: Option<String>,
}

impl Config {
    /// # Environment Variables
    /// * `PORT` - Port to listen on (default: 3000).
    /// * `BASE_URL` - Explicit public URL of the proxy (optional).
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let base_url = env::var("BASE_URL").ok();

        Self { port, base_url }
    }
}
