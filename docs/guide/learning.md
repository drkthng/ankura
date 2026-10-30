# How to learn with Ankura

[← README](../../README.md)

How to use Ankura, as it is today, to learn Japanese as fast as the research suggests you can.
It is written for a learner around JLPT N4-N3 with a big Anki habit, coming back after a break.
The evidence behind each point is in [Learning research](../research/learning-evidence.md), with
sources and how strong the evidence is. Ideas for what Ankura should do next are in
[Feature ideas](../research/feature-ideas.md).

## The short version

1. **Read a lot of text you mostly understand.** Over months, reading volume is what moves you
   forward. Read at least 95% known words; read at 98% when you read for fluency or fun.
2. **Turn look-ups into repetitions.** Reading alone teaches about 1 in 10 new words. A word you
   look up, put on a card and then meet again in texts over the next days usually sticks. Most
   words need about 10 encounters, spread out.
3. **Keep Anki small and steady.** Spaced retrieval is the most efficient way to learn a word, but
   reviews shouldn't eat your reading time. Long term, Anki should take about a third of your
   study time.
4. **Listen while you read, at the voice's pace.** It helps comprehension only when the audio sets
   the speed, not when you stop and start, and it builds the sound-to-word links that Anki and RTK
   don't.
5. **Same time every day, with a forgiving streak. Test every 4-8 weeks, not more often.**

## A day with Ankura (about 60 minutes)

| Time | What | Where in Ankura |
|---|---|---|
| 15-20 min | Anki reviews, few or no new cards while you have a backlog | Anki, then **Anki → Sync now** in Ankura |
| 15-20 min | One new text around your practice words: read it, look words up, make cards | **Write for me** → reader → **Cards** |
| 10-15 min | Read it again while it's read aloud, at normal speed | reader: **▶ Listen** (or **P**) |
| 10 min | Listen without reading, or shadow it (repeat after the voice) | reader: **Listen**, look away |

Only 20 minutes? Do 10 minutes of Anki, then one short text read with **Listen**. The reading is
the part not to skip: keep at least 15 minutes of it every day, even when reviews pile up.

Several short sessions work better than one long one. Stop reading when you notice you're no
longer taking things in.

## One session, step by step

1. **Sync.** After your Anki reviews, press **Sync now** on the **Anki** screen so Ankura's colours and
   word choices match today's Anki.
2. **Write a text.** On **Write for me**, pick 5-8 practice words. Take them in this order:
   - **fading words** (Ankura suggests them: your overdue Anki cards);
   - **stealth leeches** from **Stats** (Anki says you know them, but you keep looking them up);
   - words you looked up recently.

   Leave the topic empty (Ankura picks from your [interests](write-for-me.md#your-interests)) or
   choose one. 500 characters is a good length.
3. **First reading: your pace.** Look up every word you're not sure of; it costs one click and
   each look-up teaches Ankura what you don't know. For a word worth learning, make a card
   **with this sentence**. **I don't know this** marks it for practice.
4. **Second reading: the voice's pace.** Press **P**, keep the speed at 1×, and follow the
   highlighted sentence without stopping. This pass is what improves listening and reading speed.
5. **Optional third pass: listen only**, or shadow: pause after each sentence and say it aloud.
   10-15 minutes, a few times a week, helps listening most for learners at your level.
6. **Cards.** On **Cards**, check the drafts. If a card isn't what you want, tell the AI under it
   ("an example with ほど…ない", "the meaning used in the text") and press Enter. Then **Add to
   Anki**.
7. **Tomorrow:** use some of the same words again, on the same or a related topic (next section).

## Meet each word many times

One text uses each practice word a couple of times. That's not enough: aim for about 10
encounters, spread over days and weeks.

- **Write small series.** Write 3-5 texts on one topic over a few days, keeping about half of the
  practice words from the last text and adding a few new ones. Texts on one topic repeat their
  vocabulary by themselves (this is called narrow reading).
- **Reread old texts.** Your **Library** keeps every text. Rereading with **Listen** a few days
  later is cheap and counts.
- **Feed missed words back in.** Words you got wrong in a [practice test](practice-test.md) or
  looked up twice go into the next text's practice words.

## Coming back after a break: the Anki backlog

After months away, many cards are overdue. Ankura can't change your Anki schedule, by design. So
handle the backlog in Anki and use Ankura to make the reviews easier:

**In Anki** (expert advice from the FSRS developers and the Anki community, not controlled studies):

1. **Don't reset your collection.** The overdue cards still hold most of what you knew.
2. **Turn on FSRS** (Deck options → FSRS, Anki 23.10 or newer) and press *Optimize*. FSRS takes the
   overdue time into account when it schedules a card. Ankura then uses Anki's own recall
   estimates instead of guessing them (sync again afterwards).
3. **Set desired retention to 0.85 while you catch up, then 0.90.** Don't go above about 0.95:
   the workload rises steeply.
4. **Pause new cards**, and cap reviews at an amount you can do every day (for example 150-300).
5. **Sort reviews by "Retrievability descending"** (Deck options → Display order). Cards you can
   still save come first; the ones that are long gone wait.
6. **Answer honestly**, and expect weeks, not days.

**In Ankura:**

- Use **fading words** as practice words. Meeting a word in a text just before you review it
  makes the review easier.
- **Don't mark a word known** in the reader because it looks familiar. Reading a word is not a
  review, and a wrong "known" makes texts look easier than they are.
- Coverage is measured against what Anki says you know. While many cards are overdue, a text is
  harder than its coverage number says. If a 95% text feels hard, that's why: write shorter texts
  or use fewer practice words for a while.

## How hard should a text be?

| Known words in the text | What it's like | Use it for |
|---|---|---|
| 98% and more | Easy: you rarely stop | Fluency, listening, rereading, fun |
| 95-97% | Work, but it flows; meanings can be guessed | New practice words (Write for me's default) |
| 90-95% | Slow and tiring | Short texts you really want to read, with look-ups |
| Under 90% | Frustrating | Not yet: Simplify sentences, or come back later |

With about 1,600 known words, texts for native readers will mostly be at 80-90% for you: Japanese
needs about 9,000-10,000 words for 95% of a typical native text. That's why Write for me exists.
Mix in graded readers (Tadoku, free) and NHK News Web Easy: paste them into the **Library** and
check their coverage first. Natural texts also guard against the slightly unnatural phrasing AI
texts sometimes have.

**Simplify this sentence** (in the word pop-up) is for the one sentence you can't crack, not a
way to read texts that are too hard.

## Kanji (RTK)

- **Keep stories short and vivid**, built on RTK's primitives. Mnemonics help memory quickly but
  fade without review, and long stories lose the meaning. When a story doesn't work, **New
  mnemonic** in the kanji panel writes another; the **Kanji** screen keeps primitive names
  consistent across all your stories.
- **RTK keywords aren't words.** Learn kanji through the words you read: the kanji panel in the
  word pop-up shows each kanji's keyword and story while you read.
- Breadth pays: about 1,000 common kanji cover about 95% of the kanji in newspapers.

## Tests: when and how

- **Level check every 4-8 weeks.** It tells you what level to read and practise at. A few points
  up or down is noise; look at the trend.
- **Practice test about once a month** when you're preparing for a JLPT. Half length, with 聴解,
  timer on. Review every mistake and its explanation, send missed words to cards, and use **Retry
  the missed** two or three days later.
- **Don't retake the same questions.** Leave *Include questions from earlier tests* off for real
  tests; reused questions measure memory, not skill. Turn it on only for drill.
- **Tests don't replace reading.** They sample what you can do; reading is what builds it.
  Answering questions is valuable in itself, though: Anki reviews and test questions with their
  explanations teach as well as measure.

## Habits

- **Fix a time and a plan in one sentence**, for example "After dinner: Anki, then one Ankura
  text." Plans like this make people noticeably more likely to follow through.
- **A missed day is fine.** Habits survive single missed days. Ankura's reading streak only counts
  days you read; the totals on **Stats** (characters read, words met) never go back to zero.
- **Read about things you care about.** Set your [interests](write-for-me.md#your-interests);
  interesting topics make you read longer, and reading time is what counts.
- **Watch progress that means something:** coverage of new texts going up, stealth leeches going
  away, the level check moving.

## What Ankura doesn't do yet

- **Speaking and writing.** Producing language shows you what you can't say yet. For now, write
  2-3 sentences about each text in a notebook or a chat, or find a language exchange partner.
- **Audio on Anki cards, serial stories, questions after each text, importing web pages and
  books:** see below.

## What could come next

Ranked by how much they would speed up learning or make it more fun, compared to the work they
take (full list with sources: [Feature ideas](../research/feature-ideas.md)):

1. **"Today": one button for the daily session.** Fading words, a text, listening, quick
   questions and cards in one flow, instead of six screens.
2. **Backlog rescue.** Texts written around your most overdue Anki cards, read before you review
   them.
3. **Sentence audio on Anki cards.** The VOICEVOX audio Ankura already makes, added to new cards.
4. **Clipboard reader and web import.** Copied lines from games, visual novels and web pages flow
   straight into the coloured reader.
5. **Pre-learn a text.** The 5-10 unknown words that make a text readable, before you start it.
6. **Serial stories.** Continuing episodes that bring back earlier words over several days.
7. **Three questions after each text.** A quick comprehension and cloze check.
8. **Frequency and pitch-accent dictionaries.** Frequency ranks for choosing words; pitch accent
   in the pop-up.

Then: word-level highlighting and auto-pause while listening, a cloze game for fading words,
writing practice with corrections, grammar points in the reader, a reading log with speed and
goals, automatic Anki sync, EPUB import, shadowing, audio export for the phone.

Not planned: points, leagues and badges (they reward farming points rather than learning), and
changing your Anki schedule from Ankura.
