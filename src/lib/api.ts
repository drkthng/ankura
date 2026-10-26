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
  source: "mark" | "anki" | "parts" | "none";
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
}

export interface NewEvent {
  kind: "exposure" | "lookup" | "dont_know";
  key: WordKey;
  text_id: string | null;
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
  openText: (id: string) => invoke<{ meta: TextMeta; doc: ReaderDoc }>("open_text", { id }),
  wordInfo: (key: WordKey, surface: string) => invoke<WordInfo>("word_info", { key, surface }),
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
