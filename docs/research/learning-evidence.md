# Learning research: what makes learning with Ankura fast

[← How to learn with Ankura](../guide/learning.md)

Prepared 2026-10-30. Learner profile assumed: adult, about 20 years of Anki, about 1,600 known Japanese words, RTK for kanji, a 6-month break (large overdue backlog), roughly JLPT N4-N3.

## How to read this document

- **Evidence strength.** **Strong** means several meta-analyses or replicated experiments agree. **Moderate** means a few good studies, or meta-analyses with caveats. **Weak** means single studies, expert opinion, community simulations or extrapolation.
- **How the URLs were checked.** In this session the fetch tool could not reach most academic hosts (sciencedirect, hawaii.edu/nflrc, lancs.ac.uk, docs.ankiweb.net). It could reach GitHub. Every URL below appeared in live web-search results for the cited work, so the pages exist. GitHub pages were also opened and read. Numbers marked **[verify]** come from abstracts, secondary summaries or my memory of the paper, and should be checked against the full text before anyone quotes them in public.
- The research is overwhelmingly about English as a second language. Japanese-specific evidence is flagged wherever it exists.

---

## 1. How much vocabulary coverage you need: 95% vs 98%

### Findings
- **Hu & Nation (2000).** Learners read a 633-word story at 80%, 90%, 95% or 100% known words, with nonwords standing in for unknown words. At 80% nobody comprehended adequately, and at 90% only a small minority did. The authors' regression put **about 98% coverage** as the point where most learners comprehend unassisted fiction adequately. https://nflrc.hawaii.edu/rfl/item/43
- **Laufer & Ravenhorst-Kalovski (2010).** About **95% coverage is a "minimal" threshold**, reached at 4,000-5,000 English word families including proper nouns. **98% is "optimal"**, reached at about 8,000 families **[verify the 8,000 figure]**. Laufer stresses that the threshold is probabilistic, not a cliff. https://scholarspace.manoa.hawaii.edu/items/eaead47e-853e-4797-ba6f-596df150c1d7/full
- **Schmitt, Jiang & Grabe (2011, Modern Language Journal).** Comprehension rises **roughly linearly** with coverage, and there is **no sudden threshold**. Even at 98-100% coverage, mean comprehension was only around 60-70% **[verify]**. The authors argue that 98% is needed if you want more than about 60% comprehension. https://www.lextutor.ca/cover/papers/schmitt_etal_2011.pdf
- **Kremmel, Indrarathne, Kormos & Suzuki (2023, Language Learning)** repeated Hu & Nation with 104 Sri Lankan adults and **could not fully replicate** the original. They urge caution about treating 98% as a hard figure. A Wellington thesis argues that coverage explains less of the variation in comprehension than claimed, and that overall vocabulary size predicts it better. https://www.x-mol.com/paper/1730399947901915136 and https://openaccess.wgtn.ac.nz/articles/thesis/Thresholds_Text_Coverage_Vocabulary_Size_and_Reading_Comprehension_in_Applied_Linguistics/17057906
- **Coverage needed for learning words while reading.** Nation's usual guidance is that 98% is the target if you want to pick up words incidentally, because you need enough context to guess them. Japanese-specific figures for incidental learning are **unknown** (Journal of Extensive Reading, 2023): https://jalt-publications.org/content/index.php/jer/article/view/1221

### Japanese-specific figures
- **Komori, Mikuni & Kondo (2004), 日本語教育.** For Japanese-as-a-second-language readers, about **96% known words** was needed to answer about 80% or more of the comprehension items. **Listening** needed only about **93%** (Mikuni, Komori & Kondo 2005). This is the best Japanese-specific threshold found. It is cited in a Princeton teaching paper and a KAKEN report: https://pjpf.princeton.edu/document/831 and https://kaken.nii.ac.jp/ja/file/KAKENHI-PROJECT-24820012/24820012seika.pdf **[I did not see the primary article; verify]**
- **Matsushita (2012 PhD thesis at Victoria University of Wellington, which produced the VDRJ, built from the BCCWJ).** Texts written for native readers need roughly **9,000-12,000 words (often quoted as about 9,500) for 95% coverage** and **about 20,000 words for 98%**. Matsushita (2014) found an S-shaped relationship between vocabulary size and comprehension. The figures are cited secondhand in Matsushita-based papers (e.g. the Aozora corpus study in *System*, 2024: https://www.sciencedirect.com/science/article/pii/S0346251X2400349X) and the thesis record is at https://ir.wgtn.ac.nz/handle/123456789/28233?show=full **[verify against the thesis]**
- **Japanese demands more vocabulary than English** for the same coverage. High-frequency words make up a smaller share of running text, and genres vary more (Sato 2014, LREC, as summarised at https://github.com/tkgally/je-dict-1/blob/main/planning/wiki/research/vocabulary-size-coverage.md).
- **Kanji coverage** (newspaper corpus of over 23 million kanji tokens; Chikamatsu et al., 2000, Behavior Research Methods, https://dx.doi.org/10.3758/bf03200819):
  - top 500 kanji cover about 75-80% of kanji tokens
  - top 1,000 cover about 94-96%
  - top about 1,600 cover about 99%
  - Summary: https://scriptin.github.io/kanji-frequency **[secondary; values differ slightly by corpus]**

### What this means for an Ankura user
- **About 1,600 words is far below the roughly 9,500 needed for 95% of native text.** Authentic Japanese will mostly sit around 80-90% coverage for this learner, which is a frustration-level read. Generated texts, graded readers and NHK Easy-type material are the right diet for now. This is exactly what the generator is for.
- **Treat 95% as the floor, not the target.**
  - For "learning" texts that deliberately carry target words: 95-97% is fine, as long as looking words up is one click.
  - For fluency and enjoyment reading: aim for **98% or more**, so the reader rarely stops.
  - The Japanese figure (96% for good comprehension) sits in between.
  - A sensible default is a setting such as "comfortable 98% / stretch 95%".
- **Coverage is a dial, not a pass/fail mark.** Comprehension falls gradually, so a 93% text is not useless, just harder and slower.
- **Coverage depends on what "known" means.** If Ankura counts a word as known because Anki has it as mature, overdue and fading cards inflate the figure. After the 6-month break, real coverage is probably several points lower than computed until the backlog is cleared. The stealth-leech feature is a direct measure of this gap.

**Evidence strength:** moderate. That coverage matters is strong. The exact thresholds are contested, and the Japanese figures rest on a few studies.

---

## 2. Picking up vocabulary while reading (incidental acquisition)

### Findings
- **Uptake per text is low.**
  - Waring & Takaki (2003): Japanese learners read one graded reader. **About 1 of 25 target words** was expected to be remembered (meaning recall) after 3 months, and **none of the words met fewer than 8 times** survived to 3 months. https://www.wgtn.ac.nz/__data/assets/pdf_file/0008/1626146/Waring-Takaki-2003.pdf
  - Meta-analysis of incidental learning (Webb, Uchihara & Yanagisawa 2023, Language Teaching; 24 studies, N = 2,771): on average **about 9-18% of target words learned on immediate tests and 6-17% on delayed tests**, with similar results for reading, listening and reading-while-listening. https://www.cambridge.org/core/product/E38E3468FD2090B1FA3051051DE8E70C
- **Repetition matters, and around 10 or more encounters is a reasonable working number.**
  - Webb (2007, Applied Linguistics): sizeable gains after **10 encounters** in context, and full knowledge of a word needs more. https://www.bibsonomy.org/bibtex/22bcfaf3885670de4b5c2b0d43e7cb252/sofiagruiz92
  - Pellicer-Sánchez & Schmitt (2010), reading an authentic novel: after **10 or more exposures**, about 84% of meanings and 76% of spellings were recognised **[verify]**. Meaning *recall* was much lower. https://core.ac.uk/download/323109991.pdf
  - Uchihara, Webb & Yanagisawa (2019) meta-analysis: the correlation between encounters and learning is **r = .34 (medium)**, moderated by spacing and visual support, and stronger for adults. https://zendy.io/title/10.1111/lang.12343
  - There is no magic number. Estimates range from about 6 to over 20 encounters, depending on the word and the test.
- **Spacing encounters out helps, though less consistently than in flashcard studies.**
  - Nakata & Elgort (2021): spacing out encounters in context improved explicit knowledge of words but not tacit knowledge. https://openaccess.wgtn.ac.nz/articles/journal_contribution/Effects_of_spacing_on_contextual_vocabulary_learning_Spacing_facilitates_the_acquisition_of_explicit_but_not_tacit_vocabulary_knowledge/16306107
  - Other incidental-learning studies give mixed results.
- **Glosses and look-ups help.**
  - Meta-regression of glossing (Yanagisawa, Webb & Uchihara 2020, Studies in Second Language Acquisition; 42 studies, N = 3,802): **reading with glosses produced significantly more word learning than reading without**. Gloss mode (text, picture or audio) made no significant difference. https://www.cambridge.org/core/product/38124150D59DF3039EE1FF5AE88FE922
  - Hulstijn, Hollander & Greidanus (1996): both glosses and a dictionary beat reading alone. Words that were actually looked up were well retained, but learners **looked up few words** (around 12% of targets **[verify]**). This argues for making look-up effortless.
- **Studying words before reading boosts learning from the text.** Pellicer-Sánchez, Conklin & Vilkaitė-Lozdienė (2021, Language Learning; eye-tracking): learners taught the words before reading gained more, and read those words faster. https://nottingham-repository.worktribe.com/output/4745317
- **Word-focused tasks beat reading alone per word.** Laufer (2003): more words were acquired through word-focused tasks than through reading. https://cris.haifa.ac.il/en/publications/vocabulary-acquisition-in-a-second-language-do-learners-really-ac/
- **Reading volume.**
  - Nation (2014): about **200,000 tokens** of reading gives 12 or more encounters with words at the 2,000-word level. **About 3 million tokens** does the same at the 9,000-word level, which is roughly 2-25 novels' worth. https://scholarspace.manoa.hawaii.edu/handle/10125/66881
  - Webb & Chang (2015): learners who already knew more words gained more from extensive reading (a "rich get richer" effect). https://www.cambridge.org/core/journals/studies-in-second-language-acquisition/article/abs/how-does-prior-word-knowledge-affect-vocabulary-learning-progress-in-an-extensive-reading-program/5BAD8743697C156BE3FD7E556FB42D94
- **Narrow reading (many texts on one topic).** It recycles topic vocabulary and lowers the lexical load, and corpus studies confirm more repetition across related texts. Experimental evidence is modest: reading plus vocabulary exercises beat narrow reading alone in one study. https://scholarspace.manoa.hawaii.edu/handle/10125/66625
- **Repeated reading (reading the same text again).** L1 meta-analyses show fluency gains, e.g. Therrien 2004: fluency d = .75, comprehension d = .73. L2 evidence is thinner and mixed (Lee & Yoon 2017 meta-analysis, as cited in https://www.readingmatrix.com/files/27-h5uwmwzv.pdf). **Weak** for L2.

### What this means for an Ankura user
- **Ankura's biggest advantage over plain extensive reading is control over repetition.** Plan for each target word to appear **about 3 times in a text and 8-12 or more times across several texts spread over days**, rather than once in a single text.
- **Generate small topic series** (3-5 texts on one interest: narrow reading). Topic vocabulary then recycles naturally.
- **Don't rely on reading alone to learn new words.** Uptake is about 10-15% per word without support. The fast path is the combination: look up, add to SRS, then meet the word again in later texts (see section 4).
- **Make look-up instant, and treat every look-up as data.** Repeated look-ups of a "known" word are exactly what stealth leeches detect. Those words need re-study, not more passive exposure.

**Evidence strength:** strong that repetition matters and that uptake is low. Moderate for the 10-encounter heuristic and for spacing within reading.

---

## 3. Extensive reading meta-analyses, graded vs authentic texts, and AI-generated texts

### Findings
- **Nakanishi (2015, TESOL Quarterly):** 34 studies, N = 3,942. Effect sizes were **d = 0.46** for group comparisons and **d = 0.71** for pre-to-post gains. https://files.eric.ed.gov/fulltext/EJ1078389.pdf
- **Jeon & Day (2016):** 49 studies, N = 5,919. Effects were small to medium, about d = .54 for reading comprehension and d = .47 for vocabulary **[verify]**. Effects were larger for adults, in foreign-language (EFL) rather than second-language (ESL) settings, with web-based reading, and when ER was part of the curriculum. https://core.ac.uk/download/323110138.pdf
- **Hamada (2020, Frontiers in Psychology):** after correcting for selection bias, the comprehension effect drops from **d = 0.55 to about 0.37**. A propensity-matched study with Japanese university students found about 0.3-0.4. In other words, extensive reading helps, but less than enthusiasts claim. https://pmc.ncbi.nlm.nih.gov/articles/PMC7188915
- **Liu & Zhang (2018):** extensive reading has a significant effect on vocabulary learning. Graded readers, plus comprehension questions or vocabulary exercises, helped. https://ideas.repec.org/a/ibn/eltjnl/v11y2018i6p1.html
- **Program length.** Jeon & Day found no clear advantage for longer programs, but there were very few long programs to compare. Waring & Takaki and Nation (2014) both imply that **volume, accumulated over months, is what drives vocabulary growth**.
- **Graded vs authentic texts.** Simplified texts let readers stay at high coverage, which aids comprehension and fluency. Authentic novels can teach words to advanced readers (Pellicer-Sánchez & Schmitt 2010). There is no meta-analytic head-to-head comparison, so the key variable is *coverage*, not authenticity as such.
- **Japanese tadoku.** Most studies are small and focus on perceptions: less anxiety about kanji, higher motivation. A 144-learner experiment found audio-assisted extensive reading gave the largest gains, but the differences were not significant. https://jalt-publications.org/content/index.php/jer/article/view/petersonandwarnick
- **AI-generated or adapted texts (2023-2026). The evidence is thin and early.**
  - A randomised trial of AI-generated texts with about 10% target-word density beat natural reading at about 2% density on vocabulary measures. Comprehension was not reported. https://dl.acm.org/doi/10.1145/3756580.3756642
  - Computers & Education: AI (Dec 2025), 90 learners over 4 weeks: an AI-narrative group beat a gamified (Duolingo-style) group and a control group on immediate and delayed vocabulary tests. https://doaj.org/article/bb6987f2f84f4b4b9dddae0711a8c612
  - Storyfier (UIST 2023), a study of AI-generated stories built around target words; learning effects were mixed. https://arxiv.org/pdf/2308.03864
  - Lexical profiling (Nguyen, Huang & Dang): **ChatGPT texts only loosely hit the target level.** A1 and A2 texts needed B1-level vocabulary, so output should be checked. https://www.tesolunion.org/archives-info/345
  - Simplification study: LLM rewrites were shorter and **less cohesive**, and missed the readability target. https://www.sciencedirect.com/science/article/pii/S1560429226000892
  - SRS-Stories (EMNLP Industry 2025), generating stories constrained to a learner's SRS vocabulary: very close to Ankura's design, but no learner-outcome study. https://aclanthology.org/2025.emnlp-industry.44/

### What this means for an Ankura user
- **Reading volume is the lever.** Expect slow, cumulative gains (about d = 0.4 over a semester), not quick wins. Generated texts are only useful if the learner reads *a lot* of them.
- **Ankura's measured coverage is a real advantage over raw LLM output.** LLMs on their own over-shoot difficulty. Checking generated text against the learner's actual known words, and regenerating when it falls short, is what makes AI texts work as graded readers.
- Mix in authentic material as coverage rises, e.g. once Ankura measures 95% or more on a given source.
- Watch for "LLM-ese": unnatural phrasing and less cohesive text. Occasional native texts, such as NHK News Web Easy, graded readers or manga, guard against learning unnatural Japanese. **This is a real but unquantified risk.**

**Evidence strength:** moderate for extensive reading itself (the effects are real but modest). Weak for AI-generated texts (promising, few rigorous studies).

---

## 4. Combining flashcards/SRS with reading, card formats, spacing, and FSRS

### Findings
- **Deliberate study is efficient.** Webb, Yanagisawa & Uchihara (2020, Modern Language Journal; 22 studies):
  - about **60% meaning recall and 59% form recall** on immediate tests
  - about **39% and 25%** on delayed tests
  - flashcards are among the more effective activities
  - Compare about 10-15% for incidental learning (section 2).
  - https://www.x-mol.com/paper/1341861690617110528
- **Spacing.** Kim & Webb (2022, Language Learning; 48 experiments, N = 3,411):
  - spacing has a **medium-to-large** effect
  - **longer spacing wins on delayed tests**, although short spacing looks equally good on immediate tests
  - **expanding and equal spacing were statistically equivalent**
  - https://eld.bl.uk/catalog/100152978253_0x000001 (DOI 10.1111/lang.12479)
- **Spacing matters more than set size.** Nakata & Webb (2016): once spacing is equal, whether you study words in small or large sets barely matters. https://bvbvf1.bib-bvb.de/vufind/PrimoRecord/cdi_proquest_journals_1815527945
- **Expanding intervals.** Nakata (2015) found a limited advantage for expanding schedules. A later review by Nakata found no clear benefit. https://www.cambridge.org/core/journals/studies-in-second-language-acquisition/article/effects-of-expanding-and-equal-spacing-on-second-language-vocabulary-learning/D1D796306985C52F9BE7A1200AC50DB9
- **Studying first and then meeting the word in reading beats either alone.** Pre-teaching boosts learning from text (Pellicer-Sánchez et al. 2021, section 2), and reading plus exercises beats reading alone (Laufer 2003). There is no direct study of "add to SRS, then meet the word in generated texts". The combination is inferred from these findings. **Moderate.**
- **Sentence cards vs word cards. Evidence is thin.**
  - Webb (2007, Language Teaching Research) compared word pairs with word pairs plus one glossed sentence. My recollection is that **one sentence added little** to most kinds of word knowledge **[verify]**. https://cambridge.org/core/journals/studies-in-second-language-acquisition/article/receptive-and-productive-vocabulary-learning-the-effects-of-reading-and-writing-on-word-knowledge/DDF362AE7B13D1949B1CD591DA2F3414 (citation listing)
  - Barcroft (2004, Second Language Research): having learners *write sentences* while first learning words **hurt form learning**, because meaning-heavy tasks compete with learning the word's form (his TOPRA model). https://profiles.wustl.edu/en/publications/effects-of-sentence-writing-in-second-language-lexical-acquisitio/
  - Cloze and fill-in-the-blank exercises work, but in the 2020 meta-analysis they varied widely in effectiveness.
  - **Conclusion:** a sentence on the card is a *cue* and gives useful context. Recalling the target word is what does the work.
- **FSRS desired retention.**
  - The FSRS4Anki tutorial: "80-95% is reasonable, 90% should work fine for most people". The permitted range is 0.70-0.97, extended to 0.99 in newer Anki. Avoid going above 0.97: workload rises steeply and reviews become dreaded. https://github.com/open-spaced-repetition/fsrs4anki/blob/main/docs/tutorial.md
  - The FSRS wiki: plotted against desired retention, **workload is U-shaped**. Too low means relearning many forgotten cards; too high means very frequent reviews. Since Anki 24.04, the built-in optimiser (often called CMRR, "compute minimum recommended retention") minimises **workload per unit of knowledge**. Treat its result as a *lower bound*. https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Optimal-Retention
- **Backlog after a long break.**
  - FSRS accounts for overdue time when it computes intervals. Answering long-overdue cards honestly does not corrupt the model parameters. Rescheduling everything at once dumps a huge number of due cards on you. (Same tutorial link as above.)
  - Community simulations (Expertium and Jarrett, on the Anki forums) found that **"Retrievability descending"** is among the review orders that hold retention near target during a backlog. "Due date ascending" was among the worst. https://forums.ankiweb.net/t/ordering-request-reverse-relative-overdueness/50051 **[weak: simulations and community opinion, not peer-reviewed]**

### What this means for an Ankura user
- **The fastest loop:** meet the word in a generated text → look it up → make a card (word as the target, with the sentence from the text as context) → Ankura deliberately brings the word back in texts over the next days and weeks. Each tool covers what the other lacks: SRS fixes the form-meaning link quickly, and reading adds depth, collocations and recognition speed.
- **Cards made from Ankura sentences** come from text the learner read and cared about, which may help memory (an assumption, not tested). Keep the *recall target* the single word. Put the reading on the back with audio. **Don't make sentence-writing part of card creation.**
- **Retention setting:**
  - keep **0.90** as the default
  - drop to about **0.85** temporarily during backlog recovery
  - raise to 0.92-0.95 only for a deck the learner truly wants near-perfect, such as core vocabulary before an exam
  - stay above the "minimum recommended retention" result
- **Backlog playbook** (expert consensus, weak evidence):
  1. Don't reset the collection.
  2. Pause or strongly limit new cards.
  3. Set a sustainable daily review cap (e.g. 150-300).
  4. Sort reviews by **Retrievability descending**, so cards that can still be saved come first and the clearly forgotten ones wait.
  5. Answer honestly.
  6. Expect weeks, not days.
  - Reading in Ankura during this period helps re-activate fading words. But **a word met while reading is not a retrieval test**, so Ankura should not mark Anki cards as reviewed on the user's behalf.
- **Stealth leeches** (cards Anki calls known but that the learner keeps looking up) suggest a gap between knowing the card and recognising the word in context. Good responses: add a sentence-context card, use a lower retention target for that card, and plant the word in more texts.

**Evidence strength:** strong for spacing and retrieval practice. Moderate for combining SRS with reading. Weak for sentence vs word cards. Expert consensus for FSRS settings and backlog handling.

---

## 5. Reading while listening, TTS, and shadowing

### Findings
- **Reading-while-listening (RWL) and vocabulary.** The Webb et al. (2023) meta-analysis found similar word gains from reading (17% immediate / 15% delayed), listening (15% / 13%) and RWL (13% / 17%). https://www.x-mol.com/paper/1613961140270874624
- **Brown, Waring & Donkaewbua (2008)** compared reading, RWL and listening only (Japanese learners, graded readers). Listening only produced the fewest words. RWL was at least as good as reading, and learners preferred it **[verify direction and sizes]**. https://scholarspace.manoa.hawaii.edu/bitstream/10125/66816/1/20_2_10125_66816_brown.pdf
- **Comprehension.** Clinton-Lisell (2023 meta-analysis; 30 studies, N = 1,945):
  - overall benefit of RWL over reading alone: **trivial, g = .18**
  - **when the audio set the pace: g = .41**
  - when readers set their own pace: g = .06 (no reliable effect)
  - the authors say L2 benefits look plausible but need more studies
  - https://commons.und.edu/ehb-fac/75
- **Listening fluency.**
  - Chang & Millett (2014, ELT Journal): 13 weeks of RWL with graded readers improved listening fluency more than reading alone **[verify sizes]**. https://scite.ai/authors/sonia-millett-wbJAjL (DOI 10.1093/elt/cct052)
  - Chang & Millett (2015, System): audio-assisted reading raised reading rate and comprehension in beginners. https://eld.bl.uk/catalog/100081493716_0x000001
- **TTS quality.**
  - Bione & Cardoso (2020, Language Learning & Technology): TTS and human voices were judged **similarly comprehensible**, but TTS was rated **less natural**. https://www.lltjournal.org/item-detail/944/
  - A Japanese JACET study found natural speech slightly better understood, while synthetic speech was rated nearly as natural. https://tsukuba.repo.nii.ac.jp/record/48586/files/JacetJ_53_1.pdf
  - **No study found on Japanese TTS pitch-accent errors and what learners pick up from them. This is a known engineering issue; the learning effect is unstudied.**
- **Shadowing (repeating audio aloud as you hear it).**
  - Hamada (2016, Language Teaching Research): helps mainly **lower-proficiency listeners**, by improving bottom-up processing (sound perception, word recognition).
  - Hamada (2018, RELC review): https://journals.sagepub.com/doi/10.1177/0033688218771380
  - Typical protocols run 10-15 minutes, 3-4 times a week, for several weeks **[secondary]**.

### What this means for an Ankura user
- Use read-aloud (TTS) **with audio-paced, synced highlighting** (karaoke-style). The comprehension benefit appears only when the audio sets the pace. A good routine: first read at your own pace with look-ups, then do a second RWL pass on the same text at natural speed, which also counts as repeated reading.
- RWL does not teach *more words* than reading, but it builds sound-to-word links and listening fluency. These are a big gap for someone who has learned mostly through Anki and RTK. A third, listening-only pass later supports listening practice.
- Prefer high-quality neural TTS. Show pitch accent where possible, and treat TTS prosody as approximate.
- For N4-N3 listening, add 10-15 minutes of shadowing on short, already-understood Ankura texts a few times a week.

**Evidence strength:** moderate for RWL (small comprehension effect, conditional on pacing; plausible fluency benefits). Weak to moderate for TTS and shadowing.

---

## 6. Kanji: RTK/Heisig, mnemonics, and radicals

### Findings
- **No controlled studies of RTK/Heisig were found.** The evidence is learner reports and critiques. Language Log: it works for memorising a "dictionary" of keywords, but it is not reading ability. https://languagelog.ldc.upenn.edu/nll/?p=70538
- **Imagery and keyword mnemonics** reliably boost **immediate** recall. Their **long-term** advantage is inconsistent, and mnemonic memories can be *more* fragile without review.
  - Wang & Thomas (1995) found the keyword advantage reversed after 2 days, and **more study opportunities helped keyword learners most**. https://archive.memory-key.com/research/Wang95
  - Wang & Thomas (1992), on Chinese characters, found faster forgetting with mnemonics. https://stars.library.ucf.edu/scopus1990/1047
  - Kuwabara (2000), with kanji beginners, found imagery better than rote learning. https://jpf.repo.nii.ac.jp/record/337/files/Sekai13_matsunaga.pdf
- **Rose (2013, Modern Language Journal),** interviews with Japanese learners: mnemonics help when meaningful, but **overly complex stories lose the meaning** of the kanji, and over-reliance can backfire. https://ora.ox.ac.uk/objects/uuid:51a0c552-8389-4d77-8071-e09a37be859f
- **Component and radical awareness** is generally helpful for recognising characters. Benefits depend on proficiency and character type, and too much component detail at once can overload learners. Chen & Feng 2020: https://doaj.org/article/0481ce65bd28440e802d3c961fb5846d and the Perfetti lab: https://www.lrdc.pitt.edu/perfettilab/pubpdfs/ChineseRadicalBasedGrouping.pdf

### What this means for an Ankura user
- Mnemonics combined with spaced review is the right pairing. Review fixes the known weakness of mnemonics, which is forgetting over time.
- **Keep mnemonics short and vivid, built on RTK primitives.** Long stories are a known failure mode.
- RTK keywords are not word knowledge. Prioritise kanji that appear in the learner's known and target words. Showing the RTK keyword when a word is looked up links kanji study to vocabulary.
- The kanji coverage figures (section 1) show why breadth pays off quickly: the top 1,000 kanji cover about 95% of kanji tokens in newspapers.

**Evidence strength:** moderate for mnemonics in general (strong short-term, mixed long-term). Weak for RTK specifically (no trials). Moderate for radical awareness.

---

## 7. The testing effect and practice tests

### Findings
- **Retrieval practice beats restudying.**
  - Rowland (2014, Psychological Bulletin): overall g of about 0.50 **[verify]**. Effects are larger for recall than recognition tests, larger with feedback, and larger at retention intervals of 1 day or more. https://pubmed.ncbi.nlm.nih.gov/25150680/
  - Adesope, Trevisan & Sundararajan (2017, Review of Educational Research; 118 studies): practice tests beat restudy and every other comparison, and feedback strengthens the effect. Overall g is about 0.5-0.6 **[verify]**. https://journals.sagepub.com/doi/10.3102/0034654316689306
- **Diagnostic level tests.** No L2-specific research on how often to take them was found. Measurement reasoning (expert consensus):
  - Proficiency changes slowly. A 6-month break plus a few weeks of study shifts it by less than a short test's measurement error.
  - Re-taking the same items inflates scores through practice effects.
  - Optimising for test items ("teaching to the test", Goodhart's law) narrows learning.

### What this means for an Ankura user
- **Retrieval practice should happen daily**, through SRS and comprehension questions on texts. Questions with immediate feedback are themselves learning, not just measurement.
- **Take level or JLPT-style checks every 4-8 weeks** (or about every 30-50 hours of study). Use them for diagnosis: which skill is lagging, and what level to generate texts at.
  - Don't re-take the same form.
  - Treat small score changes as noise.
- **Weekly mini-tests on the learner's own target words** are useful and quick.
- **Don't let JLPT practice tests replace reading volume.** They sample proficiency; they don't build it.

**Evidence strength:** strong for the testing effect. Weak (reasoning only) for how often to run diagnostics.

---

## 8. Motivation and habit: streaks, goals, interest, gamification

### Findings
- **Gamification.** Sailer & Homner (2020, Educational Psychology Review):
  - cognitive outcomes g of about .49
  - motivational outcomes about .36 **[verify]**
  - behavioural outcomes about .25 **[verify]**
  - the motivational and behavioural effects were less stable in rigorous studies
  - game fiction and social interaction were significant moderators of behavioural outcomes
  - https://opus.bibliothek.uni-augsburg.de/opus4/frontdoor/index/index/docId/109056
- **Streaks.**
  - Silverman & Barasch (2023, Journal of Consumer Research; 7 studies): an intact streak on display **increases** later engagement, and a broken streak on display **decreases** it. The drop is worse when people blame themselves, and **reduced when the streak can be "repaired"**. https://www.colorado.edu/business/faculty-research/2023/04/19/or-track-how-broken-streaks-affect-consumer-decisions
  - Duolingo's own data (correlational, published by the company) say a 7-day streak predicts course completion. https://blog.duolingo.com/how-duolingo-streak-builds-habit
- **Habit formation.** Lally et al. (2010, European Journal of Social Psychology): **median 66 days** to automaticity, ranging from 18 to 254 days. **Missing a single day did not materially disrupt** habit formation. The sample was small and the study has not been replicated. https://www.spring.org.uk/2023/01/form-a-habit.php (summary)
- **Implementation intentions** ("When X happens, I will do Y"). Gollwitzer & Sheeran (2006; 94 tests): **d = .65** for reaching goals. A 2024 update covering 642 tests found d = .27-.66. https://kops.uni-konstanz.de/entities/publication/2e749bfb-8533-437c-8203-7e788c910c5f
- **Rewards and intrinsic motivation.** Deci, Koestner & Ryan (1999; 128 studies): expected, tangible rewards that depend on doing the task undermine intrinsic motivation. Informational feedback does not. The finding is disputed by Cameron and colleagues. https://depts.washington.edu/techdocs/papers/deciExtrinsicRewardsAndIntrinsicMotivation99.pdf
- **Interest and L2 reading.** High-interest, familiar topics produced better comprehension (Eidswick 2010). Topic knowledge is the more stable predictor (Joh 2006). Overall the L2 evidence is thin. https://jalt-publications.org/node/116 and https://nflrc.hawaii.edu/rfl/item/198

### What this means for an Ankura user
- **Generate texts about genuine interests.** This is likely to improve comprehension (through topic knowledge) and to keep the learner reading longer, and long reading is what matters most.
- **Streaks:** keep them, but make them *forgiving*.
  - Offer repairable streaks or freeze days.
  - Use a low minimum, such as "1 text or 10 minutes".
  - Never shame-message after a break.
  - Also show cumulative totals (words read, hours), which never reset.
  - After the 6-month break, the "fresh start" framing helps more than a big zero.
- **Set a concrete plan**, e.g. "After dinner, 15 minutes of Anki, then one Ankura text". This has a medium-to-large effect, and Ankura could prompt for it.
- **Prefer feedback that informs** (coverage rising, words learned, stealth leeches fixed) over points and badges for their own sake.

**Evidence strength:** moderate for gamification (positive but fragile), streak design, and implementation intentions. Weak for interest effects in L2.

---

## 9. How to split daily time

### Findings
- **Nation's "four strands" (2007):** roughly **equal time** for meaning-focused input (reading/listening), meaning-focused output, language-focused learning (SRS, grammar) and fluency development (easy material at speed). This is well-argued expert guidance, not an experimental result. https://core.ac.uk/download/357330216.pdf
- **Efficiency comparison.**
  - Deliberate study gives about 60% immediate recall per word studied, against about 10-15% per word met while reading (Webb et al. 2020 and 2023).
  - Reading, however, builds things flashcards don't: fluency, grammar in use, collocations, and reading speed.
  - Ankura's "stealth leeches" are a sign of what flashcards alone miss.
- **Distributing practice** (Kim & Webb 2022): daily sessions beat occasional long sessions for retention.
- **Volume.** Nation (2014) estimates about 3 million running words of input to meet mid-frequency words often enough. At a learner's reading pace, that means **hundreds of hours** of reading over years.

### Suggested allocation (well-argued, not experimentally tested)
For about **60 minutes a day** at N4-N3 (scale proportionally):

| Activity | Time | Strand |
|---|---|---|
| Anki reviews plus a few new cards (capped; more during backlog clean-up) | 15-20 min | language-focused |
| New Ankura text with target words, intensive read with look-ups and card-making | 15-20 min | input |
| Easy reading or re-reading at about 98% coverage, with audio-paced TTS | 10-15 min | fluency |
| Listening-only or shadowing of a known text, plus, later, some speaking or writing (section 10) | 10 min | fluency / output |

- **During backlog recovery**, let SRS temporarily take up to about 50% of the time. Keep **at least 15 minutes of reading every day** so that input doesn't stop.
- **Keep SRS below about a third of total time long-term.** If reviews regularly take more, reduce new cards rather than cutting reading.
- **Session length.** Several short sessions beat one long one, because of spacing. Stop a reading session when comprehension clearly drops: fatigue lowers how much you take in (this is reasoning, not a measured effect).

**Evidence strength:** weak to moderate. The principles (retrieval practice plus spacing plus volume) are strong; the exact split is expert judgement.

---

## 10. Output and pushed output (brief)

- **Swain's output hypothesis:** producing language makes learners notice gaps, test hypotheses and reflect on language. Empirical support is mixed; some studies (e.g. Izumi) did not find that output increased noticing. https://forrt.org/flora-replication-atlas/doi/10.1017/s0272263102004023/
- Productive knowledge (recalling the Japanese word from its meaning) needs productive retrieval. Receptive SRS and reading mostly build recognition. In the 2020 meta-analysis, form recall decayed faster than meaning recall (25% vs 39% on delayed tests).
- Nation's four strands assign about 25% of time to meaningful output. Writing a few sentences or speaking about a text is a natural extension.
- But Barcroft (2004) shows that sentence-writing is a *poor first-exposure* activity for new words. Output works better *after* a word is partly known.
- **What this means for Ankura:** a future feature could add, after reading,
  - a 2-3 sentence summary of the text, written in Japanese with AI correction
  - a short spoken retelling
  - L1→L2 (production) cards only for words the learner wants to use

  Until then, the learner should add a small amount of output elsewhere, such as a language-exchange partner or a journal.

**Evidence strength:** moderate that output builds productive knowledge. Weak for Swain's specific mechanisms.

---

## Sites that could not be fetched
WebFetch could not resolve sciencedirect.com, nflrc.hawaii.edu, www2.hawaii.edu, eprints.lancs.ac.uk or docs.ankiweb.net in this session. To open those primary texts directly, they need to be allowed in the session's network settings. The search engine confirmed that their URLs exist.
