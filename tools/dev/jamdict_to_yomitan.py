"""Build a Yomitan-format JMdict zip from the jamdict-data SQLite file (development only).

The app imports Yomitan dictionary zips, and on a normal network it downloads JMdict_english.zip
from yomidevs/jmdict-yomitan. Development environments that can't reach GitHub release downloads
can build an equivalent (older, 2020 JMdict) zip from the PyPI package `jamdict-data` instead:

    pip download --no-deps --no-binary :all: jamdict-data==1.5
    tar xzf jamdict_data-1.5.tar.gz && xz -dk jamdict_data-1.5/jamdict_data/jamdict.db.xz
    python3 tools/dev/jamdict_to_yomitan.py jamdict_data-1.5/jamdict_data/jamdict.db JMdict_dev.zip

JMdict is © EDRDG, CC BY-SA 4.0 (https://www.edrdg.org/edrdg/licence.html). The output stays out
of the repository.
"""
import json
import sqlite3
import sys
import zipfile
from collections import defaultdict

PRIORITY = {"news1", "ichi1", "spec1", "spec2", "gai1"}
CHUNK = 10000


def main(db_path, out_path):
    c = sqlite3.connect(db_path)
    kanji = defaultdict(list)  # idseq -> [(id, text)]
    for kid, idseq, text in c.execute("select ID, idseq, text from Kanji order by ID"):
        kanji[idseq].append((kid, text))
    kana = defaultdict(list)  # idseq -> [(id, text, nokanji)]
    for rid, idseq, text, nokanji in c.execute("select ID, idseq, text, nokanji from Kana order by ID"):
        kana[idseq].append((rid, text, nokanji))
    restr = defaultdict(set)  # kana id -> allowed kanji texts
    for rid, text in c.execute("select kid, text from KNR"):
        restr[rid].add(text)
    common_k = {kid for kid, t in c.execute("select kid, text from KJP") if t in PRIORITY}
    common_r = {rid for rid, t in c.execute("select kid, text from KNP") if t in PRIORITY}
    senses = defaultdict(list)  # idseq -> [sid]
    for sid, idseq in c.execute("select ID, idseq from Sense order by ID"):
        senses[idseq].append(sid)
    gloss = defaultdict(list)
    for sid, text in c.execute("select sid, text from SenseGloss where lang = 'eng'"):
        gloss[sid].append(text)
    pos = defaultdict(list)
    for sid, text in c.execute("select sid, text from pos"):
        pos[sid].append(text)

    rows = []
    for (idseq,) in c.execute("select idseq from Entry order by idseq"):
        glossary, tags = [], []
        for sid in senses[idseq]:
            if gloss[sid]:
                glossary.append("; ".join(gloss[sid]))
                tags.extend(p for p in pos[sid] if p not in tags)
        if not glossary or not kana[idseq]:
            continue  # proper names (JMnedict part) and empty entries
        pairs = []
        for rid, rtext, nokanji in kana[idseq]:
            if kanji[idseq] and not nokanji:
                for kid, ktext in kanji[idseq]:
                    if restr[rid] and ktext not in restr[rid]:
                        continue
                    pairs.append((ktext, rtext, kid in common_k or rid in common_r))
            else:
                pairs.append((rtext, "", rid in common_r))
        for expression, reading, common in pairs:
            score = 100 if common else 0
            term_tags = "P" if common else ""
            rows.append([expression, reading, "", "", score, glossary, idseq, term_tags])

    with zipfile.ZipFile(out_path, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr(
            "index.json",
            json.dumps(
                {
                    "title": "JMdict (dev build from jamdict-data 1.5)",
                    "revision": "jamdict-data-1.5",
                    "format": 3,
                    "sequenced": True,
                    "attribution": "JMdict © EDRDG, CC BY-SA 4.0",
                },
                ensure_ascii=False,
            ),
        )
        for i in range(0, len(rows), CHUNK):
            z.writestr(f"term_bank_{i // CHUNK + 1}.json", json.dumps(rows[i : i + CHUNK], ensure_ascii=False))
    print(f"{len(rows)} terms written to {out_path}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
