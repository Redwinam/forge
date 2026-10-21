use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::{Store, StoreExt};

/// Kept from when the webview used the store plugin directly, so existing settings still load.
pub const STORE_FILE: &str = ".settings.dat";

const ROOT_DIR: &str = "root_dir";
const LAST_OPENED_FILE: &str = "last_opened_file";
const SIDEBAR_WIDTH: &str = "sidebar_width";

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub root_dir: Option<String>,
    pub last_opened_file: Option<String>,
    pub sidebar_width: Option<f64>,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CosSettings {
    pub secret_id: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: String,
    pub cdn_domain: String,
    pub prefix: String,
}

const DEFAULT_COS_PREFIX: &str = "press/";

/// Where the settings store lives; relative paths resolve inside the app data directory.
pub struct SettingsFile(pub PathBuf);

pub fn open<R: Runtime>(app: &AppHandle<R>) -> Result<Arc<Store<R>>, String> {
    let file = app.state::<SettingsFile>();
    app.store(&file.0).map_err(|e| e.to_string())
}

fn get_string<R: Runtime>(store: &Store<R>, key: &str) -> Option<String> {
    store.get(key).and_then(|v| v.as_str().map(String::from))
}

pub fn root_dir<R: Runtime>(store: &Store<R>) -> Option<PathBuf> {
    get_string(store, ROOT_DIR).filter(|s| !s.is_empty()).map(PathBuf::from)
}

pub fn set_root_dir<R: Runtime>(store: &Store<R>, dir: &Path) -> Result<(), String> {
    store.set(ROOT_DIR, json!(dir.to_string_lossy()));
    store.save().map_err(|e| e.to_string())
}

/// Returns the root to use on startup. Earlier versions kept the root in the webview's
/// localStorage; that value is adopted once if nothing is configured here yet, after which
/// only a folder the user picks in the native dialog can change it.
pub fn initial_root_dir(configured: Option<PathBuf>, legacy: Option<&str>) -> Option<PathBuf> {
    configured.or_else(|| {
        legacy
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute() && dir.is_dir())
    })
}

pub fn load_app_state<R: Runtime>(store: &Store<R>, legacy_root_dir: Option<&str>) -> Result<AppState, String> {
    let configured = root_dir(store);
    let root = initial_root_dir(configured.clone(), legacy_root_dir);
    if configured.is_none() {
        if let Some(root) = &root {
            set_root_dir(store, root)?;
        }
    }
    Ok(AppState {
        root_dir: root.map(|p| p.to_string_lossy().into_owned()),
        last_opened_file: get_string(store, LAST_OPENED_FILE),
        sidebar_width: store.get(SIDEBAR_WIDTH).and_then(|v| v.as_f64()),
    })
}

pub fn set_last_opened_file<R: Runtime>(store: &Store<R>, path: Option<&str>) -> Result<(), String> {
    match path {
        Some(path) => store.set(LAST_OPENED_FILE, json!(path)),
        None => {
            store.delete(LAST_OPENED_FILE);
        }
    }
    store.save().map_err(|e| e.to_string())
}

pub fn set_sidebar_width<R: Runtime>(store: &Store<R>, width: f64) -> Result<(), String> {
    store.set(SIDEBAR_WIDTH, json!(width));
    store.save().map_err(|e| e.to_string())
}

pub fn cos_settings<R: Runtime>(store: &Store<R>) -> CosSettings {
    CosSettings {
        secret_id: get_string(store, "cos_secret_id").unwrap_or_default(),
        secret_key: get_string(store, "cos_secret_key").unwrap_or_default(),
        bucket: get_string(store, "cos_bucket").unwrap_or_default(),
        region: get_string(store, "cos_region").unwrap_or_default(),
        cdn_domain: get_string(store, "cos_cdn_domain").unwrap_or_default(),
        prefix: get_string(store, "cos_prefix").unwrap_or_else(|| DEFAULT_COS_PREFIX.to_string()),
    }
}

pub fn save_cos_settings<R: Runtime>(store: &Store<R>, settings: &CosSettings) -> Result<(), String> {
    for (key, value) in [
        ("cos_secret_id", &settings.secret_id),
        ("cos_secret_key", &settings.secret_key),
        ("cos_bucket", &settings.bucket),
        ("cos_region", &settings.region),
        ("cos_cdn_domain", &settings.cdn_domain),
        ("cos_prefix", &settings.prefix),
    ] {
        store.set(key, Value::String(value.trim().to_string()));
    }
    store.save().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_configured_root() {
        let dir = tempfile::tempdir().unwrap();
        let configured = Some(PathBuf::from("/configured"));
        let legacy = dir.path().to_str();
        assert_eq!(initial_root_dir(configured.clone(), legacy), configured);
    }

    #[test]
    fn adopts_an_existing_legacy_root_once() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(initial_root_dir(None, dir.path().to_str()), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn ignores_a_legacy_root_that_is_missing_or_relative() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");
        assert_eq!(initial_root_dir(None, missing.to_str()), None);
        assert_eq!(initial_root_dir(None, Some("relative/notes")), None);
        assert_eq!(initial_root_dir(None, None), None);
    }
}
