# Feature ideas: making learning faster and more fun

[← How to learn with Ankura](../guide/learning.md)

Researched 2026-10-30 for a learner at about JLPT N4-N3 with a large Anki habit (the owner: ~1,600
active notes, 845 cards 30+ days overdue, RTK kanji, FSRS off). Builds on
`docs/research/landscape.md` (which covers the open-source readers and Anki tools) and does not
repeat it.

**About the sources.** From this session only github.com could be fetched. jiten.moe,
satorireader.com, docs.asbplayer.dev, blog.duolingo.com, arxiv.org and cambridge.org did not
resolve. Every URL below appeared in a search result or was fetched, and none is made up. Marks:

- **[F]** fetched and read in this session;
- **[S]** seen in search results with a summary of its contents, not opened. Treat the details
  as secondary.

Forum and Reddit opinions are anecdotes. Where the evidence is thin, the text says so.

---

## 1. What comparable tools do that Ankura doesn't

Ankura already covers a lot: Anki-aware colouring, i+1 generated texts, AI cards in the user's
format, simplify, TTS reading, kanji/RTK, JLPT tests, stats and stealth leeches. The gaps fall
into seven groups.

### 1.1 Getting real content in (Ankura: paste only)

| Tool | Feature | What users say |
|---|---|---|
| LinguaCafe | Imports plain text, e-books (EPUB, chapter order from the spine), YouTube subtitles, subtitle files, Jellyfin subtitles, web pages | Breadth of import is its main selling point ([linuxlinks overview, S](https://www.linuxlinks.com/linguacafe-language-learners-vocabulary-reading/), [v0.13 notes, S](https://newreleases.io/project/github/simjanos-dev/LinguaCafe/release/v0.13)) |
| Readlang | Web pages through an extension; in 2025 added YouTube import and audio import with AI transcription *(secondary)* | [features page, S](https://readlang.com/features), [outilstice review, S](https://outilstice.com/en/readlang-apprendre-une-langue-en-surfant-sur-le-web/) |
| VN/game players | Textractor, then a texthooker page that shows the clipboard, then Yomitan; exSTATic adds reading stats | The standard Japanese VN setup ([vnclub tools, S](https://vnclub.org/tools/), [lazy guide, S](https://xelieu.github.io/jp-lazy-guide/setupVnOnPC)) |
| Todaii | Graded news from NHK Easy and other outlets, with furigana toggle and audio | 4.7 stars from ~30k Android ratings; complaints about awkward furigana toggling and word selection ([Tofugu review, S](https://tofugu.com/reviews/todai-easy-japanese-news-app), [android-stats, S](https://android-stats.com/d/mobi.eup.jpnews/reviews)) |
| Tadoku Supporters | Free graded readers, levels 0-5, with furigana and some audio | [tadoku.org graded readers, S](https://tadoku.org/japanese/en/graded-readers-en), [Tofugu, S](https://www.tofugu.com/japanese-learning-resources-database/tadoku-supporters/) |

### 1.2 Knowing a work before you start it

- **jpdb / Jiten:** each work's coverage and difficulty, plus a deck of the work's words in
  frequency order, to "pre-learn" before reading. Reddit users call pre-learning the "killer
  feature" and say it made reading less tedious. Others warn jpdb is weaker as an SRS
  ([r/LearnJapanese thread, S](https://reddit.rtrace.io/r/LearnJapanese/comments/1afyizs/how_to_read_books_in_jaapnese_early_on/kojjf52/?context=3),
  [switching-from-Anki thread, S](https://reddit.rtrace.io/r/LearnJapanese/comments/1biyipl/switching_from_anki_to_jpdbio_has_drastically)).
- Jiten's own guides: coverage is the headline number; about 80% is "challenging"; Anki decks
  with a "coverage %" strategy that skips known words; frequency-range auto-known
  ([what-is-jiten, S](https://jiten.moe/guides/what-is-jiten),
  [generating decks, S](https://jiten.moe/guides/generating-anki-decks)).
- **Natively:** difficulty grades built from pairwise comparisons by readers
  ([WaniKani post, S](https://community.wanikani.com/t/51419)).

### 1.3 Serial, voiced, graded stories

- **Satori Reader:** serial stories with voice acting, grammar notes per sentence, built-in SRS
  cards with audio, known-kanji furigana. Users single out the grammar notes ("75% of my reason")
  and the voice acting. They complain that the content selection is limited
  ([WaniKani appreciation thread, S](https://community.wanikani.com/t/satori-reader-appreciation-thread/64403),
  [Tofugu, S](https://tofugu.com/japanese-learning-resources-database/satori-reader),
  [FAQ, S](https://satorireader.com/help)).
- Takeaway for Ankura: an ongoing story is what keeps people coming back. Ankura's one-off
  generated texts don't have that.

### 1.4 Listening around the text

- **Language Reactor:** auto-pause after each line, repeat line, dual subtitles. Auto-pause is
  the feature reviewers describe most often ([JALT article, S](https://jalt-publications.org/articles/28605-enhancing-functionality-youtube-and-netflix-language-reactor),
  [FluentU, S](https://www.fluentu.com/blog/reviews/language-reactor/)).
- **asbplayer:** condensed playback (skips gaps without subtitles) and fast-forward through them;
  one-key mining with audio clip and screenshot ([releases, S](https://www.github.com/killergerbah/asbplayer/releases),
  [mining docs, S](https://docs.asbplayer.dev/docs/guides/mining-in-depth)).
- **Refold:** condensed audio for passive listening, made from shows you have already watched,
  so the input stays comprehensible ([Refold notion page, S](https://zenith-raincoat-5cf.notion.site/Passive-Condensed-audio-14c02a59ba934d0b9649cc054186cd44),
  [tatsumoto blog, S](https://tatsumoto.neocities.org/blog/passive-listening)).
- **Clozemaster:** "cloze-listening": hear the sentence, fill the gap
  ([forum announcement, S](https://forum.clozemaster.com/t/new-feature-cloze-listening/12633)).
- **Migaku:** pitch-accent colouring of words, and word audio
  ([legacy manual, S](https://deploy-preview-148--tourmaline-brigadeiros-5c144c.netlify.app/tools-guides/migaku-japanese/manual),
  [Tofugu: Migaku Pitch Trainer, S](https://www.tofugu.com/japanese-learning-resources-database/migaku-pitch-trainer/)).

### 1.5 Grammar as a tracked thing

- **Bunpro:** grammar SRS. Its reading passages highlight the lesson's grammar point with a
  pop-up linking to the explanation; "cram" for review on demand; "ghost" reviews for
  repeated misses ([grammar highlighting feedback, S](https://community.bunpro.jp/t/grammar-highlighting-feedback/45500),
  [ghosts thread, S](https://community.bunpro.jp/t/ghosts-on-or-off/45756/22)).
- **BunSense** (Chrome extension): grammar highlights with numbered badges on any page
  ([Chrome store, S](https://chromewebstore.google.com/detail/nghpogpmcpihnehnoplphbcnfebgapfb)).
- No reader found that applies Bunpro-style grammar detection to *arbitrary* text together with
  Anki knowledge. That is an open niche at N4-N3, where grammar holds learners back more than
  vocabulary does.

### 1.6 Quick retrieval games and motivation

- **Clozemaster:** frequency-ordered "Fluency Fast Track", points; reviewers call it "addicting"
  and "like an arcade game", but say sentences are not consistently i+1 and the app is no
  replacement for other study ([StoryLearning review, S](https://storylearning.com/clozemaster-review),
  [FluentU review, S](https://fluentu.com/blog/reviews/clozemaster), [Refold blog, S](https://www.refold.la/blog/srs-that-isnt-anki-clozemaster)).
- **Renshuu:** games, a garden, quizzes; praised as "like Anki but cuter", seen as complex
  for beginners ([Coto Academy, S](https://cotoacademy.com/learning-japanese-with-renshuu-app-review/),
  [WaniKani thread, S](https://community.wanikani.com/t/renshuu-is-toooo-underrated/68944)).
- **WaniKani:** level unlocks. Some users find them motivating ("I do need the gamification");
  others see them as throttling ([Why levels?, S](https://community.wanikani.com/t/why-levels/41146),
  [Gamification thread, S](https://community.wanikani.com/t/gamification/41026)).
- **Duolingo:** the streak is said to be its most effective growth feature. Streak freezes
  were added on habit research saying some slack keeps people going
  ([Duolingo blog, S](https://blog.duolingo.com/how-duolingo-streak-builds-habit),
  [Lenny's podcast summary, S](https://rosetta.to/u/lennyspodcast/behind-the-product-duolingo-streaks-jackson-shuttleworth-group-pm-retention-team)).
  Leagues and XP invite "gamification misuse": farming XP instead of learning
  ([qualitative study, arXiv 2203.16175, S](https://arxiv.org/pdf/2203.16175)).
- **LingQ:** daily goal plus streak, known-word count as the main progress number, review in
  multiple choice / fill-in / dictation ([help: statistics, S](https://lingq-support.groovehq.com/help/understanding-lingq-statistics),
  [LingQ 5.0, S](https://lingq.com/blog/introducing-lingq-5-0)).
- **Anki add-ons:** Review Heatmap (GitHub-style calendar and streak)
  ([repo, F](https://github.com/glutanimate/review-heatmap)). FSRS Helper: postpone, advance,
  schedule a break, load balance, easy days, disperse siblings, flatten
  ([repo, F](https://github.com/open-spaced-repetition/fsrs4anki-helper)). Their popularity shows
  how much Anki users care about **workload and backlog**.
- **Immersion trackers:** character counts, reading speed (chars/min) and logs are the
  community's standard metrics ([Japanese Reading Tracker, S](https://chromewebstore.google.com/detail/japanese-reading-tracker/mfboeaddkcphfofengfgdjjgelklopdi),
  [NihongoAutoTracker, S](https://addons.mozilla.org/en-US/firefox/addon/nihongoautotracker/),
  [22 million moji log, S](https://forums.learnnatively.com/t/road-to-22-million-moji-extensive-reading-log/5280)).

### 1.7 Speaking and AI tutors

- **Speak / Praktika:** polished AI conversation. Reviews say the feedback is weak and the
  drills repetitive; no independent outcome data found
  ([Languatalk review 3/5, S](https://languatalk.com/blog/speak-app-review/)).
- **ChatGPT voice:** good for low-stakes fluency practice. A WIRED-style month-long test found
  it useful but not enough on its own. Learning communities advise practising fluency by voice
  and accuracy in writing separately
  ([sogyotecho, S](https://sogyotecho.jp/chatgpt-advancedvoicemode/),
  [celebsover50 mirror of the WIRED test, S](https://celebsover50.pages.dev/story/ai-lab-learning-japanese-with-chatgpt-tokyo)).
- **Migaku:** AI example sentences and personalised cards from what users watch get praised;
  the price and the lack of offline mining get criticised ([app reviews, S](https://appshunter.io/ios/app/1664096855/reviews),
  [Migaku community, S](https://migaku.com/community/m/1379599898550210612)).

**Most praised across tools:** pre-learning a work's words; serial stories with good audio and
grammar notes; auto-pause and repeat; easy import of what you already want to read; visible
progress (known words, characters, heatmaps); keeping the workload under control.
**Usually called gimmicks:** leagues, XP, gems and mascots; AI tutors without a level filter or
real feedback; auto-marking words known.

---

## 2. What learning research suggests

| Finding | Source | Consequence for Ankura |
|---|---|---|
| Coverage: 98% for adequate comprehension, 95% minimal (Hu & Nation 2000); a 2023 replication could not fully reproduce the numbers, and vocabulary size may predict comprehension better | [Lancaster eprint of the replication, S](https://eprints.lancs.ac.uk/id/eprint/210922), [Nation 2006 PDF, S](https://www.wgtn.ac.nz/__data/assets/pdf_file/0018/1626120/2006-How-large-a-vocab.pdf), [VUW thesis, S](https://openaccess.wgtn.ac.nz/articles/thesis/Thresholds_Text_Coverage_Vocabulary_Size_and_Reading_Comprehension_in_Applied_Linguistics/17057906) | Ankura's 95% floor is sound; offer 98% for "easy" extensive reading, and show coverage on every imported text |
| Repetition: medium effect (r = .34) on incidental vocabulary learning; spacing and engagement moderate it; the number of encounters needed varies (2 to 20+) | Uchihara, Webb & Yanagisawa 2019, *Language Learning* ([record, S](https://zendy.io/title/10.1111/lang.12343)) | Track encounters per target word and plan more spaced encounters, not a single text |
| Reading-while-listening beats reading only in several studies (Webb & Chang 2012/2015/2022, Teng 2016); TwiLex 2024 found no difference; results vary by learner | [Webb & Chang core.ac.uk PDF, S](https://core.ac.uk/download/215386265.pdf), [Teng 2016, S](https://www.degruyterbrill.com/document/doi/10.1515/cjal-2017-0004/html?lang=en) | Ankura already reads aloud; make it the default path and improve sync (word-level highlight) |
| Narrow reading (same topic or author) recycles vocabulary | Gardner 2008, Schmitt & Carter 2000; SSLA study on narrow reading and listening ([SSLA, S](https://www.cambridge.org/core/journals/studies-in-second-language-acquisition/article/effects-of-narrow-reading-and-listening-on-l2-vocabulary-learning/636D23C4D711F7E93779D98BF7F16F8B), [Shiraz study, S](https://jtls.shirazu.ac.ir/article_5801.html)) | Series of generated texts on one topic with recurring characters |
| Retrieval practice beats re-reading at delays of 2 days and 1 week | Roediger & Karpicke 2006 ([PDF, S](https://www.gwern.net/docs/spaced-repetition/2006-roediger.pdf)) | A short comprehension or cloze check after each text, not only more reading |
| Spaced practice: medium-to-large effect on L2 learning; expanding schedules not clearly better than equal ones | Kim & Webb 2022; Nakata 2015 ([Cambridge SSLA, S](https://www.cambridge.org/core/journals/studies-in-second-language-acquisition/article/effects-of-expanding-and-equal-spacing-on-second-language-vocabulary-learning/D1D796306985C52F9BE7A1200AC50DB9), [Kansai U. repo, S](https://kansai-u.repo.nii.ac.jp/records/10256)) | Spread encounters over days; exact spacing matters less |
| Pre-reading vocabulary work improves learning and speeds processing (eye-tracking); other studies find smaller or speed-only effects | Pellicer-Sánchez, Conklin & Vilkaitė-Lozdienė 2021 ([Nottingham repo, S](https://nottingham-repository.worktribe.com/output/4745317)), pretesting study ([SSLA, S](https://resolve-he.cambridge.org/core/journals/studies-in-second-language-acquisition/article/pretesting-effects-on-incidental-l2-vocabulary-learning-through-reading/8D819C67CC7562CDC3DB6F60C46B30E0)) | "Pre-learn this text": a 3-minute warm-up on its top unknown words |
| Shadowing improves listening, depending on motivation and confidence; heavy cognitive load | Hamada 2015 ([PDF, S](https://air.repo.nii.ac.jp/record/2465/files/kkiso17%289%29.pdf)), Hamada review ([JALT, S](https://jalt-publications.org/articles/26836-shadowing-procedures-teaching-and-their-future)); for learners of Japanese: small 2025 study on prosody ([journal, S](https://journal.unnes.ac.id/journals/chie/article/view/39904)) | Optional shadowing mode on Ankura's own (comprehensible) texts |
| Output makes learners notice gaps (Swain); empirical support partial (Izumi et al. 1999) | [overview PDF, S](https://u-sacred-heart.repo.nii.ac.jp/record/195/files/inronshu38_1-Iwata.pdf), [Testing the output hypothesis, S](https://experts.umn.edu/en/publications/testing-the-output-hypothesis/) | Short writing with corrections, using the target words |
| Habits: one missed day does not break habit formation (Lally); slack helps (Duolingo's rationale for freezes) | [Duolingo blog, S](https://blog.duolingo.com/how-duolingo-streak-builds-habit) | Rest days or freezes in Ankura's streak; weekly goals |

---

## 3. Ranked candidate features

Scoring: impact H = 3, M = 2, L = 1; effort S = 1, M = 2, L = 3. Rank ≈ impact ÷ effort, then
adjusted by hand for dependencies and the owner's situation (big overdue backlog, RTK, N4-N3,
Windows desktop, wants fun and ease). "Effort" is based on the code that exists (see
`docs/NEXT-SESSION.md` §3).

### Next

**1. "Today": one button for a daily session**
- *What:* a home screen that builds the day's plan from what exists: N fading/overdue words → a
  Write-for-me text (or the next episode, #6) → listen while reading → 3 quick questions (#7) →
  cards for what was missed. One click per step; the streak and characters update at the end.
- *Why:* the parts already exist but sit on six screens, and the owner wants "fun and easy".
  Duolingo's experience is that a small daily unit plus a streak drives the habit ([S](https://blog.duolingo.com/how-duolingo-streak-builds-habit)).
  The plan follows the research order above: input at coverage, spaced encounters, retrieval.
- *Impact* H · *Effort* S-M (UI over `generate.rs`, `stats.rs`, `speech.ts`) · *Depends on*
  nothing new; better with #6/#7 · *Risk:* low; an AI call per day (cached via ADR 0016).

**2. Backlog rescue: read your overdue cards before you review them**
- *What:* a target-word source "overdue in Anki" (845 cards 30+ days overdue for the owner),
  sorted by Ankura's recall estimate. Texts are written around 6-8 of them; reading them right
  before reviews makes the Anki session easier and less discouraging. Show "you re-met 40 of your
  overdue words this week".
- *Why:* FSRS Helper's postpone, schedule-a-break and flatten features ([F](https://github.com/open-spaced-repetition/fsrs4anki-helper))
  show that backlog pain is a top Anki problem. Ankura can't reschedule (ADR 0007, add-only), but
  it can make the overdue reviews pass. Meeting a word again in context is an extra spaced
  encounter (Uchihara 2019).
- *Impact* H (for this owner) · *Effort* S (`mapping.rs` already has overdue buckets via
  `prop:due`; add a suggestion source in Write) · *Depends on* nothing · *Risk:* Anki's schedule is
  unchanged by design; say so in the UI.

**3. Sentence audio on Anki cards**
- *What:* put the VOICEVOX (or system) clip of the card's sentence, and optionally the word,
  into the note's audio field via AnkiConnect `storeMediaFile`. The clip is already cached on
  disk.
- *Why:* Satori Reader users praise its cards with embedded audio ([S](https://community.wanikani.com/t/satori-reader-appreciation-thread/64403));
  asbplayer and Migaku miners put audio on every card. Cards with audio train listening during
  1,600 daily-habit reviews at no extra time.
- *Impact* M-H · *Effort* S (`cards.rs`, `voicevox.rs`; add `storeMediaFile` to the allow-list:
  it adds a file and changes no existing note, so it fits ADR 0007) · *Depends on* VOICEVOX for
  good quality · *Risk:* web-speech voices can't be recorded easily; VOICEVOX only at first.
  Licence terms differ per VOICEVOX character (credit line); check before shipping.

**4. Clipboard reader and URL import**
- *What:* (a) "Follow clipboard" mode: every copied Japanese line is appended to a live text,
  like a texthooker page, so visual novels, games and web pages flow into the coloured reader;
  (b) paste a URL to import an article (readability-style extraction); (c) a quick source for
  NHK Easy and Tadoku (open the site, copy into Ankura).
- *Why:* paste-only is the biggest limit on how much *real* content goes through Ankura. The
  texthooker workflow is the standard for VN readers ([S](https://vnclub.org/tools/)); LinguaCafe,
  Readlang and LingQ all import from the web. More reading at the right level is the main thing
  that drives acquisition.
- *Impact* H · *Effort* S for (a) (Tauri clipboard plugin, append to a text), M for (b) (fetch +
  HTML extraction in Rust) · *Depends on* nothing · *Risk:* respect sites' terms; store only
  locally; don't scrape NHK automatically.

**5. Pre-learn a text**
- *What:* before reading an imported text, show coverage (exists) plus "the 5-10 unknown words
  that unlock it most" (unknown words weighted by count in the text and frequency, #8), each with a
  one-tap card or a 2-minute flash preview. Library sorted and filtered by coverage.
- *Why:* jpdb users call pre-learning the "killer feature" ([S](https://reddit.rtrace.io/r/LearnJapanese/comments/1afyizs/how_to_read_books_in_jaapnese_early_on/kojjf52/?context=3));
  Jiten exports decks of a work's words that skip known ones ([S](https://jiten.moe/guides/generating-anki-decks));
  pre-reading work improves learning (Pellicer-Sánchez et al. 2021, [S](https://nottingham-repository.worktribe.com/output/4745317)).
  It turns a 85% text into a 95% text.
- *Impact* H · *Effort* S (`reader.rs` coverage, `cards.rs`) · *Depends on* #8 for frequency
  weighting (optional) · *Risk:* too many cards; cap and default to "preview, not card".

**6. Serial stories (narrow reading with recycling)**
- *What:* "Continue the story": a series of generated episodes with the same characters,
  setting and topic from the interest profile. Each episode carries a short summary of the last
  one and repeats about half the previous target words, so each target meets the reader 3-5
  times across days. The series shows the words learned so far.
- *Why:* Satori's serial, voiced stories are what users love ([S](https://community.wanikani.com/t/satori-reader-appreciation-thread/64403)),
  and an ongoing story is more fun than one-off texts. Narrow reading recycles vocabulary
  (Gardner 2008; Schmitt & Carter 2000); repeated, spaced encounters predict learning (Uchihara
  2019; Webb & Chang 2015).
- *Impact* H · *Effort* M (`generate.rs` prompt and series record, target scheduler using
  existing exposure events) · *Depends on* nothing; pairs with #1 · *Risk:* long series drift in
  style; keep a "story bible" (characters, facts) in the text's `extra` JSON.

**7. Three questions after each text**
- *What:* after reading (or listening), 2-3 quick AI questions on content plus one cloze on a
  target word; answers checked; missed target words go back into the target queue. Questions
  go into the bank (ADR 0016) so rereads cost nothing.
- *Why:* retrieval beats re-reading at delays (Roediger & Karpicke 2006, [S](https://www.gwern.net/docs/spaced-repetition/2006-roediger.pdf)).
  A quiz gives a sense of completion and a score. LingQ and Clozemaster use the same short
  check ([S](https://lingq-support.groovehq.com/help/understanding-lingq-statistics)).
- *Impact* M-H · *Effort* S-M (reuse `practice.rs`/`proficiency::solve` item format and solver
  check) · *Depends on* AI · *Risk:* low; make it skippable.

**8. Frequency and pitch-accent dictionaries**
- *What:* import Yomitan `term_meta_bank` entries (frequency and pitch), which `dict.rs` now
  rejects. Show frequency rank and the pitch pattern in the pop-up and on cards; use frequency to
  rank target suggestions and #5. Roadmap item "frequency list".
- *Why:* learners at N4-N3 gain most from the next few thousand most common words; Jiten and
  jpdb order everything by frequency ([S](https://jiten.moe/guides/what-is-jiten)). Pitch display is
  one of Migaku's distinctive features ([S](https://www.tofugu.com/japanese-learning-resources-database/migaku-pitch-trainer/)).
  Kanjium has pitch data for 124,137 words under CC BY-SA 4.0 ([F](https://github.com/mifunetoshiro/kanjium)),
  usable as a separately downloaded dictionary like JMdict.
- *Impact* M-H · *Effort* S (format is known, same import path) · *Depends on* nothing · *Risk:*
  CC BY-SA attribution; frequency lists differ by corpus (let the user choose).

### Soon

**9. Better reading-while-listening: word highlight, auto-pause, listen first**
- *What:* highlight the word being spoken (VOICEVOX's `audio_query` returns accent phrases with
  mora lengths; to verify), auto-pause after each sentence with "repeat" (Language Reactor's best-liked
  feature), and a "listen first, then read" mode that hides the text for the first pass.
- *Why:* RWL helps vocabulary and listening in several studies (Webb & Chang; Teng 2016;
  [S](https://core.ac.uk/download/215386265.pdf)), with mixed results elsewhere. Auto-pause is
  what Language Reactor users praise most ([S](https://jalt-publications.org/articles/28605-enhancing-functionality-youtube-and-netflix-language-reactor)).
  Listening is usually the weakest section for N3 candidates who learn mainly from text.
- *Impact* M-H · *Effort* S (auto-pause, hide text) to M (word timing) · *Depends on*
  VOICEVOX for timing · *Risk:* system voices give no word timing; fall back to sentence level.

**10. Fading-word cloze game (with cloze-listening)**
- *What:* a 3-minute arcade round: sentences from texts the user actually read, the fading,
  stealth-leech or target word blanked; type or pick it, or hear the sentence and fill the gap.
  A score and a combo, no XP economy. Results feed Ankura's own word state (look-up-like evidence),
  not Anki.
- *Why:* Clozemaster reviewers call it addictive ([S](https://storylearning.com/clozemaster-review));
  cloze-listening is a paid feature there ([S](https://forum.clozemaster.com/t/new-feature-cloze-listening/12633)).
  Retrieval in the user's own reading context fixes stealth leeches (cards passed in Anki but not
  recognised in text).
- *Impact* M-H · *Effort* M (new view; sentences and states exist) · *Depends on* nothing ·
  *Risk:* a second SRS competing with Anki; keep it a warm-up with no schedule.

**11. Write it back: short output with corrections**
- *What:* after a text, "use 3 of these words in your own sentences" (or a 2-line diary). The AI
  returns a minimal correction with a one-line reason per change and a natural rewrite; each
  correction can become a card.
- *Why:* output makes learners notice gaps (Swain; partial support,
  [S](https://experts.umn.edu/en/publications/testing-the-output-hypothesis/)); communities
  recommend practising accuracy in writing ([S](https://sogyotecho.jp/chatgpt-advancedvoicemode/)).
  Ankura has no output at all yet. Japanese IME input works on desktop.
- *Impact* M-H · *Effort* M (prompt + diff view; cards reuse `cards.rs`) · *Depends on* AI ·
  *Risk:* AI over-corrects or misses errors; ask for minimal edits and show them as a diff.

**12. Grammar points in the reader (N5-N3)**
- *What:* detect common JLPT N5-N3 grammar patterns (〜ようにする, 〜ばかり, 〜わけではない, …)
  from token sequences; underline them on demand; the pop-up explains with an example. Track
  "grammar met / looked up" like words, and use weak points as targets in Write for me and in the
  practice test.
- *Why:* Bunpro's grammar highlights and Satori's grammar notes are their most valued features
  ([S](https://community.bunpro.jp/t/grammar-highlighting-feedback/45500),
  [S](https://community.wanikani.com/t/satori-reader-appreciation-thread/64403)). Nothing found
  does this on arbitrary text together with Anki knowledge. At N4-N3 grammar holds learners back
  more than vocabulary does.
- *Impact* H · *Effort* M-L (an own pattern list of ~250 points, matching rules over Sudachi
  tokens, explanations written once by AI and kept) · *Depends on* nothing · *Risk:* false matches;
  start with ~80 patterns and a "wrong match" button; no copying of Bunpro or dictionary content.

**13. Immersion log, reading speed and goals**
- *What:* reading speed (chars/min, from time on paragraphs), a daily or weekly goal, a
  year heatmap, rest days or streak freezes, and manual entries for time spent outside Ankura
  (anime, podcasts, VN), with totals.
- *Why:* character counts and speed are the community's standard metrics ([S](https://forums.learnnatively.com/t/road-to-22-million-moji-extensive-reading-log/5280));
  heatmaps are a popular add-on ([F](https://github.com/glutanimate/review-heatmap)); slack
  keeps streaks alive ([S](https://blog.duolingo.com/how-duolingo-streak-builds-habit)).
  Seeing reading speed rise is a concrete, motivating sign of progress.
- *Impact* M · *Effort* S (`stats.rs` has days and characters) · *Depends on* nothing · *Risk:*
  low.

**14. Incremental, automatic Anki sync**
- *What:* `cardsModTime`/`notesModTime` to fetch only changed cards; sync on start and after the
  user's Anki session, so colours reflect today's reviews without pressing "Sync now".
- *Why:* asbplayer does this because `cardsInfo` is slow (landscape.md). Up-to-date colours
  matter more once #2 makes reviews and reading alternate. Roadmap known gap.
- *Impact* M · *Effort* M (`mapping.rs`, `anki.rs`) · *Depends on* nothing · *Risk:* drift
  between full and incremental results; keep a weekly full sync.

**15. EPUB import with book progress**
- *What:* import EPUB (graded readers, light novels, Tadoku books), chapter list, per-chapter
  coverage, bookmark, "chapters read".
- *Why:* graded readers are the classic N4-N3 extensive-reading material ([S](https://tadoku.org/japanese/en/graded-readers-en));
  LinguaCafe and ttu reader import EPUB. Books keep a learner going longer than single
  texts.
- *Impact* M-H · *Effort* M (zip + XHTML in Rust; ruby tags already handled by `normalize.rs`) ·
  *Depends on* #5 for best use · *Risk:* DRM books can't be imported; long texts need paging in
  the reader.

**16. Shadowing mode**
- *What:* sentence plays, pause for the user to repeat; optional microphone recording and
  playback next to the model voice; speed control.
- *Why:* shadowing improves listening (Hamada 2015, [S](https://air.repo.nii.ac.jp/record/2465/files/kkiso17%289%29.pdf))
  and prosody for learners of Japanese (small study, [S](https://journal.unnes.ac.id/journals/chie/article/view/39904)).
  Ankura's texts are at the learner's level, which reduces the cognitive load Hamada warns about.
- *Impact* M · *Effort* M (`getUserMedia` in WebView2/WebKitGTK, audio compare UI) · *Depends on*
  #9 auto-pause · *Risk:* microphone permissions on Linux WebKitGTK; no automatic scoring (see
  "probably not").

**17. Take today's texts to the phone as audio**
- *What:* export a text (or today's session) as one audio file: sentences at chosen speed with
  short gaps, optionally each sentence twice. Condensed-audio style.
- *Why:* Refold-style passive listening of material already understood ([S](https://zenith-raincoat-5cf.notion.site/Passive-Condensed-audio-14c02a59ba934d0b9649cc054186cd44));
  fills the gap until Android exists (commute, walks).
- *Impact* M · *Effort* S (concatenate cached WAVs; encode) · *Depends on* VOICEVOX · *Risk:*
  low; VOICEVOX character credit applies to exported audio too.

**18. "Probably known" suggestions**
- *What:* words not in Anki, met in 8+ paragraphs over 3+ days and never looked up, offered in a
  weekly list to mark known in bulk (not automatic).
- *Why:* better known-word lists make generated texts and coverage more accurate. LingQ's
  automatic version inflates counts (landscape.md); a suggested list keeps the user in control.
- *Impact* M · *Effort* S (exposure events exist) · *Depends on* nothing · *Risk:* low.

**19. From RTK to words: kanji you just learned, in context**
- *What:* a target source "words using kanji you learned in RTK recently" for Write for me and
  cards, plus the kanji panel showing "words with this kanji in today's text".
- *Why:* RTK gives meaning but not readings or words; the owner learns 2,508 kanji this way, and
  linking new kanji to words in context is the known RTK gap. Satori syncs known kanji from
  WaniKani for the same reason ([S](https://satorireader.com/help)).
- *Impact* M · *Effort* S (`kanji.rs` has the kanji table and words) · *Depends on* nothing ·
  *Risk:* low.

### Later

**20. Subtitles and local video/audio** — import `.srt`/`.ass` as texts (S); play a local video
or audio file in sync with the reader and put the real sentence audio on cards (L). Anime and
drama are what keep many learners going; asbplayer and Language Reactor cover the
playback side already ([S](https://www.github.com/killergerbah/asbplayer/releases)), so start with
the import only. *Impact* H for media learners · *Effort* S then L · *Risk:* video codecs in
WebView; overlaps with asbplayer.

**21. Conversation partner at your level** — text chat first: the AI talks about the current
text or an interest topic using known words (coverage-checked like Write for me), corrects gently
on request; voice later (VOICEVOX out; speech recognition in needs Whisper or a cloud API).
Evidence for AI tutors is weak ([S](https://languatalk.com/blog/speak-app-review/)), but a
level-filtered partner is something Speak and ChatGPT lack. *Impact* M · *Effort* M (text) / L
(voice) · *Risk:* cost per turn; drift above level.

**22. Android reader** (roadmap, ADR 0004/0005) — reading where the habit happens; AnkiDroid's
`cards` URI makes it possible (landscape.md). *Impact* H · *Effort* L · *Depends on* spikes S1/S2,
sync.

**23. 発話表現 listening and listening in the level check** (roadmap) — 発話表現 is part of the
N3 test, so it matters for this learner; pictures could be described scenes or AI images.
統合理解 is N2/N1 only, so it can wait. *Impact* L-M · *Effort* M · *Risk:* picture items without
images feel unlike the exam.

**24. AnkiMorphs and Lute imports** (roadmap M4) — welcome users of archived AnkiMorphs. Little
value for the owner. *Impact* L (owner) / M (adoption) · *Effort* S.

**25. Audio cache clean-up** (ADR 0017 gap) — size limit or "older than N days, not on a card".
Housekeeping; do it with #3/#17. *Impact* L · *Effort* S.

**26. Italian and Spanish** (owner's stated next feature, ADR 0001) — no effect on Japanese
learning; large (tokenizer and lemmatizer). Ranked here only by this report's goal; the owner's
priority decides.

### Probably not

**27. XP, leagues, leaderboards, gems.** Invites farming instead of learning ([arXiv study, S](https://arxiv.org/pdf/2203.16175));
Ankura has one user and no social layer.

**28. Mascots, gardens, hearts.** Renshuu's charm works for some ([S](https://cotoacademy.com/learning-japanese-with-renshuu-app-review/)),
but it costs design time with little effect on learning. A good streak, heatmap and visible
progress (#13) cover most of the motivation.

**29. Auto-marking words known on page turn (LingQ).** Inflates the known list that Write for me
and coverage depend on; #18 gets the benefit without the error.

**30. Automatic pronunciation scores.** Speech-recognition scoring is unreliable on accents and
noise ([S](https://appireview.pages.dev/com.selabs.speak)); record-and-compare (#16) is honest
and cheap.

**31. Reading counts as an Anki review / rescheduling from Ankura.** Tempting for the backlog,
but it breaks ADR 0007 (add-only) on a 20-year collection. #2 helps the backlog without touching
the schedule.

---

## 4. Summary table

| # | Feature | Impact | Effort | Group |
|---|---|---|---|---|
| 1 | "Today" one-button daily session | H | S-M | Next |
| 2 | Backlog rescue: texts from overdue cards | H | S | Next |
| 3 | Sentence audio on Anki cards | M-H | S | Next |
| 4 | Clipboard reader + URL import | H | S-M | Next |
| 5 | Pre-learn a text (top unknown words) | H | S | Next |
| 6 | Serial stories with word recycling | H | M | Next |
| 7 | Three questions after each text | M-H | S-M | Next |
| 8 | Frequency + pitch dictionaries | M-H | S | Next |
| 9 | Word highlight, auto-pause, listen first | M-H | S-M | Soon |
| 10 | Fading-word cloze game | M-H | M | Soon |
| 11 | Write it back (output + corrections) | M-H | M | Soon |
| 12 | Grammar points in the reader | H | M-L | Soon |
| 13 | Immersion log, speed, goals, heatmap | M | S | Soon |
| 14 | Incremental automatic Anki sync | M | M | Soon |
| 15 | EPUB import with progress | M-H | M | Soon |
| 16 | Shadowing mode | M | M | Soon |
| 17 | Audio export for the phone | M | S | Soon |
| 18 | "Probably known" suggestions | M | S | Soon |
| 19 | RTK kanji → words in context | M | S | Soon |
| 20 | Subtitles, then local media sync | H | S→L | Later |
| 21 | Conversation partner at your level | M | M/L | Later |
| 22 | Android reader | H | L | Later |
| 23 | 発話表現 + level-check listening | L-M | M | Later |
| 24 | AnkiMorphs / Lute imports | L | S | Later |
| 25 | Audio cache clean-up | L | S | Later (with #3) |
| 26 | Italian / Spanish | none for JP | L | Later (owner decides) |
| 27-31 | Leagues/XP, mascots, auto-known, pronunciation scores, rescheduling Anki | — | — | Probably not |

**Why this order.** #1-#2 cost little and turn the existing pieces into a daily routine aimed at
the owner's real problem (fading and overdue words after a break). #3-#5 and #8 are small changes
with a big effect: more real input, words learned before they're needed, audio in every review.
#6-#7 add the spaced repetition and retrieval that one-off texts lack, and the story keeps it fun.
The "soon" items are either larger (grammar, EPUB, cloze game, sync) or build on the "next" ones.
Android and media sync are high impact but large, so they come later.
