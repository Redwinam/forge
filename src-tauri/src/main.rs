#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use tauri::Wry;
use tauri_plugin_store::StoreExt;
use serde_json::Value;
use reqwest::blocking::Client;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use hex;
use chrono::{Utc, Duration};

#[derive(Serialize, Clone)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[tauri::command]
fn list_files(path: String) -> Result<Vec<FileEntry>, String> {
    let root = Path::new(&path);
    if !root.exists() {
        return Err("Path does not exist".to_string());
    }

    let mut entries = Vec::new();
    match fs::read_dir(root) {
        Ok(dir) => {
            for entry in dir {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    let is_dir = path.is_dir();
                    // Filter hidden files
                    if name.starts_with(".") { continue; }
                    
                    entries.push(FileEntry {
                        name,
                        path: path.to_string_lossy().to_string(),
                        is_dir,
                    });
                }
            }
        }
        Err(e) => return Err(e.to_string()),
    }
    
    entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.cmp(&b.name)
        } else {
            if a.is_dir { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
        }
    });
    
    Ok(entries)
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|e| e.to_string())
}

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

#[tauri::command]
fn upload_image(app: tauri::AppHandle, file_data: Vec<u8>, extension: String) -> Result<String, String> {
    let store = app.store(".settings.dat").map_err(|e| e.to_string())?;
    
    let secret_id = store.get("cos_secret_id").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
    let secret_key = store.get("cos_secret_key").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
    let bucket = store.get("cos_bucket").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
    let region = store.get("cos_region").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
    let prefix = store.get("cos_prefix").and_then(|v| v.as_str().map(String::from)).unwrap_or("press/".to_string());
    let cdn_domain = store.get("cos_cdn_domain").and_then(|v| v.as_str().map(String::from)).unwrap_or("cdn.if9.cool".to_string());

    if secret_id.is_empty() || secret_key.is_empty() || bucket.is_empty() || region.is_empty() {
        return Err("COS configuration is missing. Please check settings.".to_string());
    }

    // Calculate MD5
    let digest = md5::compute(&file_data);
    let hash = format!("{:x}", digest);
    let key = format!("{}images/{}.{}", prefix, hash, extension);
    
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
            return Ok(format!("https://{}/{}", cdn_domain, key));
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
        Ok(format!("https://{}/{}", cdn_domain, key))
    } else {
        Err(format!("Upload failed: {}", res.status()))
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![list_files, read_file, write_file, upload_image])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
