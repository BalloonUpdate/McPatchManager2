use std::collections::HashMap;
use std::time::SystemTime;

use axum::body::Body;
use axum::extract::Query;
use axum::extract::State;
use axum::response::Response;
use serde::Serialize;

use crate::web::file_status::SingleFileStatus;
use crate::web::webstate::WebState;

#[derive(Serialize)]
pub struct File {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub ctime: u64,
    pub mtime: u64,
    pub state: String,
}

pub async fn api_list(Query(params): Query<HashMap<String, String>>, State(state): State<WebState>) -> Response {
    let path = match params.get("path") {
        Some(ok) => ok.trim().to_owned(),
        None => return Response::builder()
            .status(500)
            .body(Body::new("parameter 'path' is missing.".to_string()))
            .unwrap(),
    };

    // // 路径不能为空
    // if path.is_empty() {
    //     return Response::builder()
    //         .status(500)
    //         .body(Body::new("parameter 'path' is empty, and it is not allowed.".to_string()))
    //         .unwrap();
    // }

    let mut status = state.status.lock().await;

    let file = state.app_context.workspace_dir.join(&path);

    println!("list: {:?}", file);

    if !file.exists() || !file.is_dir() {
        return Response::builder()
            .status(500)
            .body(Body::new("file not exists.".to_string()))
            .unwrap();
    }

    let mut response = Vec::<File>::new();

    let mut read_dir = tokio::fs::read_dir(&file).await.unwrap();

    while let Some(entry) = read_dir.next_entry().await.unwrap() {
        let is_directory = entry.file_type().await.unwrap().is_dir();
        let metadata = entry.metadata().await.unwrap();

        let relative_path = entry.path().strip_prefix(&state.app_context.workspace_dir).unwrap().to_str().unwrap().replace("\\", "/");

        // println!("relative: {:?}", relative_path);

        let st = status.get_file_status(&relative_path);

        response.push(File {
            name: entry.file_name().to_str().unwrap().to_string(),
            is_directory,
            size: if is_directory { 0 } else { metadata.len() },
            ctime: metadata.created().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            mtime: metadata.modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            state: match st {
                SingleFileStatus::Keep => "Keep".to_owned(),
                SingleFileStatus::Added => "Added".to_owned(),
                SingleFileStatus::Modified => "Modified".to_owned(),
                SingleFileStatus::Missing => "Missing".to_owned(),
                SingleFileStatus::Gone => "Gone".to_owned(),
                SingleFileStatus::Come => "Come".to_owned(),
            },
        });
    }

    let content = serde_json::to_string(&response).unwrap();

    Response::builder()
        .status(200)
        .body(Body::new(content))
        .unwrap()
}