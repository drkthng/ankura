//! Tauri shell: app state and the commands the UI calls. Logic lives in `ankura-core`.

mod downloads;
mod secrets;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use ankura_core::anki::{AnkiConnect, AnkiSource};
use ankura_core::cards::{self, CardTarget};
use ankura_core::dict::{self, DictEntry, DictionaryInfo};
use ankura_core::{interests, kanji, practice, primitives, proficiency, voicevox};
use ankura_core::generate::{self, ChatOutcome, GenerateRequest, GenerateResult};
use ankura_core::knowledge::{KnowledgeSettings, WordState};
use ankura_core::llm::{HttpLlm, LlmConfig, Provider};
use ankura_core::mapping::{self, MappingPreview, Suggestion, SyncReport};
use ankura_core::reader::{self, ReaderDoc};
use ankura_core::store::{
    CardDraft, KnownWord, NewCardDraft, NewEvent, NoteTypeMapping, Store, StudyWord, TextMeta, WordStats,
};
use ankura_core::tokenize::SudachiTokenizer;
use ankura_core::WordKey;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

pub struct AppState {
    store: Mutex<Store>,
    /// Set to stop a running primitive analysis after its current batch.
    stop_analysis: std::sync::atomic::AtomicBool,
    /// Set to stop preparing a practice test after its current round.
    stop_practice: std::sync::atomic::AtomicBool,
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

    /// The AI for automatic mode, or a message saying what's missing. Every request and answer
    /// is logged in the database (ADR 0016).
    fn llm(&self) -> CmdResult<ankura_core::memory::Recorded<HttpLlm>> {
        let config = self.llm_config();
        if config.model.trim().is_empty() {
            return Err("Choose a model for this provider in Settings.".into());
        }
        let key = secrets::api_key(config.provider)?.ok_or("This needs an AI API key; add one in Settings.")?;
        let mut llm = HttpLlm::new(config, key);
        if let Ok(url) = std::env::var("ANKURA_LLM_URL") {
            llm = llm.with_url(&url); // development and tests: a stand-in provider
        }
        Ok(ankura_core::memory::Recorded::new(llm, self.own_store()?))
    }

    /// Its own connection, so the app's shared one isn't locked while the AI writes (WAL mode lets
    /// both work at once).
    fn own_store(&self) -> CmdResult<Store> {
        Store::open(&self.data_dir.join("ankura.db")).map_err(err)
    }

    fn card_target(&self) -> CmdResult<CardTarget> {
        match (self.setting("card_note_type"), self.setting("card_deck")) {
            (Some(note_type), Some(deck)) if !note_type.is_empty() && !deck.is_empty() => Ok(CardTarget { note_type, deck }),
            _ => Err("Choose a note type and deck for new cards in Settings first.".into()),
        }
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
    const ALLOWED: [&str; 17] = [
        "anki_url",
        "anki_query",
        "known_days",
        "fading_below",
        "count_suspended",
        "ai_provider",
        "ai_model",
        "ai_effort",
        "ai_mode",
        "card_note_type",
        "card_deck",
        "cards_auto_add",
        "tts_engine",
        "tts_system_voice",
        "tts_style",
        "tts_speed",
        "voicevox_url",
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
        let doc = reader::analyze_text(&store, tok.as_ref(), &text.body, Some(&id)).map_err(err)?;
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

/// Corrects the reading of a written form, everywhere or (with `text_id`) in one text; `None` removes it.
#[tauri::command]
async fn set_reading_fix(app: AppHandle, surface: String, reading: Option<String>, text_id: Option<String>) -> CmdResult<()> {
    blocking(app, move |s| s.store().set_reading_fix("ja", &surface, reading.as_deref(), text_id.as_deref()).map_err(err)).await
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
        let llm = s.llm()?;
        generate::generate_text(&s.own_store()?, tok.as_ref(), &llm, &request).map_err(err)
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

// --- kanji (ADR 0013) ---

#[tauri::command]
async fn kanji_mappings(app: AppHandle) -> CmdResult<Vec<kanji::KanjiMapping>> {
    blocking(app, |s| kanji::mappings(&s.store()).map_err(err)).await
}

#[tauri::command]
async fn kanji_suggest(app: AppHandle, model_id: i64, model_name: String, fields: Vec<String>) -> CmdResult<Option<kanji::KanjiMapping>> {
    blocking(app, move |s| {
        let anki = s.anki();
        let mut ids = anki.find_notes(&format!("mid:{model_id}")).map_err(err)?;
        ids.truncate(30);
        let samples: Vec<_> = anki.notes_info(&ids).map_err(err)?.into_iter().map(|n| n.fields).collect();
        Ok(kanji::suggest_mapping(model_id, &model_name, &fields, &samples))
    })
    .await
}

#[tauri::command]
async fn save_kanji_mapping(app: AppHandle, mapping: kanji::KanjiMapping) -> CmdResult<()> {
    blocking(app, move |s| kanji::save_mapping(&s.store(), &mapping).map_err(err)).await
}

#[tauri::command]
async fn delete_kanji_mapping(app: AppHandle, model_id: i64) -> CmdResult<()> {
    blocking(app, move |s| kanji::delete_mapping(&s.store(), model_id).map_err(err)).await
}

#[tauri::command]
async fn kanji_card(app: AppHandle, kanji: String) -> CmdResult<kanji::KanjiCard> {
    blocking(app, move |s| kanji::card(&s.store(), &kanji).map_err(err)).await
}

#[tauri::command]
async fn kanji_new_story(app: AppHandle, kanji: String) -> CmdResult<kanji::NewStory> {
    blocking(app, move |s| {
        let llm = s.llm()?;
        kanji::new_story(&s.own_store()?, &llm, &kanji).map_err(err)
    })
    .await
}

/// Writes a story the user confirmed (ADR 0013: mnemonic field only, backed up, undoable).
#[tauri::command]
async fn kanji_save_story(app: AppHandle, kanji: String, story: String) -> CmdResult<usize> {
    blocking(app, move |s| {
        let anki = s.anki(); // before locking the store
        kanji::save_story(&s.store(), &anki, &kanji, &story).map_err(err)
    })
    .await
}

#[tauri::command]
async fn kanji_undo_story(app: AppHandle, kanji: String) -> CmdResult<usize> {
    blocking(app, move |s| {
        let anki = s.anki();
        kanji::undo_story(&s.store(), &anki, &kanji).map_err(err)
    })
    .await
}

// --- consistent primitive names (ADR 0013 point 5) ---

#[tauri::command]
async fn primitives_status(app: AppHandle) -> CmdResult<primitives::AnalysisStatus> {
    blocking(app, |s| primitives::status(&s.store()).map_err(err)).await
}

/// Analyses the stories not analysed yet; emits `primitives-progress` {done, total}.
#[tauri::command]
async fn primitives_analyse(app: AppHandle) -> CmdResult<primitives::AnalysisStatus> {
    let handle = app.clone();
    blocking(app, move |s| {
        use std::sync::atomic::Ordering;
        s.stop_analysis.store(false, Ordering::SeqCst);
        let llm = s.llm()?;
        let progress = |done: usize, total: usize| {
            let _ = handle.emit("primitives-progress", serde_json::json!({"done": done, "total": total}));
        };
        primitives::analyse(&s.own_store()?, &llm, progress, || s.stop_analysis.load(Ordering::SeqCst)).map_err(err)
    })
    .await
}

#[tauri::command]
fn primitives_stop(app: AppHandle) {
    app.state::<AppState>().stop_analysis.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
async fn primitives_reset(app: AppHandle) -> CmdResult<()> {
    blocking(app, |s| primitives::reset_analysis(&s.store()).map_err(err)).await
}

#[tauri::command]
async fn primitives_list(app: AppHandle) -> CmdResult<Vec<primitives::PrimitiveRow>> {
    blocking(app, |s| primitives::list(&s.store()).map_err(err)).await
}

#[tauri::command]
async fn primitives_approve(app: AppHandle, names: BTreeMap<String, Vec<String>>) -> CmdResult<()> {
    blocking(app, move |s| primitives::approve(&s.store(), &names).map_err(err)).await
}

#[tauri::command]
async fn primitives_candidates(app: AppHandle) -> CmdResult<Vec<primitives::Candidate>> {
    blocking(app, |s| primitives::candidates(&s.store()).map_err(err)).await
}

/// New versions of the stories (names only); nothing is written until the user saves each one.
#[tauri::command]
async fn primitives_rewrite(app: AppHandle, kanji: Vec<String>) -> CmdResult<Vec<primitives::Rewrite>> {
    blocking(app, move |s| {
        let llm = s.llm()?;
        primitives::rewrite(&s.own_store()?, &llm, &kanji).map_err(err)
    })
    .await
}

/// Saves a rewritten story (as `kanji_save_story`) and keeps its analysis current.
#[tauri::command]
async fn primitives_save(app: AppHandle, kanji: String, story: String) -> CmdResult<usize> {
    blocking(app, move |s| {
        let anki = s.anki();
        primitives::save_rewrite(&s.store(), &anki, &kanji, &story).map_err(err)
    })
    .await
}

// --- interest profile (ROADMAP M3) ---

#[tauri::command]
async fn get_interests(app: AppHandle) -> CmdResult<interests::Interests> {
    blocking(app, |s| interests::get(&s.store()).map_err(err)).await
}

#[tauri::command]
async fn set_interests(app: AppHandle, value: interests::Interests) -> CmdResult<()> {
    blocking(app, move |s| interests::set(&s.store(), &value).map_err(err)).await
}

/// One of the learner's topics, picked by weight, for a text without a topic.
#[tauri::command]
async fn pick_topic(app: AppHandle) -> CmdResult<Option<String>> {
    blocking(app, |s| Ok(interests::pick(&interests::get(&s.store()).map_err(err)?, interests::roll()))).await
}

#[tauri::command]
async fn suggest_interests(app: AppHandle) -> CmdResult<Vec<String>> {
    blocking(app, |s| {
        let llm = s.llm()?;
        interests::suggest(&s.own_store()?, &llm).map_err(err)
    })
    .await
}

// --- level check (ADR 0014) ---

#[derive(Serialize)]
struct TestIntro {
    coverage: Vec<proficiency::LevelCoverage>,
    suggested: u8,
    credit: &'static str,
    history: Vec<proficiency::PastTest>,
}

#[tauri::command]
async fn test_intro(app: AppHandle) -> CmdResult<TestIntro> {
    blocking(app, |s| {
        let store = s.store();
        let coverage = proficiency::coverage(&store).map_err(err)?;
        Ok(TestIntro {
            suggested: proficiency::suggested_start(&coverage),
            coverage,
            credit: proficiency::CREDIT,
            history: proficiency::history(&store).map_err(err)?,
        })
    })
    .await
}

#[tauri::command]
async fn test_start(app: AppHandle, level: u8) -> CmdResult<proficiency::Started> {
    blocking(app, move |s| proficiency::start(&s.store(), level).map_err(err)).await
}

#[tauri::command]
async fn test_answer(app: AppHandle, test_id: String, answers: Vec<proficiency::Answer>) -> CmdResult<()> {
    blocking(app, move |s| proficiency::answer(&s.store(), &test_id, &answers).map_err(err)).await
}

#[tauri::command]
async fn test_grammar(app: AppHandle, test_id: String, stage: u8) -> CmdResult<Vec<proficiency::ItemView>> {
    blocking(app, move |s| {
        let llm = s.llm()?;
        proficiency::grammar(&s.own_store()?, &llm, &test_id, stage).map_err(err)
    })
    .await
}

#[tauri::command]
async fn test_reading(app: AppHandle, test_id: String) -> CmdResult<Vec<proficiency::ItemView>> {
    blocking(app, move |s| {
        let llm = s.llm()?;
        proficiency::reading(&s.own_store()?, &llm, &test_id).map_err(err)
    })
    .await
}

#[tauri::command]
async fn test_finish(app: AppHandle, test_id: String) -> CmdResult<proficiency::Estimate> {
    blocking(app, move |s| proficiency::finish(&s.store(), &test_id).map_err(err)).await
}

// --- listening (ADR 0017) ---

#[derive(Serialize)]
struct SpeechSettings {
    /// 'auto' (VOICEVOX when it runs, else the computer's voices), 'system' or 'voicevox'.
    engine: String,
    system_voice: String,
    /// VOICEVOX voice (style id); None: the engine's first.
    style: Option<u32>,
    speed: f64,
    voicevox_url: String,
}

#[tauri::command]
async fn speech_settings(app: AppHandle) -> CmdResult<SpeechSettings> {
    blocking(app, |s| {
        Ok(SpeechSettings {
            engine: s.setting("tts_engine").filter(|v| !v.is_empty()).unwrap_or_else(|| "auto".into()),
            system_voice: s.setting("tts_system_voice").unwrap_or_default(),
            style: s.setting("tts_style").and_then(|v| v.parse().ok()),
            speed: s.setting("tts_speed").and_then(|v| v.parse().ok()).unwrap_or(1.0),
            voicevox_url: s.setting("voicevox_url").filter(|v| !v.trim().is_empty()).unwrap_or_else(|| voicevox::DEFAULT_URL.into()),
        })
    })
    .await
}

impl AppState {
    fn voicevox(&self) -> voicevox::Voicevox {
        voicevox::Voicevox::new(&self.setting("voicevox_url").filter(|v| !v.trim().is_empty()).unwrap_or_else(|| voicevox::DEFAULT_URL.into()))
    }
}

/// VOICEVOX's voices; an error when it isn't running.
#[tauri::command]
async fn voicevox_speakers(app: AppHandle) -> CmdResult<Vec<voicevox::Speaker>> {
    blocking(app, |s| s.voicevox().speakers().map_err(err)).await
}

/// WAV audio for a text from VOICEVOX, kept on disk for next time.
#[tauri::command]
async fn voicevox_speak(app: AppHandle, text: String, style: u32, speed: f64) -> CmdResult<tauri::ipc::Response> {
    blocking(app, move |s| {
        let bytes = s.voicevox().speak(&s.data_dir.join("audio").join("voicevox"), &text, style, speed).map_err(err)?;
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
}

// --- practice tests (ADR 0015) ---

#[derive(Serialize)]
struct PracticeIntro {
    suggested: u8,
    credit: &'static str,
    tests: Vec<practice::PastPractice>,
    layouts: Vec<(u8, Vec<practice::PartSpec>)>,
    /// Questions in the bank per level (ADR 0016).
    bank: std::collections::BTreeMap<u8, ankura_core::memory::BankCount>,
}

#[tauri::command]
async fn practice_intro(app: AppHandle) -> CmdResult<PracticeIntro> {
    blocking(app, |s| {
        let store = s.store();
        Ok(PracticeIntro {
            suggested: practice::suggested_level(&store).map_err(err)?,
            credit: proficiency::CREDIT,
            tests: practice::list(&store).map_err(err)?,
            layouts: (1..=5).rev().map(|l| (l, practice::layout(l, false))).collect(),
            bank: ankura_core::memory::bank_counts(&store, "practice").map_err(err)?,
        })
    })
    .await
}

#[tauri::command]
async fn practice_create(app: AppHandle, level: u8, half: bool, timed: bool, reuse: u8, repeat_seen: bool) -> CmdResult<String> {
    blocking(app, move |s| practice::create(&s.store(), level, half, timed, reuse, repeat_seen).map_err(err)).await
}

/// How many questions a new test would take from the bank and how many the AI would write.
#[tauri::command]
async fn practice_plan(app: AppHandle, level: u8, half: bool, reuse: u8, repeat_seen: bool) -> CmdResult<practice::Plan> {
    blocking(app, move |s| practice::plan(&s.store(), level, half, reuse, repeat_seen).map_err(err)).await
}

/// What the AI has produced and how often stored answers were reused (ADR 0016).
#[tauri::command]
async fn memory_stats(app: AppHandle) -> CmdResult<ankura_core::memory::MemoryStats> {
    blocking(app, |s| ankura_core::memory::stats(&s.store()).map_err(err)).await
}

/// Writes the test's questions; emits `practice-progress` {done, total} (question types).
#[tauri::command]
async fn practice_prepare(app: AppHandle, test_id: String) -> CmdResult<practice::Progress> {
    let handle = app.clone();
    blocking(app, move |s| {
        use std::sync::atomic::Ordering;
        s.stop_practice.store(false, Ordering::SeqCst);
        let llm = s.llm()?;
        // Without the tokenizer, the level-drift check is skipped.
        let tok = s.tokenizer().ok();
        let progress = |done: usize, total: usize| {
            let _ = handle.emit("practice-progress", serde_json::json!({"done": done, "total": total}));
        };
        practice::prepare(
            &s.own_store()?,
            &llm,
            tok.as_deref().map(|t| t as &dyn ankura_core::tokenize::Tokenizer),
            &test_id,
            progress,
            || s.stop_practice.load(Ordering::SeqCst),
        )
        .map_err(err)
    })
    .await
}

#[tauri::command]
fn practice_stop(app: AppHandle) {
    app.state::<AppState>().stop_practice.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
async fn practice_sheet(app: AppHandle, test_id: String) -> CmdResult<practice::Sheet> {
    blocking(app, move |s| practice::sheet(&s.store(), &test_id).map_err(err)).await
}

#[tauri::command]
async fn practice_answer(app: AppHandle, test_id: String, item_id: String, choice: Option<usize>) -> CmdResult<()> {
    blocking(app, move |s| practice::answer(&s.store(), &test_id, &item_id, choice).map_err(err)).await
}

#[tauri::command]
async fn practice_time(app: AppHandle, test_id: String, part: usize, used_secs: u64, submitted: bool) -> CmdResult<()> {
    blocking(app, move |s| practice::time(&s.store(), &test_id, part, used_secs, submitted).map_err(err)).await
}

#[tauri::command]
async fn practice_finish(app: AppHandle, test_id: String) -> CmdResult<practice::PracticeResult> {
    blocking(app, move |s| practice::finish(&s.store(), &test_id).map_err(err)).await
}

#[tauri::command]
async fn practice_review(app: AppHandle, test_id: String) -> CmdResult<Vec<practice::ReviewItem>> {
    blocking(app, move |s| practice::review(&s.store(), &test_id).map_err(err)).await
}

#[tauri::command]
async fn practice_report(app: AppHandle, test_id: String, item_id: String, reported: bool) -> CmdResult<practice::PracticeResult> {
    blocking(app, move |s| practice::report(&s.store(), &test_id, &item_id, reported).map_err(err)).await
}

#[tauri::command]
async fn practice_retry(app: AppHandle, test_id: String) -> CmdResult<String> {
    blocking(app, move |s| practice::retry(&s.store(), &test_id).map_err(err)).await
}

#[tauri::command]
async fn practice_delete(app: AppHandle, test_id: String) -> CmdResult<()> {
    blocking(app, move |s| practice::delete(&s.store(), &test_id).map_err(err)).await
}

// --- stats (ROADMAP M3) ---

/// Characters read in a paragraph that counted as read; `day` is the user's local date.
#[tauri::command]
async fn record_reading(app: AppHandle, day: String, chars: i64) -> CmdResult<()> {
    blocking(app, move |s| s.store().record_reading("ja", &day, chars).map_err(err)).await
}

#[tauri::command]
async fn reading_stats(app: AppHandle, today: String) -> CmdResult<ankura_core::stats::Stats> {
    blocking(app, move |s| ankura_core::stats::stats(&s.store(), "ja", &today).map_err(err)).await
}

#[tauri::command]
async fn simplify_sentence(app: AppHandle, sentence: String) -> CmdResult<generate::Simplified> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        let llm = s.llm()?;
        generate::simplify_sentence(&s.own_store()?, tok.as_ref(), &llm, &sentence).map_err(err)
    })
    .await
}

// --- phrases and Anki cards (ADR 0012) ---

#[tauri::command]
async fn phrase_info(app: AppHandle, selection: String) -> CmdResult<Option<reader::ReaderToken>> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        reader::phrase(&s.store(), tok.as_ref(), &selection).map_err(err)
    })
    .await
}

#[derive(Serialize)]
struct CardOptions {
    note_types: Vec<String>,
    decks: Vec<String>,
    note_type: Option<String>,
    deck: Option<String>,
    auto_add: bool,
}

/// What Settings needs for "Anki cards": Anki's note types and decks, and the current choice.
#[tauri::command]
async fn card_options(app: AppHandle) -> CmdResult<CardOptions> {
    blocking(app, |s| {
        let anki = s.anki();
        Ok(CardOptions {
            note_types: anki.note_types().map_err(err)?.into_iter().map(|(n, _)| n).collect(),
            decks: anki.deck_names().map_err(err)?,
            note_type: s.setting("card_note_type").filter(|v| !v.is_empty()),
            deck: s.setting("card_deck").filter(|v| !v.is_empty()),
            auto_add: s.setting("cards_auto_add").is_some_and(|v| v == "true"),
        })
    })
    .await
}

/// Field names of the note type new cards use (for filling a card by hand).
#[tauri::command]
async fn card_field_names(app: AppHandle) -> CmdResult<Vec<String>> {
    blocking(app, |s| s.anki().field_names(&s.card_target()?.note_type).map_err(err)).await
}

#[tauri::command]
async fn card_drafts(app: AppHandle) -> CmdResult<Vec<CardDraft>> {
    blocking(app, |s| s.store().card_drafts().map_err(err)).await
}

/// Fills a draft's fields, and adds it to Anki when "add without the list" is on.
fn fill_and_maybe_add(s: &AppState, id: &str) -> CmdResult<CardDraft> {
    let tok = s.tokenizer()?;
    let target = s.card_target()?;
    let store = s.own_store()?;
    let draft = store.card_draft(id).map_err(err)?.ok_or("This card is no longer in the list.")?;
    let fields = cards::fill_fields(&store, tok.as_ref(), &s.llm()?, &s.anki(), &draft, &target.note_type).map_err(err)?;
    store.set_card_fields(id, &fields).map_err(err)?;
    if s.setting("cards_auto_add").is_some_and(|v| v == "true") {
        cards::add_to_anki(&store, &s.anki(), id, &target).map_err(err)?;
    }
    store.card_draft(id).map_err(err)?.ok_or_else(|| "This card is no longer in the list.".into())
}

/// A new draft for a word, with the given sentence or (when none) a new one written for it.
#[tauri::command]
async fn card_create(
    app: AppHandle,
    key: WordKey,
    display: String,
    sentence: Option<String>,
    text_id: Option<String>,
) -> CmdResult<CardDraft> {
    blocking(app, move |s| {
        let (sentence, source) = match sentence {
            Some(x) => (x, "text"),
            None => {
                let tok = s.tokenizer()?;
                (cards::new_sentence(&s.own_store()?, tok.as_ref(), &s.llm()?, &key, &display, true).map_err(err)?, "new")
            }
        };
        let draft = s
            .store()
            .add_card_draft(&NewCardDraft { key, display, sentence, sentence_source: source.into(), text_id })
            .map_err(err)?;
        if s.setting("cards_auto_add").is_some_and(|v| v == "true") {
            return fill_and_maybe_add(s, &draft.id);
        }
        Ok(draft)
    })
    .await
}

#[tauri::command]
async fn cards_for_text(app: AppHandle, text_id: String) -> CmdResult<Vec<CardDraft>> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        cards::drafts_for_text(&s.store(), tok.as_ref(), &text_id).map_err(err)
    })
    .await
}

#[tauri::command]
async fn card_fill(app: AppHandle, id: String) -> CmdResult<CardDraft> {
    blocking(app, move |s| fill_and_maybe_add(s, &id)).await
}

#[tauri::command]
async fn card_new_sentence(app: AppHandle, id: String) -> CmdResult<CardDraft> {
    blocking(app, move |s| {
        let tok = s.tokenizer()?;
        let store = s.own_store()?;
        let d = store.card_draft(&id).map_err(err)?.ok_or("This card is no longer in the list.")?;
        // "New sentence" asks for a different one: always the AI.
        let sentence = cards::new_sentence(&store, tok.as_ref(), &s.llm()?, &d.key, &d.display, false).map_err(err)?;
        store.set_card_sentence(&id, &sentence, "new").map_err(err)?;
        store.card_draft(&id).map_err(err)?.ok_or_else(|| "This card is no longer in the list.".into())
    })
    .await
}

#[tauri::command]
async fn card_set_fields(app: AppHandle, id: String, fields: BTreeMap<String, String>) -> CmdResult<()> {
    blocking(app, move |s| s.store().set_card_fields(&id, &fields).map_err(err)).await
}

#[tauri::command]
async fn card_discard(app: AppHandle, id: String) -> CmdResult<()> {
    blocking(app, move |s| s.store().delete_card_draft(&id).map_err(err)).await
}

#[tauri::command]
async fn card_add(app: AppHandle, id: String) -> CmdResult<CardDraft> {
    blocking(app, move |s| {
        let target = s.card_target()?;
        let anki = s.anki(); // before locking the store: reading the Anki URL needs it too
        let store = s.store();
        cards::add_to_anki(&store, &anki, &id, &target).map_err(err)?;
        store.card_draft(&id).map_err(err)?.ok_or_else(|| "This card is no longer in the list.".into())
    })
    .await
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
            let state = AppState {
                store: Mutex::new(store),
                stop_analysis: std::sync::atomic::AtomicBool::new(false),
                stop_practice: std::sync::atomic::AtomicBool::new(false),
                tokenizer: RwLock::new(None),
                data_dir,
            };
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
            set_reading_fix,
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
            simplify_sentence,
            record_reading,
            test_intro,
            test_start,
            test_answer,
            test_grammar,
            test_reading,
            test_finish,
            speech_settings,
            voicevox_speakers,
            voicevox_speak,
            practice_intro,
            practice_create,
            practice_plan,
            memory_stats,
            practice_prepare,
            practice_stop,
            practice_sheet,
            practice_answer,
            practice_time,
            practice_finish,
            practice_review,
            practice_report,
            practice_retry,
            practice_delete,
            get_interests,
            set_interests,
            pick_topic,
            suggest_interests,
            kanji_mappings,
            kanji_suggest,
            save_kanji_mapping,
            delete_kanji_mapping,
            kanji_card,
            kanji_new_story,
            kanji_save_story,
            kanji_undo_story,
            primitives_status,
            primitives_analyse,
            primitives_stop,
            primitives_reset,
            primitives_list,
            primitives_approve,
            primitives_candidates,
            primitives_rewrite,
            primitives_save,
            reading_stats,
            phrase_info,
            card_options,
            card_field_names,
            card_drafts,
            card_create,
            cards_for_text,
            card_fill,
            card_new_sentence,
            card_set_fields,
            card_discard,
            card_add,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ankura");
}
