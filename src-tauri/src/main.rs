#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use reqwest::blocking::Client;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use chrono::{Utc};
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

mod files;
mod settings;

use files::FileEntry;
use settings::{AppState, CosSettings};

/// Every file command goes through this: the path must resolve to somewhere inside the
/// notes folder the user picked.
fn resolve_in_notes<R: Runtime>(app: &AppHandle<R>, path: &str) -> Result<PathBuf, String> {
    let store = settings::open(app)?;
    let root = settings::root_dir(&store).ok_or("No notes folder selected")?;
    files::resolve_in_root(&root, Path::new(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_files<R: Runtime>(app: AppHandle<R>, path: String) -> Result<Vec<FileEntry>, String> {
    resolve_in_notes(&app, &path)?;
    files::list_dir(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_file<R: Runtime>(app: AppHandle<R>, path: String) -> Result<String, String> {
    let path = resolve_in_notes(&app, &path)?;
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file<R: Runtime>(app: AppHandle<R>, path: String, content: String) -> Result<(), String> {
    let path = resolve_in_notes(&app, &path)?;
    files::write_atomic(&path, content.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_app_state<R: Runtime>(app: AppHandle<R>, legacy_root_dir: Option<String>) -> Result<AppState, String> {
    settings::load_app_state(&*settings::open(&app)?, legacy_root_dir.as_deref())
}

/// The root can only change through the native folder picker, so a compromised webview
/// can't point the file commands somewhere else.
#[tauri::command]
async fn choose_root_dir<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, String> {
    let picker = app.dialog().file().set_title("Choose your notes folder");
    let picked = tauri::async_runtime::spawn_blocking(move || picker.blocking_pick_folder())
        .await
        .map_err(|e| e.to_string())?;
    let Some(dir) = picked else { return Ok(None) };
    let dir = dir.into_path().map_err(|e| e.to_string())?;
    settings::set_root_dir(&*settings::open(&app)?, &dir)?;
    Ok(Some(dir.to_string_lossy().into_owned()))
}

#[tauri::command]
fn set_last_opened_file<R: Runtime>(app: AppHandle<R>, path: Option<String>) -> Result<(), String> {
    settings::set_last_opened_file(&*settings::open(&app)?, path.as_deref())
}

#[tauri::command]
fn set_sidebar_width<R: Runtime>(app: AppHandle<R>, width: f64) -> Result<(), String> {
    settings::set_sidebar_width(&*settings::open(&app)?, width)
}

#[tauri::command]
fn get_cos_settings<R: Runtime>(app: AppHandle<R>) -> Result<CosSettings, String> {
    Ok(settings::cos_settings(&*settings::open(&app)?))
}

#[tauri::command]
fn save_cos_settings<R: Runtime>(app: AppHandle<R>, settings: CosSettings) -> Result<(), String> {
    settings::save_cos_settings(&*settings::open(&app)?, &settings)
}

#[allow(clippy::too_many_arguments)] // mirrors the fields of the COS signing spec
fn calculate_signature(secret_id: &str, secret_key: &str, method: &str, uri: &str, params: &HashMap<String, String>, headers: &HashMap<String, String>, start_timestamp: i64, end_timestamp: i64) -> String {
    let q_sign_algorithm = "sha1";
    let q_ak = secret_id;
    let q_sign_time = format!("{};{}", start_timestamp, end_timestamp);
    let q_key_time = format!("{};{}", start_timestamp, end_timestamp);
    
    let mut header_keys: Vec<&String> = headers.keys().collect();
    header_keys.sort();
    let q_header_list = header_keys.iter().map(|k| k.to_lowercase()).collect::<Vec<_>>().join(";");
    
    let mut url_param_keys: Vec<&String> = params.keys().collect();
    url_param_keys.sort();
    let q_url_param_list = url_param_keys.iter().map(|k| k.to_lowercase()).collect::<Vec<_>>().join(";");
    
    let format_string = |map: &HashMap<String, String>, keys: &Vec<&String>| -> String {
        keys.iter()
            .map(|k| format!("{}={}", k.to_lowercase(), urlencoding::encode(map.get(*k).unwrap())))
            .collect::<Vec<_>>()
            .join("&")
    };
    
    let http_parameters = format_string(params, &url_param_keys);
    let http_headers = format_string(headers, &header_keys);
    
    let http_string = format!("{}\n{}\n{}\n{}\n", method.to_lowercase(), uri, http_parameters, http_headers);
    
    let string_to_sign = format!("{}\n{}\n{}\n", q_sign_algorithm, q_sign_time, sha1_hex(&http_string));
    
    let sign_key = hmac_sha1(secret_key, &q_key_time);
    let signature = hmac_sha1(&sign_key, &string_to_sign);
    
    format!("q-sign-algorithm={}&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list={}&q-url-param-list={}&q-signature={}",
            q_sign_algorithm, q_ak, q_sign_time, q_key_time, q_header_list, q_url_param_list, signature)
}

fn sha1_hex(data: &str) -> String {
    use sha1::Digest;
    let mut hasher = Sha1::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

fn hmac_sha1(key: &str, data: &str) -> String {
    type HmacSha1 = Hmac<Sha1>;
    let mut mac = HmacSha1::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn upload_to_cos(app: &tauri::AppHandle, file_data: Vec<u8>, extension: &str, folder: &str) -> Result<String, String> {
    let CosSettings { secret_id, secret_key, bucket, region, cdn_domain, prefix } =
        settings::cos_settings(&*settings::open(app)?);

    if secret_id.is_empty() || secret_key.is_empty() || bucket.is_empty() || region.is_empty() {
        return Err("COS configuration is missing. Please check settings.".to_string());
    }

    // Calculate MD5
    let digest = md5::compute(&file_data);
    let hash = format!("{:x}", digest);
    let key = format!("{}{}/{}.{}", prefix, folder, hash, extension);
    
    // Check if file exists (HEAD request)
    let host = format!("{}.cos.{}.myqcloud.com", bucket, region);
    let uri = format!("/{}", key);
    let url = format!("https://{}{}", host, uri);
    
    let client = Client::new();
    
    // HEAD
    let now = Utc::now();
    let start_timestamp = now.timestamp();
    let end_timestamp = start_timestamp + 600;
    
    let mut headers = HashMap::new();
    headers.insert("Host".to_string(), host.clone());
    
    let signature = calculate_signature(&secret_id, &secret_key, "HEAD", &uri, &HashMap::new(), &headers, start_timestamp, end_timestamp);
    
    let head_res = client.head(&url)
        .header("Authorization", signature)
        .header("Host", &host)
        .send();

    if let Ok(res) = head_res {
        if res.status().is_success() {
            // File exists
            if cdn_domain.is_empty() {
                return Ok(format!("https://{}/{}", host, key));
            } else {
                return Ok(format!("https://{}/{}", cdn_domain, key));
            }
        }
    }

    // PUT
    let now = Utc::now();
    let start_timestamp = now.timestamp();
    let end_timestamp = start_timestamp + 600;
    let signature = calculate_signature(&secret_id, &secret_key, "PUT", &uri, &HashMap::new(), &headers, start_timestamp, end_timestamp);

    let res = client.put(&url)
        .header("Authorization", signature)
        .header("Host", &host)
        .body(file_data)
        .send()
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        if cdn_domain.is_empty() {
            Ok(format!("https://{}/{}", host, key))
        } else {
            Ok(format!("https://{}/{}", cdn_domain, key))
        }
    } else {
        Err(format!("Upload failed: {}", res.status()))
    }
}

// Uploads use blocking reqwest. Sync commands run on the main thread and would freeze
// the window for the whole upload, so run them on the blocking pool instead.
#[tauri::command]
async fn upload_image(app: tauri::AppHandle, file_data: Vec<u8>, extension: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || upload_to_cos(&app, file_data, &extension, "images"))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn upload_file(app: tauri::AppHandle, file_data: Vec<u8>, extension: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || upload_to_cos(&app, file_data, &extension, "files"))
        .await
        .map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            list_files,
            read_file,
            write_file,
            load_app_state,
            choose_root_dir,
            set_last_opened_file,
            set_sidebar_width,
            get_cos_settings,
            save_cos_settings,
            upload_image,
            upload_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
