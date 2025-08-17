#[macro_use]
extern crate rocket;
use chrono::DateTime;
use chrono::FixedOffset;
use env_logger;
use log::error;
use rocket::Request;
use rocket::State;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::http::uri::fmt::Path;
use rocket::request::FromSegments;
use rocket::response;
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::serde::json::serde_json;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, path::PathBuf};
use thiserror::Error;

#[derive(Responder)]
enum PathResponse {
    Directory(Json<Vec<FileSystemEntry>>),
    File(NamedFile),
}

#[derive(Error, Debug)]
pub enum SnapshotBrowserError {
    #[error("IO Error: {message}")]
    IoError {
        message: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Configuration Error: {0}")]
    ConfigError(String),
    #[error("No snapshot found for root: {0}")]
    NoSnapshotsFound(String),
    #[error("Failed to parse configuration: {0}")]
    ConfigParseError(#[from] serde_json::Error),
    #[error("Failed to parse timestamp: {0}")]
    TimestampParseError(#[from] chrono::ParseError),
    #[error("Filter error: {0}")]
    FilterError(String),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for SnapshotBrowserError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        // Log the error message
        error!("SnapshotBrowserError: {}", self);
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);

        match self {
            // in our simplistic example, we're happy to respond with the default 500 responder in all cases
            _ => Status::InternalServerError.respond_to(req),
        }
    }
}

#[derive(Serialize)]
struct SystemInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Debug)]
struct Segments {
    segments: Vec<String>,
}

impl FromSegments<'_> for Segments {
    type Error = SnapshotBrowserError;
    fn from_segments(segments: rocket::http::uri::Segments<'_, Path>) -> Result<Self, Self::Error> {
        let segments = segments
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        Ok(Segments { segments })
    }
}

// These environment variables are set by Cargo at build time.
const SYSTEM_NAME: &str = env!("CARGO_PKG_NAME");
const SYSTEM_VERSION: &str = env!("CARGO_PKG_VERSION");

#[get("/info")]
fn info() -> Json<SystemInfo> {
    Json(SystemInfo {
        name: SYSTEM_NAME,
        version: SYSTEM_VERSION,
    })
}

// Structure to hold the system configuration
// The system configuration consists of an array of snapshot roots.
// Each snapshot root is a path to a directory containing snapshots and a suffix for the snapshots.
// The system configuration is read from a file at startup.
#[derive(Deserialize)]
struct SystemConfig {
    snapshot_roots: HashMap<String, SnapshotRoot>,
}
#[derive(Deserialize)]
struct SnapshotRoot {
    path: String,
    suffix: String,
}

#[derive(Serialize)]
enum FileSystemEntryDetails {
    Directory {},
    File { size: u64 },
}

#[derive(Serialize)]
struct FileSystemEntry {
    name: String,
    details: FileSystemEntryDetails,
}

impl SystemConfig {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: SystemConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}

#[get("/roots")]
fn roots(config: &State<SystemConfig>) -> Json<Vec<String>> {
    Json(config.snapshot_roots.keys().cloned().collect())
}

// This function gets the latest snapshot by filtering the names of the snapshots in the root directory by the root suffix and then comparing the timestamps of the snapshots.
// The snapshot names are expected to be in the format "<root path>/<timestamp>_<suffix>".
// The function returns the path of the latest snapshot as a string.
fn get_latest_snapshot_path(root: &SnapshotRoot) -> Result<Option<String>, SnapshotBrowserError> {
    let path = PathBuf::from(&root.path);
    let entries = fs::read_dir(&path).map_err(|e| SnapshotBrowserError::IoError {
        message: format!("Failed to read directory: {}", path.display()),
        source: e,
    })?;
    let mut latest_snapshot: Option<(String, DateTime<FixedOffset>)> = None;

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    // Split the name on the first underscore to separate the timestamp from the suffix
                    // and check if it ends with the root suffix.
                    // This assumes the snapshot names are formatted as "<timestamp>_<suffix>".
                    // Parse the timestamp and compare it to find the latest snapshot.
                    let name_parts: Vec<&str> = name.splitn(2, '_').collect();
                    if name_parts.len() < 2 {
                        continue; // Skip if the name does not have the expected format
                    }
                    if name_parts[1].ends_with(&root.suffix) {
                        // Parse the timestamp part from an rfc3339 formatted string into a chrono DateTime.
                        let timestamp = DateTime::parse_from_rfc3339(name_parts[0])?;

                        if latest_snapshot
                            .as_ref()
                            .map_or(true, |(_, time)| timestamp > *time)
                        {
                            latest_snapshot = Some((name.to_string(), timestamp));
                        }
                    }
                }
            }
        }
    }

    match latest_snapshot {
        Some((name, _)) => Ok(Some(path.join(name).to_string_lossy().into_owned())),
        _ => Ok(None),
    }
}

#[get("/roots/<name>/path/<path..>?<hidden>")]
async fn paths(
    name: &str,
    path: Segments,
    config: &State<SystemConfig>,
    hidden: Option<bool>,
) -> Result<PathResponse, SnapshotBrowserError> {
    log::debug!("Received request for path: {:?} in root: {}", path, name);

    let hidden = hidden.unwrap_or(false);

    if let Some(segment) = path.segments.last() {
        if segment.starts_with('.') && !hidden {
            return Err(SnapshotBrowserError::FilterError(format!(
                "The requested path cannot be displayed due to filter settings: hidden = {}, path: {}",
                hidden,
                path.segments.join("/")
            )));
        }
    }

    let root = config
        .snapshot_roots
        .get(name)
        .ok_or(SnapshotBrowserError::ConfigError(format!(
            "Snapshot root '{}' not found in configuration",
            name
        )))?;
    let latest_snapshot_path = get_latest_snapshot_path(root)?
        .ok_or(SnapshotBrowserError::NoSnapshotsFound(name.into()))?;

    let full_path = PathBuf::from(latest_snapshot_path).join(path.segments.join("/"));

    if full_path.is_file() {
        // If the path is a file, return it as a NamedFile
        Ok(PathResponse::File(
            NamedFile::open(&full_path)
                .await
                .map_err(|e| SnapshotBrowserError::IoError {
                    message: format!("Failed to open file: {}", &full_path.to_string_lossy()),
                    source: e,
                })?,
        ))
    } else if full_path.is_dir() {
        match fs::read_dir(&full_path) {
            Ok(entries_iter) => {
                let mut entries = Vec::new();

                for entry in entries_iter.flatten() {
                    let file_type =
                        entry
                            .file_type()
                            .map_err(|e| SnapshotBrowserError::IoError {
                                message: format!(
                                    "Failed to get file type for entry: {}",
                                    entry.path().display()
                                ),
                                source: e,
                            })?;
                    let name = entry.file_name().into_string().map_err(|_| {
                        SnapshotBrowserError::ConfigError(format!(
                            "Failed to convert file name to string: {}",
                            entry.path().display()
                        ))
                    })?;

                    if file_type.is_dir() {
                        entries.push(FileSystemEntry {
                            name,
                            details: FileSystemEntryDetails::Directory {},
                        });
                    } else if file_type.is_file() {
                        let size = entry
                            .metadata()
                            .map_err(|e| SnapshotBrowserError::IoError {
                                message: format!(
                                    "Failed to get metadata for file: {}",
                                    entry.path().display()
                                ),
                                source: e,
                            })?
                            .len();
                        entries.push(FileSystemEntry {
                            name,
                            details: FileSystemEntryDetails::File { size },
                        });
                    }
                }
                Ok(PathResponse::Directory(entries.into()))
            }
            Err(e) => Err(SnapshotBrowserError::IoError {
                message: format!("Failed to read directory: {}", full_path.display()),
                source: e,
            }),
        }
    } else {
        Err(SnapshotBrowserError::ConfigError(format!(
            "Path '{}' is neither a file nor a directory",
            full_path.display()
        )))
    }
}

#[rocket::main]
async fn main() {
    env_logger::init();

    let config = SystemConfig::from_file(
        &env::var("SNAPSHOT_CONFIG_PATH")
            .expect("Failed to get SNAPSHOT_CONFIG_PATH environment variable"),
    )
    .expect("Failed to load config");

    let rocket = rocket::build()
        .mount("/", routes![info, roots, paths])
        .manage(config);
    if let Err(e) = rocket.launch().await {
        println!("Whoops! Rocket didn't launch!");
        // We drop the error to get a Rocket-formatted panic.
        drop(e);
    };
}
