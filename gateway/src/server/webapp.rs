//
// Copyright 2025-2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

mod admin;

use crate::context::ServerContext;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use wyldlands_common::gateway::GatewayProperty;

pub fn router(context: &ServerContext) -> Router {
    Router::new()
        .route("/", get(client_page))
        .route("/client.html", get(client_page))
        .route("/client.css", get(client_css))
        .route("/client.js", get(client_js))
        .route("/websocket", get(super::websocket::handler))
        .nest("/admin", admin::create_admin_router())
        .with_state(context.clone())
}

async fn client_page(State(context): State<ServerContext>) -> Response<Body> {
    match context
        .properties_manager()
        .get_property(GatewayProperty::ClientHtml)
        .await
    {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Error loading client page"))
            .unwrap(),
    }
}

async fn client_css(State(context): State<ServerContext>) -> Response<Body> {
    match context
        .properties_manager()
        .get_property(GatewayProperty::ClientCss)
        .await
    {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/css; charset=utf-8")
            .body(Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Error loading client CSS"))
            .unwrap(),
    }
}

async fn client_js(State(context): State<ServerContext>) -> Response<Body> {
    match context
        .properties_manager()
        .get_property(GatewayProperty::ClientJs)
        .await
    {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/javascript; charset=utf-8")
            .body(Body::from(content))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Error loading client JS"))
            .unwrap(),
    }
}
