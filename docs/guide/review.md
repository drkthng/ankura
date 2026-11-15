# Review

[← README](../../README.md)

**Review** asks you for the words and kanji you're about to forget, in short rounds of up to ten.
It keeps its own schedule, like Anki does, and tells Anki when you recalled a word, so the card
leaves your backlog there too.

## What comes up

- **Words fading in Anki:** cards that are overdue, the best-established first (they are the
  easiest to save).
- **Words from your reading that have no card:** ones you marked "I don't know this", missed in
  a text's questions, or keep looking up although Anki says you know them.
- **Words and kanji you answered here before**, when Ankura's own schedule says they are due.
  A word you got right comes back after days, then weeks, then months. A word you missed comes
  back within a day.

Words you marked as known or ignored don't come up. **Due now** on the Review screen is the
number waiting. When nothing is due, **Practise anyway** fills a round with what comes next.

## The four rounds

### Fill the gap

A sentence with one word missing, and the word's meaning under it. The sentence is one you read:
from a text where you met the word, or from your own Anki card. If Ankura has no sentence for a
word, it shows only the meaning.

Type the missing word and press **Enter**. All of these count:

- the word as it stands in the sentence (忘れた) or its dictionary form (忘れる),
- in kanji or in kana,
- or in **romaji** (`wasureta`), so you don't need a Japanese keyboard. Write ん before a vowel
  as `n'` (`kan'i`), long vowels as they are spelled in kana (`gakkou`).

**Click any other word of the sentence** to see its reading and meaning, in the same pop-up as
in the reader. That doesn't count as help: the answer still counts as recalled. It is recorded
as a look-up of the word you clicked. Translating the sentence and making a card with it are
offered in the pop-up once you have answered, because the full sentence holds the answer.

If you're stuck: **Hint** shows the first kana and the length, **Show choices** gives four words
to pick from, **I don't know** shows the answer.

### Listen and fill

The same, but the sentence is read aloud and the meaning stays hidden until you answer. Uses the
voice from Settings → Listening. Only words that have a sentence come up.

### Speed match

Five words on the left, their meanings on the right, a clock running. Pick a word, then its
meaning. A wrong pairing counts that word as missed.

### Kanji round

Your fading kanji, both ways: the kanji with four keywords to pick from, or the keyword with four
kanji. Your story is shown after each answer. Needs kanji notes set up
([Kanji](kanji.md)). Keys **1-4** pick an answer.

## What an answer does

| Your answer | In Ankura | In Anki |
|---|---|---|
| **Recalled:** typed in Fill the gap, no hint | The word moves far out on Ankura's schedule | Its card is answered **Good** |
| **Right, with help:** after a hint, from the choices, in Listen and fill, Speed match or the Kanji round | Comes back within three days, to be recalled without help | Nothing |
| **Missed** | Comes back within a day | Nothing |

## What goes to Anki

When you recall a word in Fill the gap without help, Ankura answers that word's Anki card with
**Good**, the same as pressing Good in Anki. Anki's own scheduler then sets the card's next date.

- Only cards that are **due** in Anki. A card you already reviewed there is left alone.
- Only review cards that test that one word. Suspended and buried cards are never answered.
- Wrong answers change **nothing** in Anki. The card stays due and Anki asks you itself.
- Anki has to be running. If it isn't, the answers wait and are sent when the next round ends or
  when you sync, if that is within a day. After a day they are dropped: the recall was then.
- Every answer is listed under **Anki** on the Review screen, with what happened to it.
- To keep Review out of Anki entirely, untick the box under **Anki** on the Review screen.

If Ankura says your AnkiConnect add-on can't answer cards, update it in Anki
(Tools → Add-ons → Check for Updates).

## Points, goal and streak

- **Points:** 10 for a recalled word, 5 for a right answer with help. Each right answer in a row
  adds 2 more, up to 10 extra. A miss ends the run.
- **Daily goal:** a number of right answers per day (20 unless you change it on the Review
  screen). **Today** shows it as a warm-up before the day's text.
- **Review streak:** days in a row with the goal met.
- **Rescued in Anki:** Anki cards answered from Ankura this week.

## Words that aren't in Anki yet

After a round, words without a card are listed with **+ card**. That puts them on your
[card list](cards.md), with the sentence from the round, to check and add to Anki.

## Good to know

- A round makes no AI requests.
- Reading a word in a text is not a review, here or in Anki. Only answers in Review count.
- Ankura's schedule and Anki's are separate. A word you recalled here but whose card wasn't
  answered in Anki (not due, or sending is off) still comes up in Anki as usual.
