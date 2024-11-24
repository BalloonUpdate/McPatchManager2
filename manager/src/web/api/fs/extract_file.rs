use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;

use axum::body::Body;
use axum::extract::Query;
use axum::extract::State;
use axum::response::Response;
use sha2::Digest;
use sha2::Sha256;

use crate::web::webstate::WebState;

pub async fn api_extract_file(State(state): State<WebState>, Query(params): Query<HashMap<String, String>>) -> Response {
    let signature = match params.get("sign") {
        Some(ok) => ok,
        None => return Response::builder()
            .status(403)
            .body(Body::empty())
            .unwrap(),
    };

    let mut split = signature.split(":");

    let path = match split.next() {
        Some(ok) => ok,
        None => return Response::builder().status(403).body(Body::empty()).unwrap(),
    };

    let expire = match split.next() {
        Some(ok) => match u64::from_str_radix(ok, 10) {
            Ok(ok) => ok,
            Err(_) => return Response::builder().status(403).body(Body::empty()).unwrap(),
        },
        None => return Response::builder().status(403).body(Body::empty()).unwrap(),
    };

    let digest = match split.next() {
        Some(ok) => ok,
        None => return Response::builder().status(403).body(Body::empty()).unwrap(),
    };

    let status = state.status.lock().await;
    let config = status.config.config.lock().await;

    let hash = hash(&format!("{}:{}:{}@{}", path, expire, config.web.username, config.web.password));

    if hash != digest {
        return Response::builder().status(403).body(Body::new("invalid signature".to_owned())).unwrap();
    }

    // 检查是否超过有效期
    if (SystemTime::UNIX_EPOCH + Duration::from_secs(expire)).duration_since(SystemTime::UNIX_EPOCH).is_err() {
        return Response::builder().status(403).body(Body::new("signature is outdate".to_owned())).unwrap();
    }

    let path = state.config.workspace_dir.join(path);

    let file = tokio::fs::File::options()
        .read(true)
        .open(path)
        .await
        .unwrap();

    let file = tokio_util::io::ReaderStream::new(file);

    Response::builder().body(Body::from_stream(file)).unwrap()
}

fn hash(text: &impl AsRef<str>) -> String {
    let hash = Sha256::digest(text.as_ref());
    
    base16ct::lower::encode_string(&hash)
}