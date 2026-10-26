# Spike S3: Japanese tokenizer comparison

Date: 2026-10-26. Throwaway spike from `docs/architecture-options.md` §9. It compares the three
Rust-usable options for Ankura's Japanese tokenizer:

- **Lindera 6.2.0 + IPADIC** (mecab-ipadic 2.7.0)
- **Lindera 6.2.0 + UniDic** (unidic-mecab 2.1.2)
- **sudachi.rs** at commit `1c21940` (0.7.1-a1) + **SudachiDict 20261015**, small and core
  dictionaries, split modes A and C

## Method

- **Test set:** `data/gold-ja.tsv`, 150 sentences written for this spike, in nine categories
  (verbs, adjectives, homographs, spelling variants, honorific, colloquial, compounds, literary,
  names and numbers). 295 hand-annotated checks give the expected lemma and the reading of the
  surface as written (what furigana needs).
  - I wrote the sentences myself, because Tatoeba, Wikipedia and Aozora Bunko weren't reachable
    from this environment. They aim at hard cases, so the numbers below are lower than you'd get on
    ordinary text. A run on real text (the owner's own reading material) is still worth doing.
- **Variant groups:** six groups (分かる/わかる, 子供/子ども, 綺麗/きれい, コンピューター/コンピュータ,
  一緒/いっしょ, 附属/付属). Each passes when every member gets the same key: IPADIC `base_form`,
  UniDic `lexeme`, Sudachi `normalized_form`.
- **Scoring** (`eval.py`): a check counts as *segmented* when a token has exactly the expected
  surface. Lemma and reading are scored only on those tokens. *Merged* means the tokenizer kept a
  longer unit (often fine, e.g. 生ビール); *split* means it broke the word up.
- **Speed and memory:** the 150 sentences repeated 300 times (533,100 characters), release build,
  4 vCPUs. Peak RSS includes the memory-mapped dictionary pages that were touched.

To reproduce, unpack the dictionaries (Lindera release zips; `system.dic` from the SudachiDict
wheels on PyPI), then:

```bash
cargo build --release
grep -v '^#' data/gold-ja.tsv | cut -f1 > out/sentences.txt
./target/release/s3tok <lindera-ipadic dir>  < out/sentences.txt > out/ipadic.jsonl
./target/release/s3tok <lindera-unidic dir>  < out/sentences.txt > out/unidic.jsonl
./target/release/sudachi <core system.dic> C < out/sentences.txt > out/sudachi-core-C.jsonl
python3 eval.py out/ipadic.jsonl out/unidic.jsonl out/sudachi-core-C.jsonl
```

## Results

### Accuracy

| Tokenizer | Segmented as expected | Lemma correct | Surface reading correct | Variant groups unified |
|---|---|---|---|---|
| Lindera + IPADIC | 273/295 (92.5%) | 258/258 (100%) | 252/259 (97.3%) | **0/6** |
| Lindera + UniDic | 272/295 (92.2%) | 257/257 (100%) | 251/258 (97.3%)¹ | 6/6 |
| Sudachi core, mode C | **279/295 (94.6%)** | 264/264 (100%) | 258/265 (97.4%) | 6/6 |
| Sudachi core, mode A | 270/295 (91.5%) | 255/255 (100%) | 249/256 (97.3%) | 6/6 |
| Sudachi small, mode C | 271/295 (91.9%) | 256/256 (100%) | 250/257 (97.3%) | 6/6 |

¹ UniDic 2.1.2 has no kana surface reading. Lindera's field `reading` is the *lemma's* reading
(行っ → イク), and the surface form is only given phonetically (東京 → トーキョー). `eval.py` rebuilds
the surface reading: the phonetic form when it has no ー, otherwise the lemma reading of the kanji
stem plus the surface's own kana. A first version of that rule scored 96.1% (it got 来 and
やっぱり wrong); the number above is after the fix. Also, UniDic's `lexeme` is katakana for proper
nouns (東京 → トウキョウ), so the lemma has to come from `orthographic_base_form`.

**Compounds** (61 checks: 自動販売機, 携帯電話, 記者会見, 人工知能, 消費税, 電話番号, …): Sudachi core
in mode C keeps 58 as one unit. Every other configuration keeps 45. Sudachi can also split one
mode-C unit into its mode-A parts afterwards (`Morpheme::split`), so 海外旅行 can match a card for
海外旅行 *and* one for 海外.

### Errors every tokenizer makes

These need handling in Ankura whichever tokenizer it uses:

- **Context-dependent readings:** 十分 (じゅっぷん in 歩いて十分, not じゅうぶん), 窓が開いている (あい,
  not ひらい), 人気のない道 (ひとけ), この方 (かた), 市場で魚を買った (いちば). The tokenizers choose
  the reading that's most frequent overall.
- **Counter sound changes:** 三匹 (さんびき) comes out as ひき. 一本 (いっぽん) is ほん with IPADIC
  (UniDic and Sudachi get ぽん).
- **Other segmentation slips:** 一日中 is split 一 + 日中 by both UniDic-based tokenizers, which gives
  the wrong word. 乗り遅れる and 金閣寺 are split by Sudachi and UniDic. お茶 becomes お + 茶 with
  UniDic and Sudachi.

**Consequences for the design:**
1. Furigana must be correctable: tap a word to choose another reading from the dictionary, and
   remember the choice.
2. When a word is matched to an Anki card that has a reading field, use the card's reading.
3. Add a small rule table for counter sound changes (本, 匹, 杯, 分, 回, …).
4. Match Anki cards against both the long unit and its parts.

### Speed, memory, size

| | Throughput | Peak RSS | Dictionary on disk | deflate-9 (APK asset) | xz-6 (download) | Load time |
|---|---|---|---|---|---|---|
| Lindera + IPADIC | ~270k chars/s | 32 MB | 45 MB | 9.9 MB | 6.9 MB | ~1 ms (mmap) |
| Lindera + UniDic | ~170k chars/s | 57 MB | 190 MB | 43.9 MB | 27.2 MB | ~1 ms (mmap) |
| Sudachi small | ~430k chars/s | 72 MB | 110 MB | 40.5 MB | 22.9 MB | ~15 ms |
| Sudachi core | ~440k chars/s | 137 MB | 193 MB | 73.3 MB | 45.4 MB | ~30 ms |

- Speed isn't a factor: a 600-character page takes under 5 ms with any of them.
- The program itself is small: 1.0 MB (Lindera binary), 4.3 MB (Sudachi binary), x86-64 release.

### Android build

- `cargo check --target aarch64-linux-android` passes for Lindera.
- **sudachi.rs fails to compile for Android.** Its plugin loader defines `make_system_specific_name`
  for Linux, FreeBSD, Windows, macOS and Apple's embedded platforms, but not for Android
  (`sudachi/src/plugin/loader.rs:55-82`). Adding `target_os = "android"` to the Linux branch fixes
  it; the whole crate then type-checks. That's a one-line upstream PR, or a `[patch]` until then.
- Nothing was linked or run on a device (no NDK here). That belongs to spike S1.

## Assessment

| | Lindera + IPADIC | Lindera + UniDic | Sudachi core |
|---|---|---|---|
| Lemmas | Same | Same | Same |
| Readings for furigana | Direct | Needs a reconstruction rule | Direct |
| Compounds kept whole | No | No | **Yes, with split modes** |
| Spelling variants unified | **No** | Yes | Yes (`normalized_form`) |
| Size (download) | **7 MB** | 27 MB | 45 MB |
| Size (disk) / memory | **45 MB / 32 MB** | 190 MB / 57 MB | 193 MB / 137 MB |
| Library maturity | crates.io, active; v7 coming | Same | Not on crates.io, 0.7 marked unstable, Android patch needed |
| Dictionary licence | NAIST (permissive) | BSD (of GPL/LGPL/BSD) | Apache-2.0 |

For Ankura, compounds and variant unification matter more than for a plain reader, because the
tokenizer output is matched against twenty years of Anki cards. A card says 分かる and the text
says わかる; a card holds 携帯電話 and IPADIC returns 携帯 + 電話. With IPADIC, Ankura would have to
add both itself: a JMdict-based longest match for compounds, and JMdict entry IDs as keys for
variants. That's what Yomitan and jpdb do. It's a reasonable design anyway, but it's extra work.

**Recommendation:** Sudachi with the **core** dictionary, mode C for display units and mode A
parts for matching. Use the same dictionary on desktop and Android, so word keys match across
devices and Ankura's sync never compares two different tokenizations. Ship it as a first-run
download (about 45 MB) rather than inside the APK. Pin the git commit, carry the Android patch until
upstream takes it, and keep everything behind the tokenizer interface so Lindera + IPADIC stays a
fallback.

**Alternative:** Lindera + IPADIC if a small app matters more (7 MB download, 32 MB memory), with
JMdict doing compound matching and variant keys. More of our own code, less dependency risk.

## Not covered

- Real text: news, novels and the owner's own Anki sentences. Re-run `eval.py` on a sample with
  spot-checked gold labels.
- APK size and memory on a phone (spike S1), and how Android's page cache handles a 193 MB
  memory-mapped dictionary on low-memory devices.
- Lindera with SudachiDict (`embed-sudachidict`), Vibrato, and UniDic 3.x (unidic-cwj, which has a
  kana surface field).
- Italian and Spanish lemmatizers (ADR 0001's open item).
