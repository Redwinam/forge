#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::fs;
use std::path::{Path, PathBuf};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

mod cos;
mod files;
mod settings;

use cos::UploadKind;
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
    let settings = settings::cos_settings(&*settings::open(&app)?);
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

/// Registers the commands the webview talks to and the store they keep settings in.
fn with_commands<R: Runtime>(builder: tauri::Builder<R>, settings_file: PathBuf) -> tauri::Builder<R> {
    builder
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(settings::SettingsFile(settings_file))
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
}

fn main() {
    with_commands(tauri::Builder::default(), PathBuf::from(settings::STORE_FILE))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use tauri::ipc::CallbackFn;
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY};
    use tauri::webview::{InvokeRequest, WebviewWindow};

    struct TestApp {
        dir: tempfile::TempDir,
        app: tauri::App<MockRuntime>,
        webview: WebviewWindow<MockRuntime>,
    }

    impl TestApp {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let app = with_commands(mock_builder(), dir.path().join("settings.json"))
                .build(mock_context(noop_assets()))
                .unwrap();
            let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
            TestApp { dir, app, webview }
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

        fn path(&self, rel: &str) -> String {
            self.dir.path().join(rel).to_string_lossy().into_owned()
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
}
