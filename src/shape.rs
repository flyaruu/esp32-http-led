use alloc::string::{String, ToString};
use picoserve::{extract::FromRequest, response::{IntoResponse, status::BAD_REQUEST}};
use serde::{Deserialize, Serialize};

use crate::web::WebState;


pub enum ShapeError {
    DeserializationError(String),
}

impl IntoResponse for ShapeError {
    async fn write_to<W: picoserve::response::ResponseWriter>(
        self,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        match self {
            ShapeError::DeserializationError(msg) => (BAD_REQUEST,msg.as_str()).write_to(response_writer).await,
        }
    }
}

#[derive(Deserialize,Serialize,Clone, Copy, Debug)]
pub struct Point {
    x: u16,
    y: u16,
}

#[derive(Deserialize,Serialize,Clone, Copy, Debug)]
pub enum Shape {
    Triangle{a: Point, b: Point, c: Point}
}

impl FromRequest<WebState> for Shape {
    type Rejection = ShapeError;

    async fn from_request(state: &WebState, request: &picoserve::request::Request<'_>) -> Result<Self, Self::Rejection> {
        serde_json::from_slice(request.body())
            .map_err(|e| ShapeError::DeserializationError(e.to_string()))
    }
}