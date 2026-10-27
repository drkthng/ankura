// Typed wrappers around the Tauri commands in src-tauri/src/lib.rs.
import { invoke } from "@tauri-apps/api/core";

export type WordState = "unknown" | "new" | "learning" | "fading" | "known" | "marked_known" | "ignored";
export type TokenKind = "word" | "name" | "grammar" | "number" | "symbol" | "space";

export interface WordKey {
  lang: string;
  lemma: string;
  reading: string;
}

export interface Segment {
  text: string;
  ruby: string | null;
}

export interface ReaderToken {
  surface: string;
  /** As written in the text; key.lemma is the normalized spelling used for matching. */
  dictionary_form: string;
  segments: Segment[];
  kind: TokenKind;
  key: WordKey | null;
  state: WordState;
  source: "mark" | "anki" | "basic" | "parts" | "none";
  has_kanji: boolean;
}

export interface Coverage {
  words: number;
  known: number;
  learning: number;
  new: number;
  unknown: number;
  ratio: number;
}

export interface ReaderDoc {
  paragraphs: ReaderToken[][];
  coverage: Coverage;
}

export interface TextMeta {
  id: string;
  lang: string;
  title: string;
  source: string;
  created_at: number;
  opened_at: number | null;
  chars: number;
}

export interface DictionaryInfo {
  id: number;
  title: string;
  revision: string;
  terms: number;
}

export interface KnowledgeSettings {
  known_days: number;
  fading_below: number;
  count_suspended: boolean;
}

export interface Status {
  tokenizer_ready: boolean;
  sudachi_path: string | null;
  dictionaries: DictionaryInfo[];
  anki_url: string;
  anki_query: string;
  mappings: number;
  knowledge: Record<string, number>;
  settings: KnowledgeSettings;
  data_dir: string;
}

export interface DictEntry {
  expression: string;
  reading: string;
  senses: string[];
  tags: string;
  score: number;
  dictionary: string;
}

export interface WordInfo {
  entries: DictEntry[];
  readings: string[];
  anki: { state: WordState; strength: number; cards: number } | null;
  mark: WordState | null;
  stats: { exposures: number; lookups: number };
}

export type TargetMode = "field" | "marked";

export interface NoteTypeMapping {
  model_id: number;
  model_name: string;
  target_field: string;
  reading_field: string | null;
  /** "field": the whole field is the word. "marked": the bold/italic part of the field is. */
  target_mode: TargetMode;
}

export interface Suggestion {
  mapping: NoteTypeMapping;
  reason: string;
}

export interface NoteType {
  name: string;
  id: number;
  fields: string[];
  notes: number;
  mapping: NoteTypeMapping | null;
}

export type FieldOutcome = "word" | "phrase" | "empty";

export interface MappingPreview {
  notes_total: number;
  sampled: number;
  words: number;
  phrases: number;
  empty: number;
  examples: { note_id: number; field: string; outcome: FieldOutcome; keys: WordKey[] }[];
}

export interface SyncReport {
  notes: number;
  cards: number;
  words: number;
  phrases: number;
  empty_fields: number;
  fsrs: boolean;
  suspended: number;
  overdue: number;
  note_types: { model_name: string; notes: number; words: number; phrases: number; empty_fields: number }[];
  kanji: number;
}

export type Provider = "anthropic" | "open_router";

export interface LlmConfig {
  provider: Provider;
  model: string;
  effort: string;
}

export type AiMode = "api" | "chat";

export interface AiSettings {
  config: LlmConfig;
  has_key: boolean;
  /** "api": Ankura calls the AI itself. "chat": copy & paste through claude.ai. */
  mode: AiMode;
}

export interface StudyWord {
  key: WordKey;
  /** How to show the word (the card's or the text's spelling). */
  display: string;
  reason: "fading" | "learning" | "new" | "looked_up";
  lookups: number;
}

export interface GenerateRequest {
  targets: string[];
  topic: string;
  length: number;
  polite: boolean;
}

export interface Example {
  word: string;
  sentence: string;
  translation: string;
}

export interface GenerateResult {
  text: TextMeta;
  coverage: number;
  attempts: number;
  missing_targets: string[];
  unknown_words: string[];
  examples: Example[];
}

export interface ChatOutcome {
  coverage: number;
  missing_targets: string[];
  unknown_words: string[];
  /** The follow-up to paste when the reply wasn't good enough (and wasn't saved). */
  rewrite: string | null;
  saved: GenerateResult | null;
}

export interface KanjiMapping {
  model_id: number;
  model_name: string;
  kanji_field: string;
  keyword_field: string | null;
  /** The mnemonic: the only field Ankura may change (ADR 0013). */
  story_field: string | null;
}

export interface KanjiCard {
  kanji: string;
  info: { kanji: string; keyword: string; story: string; state: WordState; strength: number; notes: { note_id: number; model_id: number }[] } | null;
  story_text: string;
  words: { display: string; reading: string; state: WordState }[];
  last_edit: { id: string; old: string; new: string; created_at: number } | null;
}

export interface NewStory {
  story: string;
  primitives: { component: string; name: string }[];
}

export interface PrimitiveRow {
  component: string;
  names: { name: string; count: number; kanji: string[] }[];
  rtk_name: string;
  approved: string[];
  is_approved: boolean;
  clash: boolean;
}

export interface StoryCandidate {
  kanji: string;
  keyword: string;
  story: string;
  replacements: { component: string; old: string; new: string[] }[];
}

export interface StoryRewrite {
  kanji: string;
  old: string;
  new: string;
}

export interface Interests {
  topics: { name: string; weight: number }[];
  avoid: string[];
}

export interface Friction {
  key: WordKey;
  display: string;
  state: WordState;
  strength: number | null;
  lookups: number;
  last: number;
}

export interface Stats {
  known_words: number;
  chars_total: number;
  chars_today: number;
  days: { day: string; chars: number }[];
  streak: number;
  best_streak: number;
  texts_read: number;
  words_seen_30d: number;
  words_looked_up_30d: number;
  stealth_leeches: Friction[];
  most_looked_up: Friction[];
}

/** The user's local date, YYYY-MM-DD. */
export const localDay = (d = new Date()) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

export interface Simplified {
  tokens: ReaderToken[];
  simple: string;
  /** English translation of the original sentence. */
  translation: string;
  coverage: number;
}

export interface CardDraft {
  id: string;
  key: WordKey;
  display: string;
  sentence: string;
  sentence_source: "text" | "new";
  text_id: string | null;
  /** Anki field → value, once filled. */
  fields: Record<string, string> | null;
  status: "draft" | "added";
  note_id: number | null;
  created_at: number;
}

export interface CardOptions {
  note_types: string[];
  decks: string[];
  note_type: string | null;
  deck: string | null;
  auto_add: boolean;
}

export interface TextExtra {
  targets: string[];
  topic: string;
  coverage: number;
  attempts: number;
  examples: Example[];
}

export interface NewEvent {
  kind: "exposure" | "lookup" | "dont_know";
  key: WordKey;
  text_id: string | null;
  /** The word as written in the text. */
  display?: string;
}

export const api = {
  status: () => invoke<Status>("app_status"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  setSudachiPath: (path: string) => invoke<void>("set_sudachi_path", { path }),
  downloadSudachi: () => invoke<string>("download_sudachi"),
  importDictionary: (path: string) => invoke<DictionaryInfo>("import_dictionary", { path }),
  downloadJmdict: () => invoke<DictionaryInfo>("download_jmdict"),

  listTexts: () => invoke<TextMeta[]>("list_texts"),
  addText: (title: string, body: string) => invoke<TextMeta>("add_text", { title, body }),
  renameText: (id: string, title: string) => invoke<void>("rename_text", { id, title }),
  deleteText: (id: string) => invoke<void>("delete_text", { id }),
  openText: (id: string) => invoke<{ meta: TextMeta; doc: ReaderDoc; extra: TextExtra | null }>("open_text", { id }),
  wordInfo: (key: WordKey, surface: string) => invoke<WordInfo>("word_info", { key, surface }),
  setReadingFix: (surface: string, reading: string | null, textId: string | null) =>
    invoke<void>("set_reading_fix", { surface, reading, textId }),
  setWordMark: (key: WordKey, state: WordState | null) => invoke<void>("set_word_mark", { key, state }),
  recordEvents: (events: NewEvent[]) => invoke<void>("record_events", { events }),

  ankiCheck: () => invoke<number>("anki_check"),
  ankiNoteTypes: () => invoke<NoteType[]>("anki_note_types"),
  ankiPreview: (mapping: NoteTypeMapping) => invoke<MappingPreview>("anki_preview", { mapping }),
  ankiSuggest: (modelId: number, modelName: string, fields: string[]) =>
    invoke<Suggestion | null>("anki_suggest", { modelId, modelName, fields }),
  saveMapping: (mapping: NoteTypeMapping) => invoke<void>("save_mapping", { mapping }),
  deleteMapping: (modelId: number) => invoke<void>("delete_mapping", { modelId }),
  ankiSync: () => invoke<SyncReport>("anki_sync"),

  aiSettings: () => invoke<AiSettings>("ai_settings"),
  setApiKey: (provider: Provider, key: string) => invoke<void>("set_api_key", { provider, key }),
  studyWords: () => invoke<StudyWord[]>("study_words"),
  generateText: (request: GenerateRequest) => invoke<GenerateResult>("generate_text", { request }),
  chatPrompt: (request: GenerateRequest) => invoke<string>("chat_prompt", { request }),
  chatSubmit: (request: GenerateRequest, answer: string, attempts: number, keep: boolean) =>
    invoke<ChatOutcome>("chat_submit", { request, answer, attempts, keep }),
  openClaudeAi: () => invoke<void>("open_claude_ai"),

  kanjiMappings: () => invoke<KanjiMapping[]>("kanji_mappings"),
  kanjiSuggest: (modelId: number, modelName: string, fields: string[]) =>
    invoke<KanjiMapping | null>("kanji_suggest", { modelId, modelName, fields }),
  saveKanjiMapping: (mapping: KanjiMapping) => invoke<void>("save_kanji_mapping", { mapping }),
  deleteKanjiMapping: (modelId: number) => invoke<void>("delete_kanji_mapping", { modelId }),
  kanjiCard: (kanji: string) => invoke<KanjiCard>("kanji_card", { kanji }),
  kanjiNewStory: (kanji: string) => invoke<NewStory>("kanji_new_story", { kanji }),
  kanjiSaveStory: (kanji: string, story: string) => invoke<number>("kanji_save_story", { kanji, story }),
  kanjiUndoStory: (kanji: string) => invoke<number>("kanji_undo_story", { kanji }),
  primitivesStatus: () => invoke<{ with_story: number; analysed: number }>("primitives_status"),
  primitivesAnalyse: () => invoke<{ with_story: number; analysed: number }>("primitives_analyse"),
  primitivesStop: () => invoke<void>("primitives_stop"),
  primitivesReset: () => invoke<void>("primitives_reset"),
  primitivesList: () => invoke<PrimitiveRow[]>("primitives_list"),
  primitivesApprove: (names: Record<string, string[]>) => invoke<void>("primitives_approve", { names }),
  primitivesCandidates: () => invoke<StoryCandidate[]>("primitives_candidates"),
  primitivesRewrite: (kanji: string[]) => invoke<StoryRewrite[]>("primitives_rewrite", { kanji }),
  primitivesSave: (kanji: string, story: string) => invoke<number>("primitives_save", { kanji, story }),
  getInterests: () => invoke<Interests>("get_interests"),
  setInterests: (value: Interests) => invoke<void>("set_interests", { value }),
  pickTopic: () => invoke<string | null>("pick_topic"),
  suggestInterests: () => invoke<string[]>("suggest_interests"),
  recordReading: (chars: number) => invoke<void>("record_reading", { day: localDay(), chars }),
  readingStats: () => invoke<Stats>("reading_stats", { today: localDay() }),
  simplifySentence: (sentence: string) => invoke<Simplified>("simplify_sentence", { sentence }),
  phraseInfo: (selection: string) => invoke<ReaderToken | null>("phrase_info", { selection }),
  cardOptions: () => invoke<CardOptions>("card_options"),
  cardFieldNames: () => invoke<string[]>("card_field_names"),
  cardDrafts: () => invoke<CardDraft[]>("card_drafts"),
  /** No sentence: the AI writes a new one in which only this word is unknown. */
  cardCreate: (key: WordKey, display: string, sentence: string | null, textId: string | null) =>
    invoke<CardDraft>("card_create", { key, display, sentence, textId }),
  cardsForText: (textId: string) => invoke<CardDraft[]>("cards_for_text", { textId }),
  cardFill: (id: string) => invoke<CardDraft>("card_fill", { id }),
  cardNewSentence: (id: string) => invoke<CardDraft>("card_new_sentence", { id }),
  cardSetFields: (id: string, fields: Record<string, string>) => invoke<void>("card_set_fields", { id, fields }),
  cardDiscard: (id: string) => invoke<void>("card_discard", { id }),
  cardAdd: (id: string) => invoke<CardDraft>("card_add", { id }),
};

export const keyId = (k: WordKey) => `${k.lang}\u0001${k.lemma}\u0001${k.reading}`;

export const stateLabel: Record<WordState, string> = {
  unknown: "Not in Anki",
  new: "New in Anki",
  learning: "Learning",
  fading: "Known, fading",
  known: "Known",
  marked_known: "Marked known",
  ignored: "Ignored",
};
