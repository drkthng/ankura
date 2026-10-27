# AI setup

[← README](../../README.md)

These features use an AI model:

- [Write for me](write-for-me.md) texts and interest suggestions;
- *Simplify this sentence* ([Reading](reading.md#help-with-a-sentence));
- new sentences and filled-in fields for [cards](cards.md);
- new kanji mnemonics and consistent primitive names ([Kanji](kanji.md)).

Everything else works without one.

## Two ways to connect

In **Settings → AI**, choose how Ankura asks the AI.

**Automatically, with an API key** (all features). Ankura calls the AI itself and checks the
result.

- **Anthropic (Claude):** create a key at [console.anthropic.com](https://console.anthropic.com).
  The default model is Claude Opus 5.5 at medium effort. API use is billed per request, separately
  from a Claude subscription: a text costs a few cents, a card or sentence less.
- **Claude Max and Team plans include monthly API credits** that cover this. Claim them by linking
  a Console organization to your plan
  ([how](https://support.claude.com/en/articles/17154008)), then create the key there.
- **OpenRouter** gives access to many models with one key; enter the model name in Settings.
- Keys are stored in your system's keychain (Windows Credential Manager, macOS Keychain, Secret
  Service on Linux), not in Ankura's files.

**Copy & paste through claude.ai** (texts only). No key needed; it uses your Claude subscription.
Ankura shows the request with a Copy button and opens claude.ai. You paste Claude's reply back
and Ankura checks it. Simplify and card filling need the automatic mode; in copy & paste mode you
fill cards by hand.

## What is sent

Only when you use an AI feature, and only to the provider you chose:

- **Texts:** your practice words, topic, length, style, topics to avoid, up to 400 of your
  best-known words, and a few fading ones. A rewrite request also contains the previous version and the words it should
  avoid.
- **Interest suggestions:** the titles and first lines of your 30 most recent pasted texts, the
  topics you asked texts to be about, and your current interests.
- **Simplify:** the sentence and up to 400 known words.
- **New card sentence:** the word and up to 400 known words.
- **New kanji mnemonic:** the kanji, its keyword and current story, and up to six of your other
  kanji stories (as examples of your style and primitive names).
- **Primitive names:** your kanji, keywords and stories (all of them, in batches, when you analyse),
  and the stories to update with their old and approved names.
- **Filling a card:** the word, sentence, readings, dictionary meanings, and your three newest
  notes of the chosen note type (their field contents), as examples.

Nothing else from your collection or reading history leaves your computer.
