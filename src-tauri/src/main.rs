#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::fs;
use std::path::{Path, PathBuf};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;

mod cos;
mod files;
mod secrets;
mod settings;

use cos::UploadKind;
use files::FileEntry;
use secrets::{Keychain, Secrets};
use settings::{AppState, CosSettingsInput, CosSettingsView};

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
    let documents = app.path().document_dir().ok();
    settings::load_app_state(&*settings::open(&app)?, legacy_root_dir.as_deref(), documents)
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
fn get_cos_settings<R: Runtime>(app: AppHandle<R>) -> Result<CosSettingsView, String> {
    Ok(settings::cos_settings_view(&*settings::open(&app)?))
}

#[tauri::command]
fn save_cos_settings<R: Runtime>(app: AppHandle<R>, secrets: State<'_, Secrets>, settings: CosSettingsInput) -> Result<(), String> {
    settings::save_cos_settings(&*settings::open(&app)?, &secrets, &settings)
}

#[tauri::command]
fn clear_cos_credentials<R: Runtime>(app: AppHandle<R>, secrets: State<'_, Secrets>) -> Result<(), String> {
    settings::clear_cos_credentials(&*settings::open(&app)?, &secrets)
}

/// Header carrying the file extension next to the raw upload body.
const EXTENSION_HEADER: &str = "x-forge-extension";

// The webview sends the file as a raw IPC body (an ArrayBuffer), not as a JSON array of numbers.
fn upload_payload(request: &Request<'_>, kind: UploadKind) -> Result<(Vec<u8>, String), String> {
    let InvokeBody::Raw(data) = request.body() else {
        return Err("Upload must be sent as raw bytes".to_string());
    };
    let extension = request
        .headers()
        .get(EXTENSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let extension = cos::validate_extension(extension, kind)?;
    cos::validate_size(data.len(), kind)?;
    Ok((data.clone(), extension))
}

async fn upload<R: Runtime>(app: AppHandle<R>, request: Request<'_>, kind: UploadKind) -> Result<String, String> {
    let (data, extension) = upload_payload(&request, kind)?;
    let settings = settings::cos_settings(&*settings::open(&app)?, &app.state::<Secrets>())?;
    // Uploads use blocking reqwest. Sync commands run on the main thread and would freeze
    // the window for the whole upload, so run them on the blocking pool instead.
    tauri::async_runtime::spawn_blocking(move || cos::upload(&settings, data, &extension, kind))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn upload_image<R: Runtime>(app: AppHandle<R>, request: Request<'_>) -> Result<String, String> {
    upload(app, request, UploadKind::Image).await
}

#[tauri::command]
async fn upload_file<R: Runtime>(app: AppHandle<R>, request: Request<'_>) -> Result<String, String> {
    upload(app, request, UploadKind::File).await
}

fn on_startup<R: Runtime>(app: &AppHandle<R>) {
    let migrated = settings::open(app)
        .and_then(|store| settings::migrate_plaintext_credentials(&store, &app.state::<Secrets>()));
    if let Err(e) = migrated {
        // The plain-text keys stay where they are and the move is retried on the next launch
        eprintln!("Could not move the COS keys into the system keychain: {e}");
    }
}

/// Registers the commands the webview talks to, the store they keep settings in and the
/// credential store for the COS keys.
fn with_commands<R: Runtime>(builder: tauri::Builder<R>, settings_file: PathBuf, secrets: Secrets) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(settings::SettingsFile(settings_file))
        .manage(secrets)
        .setup(|app| {
            on_startup(app.handle());
            Ok(())
        })
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
            clear_cos_credentials,
            upload_image,
            upload_file
        ])
}

fn main() {
    let context = tauri::generate_context!();
    let secrets = Secrets(Box::new(Keychain::new(&context.config().identifier)));
    with_commands(tauri::Builder::default(), PathBuf::from(settings::STORE_FILE), secrets)
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .run(context)
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::MemoryStore;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tauri::ipc::CallbackFn;
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
    use tauri::webview::{InvokeRequest, WebviewWindow};

    struct TestApp {
        dir: tempfile::TempDir,
        app: tauri::App<MockRuntime>,
        webview: WebviewWindow<MockRuntime>,
        keychain: Arc<MemoryStore>,
    }

    impl TestApp {
        fn new() -> Self {
            Self::with_settings(json!({}), MemoryStore::default())
        }

        /// Starts the app on an existing settings file, as after an upgrade.
        fn with_settings(settings: Value, keychain: MemoryStore) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let settings_file = dir.path().join("settings.json");
            fs::write(&settings_file, settings.to_string()).unwrap();
            let keychain = Arc::new(keychain);
            let app = with_commands(mock_builder(), settings_file, Secrets(Box::new(keychain.clone())))
                .build(mock_context(noop_assets()))
                .unwrap();
            // The mock runtime has no event loop, so the setup hook never fires on its own
            on_startup(app.handle());
            let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
            TestApp { dir, app, webview, keychain }
        }

        fn settings_on_disk(&self) -> String {
            fs::read_to_string(self.dir.path().join("settings.json")).unwrap()
        }

        fn keychain_value(&self) -> Option<Value> {
            let value = self.keychain.value.lock().unwrap().clone();
            value.map(|v| serde_json::from_str(&v).unwrap())
        }

        /// A notes folder inside the temp dir, with a secret file next to it.
        fn with_notes(self) -> Self {
            let notes = self.dir.path().join("notes");
            fs::create_dir_all(&notes).unwrap();
            fs::write(notes.join("a.md"), "# A").unwrap();
            fs::write(self.dir.path().join("secret.txt"), "secret").unwrap();
            settings::set_root_dir(&*settings::open(self.app.handle()).unwrap(), &notes).unwrap();
            self
        }

        /// `rel` uses `/`; it is joined component by component so the result is a native path.
        fn path(&self, rel: &str) -> String {
            rel.split('/')
                .fold(self.dir.path().to_path_buf(), |path, part| path.join(part))
                .to_string_lossy()
                .into_owned()
        }

        fn invoke(&self, cmd: &str, body: InvokeBody, headers: &[(&'static str, &str)]) -> Result<Value, Value> {
            let mut header_map = tauri::http::HeaderMap::new();
            for (name, value) in headers {
                header_map.insert(*name, value.parse().unwrap());
            }
            get_ipc_response(
                &self.webview,
                InvokeRequest {
                    cmd: cmd.into(),
                    callback: CallbackFn(0),
                    error: CallbackFn(1),
                    url: "http://tauri.localhost".parse().unwrap(),
                    body,
                    headers: header_map,
                    invoke_key: INVOKE_KEY.to_string(),
                },
            )
            .map(|body| body.deserialize::<Value>().unwrap())
        }

        fn invoke_json(&self, cmd: &str, args: Value) -> Result<Value, Value> {
            self.invoke(cmd, InvokeBody::Json(args), &[])
        }
    }

    fn error_text(result: Result<Value, Value>) -> String {
        result.expect_err("expected the command to fail").to_string()
    }

    #[test]
    fn upload_accepts_raw_bytes() {
        let app = TestApp::new();
        // Validation passed and the request reached the COS step, which needs credentials
        let err = error_text(app.invoke("upload_image", InvokeBody::Raw(vec![1, 2, 3]), &[(EXTENSION_HEADER, "PNG")]));
        assert!(err.contains("COS configuration is missing"), "{err}");
    }

    #[test]
    fn upload_rejects_a_json_number_array() {
        let app = TestApp::new();
        let err = error_text(app.invoke_json("upload_image", json!({ "fileData": [1, 2, 3], "extension": "png" })));
        assert!(err.contains("raw bytes"), "{err}");
    }

    #[test]
    fn upload_validates_the_extension_and_size() {
        let app = TestApp::new();
        let raw = || InvokeBody::Raw(vec![1, 2, 3]);
        assert!(error_text(app.invoke("upload_image", raw(), &[(EXTENSION_HEADER, "exe")])).contains("Not an image"));
        assert!(error_text(app.invoke("upload_file", raw(), &[(EXTENSION_HEADER, "../x")])).contains("extension"));
        assert!(error_text(app.invoke("upload_file", raw(), &[])).contains("extension"));
        assert!(error_text(app.invoke("upload_file", InvokeBody::Raw(vec![]), &[(EXTENSION_HEADER, "pdf")])).contains("empty"));
    }

    #[test]
    fn file_commands_need_a_notes_folder() {
        let app = TestApp::new();
        let err = error_text(app.invoke_json("read_file", json!({ "path": app.path("secret.txt") })));
        assert!(err.contains("No notes folder"), "{err}");
    }

    #[test]
    fn file_commands_work_inside_the_notes_folder() {
        let app = TestApp::new().with_notes();
        assert_eq!(app.invoke_json("read_file", json!({ "path": app.path("notes/a.md") })).unwrap(), json!("# A"));
        app.invoke_json("write_file", json!({ "path": app.path("notes/a.md"), "content": "# B" })).unwrap();
        assert_eq!(fs::read_to_string(app.path("notes/a.md")).unwrap(), "# B");
        let listed = app.invoke_json("list_files", json!({ "path": app.path("notes") })).unwrap();
        assert_eq!(listed, json!([{ "name": "a.md", "path": app.path("notes/a.md"), "is_dir": false }]));
    }

    #[test]
    fn file_commands_refuse_paths_outside_the_notes_folder() {
        let app = TestApp::new().with_notes();
        for path in [app.path("secret.txt"), app.path("notes/../secret.txt")] {
            assert!(error_text(app.invoke_json("read_file", json!({ "path": path }))).contains("outside"));
            assert!(error_text(app.invoke_json("write_file", json!({ "path": path, "content": "x" }))).contains("outside"));
        }
        assert!(error_text(app.invoke_json("list_files", json!({ "path": app.path("") }))).contains("outside"));
        assert!(error_text(app.invoke_json("write_file", json!({ "path": app.path("new.md"), "content": "x" }))).contains("outside"));
        assert_eq!(fs::read_to_string(app.path("secret.txt")).unwrap(), "secret");
        assert!(!Path::new(&app.path("new.md")).exists());
    }

    const PLAINTEXT: &str = "plaintext-secret-key";

    fn old_settings() -> Value {
        json!({
            "cos_secret_id": "AKIDold",
            "cos_secret_key": PLAINTEXT,
            "cos_bucket": "bucket-1250000000",
            "cos_region": "ap-shanghai",
            "sidebar_width": 300
        })
    }

    #[test]
    fn moves_plaintext_keys_into_the_keychain_on_startup() {
        let app = TestApp::with_settings(old_settings(), MemoryStore::default());

        assert_eq!(app.keychain_value(), Some(json!({ "secretId": "AKIDold", "secretKey": PLAINTEXT })));
        let on_disk = app.settings_on_disk();
        assert!(!on_disk.contains(PLAINTEXT) && !on_disk.contains("AKIDold"), "{on_disk}");
        // Everything else is left alone
        let view = app.invoke_json("get_cos_settings", json!({})).unwrap();
        assert_eq!(view["bucket"], "bucket-1250000000");
        assert_eq!(view["hasCredentials"], true);
        assert_eq!(app.invoke_json("load_app_state", json!({})).unwrap()["sidebarWidth"], 300.0);
    }

    #[test]
    fn keeps_plaintext_keys_when_the_keychain_cannot_store_them() {
        let app = TestApp::with_settings(old_settings(), MemoryStore { fail_writes: true, ..Default::default() });

        assert_eq!(app.keychain_value(), None);
        assert!(app.settings_on_disk().contains(PLAINTEXT));
    }

    #[test]
    fn never_hands_the_keys_to_the_webview() {
        let app = TestApp::with_settings(old_settings(), MemoryStore::default());
        let view = app.invoke_json("get_cos_settings", json!({})).unwrap();
        assert!(!view.to_string().contains(PLAINTEXT), "{view}");
        assert!(!view.to_string().contains("AKIDold"), "{view}");
    }

    #[test]
    fn saves_new_keys_to_the_keychain_only() {
        let app = TestApp::new();
        let input = json!({ "bucket": "b-1", "region": "ap-guangzhou", "cdnDomain": "", "prefix": "notes/", "secretId": "AKIDnew", "secretKey": "new-key" });
        app.invoke_json("save_cos_settings", json!({ "settings": input })).unwrap();

        assert_eq!(app.keychain_value(), Some(json!({ "secretId": "AKIDnew", "secretKey": "new-key" })));
        assert!(!app.settings_on_disk().contains("new-key"));
        assert_eq!(app.invoke_json("get_cos_settings", json!({})).unwrap()["prefix"], "notes/");

        // Leaving the key fields empty keeps the saved keys
        let input = json!({ "bucket": "b-2", "region": "ap-guangzhou", "cdnDomain": "", "prefix": "notes/", "secretId": "", "secretKey": "" });
        app.invoke_json("save_cos_settings", json!({ "settings": input })).unwrap();
        assert_eq!(app.keychain_value(), Some(json!({ "secretId": "AKIDnew", "secretKey": "new-key" })));
        assert_eq!(app.invoke_json("get_cos_settings", json!({})).unwrap()["bucket"], "b-2");

        app.invoke_json("clear_cos_credentials", json!({})).unwrap();
        assert_eq!(app.keychain_value(), None);
        assert_eq!(app.invoke_json("get_cos_settings", json!({})).unwrap()["hasCredentials"], false);
    }

    #[test]
    fn adopts_the_legacy_root_only_while_none_is_configured() {
        let app = TestApp::new();
        let first = app.path("first");
        let second = app.path("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();

        let state = app.invoke_json("load_app_state", json!({ "legacyRootDir": first })).unwrap();
        assert_eq!(state["rootDir"], json!(first));
        // A later value from the webview can't move the root anymore
        let state = app.invoke_json("load_app_state", json!({ "legacyRootDir": second })).unwrap();
        assert_eq!(state["rootDir"], json!(first));
    }

    #[test]
    fn remembers_the_last_file_and_sidebar_width() {
        let app = TestApp::new().with_notes();
        let note = app.path("notes/a.md");
        app.invoke_json("set_last_opened_file", json!({ "path": note })).unwrap();
        app.invoke_json("set_sidebar_width", json!({ "width": 320.0 })).unwrap();

        let state = app.invoke_json("load_app_state", json!({})).unwrap();
        assert_eq!(state["lastOpenedFile"], json!(note));
        assert_eq!(state["sidebarWidth"], json!(320.0));
        assert!(app.settings_on_disk().contains("320"));

        app.invoke_json("set_last_opened_file", json!({ "path": null })).unwrap();
        assert_eq!(app.invoke_json("load_app_state", json!({})).unwrap()["lastOpenedFile"], Value::Null);
    }

    #[test]
    fn saves_notes_atomically_through_the_command() {
        let app = TestApp::new().with_notes();
        app.invoke_json("write_file", json!({ "path": app.path("notes/a.md"), "content": "# 新内容" })).unwrap();

        let names: Vec<_> = fs::read_dir(app.path("notes")).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, ["a.md"], "no temp file left behind");
        assert_eq!(fs::read_to_string(app.path("notes/a.md")).unwrap(), "# 新内容");
    }
}
