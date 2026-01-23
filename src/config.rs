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
    /// Whether to disable the "Not Official" warning banner.
    pub disable_warning: bool,
    /// Whether we should proxy spsejecna.cz or jidelna
    pub mode: Mode
}

#[derive(Debug, Clone)]
pub enum Mode {
    SPSEJECNA,
    JIDELNA
}

impl Mode {
    fn from_env() -> Self {
        match env::var("MODE")
            .ok()
            .map(|v| v.to_lowercase())
            .as_deref()
        {
            Some("jidelna") => Mode::JIDELNA,
            Some("spsejecna") => Mode::SPSEJECNA,
            _ => Mode::SPSEJECNA, // default
        }
    }

    pub fn url(&self) -> &'static str {
        match self {
            Mode::SPSEJECNA => "https://spsejecna.cz",
            Mode::JIDELNA => "https://strav.nasejidelna.cz",
        }
    }

    pub fn get_all_variants(&self) -> Vec<&'static str> {
        match self {
            Mode::SPSEJECNA => vec![
                "https://www.spsejecna.cz",
                "https://spsejecna.cz",
                "http://www.spsejecna.cz",
                "http://spsejecna.cz"
            ],
            Mode::JIDELNA => vec![
                "https://strav.nasejidelna.cz",
                "http://strav.nasejidelna.cz",
            ]
        }
    }
}

impl Config {
    /// # Environment Variables
    /// * `PORT` - Port to listen on (default: 3000).
    /// * `BASE_URL` - Explicit public URL of the proxy (optional).
    /// * `DISABLE_WARNING` - Set to "true" or "1" to disable the banner.
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let base_url = env::var("BASE_URL").ok();
        let disable_warning = env::var("DISABLE_WARNING")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        
        let mode = Mode::from_env();

        Self {
            port,
            base_url,
            disable_warning,
            mode,
        }
    }
}
