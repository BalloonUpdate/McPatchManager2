use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;

use crate::upload::file_list_cache::FileListCache;
use crate::upload::s3::S3Target;
use crate::upload::webdav::WebdavTarget;
use crate::upload::SyncTarget;
use crate::web::webstate::WebState;

/// 打包新版本
pub async fn api_upload(State(state): State<WebState>, headers: HeaderMap) -> Response {
    let wait = headers.get("wait").is_some();

    state.clone().te.lock().await
        .try_schedule(wait, state.clone(), move || do_upload(state)).await
}

fn do_upload(state: WebState) -> u8 {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();

    runtime.block_on(async_upload(state));
    
    0
}

async fn async_upload(state: WebState) -> u8 {
    let webdav_config = state.config.webdav.clone();
    let s3_config = state.config.s3.clone();

    if webdav_config.enabled {
        // if webdav_config

        if let Err(err) = upload("webdav", state.clone(), FileListCache::new(WebdavTarget::new(webdav_config).await)).await {
            let mut console = state.console.blocking_lock();
    
            console.log_error(err);
    
            return 1;
        }
    }

    if s3_config.enabled {
        if let Err(err) = upload("s3", state.clone(), FileListCache::new(S3Target::new(s3_config).await)).await {
            let mut console = state.console.blocking_lock();
    
            console.log_error(err);
    
            return 1;
        }
    }

    0
}

async fn upload(name: &str, state: WebState, mut target: impl SyncTarget) -> Result<(), String> {
    let mut console = state.console.blocking_lock();

    console.log_debug("收集本地文件列表...");
    let local = get_local(&state).await;

    console.log_debug(format!("收集 {} 上的文件列表...", name));
    let remote = target.list().await?;

    console.log_debug("计算文件列表差异...");

    // 寻找上传的文件
    let mut need_upload = Vec::new();
    
    for f in &local {
        if !remote.contains(&f) {
            need_upload.push(f.clone());
        }
    }

    // 寻找删除的文件
    let mut need_delete = Vec::new();

    for f in &remote {
        if !local.contains(&f) {
            need_delete.push(f.clone());
        }
    }

    // 上传文件
    for f in &need_upload {
        target.upload(&f, state.app_path.public_dir.join(&f)).await?;
    }

    // 删除文件
    for f in &need_delete {
        target.delete(&f).await?;
    }

    Ok(())
}

async fn get_local(state: &WebState) -> Vec<String> {
    let mut dir = tokio::fs::read_dir(&state.app_path.public_dir).await.unwrap();

    let mut files = Vec::new();

    while let Some(entry) = dir.next_entry().await.unwrap() {
        files.push(entry.file_name().to_str().unwrap().to_owned());
    }

    files
}