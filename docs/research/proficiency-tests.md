# Research: proficiency tests for Ankura

Researched 2026-10-27 for the ROADMAP item "Proficiency test". This is the evidence the design
will rest on, not the design itself.

**How this was researched.** The research environment could not open most primary sites directly
(jlpt.jp, coe.int, cervantes.es, unistrasi.it, unistrapg.it, lextale.com, Wikipedia all refused the
connection). Facts below come from search-engine extracts of those pages and PDFs, which quote the
official text, plus background knowledge where marked. Links point at the primary page the fact
comes from. Claims marked *(unverified)* could not be confirmed from any source text in this
session and should be checked before they reach the UI. Numbers that will be shown to users
(pass marks, CEFR tables) should be re-read from the official page when the feature is built.

## Summary

- **JLPT** tests only reading, listening and "language knowledge" (script, vocabulary, grammar).
  Scores are IRT-scaled, so a raw-count mock can't reproduce a JLPT score. It can estimate a
  level and a pass likelihood.
- **There are no official JLPT word or kanji lists since 2010.** The commonly quoted counts
  (N5 ~800 words / ~100 kanji up to N1 ~10,000 / ~2,000) are the pre-2010 official numbers
  stretched over the new five levels. Treat them as rough.
- **Official JLPT questions can't be copied.** The Japan Foundation sells two official workbooks
  and publishes free sample questions; both are copyright and marked "do not reproduce".
- **The Japan Foundation now has an official JLPT-to-CEFR reference** (from the December 2025
  test): N5 pass = A1, N4 = A2, N3 = A2 or B1, N2 = B1 or B2, N1 = B2 or C1, by total score.
  Folk mappings like "N2 = B2" are wrong for low passers.
- **For a quick in-app test,** the best-supported building blocks are a yes/no vocabulary test
  with pseudowords (LexTALE family; a Japanese version, LexJP, appeared in 2025), short
  multiple-choice grammar/vocabulary items, and a few short reading passages. C-tests work in
  Japanese but are harder to generate correctly.
- **Adaptivity needs calibrated items,** which a single-user app does not have at first. A
  two-stage test with difficulty priors from frequency and level tags is the honest v1.
- **Italian:** model on CILS (it has a separate, machine-scorable grammar/structures section) and
  use PLIDA's public specifications as item-writing guides. **Spanish:** DELE for level mocks,
  SIELE's single multi-level scale for the general test's report.

## 1. JLPT N5-N1

### Levels and can-do descriptions (official)

The official [level summary](https://www.jlpt.jp/e/about/levelsummary.html) describes each level
by what the examinee can read and listen to, and adds that vocabulary and grammar knowledge are
needed to do those things. N4 and N5 cover "basic Japanese mainly learned in class"; N1 and N2
cover "Japanese used in a broad range of scenes in actual everyday life"; N3 bridges the two.
In short (paraphrased; read the page for the exact text):

| Level | Reading | Listening |
|---|---|---|
| N1 | Logically complex or abstract texts (editorials, critiques); profound texts, following narrative and the writer's intent | Coherent conversations, news, lectures at natural speed in many settings; relationships, logical structure, main points |
| N2 | Clearly written articles and commentaries in newspapers and magazines, simple critiques; general texts and writer's intent | Conversations and news at nearly natural speed, everyday and a variety of settings |
| N3 | Everyday texts with specific content; newspaper headlines; slightly difficult texts if paraphrases are available | Coherent everyday conversations at near-natural speed; relationships between speakers |
| N4 | Passages on familiar daily topics in basic vocabulary and kanji | Daily-life conversations, if spoken slowly |
| N5 | Typical expressions and sentences in hiragana, katakana and basic kanji | Slow, short classroom and daily-life conversations; picking out needed information |

The **JLPT Can-do Self-Evaluation List** ([index](https://www.jlpt.jp/e/about/candolist.html),
[reading](https://www.jlpt.jp/e/about/candolist_reading.html)) is a separate thing: about 65,000
examinees in 2010-2011 rated 20 statements per skill (listening, speaking, reading, writing), and
the list shows what passers near the pass line said they could do. The JLPT says it is "not a
syllabus" and does not guarantee passers' ability. Useful as a self-assessment item pool (with
attribution; check the site policy before shipping the text), not as a definition of the levels.

### Sections and times

Current [test sections and times](https://www.jlpt.jp/e/guideline/testsections.html):

| Level | Sections (minutes) |
|---|---|
| N1 | 言語知識（文字・語彙・文法）・読解 110 · 聴解 55 |
| N2 | 言語知識（文字・語彙・文法）・読解 105 · 聴解 50 |
| N3 | 言語知識（文字・語彙）30 · 言語知識（文法）・読解 70 · 聴解 40 |
| N4 | 言語知識（文字・語彙）25 · 言語知識（文法）・読解 55 · 聴解 35 |
| N5 | 言語知識（文字・語彙）20 · 言語知識（文法）・読解 40 · 聴解 30 |

N4/N5 times and item counts were cut from the December 2020 test
([notice](https://www.jlpt.jp/topics/202011051597200631.html)); N1 listening went from 60 to 55
minutes from December 2022 ([notice](https://www.jlpt.jp/topics/202208051659676565.html)). Older
PDFs on jlpt.jp still show the old numbers. Per-question-type item counts are published only as
"目安" (approximate targets) and change; the 2009 [guidebook](https://www.jlpt.jp/reference/pdf/guidebook1e.pdf)
has the original table.

### Question types (大問) and what they test

From the 2009 guidebook's "大問のねらい" pages and the per-level guideline PDFs
([N1](https://www.jlpt.jp/guideline/pdf/n1.pdf), [N2](https://www.jlpt.jp/guideline/pdf/n2.pdf),
[N4](https://www.jlpt.jp/guideline/pdf/n4.pdf)). The level ranges are from background knowledge
and match the guidebook tables as far as the extracts showed; confirm before relying on them.

| Section | Type | What it tests | Levels |
|---|---|---|---|
| 文字・語彙 | 漢字読み | Reading of a kanji word in a sentence (choose the kana) | all |
| | 表記 | Writing a kana word in kanji (choose the kanji spelling) | N2-N5 |
| | 語形成 | Derived and compound words (prefixes, suffixes) | N2 |
| | 文脈規定 | Choose the word that fits the context | all |
| | 言い換え類義 | Choose the word or phrase closest in meaning | all |
| | 用法 | Choose the sentence that uses a word correctly | N1-N4 |
| 文法 | 文の文法1（文法形式の判断） | Choose the grammar form that fits the sentence | all |
| | 文の文法2（文の組み立て） | Order four fragments; answer is the one in the ★ slot | all |
| | 文章の文法 | Choose forms that fit the flow of a passage | all |
| 読解 | 内容理解（短文/中文/長文） | Short (~200 chars), mid (~500) and long (~1000) texts: main idea, reasons, details | short and mid all; long N1-N3 |
| | 統合理解 | Compare two or more short texts | N1, N2 |
| | 主張理解（長文） | Grasp the writer's argument in an editorial-style text | N1, N2 |
| | 情報検索 | Find information in ads, notices, brochures | all |
| 聴解 | 課題理解 | Hear a task conversation, decide what to do next | all |
| | ポイント理解 | Listen for a specific point (reason, problem) | all |
| | 概要理解 | Grasp the speaker's intent or overall point | N1-N3 |
| | 発話表現 | Look at a picture, choose the right utterance | N3-N5 |
| | 即時応答 | Hear a short line, choose the right reply | all |
| | 統合理解 | Combine information from a longer exchange | N1, N2 |

All items are four-option multiple choice (some listening items have three options). There is no
speaking or writing.

### Scoring

From the [results page](https://www.jlpt.jp/e/guideline/results.html) and the
[2011 notice](https://jlpt.jp/sp/e/topics/201101311296451824.html) setting the standards:

| Level | Scoring sections (range) | Overall pass | Sectional minimum |
|---|---|---|---|
| N1 | Language knowledge 0-60, Reading 0-60, Listening 0-60 | 100/180 | 19 each |
| N2 | same | 90/180 | 19 each |
| N3 | same | 95/180 | 19 each |
| N4 | Language knowledge + Reading 0-120, Listening 0-60 | 90/180 | 38 and 19 |
| N5 | same as N4 | 80/180 | 38 and 19 |

Failing one sectional minimum fails the test regardless of the total.

Scores are **scaled with item response theory**, not counted: the
[scaled score explainer](https://www.jlpt.jp/e/about/pdf/scaledscore_e.pdf) says the pattern of
right and wrong answers is the input, and that this is "completely different" from counting
correct answers. A [2022 Language Testing article](https://journals.sagepub.com/doi/10.1177/02655322221080898)
also describes JLPT raw scores as IRT-scaled and equated across forms. Consequence for Ankura: no
fixed "N correct = pass" rule exists, and an app mock can't claim a JLPT score.

The score report also has "reference information" letter grades for sub-areas such as 文字・語彙
and 文法: A = 67% or more correct, B = 34-67%, C = under 34%
([score report guide 2025](https://www.jlpt.jp/guideline/pdf/understandingscorereport_2025.pdf)).
This is a usable, official-looking way to show sub-area results.

### Vocabulary and kanji counts

**Official, pre-2010 only.** The old four-level JLPT published 出題基準 (Test Content
Specifications) with word and kanji lists. Its headline figures, as commonly reproduced (e.g.
[Wikipedia](https://en.wikipedia.org/wiki/Japanese-Language_Proficiency_Test)), were: Level 4
~100 kanji / ~800 words / ~150 study hours; Level 3 ~300 / ~1,500 / ~300 h; Level 2 ~1,000 /
~6,000 / ~600 h; Level 1 ~2,000 / ~10,000 / ~900 h. *(These figures are from background knowledge;
the search could not reach a copy of the 出題基準 itself.)*

**Since 2010, none.** The JLPT FAQ (quoted in a
[WaniKani thread](https://community.wanikani.com/t/does-anyone-know-where-any-of-these-jlpt-kanji-lists-originated/32774))
says that because the test measures the ability to use knowledge for communication, publishing
lists of vocabulary, kanji and grammar "was not necessarily appropriate", and points to the level
summaries, the test-item composition and the sample questions instead. The 2009 guidebook says the
new N1/N2/N4/N5 sit at roughly the old levels 1/2/3/4 (N1 slightly harder) and N3 is new.

**Community estimates.** Widely repeated: N5 ~800 words / ~100 kanji, N4 ~1,500 / ~300,
N3 ~3,700 / ~650, N2 ~6,000 / ~1,000, N1 ~10,000 / ~2,000 (cumulative), e.g.
[Skritter](https://blog.skritter.com/2015/02/jlpt-n5-n1-lists),
[japonin](https://www.japonin.com/jlpt-study-guide.html). These are the old official figures with
N3 interpolated. The actual community lists are much shorter: dictionary tag counts run about
650-750 words for N5 and about 3,000 for N1 ([JapanDict](https://www.japandict.com/lists/jlpt)),
because the lists are incremental and incomplete.

**Where the lists come from.** Jonathan Waller's [tanos.co.uk](https://www.tanos.co.uk/jlpt/)
lists are the source for most sites, apps and Anki decks (jlptsensei, Yomitan tags, many GitHub
JSON dumps). They are based on the old 出題基準 lists, with the old Level 2 split to make N3, plus
some later adjustment. They are licensed **CC BY** (stated by derived projects such as
[Bluskyo/JLPT_Vocabulary](https://github.com/Bluskyo/JLPT_Vocabulary); confirm on tanos itself).
Kanjikana describes its lists as old lists adjusted by observed frequency in recent tests
([kanjikana](https://kanjikana.com/en/kanji/jlpt)).

**Reliability.** Good enough for a *difficulty prior* ("this word is roughly N3"), not for
claims like "you know 82% of N2 vocabulary". The N3 boundary is the weakest; a 2013 EDRDG thread
notes the confusion caused by applying old lists to the new levels
([edrdg](https://www.edrdg.org/jmdict_edict_list/2013/msg00045.html)). JMdict itself has no JLPT
tags; its priority tags (news1/2, ichi1/2, nf01-nf48) are a rough frequency signal.

### Official sample questions and workbooks

- **Free sample questions** per level, with audio, on jlpt.jp
  ([learners' page](https://www.jlpt.jp/samples/forlearners.html)). The sample-question page says
  the questions must not be reproduced without permission.
- **『日本語能力試験公式問題集』** (2012) and **第二集** (2018), sold by 凡人社. Each has one full
  test's worth of real past questions per level ([index](https://www.jlpt.jp/samples/sampleindex.html)).
  As of this search there are two; no third volume was found.
- **Licence:** the [FAQ](https://www.jlpt.jp/faq/) says reproducing test questions without
  permission is prohibited by law and that some questions include third-party copyright material.
  The [site policy](https://www.jlpt.jp/policy.html) allows reuse of some site content with a
  credit line ("出典 日本語能力試験公式ウェブサイト"), but the question pages are explicitly
  excluded. **Ankura must not ship or paraphrase-copy these questions.** It may reproduce the
  *format* (question types, instructions style, timing), which is described in public guides and
  is not protected as such. The workbooks are fine for the owner's own calibration (take a
  workbook test, take Ankura's test, compare) but not as app content.

## 2. Italian and Spanish exams

### Italian

The four certificates recognised under the Foreign Ministry's **CLIQ** quality scheme (2013) are
CILS (Siena), CELI (Perugia), CERT.IT (Roma Tre) and PLIDA (Dante Alighieri)
([esteri.it](https://www.esteri.it/en/sala_stampa/archivionotizie/comunicati/2013/02/20130206_cliq/)).
CLIQ certificates are the ones accepted for the B1 citizenship requirement.

**CILS** (Università per Stranieri di Siena). Levels A1, A2, UNO (B1), DUE (B2), TRE (C1),
QUATTRO (C2), plus integration and citizenship variants. From B1 up there are five sections:
Ascolto, Comprensione della lettura, **Analisi delle strutture di comunicazione**, Produzione
scritta, Produzione orale ([Padova CLA](https://cla.unipd.it/certificazioni/certificazione-di-italiano-cils/esame/);
B2 times in the [official instructions](https://cils.unistrasi.it/public/articoli/28/Files/istruzioni(5).pdf):
listening 30 min, reading 50, structures 60, writing 70, oral ~10). The structures section uses
multiple choice, cloze and completion at B1
([IIC Buenos Aires B1](https://iicbuenosaires.esteri.it/wp-content/uploads/2025/05/Contenuti-Livello-B1.pdf))
and adds sentence transformation at C1 ([C1](https://iicbuenosaires.esteri.it/wp-content/uploads/2025/05/Contenuti-livello-C1.pdf)).
Scoring: each section out of 20, pass mark 11/20 in each
([Pavia CLA](https://cla.unipv.it/?page_id=54510)); A1/A2 use a smaller scale (total 60 instead of
100, per [IIC Cairo](https://iiccairo.esteri.it/it/news/dall_istituto/2025/04/lesame-cils-certificati-di-conoscenza-della-lingua-italiana/)).
Sources differ on the overall-total rules; check Siena's regulations.

**CELI** (CVCL, Università per Stranieri di Perugia). CELI Impatto (A1), CELI 1 (A2), CELI 2
(B1), CELI 3 (B2), CELI 4 (C1), CELI 5 (C2) ([CVCL](https://cvcl.unistrapg.it/pagine/esami-celi-generici)).
Written part in booklets: reading + writing, **competenza linguistica** (grammar: tenses,
conditional, subjunctive, passive, connectives), and listening; then a separate oral
([CELI 3 competences](https://www.unistrapg.it/sites/default/files/docs/certificazioni/celi-3-competenze-richieste.pdf)).
CELI 3 needs at least 84 points on the written part and 33 on the oral
([CELI 3 scoring](https://www.unistrapg.it/sites/default/files/docs/certificazioni/celi-3-valutazione.pdf));
written and oral must both pass ([IIC Montevideo](https://iicmontevideo.esteri.it/it/lingua-e-cultura/certificazioni/celi-certificato-di-lingua-italiana/)).
Higher CELI levels report grades A-E, A-C passing *(unverified; only a third-party blog)*.

**PLIDA** (Società Dante Alighieri). A1-C2, four tests: Ascoltare, Leggere, Scrivere, Parlare,
each out of 30, pass 18/30 in each; passed parts can be kept while retaking others
([Dante](https://societadantealighieri.org/plida/), [unibz](https://guide.unibz.it/it/languages/international-certificates/plida)).
No separate grammar section. Its strength for us: the **published per-level specifications**
("Quaderno delle specifiche", e.g. [B1](https://cms.plida.dante.global/wp-content/uploads/2022/11/Quaderno-delle-specifiche-PLIDA-B1.pdf),
67 pages) and exam guides ([A2](https://cms.plida.dante.global/wp-content/uploads/2025/08/PLIDA-A2-inItalia-Guida-allesame-1.pdf):
listening 25 min, two parts, 15 questions). These are the most detailed public task specs of any
Italian exam. Copyright Società Dante Alighieri; use them as a design reference, don't copy items.

**AIL** (Accademia Italiana di Lingua, Florence, 1984). DELI A1/A2, DILI B1/B2, DILC B1 (business),
DALI C1/C2, DALC C1 ([AIL FAQ](https://acad.it/en/faq.html)). Not part of CLIQ, so less widely
recognised ([Europass Italian](https://www.europassitalian.com/learn/certifications/)); its main
official recognition is Swiss. Not worth modelling.

### Spanish

**DELE** (Instituto Cervantes, for Spain's Ministry of Education; Salamanca takes part in exam
design and marking). One exam per level, A1-C2, result *apto/no apto*, no expiry. A1-C1 have four
tests of 25 points each, grouped in two blocks of 50: reading + writing, and listening + speaking;
you need 30 in each block, 60 overall ([DELE B2 guide](https://examenes.cervantes.es/sites/default/files/guia_examen_dele_b2_0.pdf);
the grouping is reported inconsistently by third parties, the official guide is authoritative).
B2 reading: 4 tasks, 36 items (6-10-6-14), 70 minutes, scored automatically and converted to 25
points ([B2 specifications](https://examenes.cervantes.es/sites/default/files/dele-b2_especificaciones-2018.pdf)).
C2 has three integrated tests (use of language + reading + listening; integrated written skills;
integrated reading + speaking), each about a third of the points, all of which must be passed
([Cervantes C2](https://agenda.cervantes.es/es/node/27166886),
[A por el DELE](https://www.aporeldele.com/en/blog/como-se-corrige-el-dele-claves-para-obtener-el-apto)).
Free model exams are on examenes.cervantes.es (copyright Instituto Cervantes).

**SIELE** (Instituto Cervantes, Salamanca, UNAM, Universidad de Buenos Aires). One computer-based,
**multi-level** exam covering A1-C1 (no C2). Four tests (reading, listening, writing, speaking),
each 0-250, total 0-1000; each test score maps to a CEFR level; certificate valid 5 years
([IC Chicago](https://chicago.cervantes.es/en/siele_certificate_spanish/siele_exam_spanish.htm),
[IC Manila](https://manila.cervantes.es/en/diplomas_spanish/spanish_exams_SIELE_about.htm)).
Tasks get harder through each test, but no source describes it as item-adaptive. The CEFR
cut-scores quoted by prep sites conflict, so use only official tables.

### Which to model

| | CILS | CELI | PLIDA | DELE | SIELE |
|---|---|---|---|---|---|
| CEFR | A1-C2 | A1-C2 | A1-C2 | A1-C2 | A1-C1 |
| One exam per level | yes | yes | yes | yes | no (one scale) |
| Separate grammar/vocab section | yes (strutture) | yes (competenza linguistica) | no | no (C2 only: uso de la lengua) | no |
| Public task specs | contents sheets | competence sheets | full specification books | full specifications | handbook |

- **Italian: model on CILS.** Its five-section layout maps cleanly onto Ankura's JLPT design
  (receptive skills plus a scored grammar/structures section) and it is the most widely taken
  Italian certificate. CELI is a close second and very similar. Use **PLIDA's specification
  books** as the item-writing reference for reading and listening tasks, because they say the
  most about text types, lengths and item counts per level.
- **Spanish: DELE for the per-level mock, SIELE for the general test's report.** DELE matches the
  JLPT model (pick a level, pass or fail). SIELE's single 0-1000-style scale with CEFR bands is
  what the general adaptive test produces anyway. The Plan Curricular del Instituto Cervantes
  (2006, three volumes A1-A2, B1-B2, C1-C2) is the standard level-by-level inventory of grammar,
  functions and notions for Spanish ([overview](https://marcoele.com/los-niveles-de-referencia-para-la-ensenanza-de-la-lengua-espanola/));
  it is the best source for level-tagging generated Spanish items.
- Speaking and writing sections of all of these are out of scope for v1.

## 3. CEFR and the JLPT mapping

**CEFR levels.** Six levels A1-C2 in three bands (basic, independent, proficient), with "plus"
levels A2+, B1+, B2+ ([CoE level descriptions](https://www.coe.int/web/common-european-framework-reference-languages/level-descriptions)).
The global scale, in brief: A1 can understand and use familiar everyday expressions and very
basic phrases; A2 handles routine exchanges on familiar matters; B1 understands the main points
of clear standard input on familiar matters; B2 understands the main ideas of complex text,
including technical discussion in their field; C1 understands a wide range of demanding, longer
texts and recognises implicit meaning; C2 understands with ease virtually everything heard or read.
Descriptors are can-do statements per activity (reception, production, interaction, mediation)
and per competence, and are language-neutral.

**Companion Volume (2020)** ([PDF](https://rm.coe.int/16809ea0d4),
[CoE](https://www.coe.int/en/web/education/-/common-european-framework-of-reference-for-languages-learning-teaching-assessment-14)):
replaces the 2001 descriptor set with modality-inclusive, gender-neutral wording; adds a Pre-A1
level; enriches A1 and the C levels; adds scales for mediation, online interaction,
plurilingual/pluricultural competence and sign languages; replaces the phonology scale; adds
detail to reading and listening. For Ankura the relevant scales are reading (overall reading
comprehension, reading for information, reading as a leisure activity) and the linguistic
competence scales (vocabulary range, grammatical accuracy). The leisure-reading scale is new in
the Companion Volume and fits Ankura's purpose well.

**Japan Foundation position.** From the **December 2025** test, JLPT score reports show a CEFR
level as reference information for passers only
([JF news](https://www.jpf.go.jp/e/project/japanese/teach/tsushin/news/202601.html),
[jlpt.jp CEFR page](https://www.jlpt.jp/e/cefrlevel/index.html),
[announcement](https://www.jlpt.jp/cefrlevel/cefrnews.html)):

| Level | Total score → CEFR |
|---|---|
| N5 | 80+ → A1 |
| N4 | 90+ → A2 |
| N3 | 95-103 → A2; 104+ → B1 |
| N2 | 90-111 → B1; 112+ → B2 |
| N1 | 100-141 → B2; 142+ → C1 |

The link was made by a standard-setting procedure (self-audit of content, standardisation
training and benchmarking with experts, standard setting, validation) reported in a Japanese
report released February 2024 ([FAQ/reference](https://www.jlpt.jp/e/about/cefr_reference.html)).
It covers only CEFR "linguistic" and "reception"; it says nothing about speaking, writing or
interaction, and it stops at C1. An earlier JF study related the JLPT to the CEFR-based JF
Standard ([2017 report](https://www.jfstandard.jpf.go.jp/pdf/jfs_jlpt_report2017.pdf), not read).

**How solid the folk mappings are.** Before 2025 the common chart was N5=A1, N4=A2, N3=B1, N2=B2,
N1=C1 (e.g. [japonin](https://www.japonin.com/articles/jlpt-and-cefr-comparison-chart)), with
some teachers arguing for one level lower. The official table partly confirms both: a high N2 is
B2, but a bare N2 pass is B1; a bare N1 pass is B2. **Use the official table and nothing else**,
and say that it covers reading/listening only.

## 4. Measuring proficiency quickly

### Vocabulary size

Vocabulary size is the single strongest quick predictor of reading ability, and it is what
Ankura already models. Two formats matter:

- **Frequency-band sampling (VST).** Nation & Beglar's Vocabulary Size Test samples 10 items per
  1,000-word family band (140 items for 14k) and multiplies the score by 100
  ([specification](https://www.wgtn.ac.nz/lals/resources/paul-nations-resources/vocabulary-tests/the-vocabulary-size-test/Vocabulary-Size-Test-information-and-specifications.pdf)).
  Free for research without permission. It is for English. The method transfers; the items
  don't. A later study using Japanese learners' data found 5-10 items per band are rarely enough
  and recommends 30+ per band and fewer bands
  ([Gyllstad et al., Language Testing](https://journals.sagepub.com/doi/full/10.1177/0265532220979562)).
  For Japanese L2 there is a Japanese Vocabulary Levels Test built on Matsushita's word list,
  5,000 words, KR-21 .98 in a small university sample
  ([Rothville, JALT](https://jalt-publications.org/content/index.php/jer/article/view/1221)).
- **Yes/no with pseudowords (LexTALE family).** The learner says whether each item is a real
  word; pseudowords catch overclaiming. LexTALE (English) is 60 items, 40 words and 20 nonwords,
  about 5 minutes ([Lemhöfer & Broersma 2012](https://link.springer.com/article/10.3758/s13428-011-0146-0)).
  Versions: English, Dutch, German (free on lextale.com per the
  [gasparl/lextale](https://github.com/gasparl/lextale) implementation), Spanish **LexTALE-Esp**
  (Izura, Cuetos & Brysbaert 2014, Psicológica; [record](https://cronfa.swan.ac.uk/Record/cronfa16250)),
  Italian **LexITA** (Amenta, Badan & Brysbaert, Applied Linguistics 2021;
  [record](https://www.research.unipd.it/handle/11577/3544528)), and Japanese **LexJP** (Suzukida &
  Saito 2025, Research Methods in Applied Linguistics,
  [article](https://www.sciencedirect.com/science/article/pii/S2772766125000291)), validated
  against JLPT levels with 77 learners and hosted on
  [Gorilla Open Materials](https://app.gorilla.sc/openmaterials/1044655).
  Validity is moderate: for Japanese students' English, LexTALE correlated r = .31-.46 with other
  vocabulary and TOEFL scores ([Nakata et al. 2020](https://www.researchgate.net/publication/342893013_Examining_the_Validity_of_the_LexTALE_Test_for_Japanese_College_Students)).
  Scoring: hits minus false alarms does about as well as the more complex signal-detection
  formulas ([Mochida & Harrington 2006](https://journals.sagepub.com/doi/abs/10.1191/0265532206lt321oa);
  [Stubbe & Stewart, Shiken 16(2)](https://teval.jalt.org/sites/default/files/SRB-16-2-Full.pdf)).
- **Generated yes/no tests.** "WikiVocab" builds LexTALE-like tests automatically from a corpus
  (rare real words plus matched pseudowords) for 1,939 languages and separates native, L2 and
  non-speakers in about a minute ([PNAS 2026](https://www.pnas.org/doi/10.1073/pnas.2420179123);
  [preprint](https://arxiv.org/pdf/2302.01614)). The Duolingo English Test uses generated
  pseudowords too, 18 yes/no items per session ([BanditCAT paper](https://arxiv.org/pdf/2410.21033)).
  This is the evidence that Ankura can build its own version rather than license one.

**Licences.** No source states a reuse licence for LexTALE-Esp, LexITA or LexJP items. Gorilla
open materials often use a non-commercial, research-only licence
([Gorilla licence](https://app.gorilla.sc/info/licences/non-commercial-research-only)); if LexJP
uses it, an app may not ship it. **Assume none of these can be shipped**; build Ankura's own lists
with the same method, and email the authors if the owner wants a validated benchmark.

### C-tests and cloze

A C-test deletes the second half of every second word in several short texts; the learner
restores them. C-tests are cheap, quick and correlate well with global written proficiency, and
work across scripts including Japanese, Korean and Bangla
([Georgetown AELRC brief](https://aelrc.georgetown.edu/resources/research-briefs/c-test-research-brief/)).
Norris (ed., 2018) has a validated Japanese C-test chapter by Sasayama
([Peter Lang](https://www.peterlang.com/document/1113405)); five-text C-tests in eight languages
showed strong reliability and correlations with self-assessment and course level. Japanese has
no spaces, so the deletion rule has to be redefined (by character count within a word or 文節),
which makes auto-generation fiddly: the restored part must be unique and the tokenizer must
agree with the learner about where words end.

**SPOT** (Simple Performance-Oriented Test, Tsukuba) is a Japanese-specific relative: the learner
hears a sentence at natural speed and fills one missing hiragana. It is widely used for
placement; it is said to measure real-time processing more than vocabulary or kanji knowledge,
and what it measures exactly is debated
([NINJAL](https://repository.ninjal.ac.jp/record/2000154/files/papers2603.pdf),
[ISCT abstract](https://strdb.s.isct.ac.jp/html/100001979_en.html)). Its items are not free to
reuse. Plain cloze (every nth word) is less reliable than C-tests and multiple-choice rational
cloze; JLPT's 文章の文法 is a rational cloze.

### Adaptive testing

- **J-CAT** (Japanese Computerized Adaptive Test): four sections (vocabulary, listening, grammar,
  reading), each 0-100, IRT-based, no level choice needed
  ([JF article](https://www.jpf.go.jp/j/project/japanese/teach/tsushin/research/201111.html)).
  It was free in 2011 and is now run as a paid service, which limits research use (NINJAL paper
  above). It shows the format works for Japanese.
- **Duolingo English Test** builds an adaptive test from automatically generated items with
  difficulty predicted by language models instead of piloting
  ([Settles, LaFlair & Hagiwara 2020](https://aclanthology.org/2020.tacl-1.17)).
- **How many items.** No source gives a CEFR-specific number. What the literature does give: a
  12-item CAT sorted examinees into five levels "fairly well", better with more items
  ([Behavior Research Methods 2012](https://link.springer.com/article/10.3758/s13428-012-0237-6));
  gains often flatten around 30 items in other domains; precision-based stopping rules
  (stop at SE ≤ 0.3-0.5) are the standard alternative to fixed length
  ([Rasch memo 69](https://www.rasch.org/memo69.pdf)). A back-of-envelope check under the Rasch
  model: a perfectly targeted item gives information 0.25, so SE = 1/√(0.25·n), which is 0.5
  logits after 16 items and 0.4 after 25. Real items are less well targeted and four-option
  guessing lowers information, so **plan on 25-40 scored items per skill estimate** to get an
  interval about one level wide. (This arithmetic is mine, not from a source.)
- **The catch for Ankura:** adaptivity needs item difficulties on a common scale. Official exams
  get them from thousands of test takers. A single-user desktop app has one test taker. Options:
  predicted difficulty from features (frequency rank, level tags, sentence length, grammar
  point), as Duolingo did, then slowly correct with the user's own responses; or opt-in sharing
  of anonymous item responses later. Difficulty prediction is only moderately accurate even for
  well-funded teams ([BEA 2024 shared task entry](https://aclanthology.org/2024.bea-1.43.pdf)).
  So v1 should adapt coarsely (two stages), not item by item.

### Grammar and reading items

Short multiple-choice grammar items (JLPT 文法形式の判断 style) are the most efficient way to
cover grammar; sentence-ordering (★) items are good but easy to generate with more than one
correct order. Reading needs passages, which cost time: a 200-character passage with two
questions takes about two minutes. Reading items have a known weakness: many can be answered
without the passage, or from the question alone
([Kaushik & Lipton 2018](https://aclanthology.org/D18-1546/)), so each item should be checked for
passage-dependence. Lexical coverage research says about 98% known words is needed for adequate
unassisted reading and 95% for minimal comprehension
([Hu & Nation 2000](https://aila2021.dryfta.com/program/abstract-archive/abstract/public/416/unknown-vocabulary-density-reading-comprehension-and-the-98-coverage-figure-replicating-hu-nation-2000)),
though a 2023 replication could not fully reproduce it (same link). Ankura's generated-text
pipeline already targets 95% (ADR 0010).

### Turning results into a level with honest uncertainty

- Put all item difficulties on one latent scale (Rasch or 2PL), anchored to level boundaries
  (N5...N1, or CEFR A1...C1). Estimate ability with a Bayesian posterior (EAP), starting from a
  weak prior (self-assessment or Anki data), and report the posterior over levels: "Most likely
  N3 (60%); N4 25%, N2 15%."
- Report the interval, not a point: "between N4 and N3". When item difficulties are only
  predicted, widen the interval and label the result provisional.
- Report sub-scores (vocabulary, kanji, grammar, reading) with the JLPT's own A/B/C bands
  (≥67%, 34-66%, <34%) for the level tested; it is familiar and coarse enough to be honest.
- Never say "you would pass N2". Say "estimated level, JLPT-style; not an official score". The
  JLPT's IRT scaling and sectional minimums can't be reproduced.
- Self-assessment is cheap but noisy: two meta-analyses found wide variation in how accurately
  learners judge their own level ([review](https://www.cambridge.org/core/product/786D4E0E0D500DCDD43CCE24ED610592/core-reader),
  citing Ross 1998 and Li & Zhang 2021). Use it as a prior or a starting point, not as evidence.
- **Anki data** is strong but biased evidence: words with mature cards are known for
  recognition, but the collection is not a random sample of the language. Use it to (a) skip
  testing words the user clearly knows, (b) give a lower bound on vocabulary size per frequency
  band, and (c) cross-check yes/no answers (claimed-unknown words with mature cards suggest
  under-claiming). Don't count Anki words as random-sample hits when estimating vocabulary size.

## 5. Generated questions

Ankura has an LLM (ADR 0006, 0010), JMdict, SudachiDict and a coverage check. It does not yet
have a frequency list; JMdict priority tags are a weak substitute. Candidate Japanese lists: BCCWJ
frequency lists (NINJAL), Matsushita's VDRJ (built from BCCWJ, with rankings for learners), and
the 日本語教育語彙表 (about 8,000 headwords with six teacher-rated difficulty levels and old-JLPT
grades, per a [Tsukuba report](https://tsukuba.repo.nii.ac.jp/record/37403/files/23242026seika.pdf)).
Licences for all three need checking. BCCWJ uses UniDic short units; Sudachi's split modes don't
match one to one, so the lookup needs a mapping layer.

**Generate what can be built from data, not from the LLM.** Kanji reading items (漢字読み/表記),
yes/no vocabulary and pseudowords can be made deterministically from JMdict and a frequency list:
distractor readings from the same kanji's other readings and from voicing/length changes (きょう
vs きょお vs きよう), each checked against JMdict so no distractor is itself a valid reading.
Pseudowords: real kanji in non-existent compounds, and katakana strings, checked against JMdict
and the corpus. Use the LLM where language judgement is needed: example sentences, 文脈規定 and
grammar items, reading passages and questions.

**Known pitfalls** (from the literature and from what LLMs do with Japanese):

- **Ambiguous keys and plausible-but-correct distractors.** LLM-written MCQs often have more than
  one defensible answer or implausible distractors
  ([docimological study](https://link.springer.com/article/10.1007/s42979-024-02963-6)); LLMs
  write valid distractors more easily than plausible ones
  ([NAACL 2024](https://aclanthology.org/2024.findings-naacl.193.pdf)). In Japanese this hits
  文脈規定, 言い換え類義 (near-synonyms), particles and ★ ordering hardest.
- **Answer leakage.** Correct option longer or more specific than the rest; the stem's kanji or
  okurigana revealing the reading; the key reused from the passage verbatim; questions answerable
  without the passage. Shuffle options in code, not in the prompt (LLMs have position bias).
- **Level drift.** Asked for N4, the model writes N2 vocabulary or grammar. Ankura already
  re-tokenizes generated text and checks coverage (ADR 0010); the same check works for level:
  every content word in stem and options must be at or below the target level, except the
  tested word.
- **Unnatural Japanese.** Translated-sounding sentences and wrong collocations, which a learner
  can't detect and which teach errors.
- **Copying.** Models have seen JLPT prep books and the official workbooks and may reproduce
  items nearly verbatim. Ankura can't check items against those books, so prompts should forbid
  copying and items should be built around Ankura's own word choices.

**How published work validates generated items.** Duolingo's generated reading items were checked
by expert review plus psychometric analysis of pilot responses on its practice test
([Attali et al. 2022](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9354894/)). Difficulty can be
moved systematically by changing distractor properties, and the resulting items fit IRT models
([AIED 2025](https://link.springer.com/chapter/10.1007/978-3-031-99261-2_14)). LLMs as simulated
test takers give item statistics that correlate only weakly with human data
([BEA 2025](https://aclanthology.org/2025.bea-1.21/)) or less accurately than human data
([Maeda 2025](https://journals.sagepub.com/doi/10.1177/00131644241281053)). LLM judges have
been used to rate the JLPT level of generated sentences ([arXiv 2506.03580](https://arxiv.org/pdf/2506.03580)),
which is a usable filter but not ground truth.

**A practical pipeline for Ankura:**

1. Generate from a structured spec (type, target level, target word or grammar point, which
   words are allowed), with JSON output.
2. Check in code: tokenize; every word in JMdict; level/coverage check; option lengths within a
   ratio; no option appears in the stem; readings checked against JMdict.
3. Check with a second, independent LLM call: solve the item cold, several times; keep it only if
   the key is chosen every time and each distractor is judged wrong with a reason. Also solve
   reading items *without* the passage; drop any it gets right.
4. Store items in a local bank with their predicted difficulty. Record every response; after
   enough attempts, flag items that behave oddly (high-ability users miss them, low-ability
   users get them) and retire them.
5. The owner (a long-time learner) reviews a sample per type before release, and can report a bad
   item from the results screen.

## 6. Listening (bonus)

**How exams test it.** JLPT 聴解 plays each recording once; some question types have printed
options and pictures, 即時応答 and some others are audio only (types listed in §1). DELE plays
recordings twice *(from background knowledge)*. CILS, CELI, PLIDA and SIELE all have separate
listening tests with multiple-choice, matching and short-completion tasks.

**What an app can do with TTS.** 即時応答 (short line → reply), 課題理解 and ポイント理解 with
short two-person dialogues are feasible: the LLM writes the script, TTS voices it with two
voices, the question is text. Main risks for Japanese: wrong kanji readings and wrong pitch
accent, which learners can't detect; homographs like 橋/箸/端 differ only in accent
([Fish Audio docs](https://docs.fish.audio/developer-guide/core-features/fine-grained-control/japanese)).
Feed the TTS the reading (kana) Ankura already has from Sudachi, not raw kanji. Automatic
accent-error scoring is a research topic ([PASQA](https://arxiv.org/pdf/2606.20137)).

| Option | Languages | Runs | Licence | Naturalness |
|---|---|---|---|---|
| macOS system voices (AVSpeechSynthesizer; Kyoko, Otoya; Alice, Federica, Luca; Mónica, Jorge, Paulina) | ja, it, es | offline | OS | Good with downloaded "Enhanced/Premium" voices; Siri voices not available to apps *(unverified)* |
| Windows OneCore/SAPI voices (Haruka, Ayumi, Ichiro; Elsa, Cosimo; Helena, Laura, Pablo) | ja, it, es | offline | OS | Dated, robotic. Windows 11 Narrator "natural voices" are not open to third-party apps; the only route is an unofficial adapter that can break ([NaturalVoiceSAPIAdapter](https://github.com/gexgd0419/NaturalVoiceSAPIAdapter)) |
| Kokoro-82M | ja (5 voices), it (2), es (3) and others | offline, CPU | Apache-2.0 | Good for English; Japanese and Italian/Spanish weaker; espeak-ng fallback has gaps in kanji, so pass kana ([summary](https://huggingface.co/simonfxr/kokoro.cpp-GGUF)) |
| Piper (now [OHF-Voice/piper1-gpl](https://github.com/OHF-Voice/piper1-gpl)) | it, es; no Japanese voice found | offline, fast | engine GPL-3.0 (old repo MIT, archived Oct 2025); voices licensed per model card | Decent; GPL engine would have to run as a separate process for MIT Ankura |
| VOICEVOX | ja only | offline | engine LGPL-3.0; each voice has its own terms, most allow commercial use with a credit line ([summary](https://crystal-method.com/blog/voicevox-commercial/)) | Natural, editable accent; anime-style character voices |
| Open JTalk | ja only | offline, tiny | Modified BSD | Robotic but predictable; good accent control |
| Cloud: Azure, Google, OpenAI, ElevenLabs | all three | online, paid per character | commercial | Best naturalness; cost and an extra API key |

For a test (as opposed to reading practice), predictability matters more than charm: the macOS
voices, or a cloud voice with the reading supplied, are the safest. Listening stays a later
feature.

## Recommendations

**v1: a Japanese "level check" (15-20 minutes, two-stage adaptive).**

1. **Start (1 min):** the user picks a rough level or answers four short can-do statements;
   Ankura adds what it knows from Anki (mature-card count per frequency band). This sets the
   prior and the first stage's difficulty.
2. **Vocabulary, yes/no (4 min, ~60 items):** 40 real words sampled across frequency bands
   (more items in the bands near the prior), 20 pseudowords. Real words the user has mature Anki
   cards for are excluded, so the test measures what Anki can't. Scored hits minus false alarms
   per band; output a vocabulary-size estimate with a range. Generated from the frequency list
   and JMdict, no LLM.
3. **Kanji reading (3 min, 12 items):** 漢字読み format, generated from JMdict with checked
   distractors.
4. **Grammar and vocabulary in context (6 min, 20 items):** 文法形式の判断 and 文脈規定 items.
   Stage 1 is 8 items at the prior level; stage 2 is 12 items one level easier, the same, or one
   harder depending on stage 1. LLM-generated, through the pipeline in §5.
5. **Reading (5 min, 3 short passages × 2 questions):** at the level indicated so far,
   passage-dependence checked.
6. **Result:** most likely JLPT-style level with the probability of the neighbouring levels;
   sub-scores with JLPT-style A/B/C bands; vocabulary size range; the JF's CEFR reference for that
   level, labelled reading/listening only; and a link into Ankura (texts and words to study at
   that level). Marked provisional until items have response data. Suggest a retest after a
   month, with different items.

Not computer-adaptive item by item in v1, because there is no calibrated bank. The two-stage
design still uses most of the information. Store every response so that a later version can
calibrate items and switch to a real CAT.

**v1.5: JLPT-style practice tests per level.** Fixed forms that follow the 大問 layout and
timing for one level (without listening), report the JLPT's score-section layout and a "likely
pass / borderline / unlikely" verdict with sectional minimums applied. Own items only.

**Later:**

- Italian (CILS layout, CEFR scale) and Spanish (DELE per level, SIELE-style scale for the
  general test). The general engine is the same; item generation needs per-language level
  inventories (Plan Curricular for Spanish; an Italian equivalent still to be chosen) and
  frequency lists.
- Listening with TTS (即時応答 and short dialogues first).
- C-tests (Japanese deletion rule needs work), a real CAT, opt-in shared item statistics.
- Speaking and writing: out of scope.

**Open questions for the owner:** which frequency list (licence decides); whether to email the
LexJP/LexITA authors about using their items as a calibration benchmark; whether to support
opt-in anonymous response sharing to calibrate items.

**Where sources disagree or are thin:** the CILS overall-total rules; CELI grade bands; DELE block
grouping (third parties disagree, the official guide is authoritative); SIELE CEFR cut-scores;
the exact per-question-type JLPT item counts; and how many items an adaptive test needs for CEFR
placement, for which no language-specific study was found.
