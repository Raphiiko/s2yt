#[macro_use]
extern crate lazy_static;
use invidious::structs::hidden::SearchItem;
use tokio::sync::Mutex;

use actix_web::{get, HttpRequest, HttpResponse, HttpServer, Responder};
use rspotify::{model::FullTrack, prelude::*, scopes, AuthCodePkceSpotify, Credentials, OAuth};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct AppConfig {
    spotify_client_id: String,
    #[serde(default)]
    spotify_token: Option<rspotify::Token>,
}

impl ::std::default::Default for AppConfig {
    fn default() -> Self {
        Self {
            spotify_client_id: "".into(),
            spotify_token: None,
        }
    }
}

lazy_static! {
    static ref SPOTIFY: Mutex<Option<rspotify::AuthCodePkceSpotify>> = Mutex::default();
    static ref CONFIG: Mutex<Option<AppConfig>> = Mutex::default();
    static ref LAST_TRACK_ID: Mutex<Option<String>> = Mutex::default();
}

#[tokio::main]
async fn main() {
    println!("S2YT - Spotify to YouTube - Raphiiko\n\n");
    // let cfg_path = confy::get_configuration_file_path("s2yt", None).unwrap();
    // println!("Config path: {:#?}", cfg_path);
    // Load config
    *CONFIG.lock().await = Some(confy::load::<AppConfig>("s2yt", None).unwrap());
    // Setup http server
    let server = HttpServer::new(|| actix_web::App::new().service(spotify_callback))
        .bind(("localhost", 8888))
        .unwrap()
        .run();
    // Ensure spotify credentials
    ensure_spotify().await;
    // Start track listener
    listen_for_tracks();
    // Keep process alive
    let _ = server.await;
}

fn listen_for_tracks() {
    tokio::spawn(async {
        // Wait for usable spotify client
        loop {
            let mut spotify_guard = SPOTIFY.lock().await;
            let spotify_guard = &mut *spotify_guard;
            let spotify = spotify_guard.clone().unwrap();
            let token_guard = spotify.token.lock().await.unwrap();
            let token = token_guard.as_ref();
            drop(spotify_guard);
            let mut token_found = false;
            if let Some(_) = token {
                token_found = true;
            }
            drop(token_guard);
            if token_found {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        // Poll currently playing track
        loop {
            let mut spotify_guard = SPOTIFY.lock().await;
            let spotify_guard = &mut *spotify_guard;
            let spotify = spotify_guard.clone().unwrap();
            let market = rspotify::model::Market::FromToken;
            match spotify
                .current_playing(Some(&market), None::<&[rspotify::model::AdditionalType; 0]>)
                .await
            {
                Ok(response) => match response {
                    Some(context) => {
                        if let Some(item) = context.item {
                            if let rspotify::model::PlayableItem::Track(track) = item {
                                update_track(track).await;
                            }
                        }
                    }
                    None => (),
                },
                Err(e) => eprintln!("Error: {:#?}", e),
            };
            drop(spotify_guard);
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

async fn update_track(track: FullTrack) {
    // Stop if track has not changed
    let track_id = String::from(track.id.unwrap().id());
    let mut last_track_id_guard = LAST_TRACK_ID.lock().await;
    let last_track_id_guard = &mut *last_track_id_guard;
    let last_track_id = last_track_id_guard.clone();
    if last_track_id.is_some() && last_track_id.unwrap() == track_id {
        return;
    }
    // Update last track id
    *last_track_id_guard = Some(track_id);
    println!(
        "Track changed: {} - {}. Searching YouTube...",
        track.artists[0].name, track.name
    );
    // Search YouTube
    let invidious_client =
        invidious::reqwest::asynchronous::Client::new(String::from("https://vid.puffyan.us"));
    let query = format!(
        "type=video&q={}",
        urlencoding::encode(format!("{} - {}", track.artists[0].name, track.name).as_str())
            .into_owned(),
    );
    let results: Vec<String> = invidious_client
        .search(Some(query.as_str()))
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|item| match item {
            SearchItem::Video {
                id,
                title: _,
                author: _,
                author_id: _,
                author_url: _,
                length: _,
                thumbnails: _,
                description: _,
                description_html: _,
                views: _,
                published: _,
                published_text: _,
                live: _,
                paid: _,
                premium: _,
            } => id,
            _ => panic!("Unexpected search item type"),
        })
        .collect();
    // No result
    if results.is_empty() {
        println!("No results found on YouTube. Nothing copied to clipboard.");
        return;
    }
    // Copy first result to clipboard
    let result = results[0].clone();
    let youtube_url = format!("https://www.youtube.com/watch?v={}", result);
    println!(
        "Found {} result(s). Copying first result to clipboard: {}",
        results.len(),
        youtube_url
    );
    cli_clipboard::set_contents(youtube_url.to_owned()).unwrap();
}

#[get("/callback")]
async fn spotify_callback(_req: HttpRequest) -> impl Responder {
    let spotify_guard = SPOTIFY.lock().await;
    let mut spotify = spotify_guard.as_ref().unwrap().to_owned();
    drop(spotify_guard);
    let code = spotify
        .parse_response_code(format!("http://localhost:8888{}", _req.uri().to_string()).as_str());
    let code = match code {
        Some(code) => code,
        None => {
            eprintln!(
                "Could not authorize with Spotify due to missing code in response. Stopping..."
            );
            std::process::exit(1);
        }
    };
    match spotify.request_token(code.as_str()).await {
        Ok(_) => {
            println!("Successfully authorized with Spotify!");
            // Save token
            let spotify_token = spotify.get_token().lock().await.unwrap().clone();
            let cfg_guard = CONFIG.lock().await;
            let mut cfg = (*cfg_guard.as_ref().unwrap()).to_owned();
            drop(cfg_guard);
            cfg.spotify_token = spotify_token;
            *CONFIG.lock().await = Some(cfg.clone());
            confy::store("s2yt", None, &cfg).unwrap();
            // Give response
            HttpResponse::Ok()
                .content_type("text/html")
                .body("<img src=\"https://media.tenor.com/JS6Vtap-SYEAAAAC/wwe-wrestling.gif\" />")
        }
        Err(e) => {
            eprintln!("Could not authorize with Spotify due to error: {}", e);
            eprintln!("Stopping...");
            std::process::exit(1);
        }
    }
}

async fn ensure_spotify() {
    let cfg_guard = CONFIG.lock().await;
    let mut cfg = (*cfg_guard.as_ref().unwrap()).to_owned();
    drop(cfg_guard);
    // Ensure Client ID
    let mut client_id = cfg.spotify_client_id.clone();
    if client_id.is_empty() {
        println!("\n\nEnter your Spotify client ID (Required only once):");
        std::io::stdin().read_line(&mut client_id).unwrap();
        client_id = client_id.trim().to_string();
        cfg.spotify_client_id = client_id.clone();
        *CONFIG.lock().await = Some(cfg.clone());
    }
    // Setup Spotify
    let creds = Credentials::new_pkce(client_id.as_str());
    let oauth = OAuth {
        redirect_uri: "http://localhost:8888/callback".to_string(),
        scopes: scopes!("user-read-currently-playing"),
        ..Default::default()
    };
    let config = rspotify::Config {
        token_refreshing: true,
        ..Default::default()
    };
    let mut spotify = AuthCodePkceSpotify::with_config(creds.clone(), oauth.clone(), config);
    // Check for existing refresh token and use it if possible
    let token = cfg.spotify_token.clone();
    let mut token_loaded = false;
    if let Some(_) = token {
        *spotify.token.lock().await.unwrap() = token.clone();
        match spotify.refresh_token().await {
            Ok(_) => {
                println!("Refreshed Spotify token successfully. No need to re-authenticate.");
                token_loaded = true;
                // Save new token
                let spotify_token = spotify.get_token().lock().await.unwrap().clone();
                cfg.spotify_token = spotify_token;
                *CONFIG.lock().await = Some(cfg.clone());
                confy::store("s2yt", None, &cfg).unwrap();
            }
            Err(_) => {
                *spotify.token.lock().await.unwrap() = None;
                eprintln!("Failed to refresh Spotify token. Reauthenticating.")
            }
        };
    };
    // No refresh token, so we need to get one
    if !token_loaded {
        let url = spotify.get_authorize_url(None).unwrap();
        println!(
            "\n\nAuthenticating with Spotify.\nCheck your browser, or click this link:\n{}",
            url
        );
        open::that(url).unwrap();
    }
    // Store spotify instance
    *SPOTIFY.lock().await = Some(spotify);
}
