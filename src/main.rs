use axum::{body::Body, http::Response, response::IntoResponse, routing::get, Json, Router};
use mpris::PlayerFinder;
use serde::Serialize;
use tokio::fs::File;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service(
            "/",
            ServeDir::new("dist").append_index_html_on_directories(true),
        )
        .route("/api/icon", get(icon))
        .route("/api/metadata", get(metadata))
        .route("/api/playpause", get(playpause));

    let listener = tokio::net::TcpListener::bind("192.168.2.2:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[axum::debug_handler]
async fn icon() -> impl IntoResponse {
    let art_url = {
        let player_finder = PlayerFinder::new().unwrap();
        let player = player_finder.find_active().unwrap();
        let metadata = player.get_metadata().unwrap();
        metadata.art_url().unwrap().to_owned()
    };

    let body = if let Some(path) = art_url.strip_prefix("file://") {
        let file = File::open(path).await.unwrap();
        Body::from_stream(tokio_util::io::ReaderStream::new(file))
    } else if art_url.starts_with("http") {
        let response = reqwest::get(art_url).await.unwrap();
        Body::from_stream(response.bytes_stream())
    } else {
        Body::empty()
    };

    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert("Content-Type", "image/jpeg".parse().unwrap());
    response
}

#[derive(Serialize)]
struct Info {
    position: Option<u64>,
    length: Option<u64>,
    title: Option<String>,
    running: bool,
}

fn metadata_inner() -> Info {
    let player_finder = PlayerFinder::new().unwrap();
    let player = player_finder.find_active().unwrap();
    let metadata = player.get_metadata().unwrap();
    Info {
        position: player.get_position_in_microseconds().ok(),
        length: metadata.length_in_microseconds(),
        title: metadata.title().map(ToOwned::to_owned),
        running: player.get_playback_status().unwrap() == mpris::PlaybackStatus::Playing,
    }
}

#[axum::debug_handler]
async fn metadata() -> impl IntoResponse {
    Json(metadata_inner())
}

#[axum::debug_handler]
async fn playpause() -> impl IntoResponse {
    let player_finder = PlayerFinder::new().unwrap();
    let player = player_finder.find_active().unwrap();
    let mut x = metadata_inner();
    x.running = !x.running;
    player.play_pause().unwrap();
    Json(x)
}
