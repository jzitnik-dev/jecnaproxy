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

use crate::config::Config;
use reqwest::Client;
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// The HTTP client used to forward requests to the upstream server.
    pub client: Client,
    /// The application configuration.
    pub config: Arc<Config>,
}
