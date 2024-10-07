use std::{
    convert::Infallible,
    hash::{Hash, Hasher},
};

use anyhow::{anyhow, bail};
use axum::{
    body::Body,
    extract::Path,
    http::Response,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use http::status::StatusCode;
use mpris::{DBusError, Player, PlayerFinder};
use serde::Serialize;
use tokio::fs::File;
use tower_http::services::ServeDir;

mod error;

use error::AppResult;

fn find_player_by_id(id: &str) -> Result<Option<Player>, DBusError> {
    let player_finder = PlayerFinder::new()?;
    Ok(player_finder
        .iter_players()?
        .find(|player| {
            !player
                .as_ref()
                .is_ok_and(|player| player.unique_name() != id)
        })
        .transpose()?)
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
        .route("/playpause/:id", post(playpause))
        .route("/seek/:id/:dtime", post(seek));

    let listener = tokio::net::TcpListener::bind("192.168.2.2:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[axum::debug_handler]
async fn list() -> AppResult<impl IntoResponse> {
    let player_finder = PlayerFinder::new()?;
    let vec = player_finder
        .iter_players()?
        .map(|player| player.map(|player| player.unique_name().to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(vec))
}

#[axum::debug_handler]
async fn metadata(Path(id): Path<String>) -> Response<Body> {
    #[derive(Serialize)]
    struct Info {
        position: u64,
        length: Option<u64>,
        title: Option<String>,
        running: bool,
        playback_rate: Option<f64>,
        art_url_hash: u64,

        can_control: bool,
        can_go_next: bool,
        can_go_previous: bool,
        can_seek: bool,
        //has_volume: bool,
        //volume: f64,
    }

    let (tx, rx) = tokio::sync::mpsc::channel(16);

    std::thread::spawn(move || {
        let player = find_player_by_id(&id).unwrap().unwrap();
        for () in [()].into_iter().chain(player.events().unwrap().map(|_| ())) {
            let metadata = player.get_metadata().unwrap();
            let art_url = metadata.art_url();
            let mut hasher = std::hash::DefaultHasher::new();
            art_url.hash(&mut hasher);
            let art_url_hash = hasher.finish();
            let info = Info {
                position: player.get_position_in_microseconds().unwrap(),
                length: metadata.length_in_microseconds(),
                title: metadata.title().map(ToOwned::to_owned),
                running: player.get_playback_status().unwrap() == mpris::PlaybackStatus::Playing,
                playback_rate: player.get_playback_rate().ok(),
                art_url_hash,

                can_control: player.can_control().unwrap(),
                can_go_next: player.can_go_next().unwrap(),
                can_go_previous: player.can_go_previous().unwrap(),
                can_seek: player.can_seek().unwrap(),
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
async fn icon(Path((id, _hash)): Path<(String, u64)>) -> AppResult<Response<Body>> {
    let art_url = {
        let Some(player) = find_player_by_id(&id)? else {
            return Ok((StatusCode::NOT_FOUND, "Player not found\n").into_response());
        };
        let metadata = player.get_metadata()?;
        let Some(art_url) = metadata.art_url() else {
            return Ok((StatusCode::NOT_FOUND, "No art URL\n").into_response());
        };
        art_url.to_owned()
    };

    if let Some(path) = art_url.strip_prefix("file://") {
        let file = File::open(path).await?;
        return Ok(Body::from_stream(tokio_util::io::ReaderStream::new(file)).into_response());
    }

    if art_url.starts_with("http") {
        let response = reqwest::get(art_url).await?;
        return Ok(Body::from_stream(response.bytes_stream()).into_response());
    }

    Err(anyhow!("Unsupported art URL: {}", art_url))?
}

#[axum::debug_handler]
async fn playpause(Path(id): Path<String>) -> AppResult<impl IntoResponse> {
    let Some(player) = find_player_by_id(&id)? else {
        return Ok((StatusCode::NOT_FOUND, "Player not found\n"));
    };
    player.play_pause()?;
    Ok((StatusCode::OK, "Play/pause successfull\n"))
}

#[axum::debug_handler]
async fn seek(Path((id, dtime)): Path<(String, i64)>) -> AppResult<impl IntoResponse> {
    let Some(player) = find_player_by_id(&id)? else {
        return Ok((StatusCode::NOT_FOUND, "Player not found\n"));
    };
    player.seek(dtime)?;
    Ok((StatusCode::OK, "Seek successfull\n"))
}
