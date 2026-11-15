// The language being learned (ADR 0023), for the views that differ by language. App.svelte keeps
// it in step with the app's status.
export const app = $state({ lang: "ja", name: "Japanese" });

/** Japanese has kanji, furigana, VOICEVOX and the JLPT; Italian and Spanish don't. */
export const isJapanese = () => app.lang === "ja";
