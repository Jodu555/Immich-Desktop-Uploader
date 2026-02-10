use chrono::{Datelike, Timelike, Utc};
use hex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tokio::time::{interval, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    server_url: String,
    api_key: String,
    paths: Vec<PathConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PathConfig {
    directory: String,
    #[serde(rename = "cronExpressions")]
    cron_expressions: Vec<String>,
    recursive: bool,
}

#[derive(Debug, Clone)]
struct SchedulerState {
    running: bool,
    config: Option<Config>,
}

struct AppState {
    scheduler: Arc<Mutex<SchedulerState>>,
    http_client: Client,
}

#[derive(Clone, Serialize)]
struct UploadEvent {
    r#type: String,
    message: String,
}

#[tauri::command]
async fn test_immich_connection(server_url: String, api_key: String) -> Result<bool, String> {
    let client = Client::new();
    let url = format!("{}/api/server/statistics", server_url.trim_end_matches('/'));

    let response = client
        .get(&url)
        .header("x-api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    Ok(response.status().is_success())
}

#[tauri::command]
async fn save_config(
    config: Config,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let app_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?;

    fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

    let config_path = app_dir.join("config.json");
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(config_path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    // Update the in-memory state
    let mut scheduler = state.scheduler.lock().unwrap();
    scheduler.config = Some(config);

    Ok(())
}

#[tauri::command]
async fn load_config(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<Config>, String> {
    let app_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?;

    let config_path = app_dir.join("config.json");

    if !config_path.exists() {
        return Ok(None);
    }

    let json =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: Config =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse config: {}", e))?;

    // Update the in-memory state
    let mut scheduler = state.scheduler.lock().unwrap();
    scheduler.config = Some(config.clone());

    Ok(Some(config))
}

#[tauri::command]
async fn status_scheduler(state: State<'_, AppState>) -> Result<bool, String> {
    let scheduler = {
        let scheduler = state.scheduler.lock().unwrap();
        scheduler.running
    };

    Ok(scheduler)
}

#[tauri::command]
async fn start_scheduler(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    let config = {
        let scheduler = state.scheduler.lock().unwrap();
        scheduler.config.clone()
    };

    let config = config.ok_or("No configuration loaded")?;

    {
        let mut scheduler = state.scheduler.lock().unwrap();
        if scheduler.running {
            return Err("Scheduler already running".to_string());
        }
        scheduler.running = true;
        scheduler.config = Some(config.clone());
    }

    let scheduler_arc = state.scheduler.clone();
    let client = state.http_client.clone();

    tauri::async_runtime::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            let (running, config) = {
                let scheduler = scheduler_arc.lock().unwrap();
                (scheduler.running, scheduler.config.clone())
            };

            if !running {
                break;
            }

            if let Some(cfg) = config {
                check_and_upload(&app, &client, &cfg).await;
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_scheduler(state: State<'_, AppState>) -> Result<(), String> {
    let mut scheduler = state.scheduler.lock().unwrap();
    scheduler.running = false;
    Ok(())
}

#[tauri::command]
async fn trigger_upload(
    directory: String,
    recursive: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let config = {
        let scheduler = state.scheduler.lock().unwrap();
        scheduler.config.clone()
    };

    let config = config.ok_or("No configuration loaded")?;

    let path_config = PathConfig {
        directory,
        cron_expressions: vec![],
        recursive,
    };

    emit_event(
        &app,
        "info",
        &format!(
            "Triggering upload for {} (recursive: {})",
            path_config.directory, path_config.recursive
        ),
    );

    match upload_directory(&state.http_client, &config, &path_config).await {
        Ok(count) => {
            emit_event(
                &app,
                "success",
                &format!("Uploaded {} files from {}", count, path_config.directory),
            );
        }
        Err(e) => {
            emit_event(&app, "error", &format!("Upload failed: {}", e));
        }
    }

    Ok(())
}

async fn check_and_upload(app: &tauri::AppHandle, client: &Client, config: &Config) {
    for path_config in &config.paths {
        for cron_expr in &path_config.cron_expressions {
            if should_run_now(cron_expr) {
                emit_event(
                    app,
                    "info",
                    &format!("Starting upload for {}", path_config.directory),
                );

                match upload_directory(client, config, path_config).await {
                    Ok(count) => {
                        emit_event(
                            app,
                            "success",
                            &format!("Uploaded {} files from {}", count, path_config.directory),
                        );
                    }
                    Err(e) => {
                        emit_event(app, "error", &format!("Upload failed: {}", e));
                    }
                }
            }
        }
    }
}

fn should_run_now(cron_expr: &str) -> bool {
    // Simple CRON parser - Maybe use something more robust later maybe a crate that already exists
    // This is a simplified version that checks minute, hour, day, month, weekday
    let now = Utc::now();
    let parts: Vec<&str> = cron_expr.split_whitespace().collect();

    if parts.len() != 5 {
        return false;
    }

    let values = [
        now.minute(),
        now.hour(),
        now.day(),
        now.month(),
        now.weekday().num_days_from_sunday(),
    ];

    for (i, part) in parts.iter().enumerate() {
        if !matches_cron_field(part, values[i]) {
            return false;
        }
    }

    true
}

fn matches_cron_field(field: &str, value: u32) -> bool {
    if field == "*" {
        return true;
    }

    if let Ok(num) = field.parse::<u32>() {
        return num == value;
    }

    if field.starts_with("*/") {
        if let Ok(step) = field[2..].parse::<u32>() {
            return value % step == 0;
        }
    }

    false
}

async fn upload_directory(
    client: &Client,
    config: &Config,
    path_config: &PathConfig,
) -> Result<usize, String> {
    let path = PathBuf::from(&path_config.directory);
    let mut files = Vec::new();

    collect_image_files(&path, path_config.recursive, &mut files)?;

    if files.is_empty() {
        return Ok(0);
    }

    // println!("Found {} files", files.len());

    // Calculate checksums for all files
    let mut file_checksums = Vec::new();
    for file in &files {
        let data = fs::read(file).map_err(|e| format!("Failed to read file {:?}: {}", file, e))?;
        let mut hasher = Sha1::new();
        hasher.update(&data);
        // let checksum = digest(&data);
        let result = hasher.finalize().to_vec();
        // let checksum = match String::from_utf8(pre_check_sum) {
        //     Ok(v) => v,
        //     Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        // };
        let checksum = hex::encode(result);
        file_checksums.push((file.clone(), checksum, data));
    }

    // println!("Checked {} files", file_checksums.len());

    // Bulk check which files need to be uploaded
    let checksums_to_check: Vec<String> = file_checksums
        .iter()
        .map(|(_, checksum, _)| checksum.clone())
        .collect();

    let files_to_upload =
        bulk_check_assets(client, config, checksums_to_check, &file_checksums).await?;

    // println!("To Upload {} files", files_to_upload.len());

    // Upload only the files that don't exist
    let mut uploaded = 0;
    for (file_path, checksum, data) in files_to_upload {
        match upload_file_with_data(client, config, &file_path, data, checksum).await {
            Ok(true) => uploaded += 1,
            Ok(false) => {}
            Err(e) => eprintln!("Failed to upload {:?}: {}", file_path, e),
        }
    }

    Ok(uploaded)
}

async fn bulk_check_assets(
    client: &Client,
    config: &Config,
    checksums: Vec<String>,
    file_data: &[(PathBuf, String, Vec<u8>)],
) -> Result<Vec<(PathBuf, String, Vec<u8>)>, String> {
    let check_url = format!(
        "{}/api/assets/bulk-upload-check",
        config.server_url.trim_end_matches('/')
    );

    #[derive(Serialize)]
    struct BulkCheckRequest {
        assets: Vec<AssetCheckItem>,
    }

    #[derive(Serialize)]
    struct AssetCheckItem {
        id: String,
        checksum: String,
    }

    #[derive(Deserialize)]
    struct BulkCheckResponse {
        results: Vec<AssetCheckResult>,
    }

    #[derive(Deserialize, Debug)]
    struct AssetCheckResult {
        id: String,
        action: String,
        reason: Option<String>,
    }

    // Prepare the bulk check request
    let assets: Vec<AssetCheckItem> = checksums
        .iter()
        .enumerate()
        .map(|(idx, checksum)| AssetCheckItem {
            id: format!("file_{}", idx),
            checksum: checksum.clone(),
        })
        .collect();

    let request_body = BulkCheckRequest { assets };

    let response = client
        .post(&check_url)
        .header("x-api-key", &config.api_key)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Bulk check request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Bulk check failed with status {}: {}",
            status, body
        ));
    }

    let check_response: BulkCheckResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse bulk check response: {}", e))?;

    // Filter files that need to be uploaded (action == "accept")
    let mut files_to_upload = Vec::new();
    for result in check_response.results {
        // println!("Result: {:?}", result);
        if result.action == "accept" {
            // Extract the index from the id
            if let Some(idx_str) = result.id.strip_prefix("file_") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if idx < file_data.len() {
                        files_to_upload.push(file_data[idx].clone());
                    }
                }
            }
        }
    }

    Ok(files_to_upload)
}

fn collect_image_files(
    path: &PathBuf,
    recursive: bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let extensions = ["jpg", "jpeg", "png", "gif", "heic", "webp", "tiff"];

    let entries = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap_or("").to_lowercase().as_str()) {
                    files.push(path);
                }
            }
        } else if path.is_dir() && recursive {
            collect_image_files(&path, recursive, files)?;
        }
    }

    Ok(())
}

async fn upload_file_with_data(
    client: &Client,
    config: &Config,
    file_path: &PathBuf,
    data: Vec<u8>,
    checksum: String,
) -> Result<bool, String> {
    // Get file metadata
    let metadata =
        fs::metadata(file_path).map_err(|e| format!("Failed to get file metadata: {}", e))?;

    let file_size = metadata.len();

    // Get file modified time
    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modified time: {}", e))?;
    let modified_time: chrono::DateTime<Utc> = modified.into();

    // Get file created time (use modified as fallback)
    let created = metadata.created().unwrap_or(modified);
    let created_time: chrono::DateTime<Utc> = created.into();

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Create deviceAssetId: filename-filesize with no spaces
    let device_asset_id = format!("{}-{}", file_name, file_size).replace(char::is_whitespace, "");

    // Upload file
    let upload_url = format!("{}/api/assets", config.server_url.trim_end_matches('/'));

    let form = reqwest::multipart::Form::new()
        .text("deviceAssetId", device_asset_id)
        .text("deviceId", "ImmichAutoUploader")
        .text("fileCreatedAt", created_time.to_rfc3339())
        .text("fileModifiedAt", modified_time.to_rfc3339())
        .text("isFavorite", "false")
        .part(
            "assetData",
            reqwest::multipart::Part::bytes(data).file_name(file_name.to_string()),
        );

    let response = client
        .post(&upload_url)
        .header("x-api-key", &config.api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Upload request failed: {}", e))?;

    if response.status().is_success() {
        Ok(true)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Upload failed with status {}: {}", status, body))
    }
}

fn emit_event(app: &tauri::AppHandle, event_type: &str, message: &str) {
    let _ = app.emit(
        "upload-event",
        UploadEvent {
            r#type: event_type.to_string(),
            message: message.to_string(),
        },
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState {
                scheduler: Arc::new(Mutex::new(SchedulerState {
                    running: false,
                    config: None,
                })),
                http_client: Client::new(),
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            test_immich_connection,
            save_config,
            load_config,
            status_scheduler,
            start_scheduler,
            stop_scheduler,
            trigger_upload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
