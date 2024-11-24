use axum::extract::State;
use axum::response::Response;
use serde::Serialize;

use crate::web::api::PublicResponseBody;
use crate::web::webstate::WebState;

#[derive(Serialize)]
pub struct ResponseData {
    /// 返回的日志文本
    pub content: String,
}

pub async fn api_more(State(state): State<WebState>) -> Response {
    let mut console = state.console.lock().await;

    let mut buf = String::with_capacity(1024);

    for log in console.get_logs(false) {
        buf += &log.content;
        buf += "\n";
    }

    PublicResponseBody::<ResponseData>::ok(ResponseData { content: buf })
}