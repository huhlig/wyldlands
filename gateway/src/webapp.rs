//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use axum::response::Response;
use std::net::SocketAddr;

pub(crate) async fn client_page(ConnectInfo(_addr): ConnectInfo<SocketAddr>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(include_str!("webapp/client.html")))
        .unwrap()
}

pub(crate) async fn client_css(ConnectInfo(_addr): ConnectInfo<SocketAddr>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css; charset=utf-8")
        .body(Body::from(include_str!("webapp/client.css")))
        .unwrap()
}

pub(crate) async fn client_js(ConnectInfo(_addr): ConnectInfo<SocketAddr>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/javascript; charset=utf-8")
        .body(Body::from(include_str!("webapp/client.js")))
        .unwrap()
}
