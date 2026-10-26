//! Tauri shell: app state and the commands the UI calls. Logic lives in `ankura-core`.

mod downloads;
mod secrets;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use ankura_core::anki::{AnkiConnect, AnkiSource};
use ankura_core::dict::{self, DictEntry, DictionaryInfo};
use ankura_core::generate::{self, ChatOutcome, GenerateRequest, GenerateResult};
use ankura_core::knowledge::{KnowledgeSettings, WordState};
use ankura_core::llm::{HttpLlm, LlmConfig, Provider};
use ankura_core::mapping::{self, MappingPreview, Suggestion, SyncReport};
use ankura_core::reader::{self, ReaderDoc};
use ankura_core::store::{KnownWord, NewEvent, NoteTypeMapping, Store, StudyWord, TextMeta, WordStats};
use ankura_core::tokenize::SudachiTokenizer;
use ankura_core::WordKey;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub struct AppState {
    store: Mutex<Store>,
    tokenizer: RwLock<Option<Arc<SudachiTokenizer>>>,
    data_dir: PathBuf,
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl AppState {
    fn tokenizer(&self) -> CmdResult<Arc<SudachiTokenizer>> {
        self.tokenizer
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| "The Japanese dictionary (SudachiDict) isn't set up yet. Open Settings to download it.".into())
    }

    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap()
    }

    fn setting(&self, key: &str) -> Option<String> {
        self.store().setting(key).ok().flatten()
    }

    fn llm_config(&self) -> LlmConfig {
        let d = LlmConfig::default();
        let provider = match self.setting("ai_provider").as_deref() {
            Some("open_router") => Provider::OpenRouter,
            _ => Provider::Anthropic,
        };
        let model = self.setting("ai_model").filter(|m| !m.trim().is_empty()).unwrap_or_else(|| match provider {
            Provider::Anthropic => d.model.clone(),
            Provider::OpenRouter => String::new(),
        });
        LlmConfig { provider, model, effort: self.setting("ai_effort").unwrap_or(d.effort) }
    }

    fn anki(&self) -> AnkiConnect {
        AnkiConnect::new(&self.setting("anki_url").unwrap_or_else(|| AnkiConnect::DEFAULT_URL.into()))
    }

    fn scope(&self) -> mapping::Scope {
        mapping::Scope {
            extra_query: self.setting("anki_query").unwrap_or_default(),
            include_suspended: self.knowledge_settings().count_suspended,
        }
    }

    fn knowledge_settings(&self) -> KnowledgeSettings {
        let d = KnowledgeSettings::default();
        let num = |k: &str, default: f64| self.setting(k).and_then(|v| v.parse().ok()).unwrap_or(default);
        KnowledgeSettings {
            known_days: num("known_days", d.known_days),
            fading_below: num("fading_below", d.fading_below),
            count_suspended: self.setting("count_suspended").is_some_and(|v| v == "true"),
        }
    }

    fn load_tokenizer(&self, path: &std::path::Path) -> CmdResult<()> {
        let t = SudachiTokenizer::load(path).map_err(err)?;
        *self.tokenizer.write().unwrap() = Some(Arc::new(t));
        Ok(())
    }
}

/// Runs blocking work off the UI thread.
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppState) -> CmdResult<T> + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(err)?
}

// --- status and settings ---

#[derive(Serialize)]
struct Status {
    tokenizer_ready: bool,
    sudachi_path: Option<String>,
    dictionaries: Vec<DictionaryInfo>,
    anki_url: String,
    anki_query: String,
    mappings: usize,
    knowledge: std::collections::HashMap<String, i64>,
    settings: KnowledgeSettings,
    data_dir: String,
}

#[tauri::command]
async fn app_status(app: AppHandle) -> CmdResult<Status> {
    blocking(app, |s| {
        let store = s.store();
        Ok(Status {
            tokenizer_ready: s.tokenizer.read().unwrap().is_some(),
            sudachi_path: store.setting("sudachi_dic").map_err(err)?,
            dictionaries: dict::dictionaries(&store).map_err(err)?,
            anki_url: store.setting("anki_url").map_err(err)?.unwrap_or_else(|| AnkiConnect::DEFAULT_URL.into()),
            anki_query: store.setting("anki_query").map_err(err)?.unwrap_or_default(),
            mappings: store.mappings().map_err(err)?.len(),
            knowledge: store.anki_summary().map_err(err)?,
            settings: {
                drop(store);
                s.knowledge_settings()
            },
            data_dir: s.data_dir.display().to_string(),
        })
    })
    .await
}

#[tauri::command]
async fn set_setting(app: AppHandle, key: String, value: String) -> CmdResult<()> {
    const ALLOWED: [&str; 9] = [
        "anki_url",
        "anki_query",
        "known_days",
        "fading_below",
        "count_suspended",
        "ai_provider",
        "ai_model",
        "ai_effort",
        "ai_mode",
    ];
    if !ALLOWED.contains(&key.as_str()) {
        return Err(format!("unknown setting {key}"));
    }
    blocking(app, move |s| s.store().set_setting(&key, &value).map_err(err)).await
}

#[tauri::command]
async fn set_sudachi_path(app: AppHandle, path: String) -> CmdResult<()> {
    blocking(app, move |s| {
        s.load_tokenizer(std::path::Path::new(&path))?;
        s.store().set_setting("sudachi_dic", &path).map_err(err)
    })
    .await
}

#[tauri::command]
async fn download_sudachi(app: AppHandle) -> CmdResult<String> {
    let handle = app.clone();
    blocking(app, move |s| {
        let path = downloads::sudachi_core(&handle, &s.data_dir.join("sudachi"))?;
        s.load_tokenizer(&path)?;
        let p = path.display().to_string();
        s.store().set_setting("sudachi_dic", &p).map_err(err)?;
        Ok(p)
    })
    .await
}

#[tauri::command]
async fn import_dictionary(app: AppHandle, path: String) -> CmdResult<DictionaryInfo> {
    blocking(app, move |s| dict::import_yomitan_zip(&mut s.store(), std::path::Path::new(&path)).map_err(err)).await
}

#[tauri::command]
async fn download_jmdict(app: AppHandle) -> CmdResult<DictionaryInfo> {
    let handle = app.clone();
    blocking(app, move |s| {
        let zip = downloads::jmdict(&handle, &s.data_dir.join("downloads"))?;
        let info = dict::import_yomitan_zip(&mut s.store(), &zip).map_err(err)?;
        let _ = std::fs::remove_file(zip);
        Ok(info)
    })
    .await
}

// --- texts and reading ---

#[tauri::command]
async fn list_texts(app: AppHandle) -> CmdResult<Vec<TextMeta>> {
    blocking(app, |s| s.store().list_texts().map_err(err)).await
}

#[tauri::command]
async fn add_text(app: AppHandle, title: String, body: String) -> CmdResult<TextMeta> {
    blocking(app, move |s| {
        let title = if title.trim().is_empty() {
            body.lines().find(|l| !l.trim().is_empty()).unwrap_or("Untitled").chars().take(30).collect()
        } else {
            title.trim().to_string()
        };
        s.store().add_text("ja", &title, &body, "paste").map_err(err)
    })
    .await
}

#[tauri::command]
async fn rename_text(app: AppHandle, id: String, title: String) -> CmdResult<()> {
    blocking(app, move |s| s.store().rename_text(&id, &title).map_err(err)).await
}

#[tauri::command]
async fn delete_text(app: AppHandle, id: String) -> CmdResult<()> {
    blocking(app, move |s| s.store().delete_text(&id).map_err(err)).await
}

#[derive(Serialize)]
struct OpenedText {
    meta: TextMeta,
    doc: ReaderDoc,
    /// Generation details for generated texts (targets, example sentences).
    extra: Option<serde_json::Value>,
}

#[tauri::command]
async fn open_text(app: AppHandle, id: String) -> CmdResult<OpenedText> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        let store = s.store();
        let text = store.open_text(&id).map_err(err)?.ok_or("This text no longer exists.")?;
        let doc = reader::analyze(&store, tok.as_ref(), &text.body).map_err(err)?;
        Ok(OpenedText { meta: text.meta, doc, extra: text.extra })
    })
    .await
}

#[derive(Serialize)]
struct WordInfo {
    entries: Vec<DictEntry>,
    readings: Vec<String>,
    anki: Option<KnownWord>,
    mark: Option<WordState>,
    stats: WordStats,
}

#[tauri::command]
async fn word_info(app: AppHandle, key: WordKey, surface: String) -> CmdResult<WordInfo> {
    blocking(app, move |s| {
        let store = s.store();
        // The form as written first (ぬいぐるみ), then the normalized spelling (縫い包み).
        let mut entries = dict::lookup(&store, &surface, &key.reading).map_err(err)?;
        if entries.is_empty() && surface != key.lemma {
            entries = dict::lookup(&store, &key.lemma, &key.reading).map_err(err)?;
        }
        Ok(WordInfo {
            entries,
            readings: dict::readings(&store, &key.lemma).map_err(err)?,
            anki: store.knowledge(&key).map_err(err)?,
            mark: store.word_override(&key).map_err(err)?,
            stats: store.word_stats(&key).map_err(err)?,
        })
    })
    .await
}

#[tauri::command]
async fn set_word_mark(app: AppHandle, key: WordKey, state: Option<WordState>) -> CmdResult<()> {
    blocking(app, move |s| s.store().set_word_override(&key, state).map_err(err)).await
}

#[tauri::command]
async fn record_events(app: AppHandle, events: Vec<NewEvent>) -> CmdResult<()> {
    blocking(app, move |s| s.store().record_events(&events).map_err(err)).await
}

// --- Anki ---

#[derive(Serialize)]
struct NoteType {
    name: String,
    id: i64,
    fields: Vec<String>,
    notes: usize,
    mapping: Option<NoteTypeMapping>,
}

#[tauri::command]
async fn anki_check(app: AppHandle) -> CmdResult<i64> {
    blocking(app, |s| s.anki().version().map_err(err)).await
}

#[tauri::command]
async fn anki_note_types(app: AppHandle) -> CmdResult<Vec<NoteType>> {
    blocking(app, |s| {
        let anki = s.anki();
        let mappings = s.store().mappings().map_err(err)?;
        let mut out = Vec::new();
        for (name, id) in anki.note_types().map_err(err)? {
            let notes = anki.find_notes(&format!("mid:{id}")).map_err(err)?.len();
            if notes == 0 {
                continue;
            }
            out.push(NoteType {
                fields: anki.field_names(&name).map_err(err)?,
                mapping: mappings.iter().find(|m| m.model_id == id).cloned(),
                name,
                id,
                notes,
            });
        }
        out.sort_by(|a, b| b.notes.cmp(&a.notes));
        Ok(out)
    })
    .await
}

#[tauri::command]
async fn anki_preview(app: AppHandle, mapping: NoteTypeMapping) -> CmdResult<MappingPreview> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        mapping::preview_mapping(&s.anki(), tok.as_ref(), &mapping, &s.scope(), 200).map_err(err)
    })
    .await
}

#[tauri::command]
async fn anki_suggest(app: AppHandle, model_id: i64, model_name: String, fields: Vec<String>) -> CmdResult<Option<Suggestion>> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        mapping::suggest_mapping(&s.anki(), tok.as_ref(), model_id, &model_name, &fields, &s.scope()).map_err(err)
    })
    .await
}

#[tauri::command]
async fn save_mapping(app: AppHandle, mapping: NoteTypeMapping) -> CmdResult<()> {
    blocking(app, move |s| s.store().save_mapping(&mapping).map_err(err)).await
}

#[tauri::command]
async fn delete_mapping(app: AppHandle, model_id: i64) -> CmdResult<()> {
    blocking(app, move |s| s.store().delete_mapping(model_id).map_err(err)).await
}

#[tauri::command]
async fn anki_sync(app: AppHandle) -> CmdResult<SyncReport> {
    blocking(app, |s| {
        let tok = s.tokenizer()?;
        let settings = s.knowledge_settings();
        let query = s.setting("anki_query").unwrap_or_default();
        let anki = s.anki();
        mapping::sync_anki(&anki, tok.as_ref(), &mut s.store(), &settings, &query).map_err(err)
    })
    .await
}

// --- AI-written texts ---

#[derive(Serialize)]
struct AiSettings {
    config: LlmConfig,
    has_key: bool,
    /// "api": Ankura calls the provider itself. "chat": copy & paste through claude.ai (ADR 0011).
    mode: String,
}

#[tauri::command]
async fn ai_settings(app: AppHandle) -> CmdResult<AiSettings> {
    blocking(app, |s| {
        let config = s.llm_config();
        let has_key = secrets::api_key(config.provider)?.is_some();
        let mode = match s.setting("ai_mode").as_deref() {
            Some(m @ ("api" | "chat")) => m.to_string(),
            _ if has_key => "api".into(),
            _ => "chat".into(),
        };
        Ok(AiSettings { config, has_key, mode })
    })
    .await
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProviderArg {
    Anthropic,
    OpenRouter,
}

#[tauri::command]
async fn set_api_key(app: AppHandle, provider: ProviderArg, key: String) -> CmdResult<()> {
    let provider = match provider {
        ProviderArg::Anthropic => Provider::Anthropic,
        ProviderArg::OpenRouter => Provider::OpenRouter,
    };
    blocking(app, move |_| secrets::set_api_key(provider, key.trim())).await
}

#[tauri::command]
async fn study_words(app: AppHandle) -> CmdResult<Vec<StudyWord>> {
    blocking(app, |s| s.store().study_words(40).map_err(err)).await
}

#[tauri::command]
async fn generate_text(app: AppHandle, request: GenerateRequest) -> CmdResult<GenerateResult> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        let config = s.llm_config();
        if config.model.trim().is_empty() {
            return Err("Choose a model for this provider in Settings.".into());
        }
        let key = secrets::api_key(config.provider)?.ok_or("Add an API key in Settings first.")?;
        let mut llm = HttpLlm::new(config, key);
        if let Ok(url) = std::env::var("ANKURA_LLM_URL") {
            llm = llm.with_url(&url); // development and tests: a stand-in provider
        }
        // Its own connection, so the app's shared one isn't locked while the model writes
        // (WAL mode lets both work at once).
        let store = Store::open(&s.data_dir.join("ankura.db")).map_err(err)?;
        generate::generate_text(&store, tok.as_ref(), &llm, &request).map_err(err)
    })
    .await
}

// --- AI-written texts by copy & paste through claude.ai (ADR 0011) ---

#[tauri::command]
async fn chat_prompt(app: AppHandle, request: GenerateRequest) -> CmdResult<String> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        let prep = generate::prepare(&s.store(), tok.as_ref(), &request).map_err(err)?;
        Ok(generate::chat_prompt(&prep))
    })
    .await
}

#[tauri::command]
async fn chat_submit(
    app: AppHandle,
    request: GenerateRequest,
    answer: String,
    attempts: usize,
    keep: bool,
) -> CmdResult<ChatOutcome> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        generate::submit_answer(&s.store(), tok.as_ref(), &request, &answer, attempts, keep).map_err(err)
    })
    .await
}

/// Opens claude.ai in the default browser. The URL is fixed, so nothing user-supplied reaches the shell.
#[tauri::command]
fn open_claude_ai() -> CmdResult<()> {
    const URL: &str = "https://claude.ai/new";
    #[cfg(target_os = "windows")]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", URL]).creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(URL);
        c
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(URL);
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| format!("couldn't open the browser ({e}); go to {URL} yourself"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = Store::open(&data_dir.join("ankura.db"))?;
            // Development and testing: preset dictionaries through the environment.
            let sudachi = std::env::var("ANKURA_SUDACHI_DIC").ok().or(store.setting("sudachi_dic")?);
            let state = AppState { store: Mutex::new(store), tokenizer: RwLock::new(None), data_dir };
            if let Some(path) = sudachi {
                if let Err(e) = state.load_tokenizer(std::path::Path::new(&path)) {
                    eprintln!("Sudachi dictionary at {path} didn't load: {e}");
                }
            }
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            set_setting,
            set_sudachi_path,
            download_sudachi,
            import_dictionary,
            download_jmdict,
            list_texts,
            add_text,
            rename_text,
            delete_text,
            open_text,
            word_info,
            set_word_mark,
            record_events,
            anki_check,
            anki_note_types,
            anki_preview,
            anki_suggest,
            save_mapping,
            delete_mapping,
            anki_sync,
            ai_settings,
            set_api_key,
            study_words,
            generate_text,
            chat_prompt,
            chat_submit,
            open_claude_ai,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ankura");
}
