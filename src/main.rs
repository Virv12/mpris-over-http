use std::{
    convert::Infallible,
    hash::{Hash, Hasher},
};

use axum::{
    body::Body,
    extract::Path,
    http::Response,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use mpris::{Player, PlayerFinder};
use serde::Serialize;
use tokio::fs::File;
use tower_http::services::ServeDir;

fn find_player_by_id(id: &str) -> Option<Player> {
    let player_finder = PlayerFinder::new().unwrap();
    player_finder
        .iter_players()
        .unwrap()
        .map(|player| player.unwrap())
        .find(|player| player.unique_name() == id)
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service(
            "/",
            ServeDir::new("dist").append_index_html_on_directories(true),
        )
        .route("/list", get(list))
        .route("/metadata/:id", get(metadata))
        .route("/icon/:id/:hash", get(icon))
        .route("/playpause/:id", post(playpause));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[axum::debug_handler]
async fn list() -> impl IntoResponse {
    let player_finder = PlayerFinder::new().unwrap();
    let vec = player_finder
        .iter_players()
        .unwrap()
        .map(|player| player.unwrap().unique_name().to_owned())
        .collect::<Vec<_>>();
    Json(vec)
}

#[axum::debug_handler]
async fn metadata(Path(id): Path<String>) -> Response<Body> {
    #[derive(Serialize)]
    struct Info {
        position: Option<u64>,
        length: Option<u64>,
        title: Option<String>,
        running: bool,
        playback_rate: f64,
        art_url_hash: u64,

        can_control: bool,
        can_go_next: bool,
        can_go_previous: bool,

        //has_volume: bool,
        //volume: f64,
    }

    let (tx, rx) = tokio::sync::mpsc::channel(16);

    std::thread::spawn(move || {
        let player = find_player_by_id(&id).unwrap();
        for () in [()].into_iter().chain(player.events().unwrap().map(|_| ())) {
            let metadata = player.get_metadata().unwrap();
            let art_url = metadata.art_url();
            let mut hasher = std::hash::DefaultHasher::new();
            art_url.hash(&mut hasher);
            let art_url_hash = hasher.finish();
            let info = Info {
                position: player.get_position().ok().map(|d| d.as_secs()),
                length: metadata.length().map(|d| d.as_secs()),
                title: metadata.title().map(ToOwned::to_owned),
                running: player.get_playback_status().unwrap() == mpris::PlaybackStatus::Playing,
                playback_rate: player.get_playback_rate().unwrap_or(1.),
                art_url_hash,

                can_control: player.can_control().unwrap(),
                can_go_next: player.can_go_next().unwrap(),
                can_go_previous: player.can_go_previous().unwrap(),
            };
            let res = tx.blocking_send(info);
            if res.is_err() {
                break;
            }
        }
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx)
        .map(move |info| {
            let mut json = b"event: update\ndata: ".to_vec();
            serde_json::to_writer(&mut json, &info).unwrap();
            json.extend_from_slice(b"\n\n");
            Ok::<_, Infallible>(json)
        })
        .chain(futures::stream::iter([Ok(
            b"event: end\ndata: \n\n".to_vec()
        )]));
    let body = Body::from_stream(stream);
    let mut res = Response::new(body);
    res.headers_mut()
        .insert("Content-Type", "text/event-stream".parse().unwrap());
    res
}

#[axum::debug_handler]
async fn icon(Path((id, _hash)): Path<(String, u64)>) -> impl IntoResponse {
    let art_url = {
        let player = find_player_by_id(&id).unwrap();
        let metadata = player.get_metadata().unwrap();
        metadata.art_url().map(ToOwned::to_owned)
    };
    let Some(art_url) = art_url else {
        return Response::new(Body::empty());
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

    Response::new(body)
}

#[axum::debug_handler]
async fn playpause(Path(id): Path<String>) -> impl IntoResponse {
    let player = find_player_by_id(&id).unwrap();
    player.play_pause().unwrap();
    Response::new(Body::empty())
}
