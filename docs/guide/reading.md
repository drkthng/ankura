# Reading

[← README](../../README.md)

## Adding a text

In **Library**, paste any Japanese text (an article, a chapter, lyrics), give it a title if you
like, and press **Start reading**. Texts that Ankura writes for you ([Write for me](write-for-me.md))
land in the Library too.

## What the colours mean

The bar at the top shows the share of words you know, split by Anki state:

| In the text | Meaning |
|---|---|
| plain | You know it: Anki says it's known (interval or FSRS stability of at least 21 days, adjustable in Settings), you marked it known, or it's a very common word |
| dotted underline | Known, but fading: Anki estimates you're likely to forget it soon |
| yellow | Learning in Anki |
| blue | New in Anki (a card you haven't studied yet) |
| red | Not in Anki at all |

Particles, names and numbers aren't counted. A compound like 一時的 counts as known when you know
all its parts. Turn colours off with the **Colours** switch.

**Furigana** can be shown on words you don't know (the default), on all words, or not at all.
**Vertical** switches to vertical writing.

## The word pop-up

Click a word to see its reading, meanings from your dictionaries, what Anki knows about it, and
how often you've seen and looked it up. Then:

- **I don't know this:** remembers that you struggled with it. Such words show up as suggestions
  in [Write for me](write-for-me.md) and on the [Cards](cards.md) screen.
- **I know this:** marks it known everywhere, whatever Anki says. **Ignore** is for names and
  foreign words you never want highlighted. **Clear mark** undoes either.
- **Anki card:** puts the word on your card list, with this sentence or a new one. See
  [Cards](cards.md).

## Wrong furigana?

Ankura works out readings with a dictionary of how words are usually read, and corrects the
sound changes of numbers with counters (三匹 さんびき, 一本 いっぽん, 六分 ろっぷん, 二日 ふつか). Some words
are read differently depending on context (人気のない ひとけ, 十分 じっぷん when it means ten minutes,
この方 かた or ほう), and Ankura can get those wrong.

Click the word and the **✎** next to its reading. Pick one of the readings the dictionary lists,
or type one, and choose whether it applies **in this text** or **everywhere**. For a word in its
dictionary form the correction also changes which word Ankura takes it for (人気 read ひとけ is
"sign of life", not "popularity"), so the meaning and your Anki state follow. **Reset** goes back
to Ankura's reading.

## Phrases

Drag across several words to look them up as one phrase. Ankura reduces it to its dictionary form
(気になります → 気になる) and shows the dictionary entry for the whole phrase. Phrases work like
words: mark them, or make a card. Once a phrase is in Anki or marked known, the reader shows it
as one word wherever it appears, also conjugated (気になって).

## Help with a sentence

**Simplify this sentence** in the pop-up asks the AI to rewrite the sentence with words you know.
Ankura checks the result and asks once more if it still has too many unknown words. You see the
simpler version with furigana on the words you don't know yet, and the English translation only
when you click **Show translation**. This needs the AI ([AI setup](ai.md)).

## What Ankura counts

A paragraph counts as read after about three seconds on screen. Ankura then records that you saw
its words and how many characters you read. Those numbers feed the [Stats](stats.md) and the
word suggestions. They stay on your computer.
