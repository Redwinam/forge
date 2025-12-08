#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::Serialize;
use std::fs;
use std::path::Path;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[derive(Serialize, Clone)]
struct CosConfig {
    secret_id: String,
    secret_key: String,
    bucket: String,
    region: String,
    prefix: String,
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

#[tauri::command]
fn get_cos_config(env_path: String) -> Result<CosConfig, String> {
    let content = fs::read_to_string(&env_path).map_err(|_| format!("Could not read .env file at {}", env_path))?;
    
    let mut config_map = HashMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            config_map.insert(key.trim(), value.trim());
        }
    }
    
    Ok(CosConfig {
        secret_id: config_map.get("TENCENT_SECRET_ID").unwrap_or(&"").to_string(),
        secret_key: config_map.get("TENCENT_SECRET_KEY").unwrap_or(&"").to_string(),
        bucket: config_map.get("COS_BUCKET").unwrap_or(&"").to_string(),
        region: config_map.get("COS_REGION").unwrap_or(&"").to_string(),
        prefix: config_map.get("COS_PREFIX").unwrap_or(&"press/").to_string(),
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_files, read_file, write_file, get_cos_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
