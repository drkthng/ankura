"""Spike S3: score tokenizer output against data/gold-ja.tsv.

Usage: python3 eval.py out/<name>.jsonl ...   (file name before .jsonl picks the field mapping)
Prints a summary table and writes out/failures-<name>.txt.
"""
import json
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).parent


def kata_to_hira(s):
    return "".join(chr(ord(c) - 0x60) if "ァ" <= c <= "ヶ" else c for c in s)


def is_kana(c):
    return "ぁ" <= c <= "ゖ" or "ァ" <= c <= "ヺ" or c == "ー"


def trailing_kana(s):
    i = len(s)
    while i > 0 and is_kana(s[i - 1]):
        i -= 1
    return s[:i], s[i:]


def unidic_surface_reading(t):
    """UniDic 2.1.2 via Lindera has no kana surface reading: 'reading' is the lemma's reading
    (lForm) and the surface form is only given phonetically (with ー). Rebuild the surface
    reading from the lemma reading: reading(kanji stem) + the surface's own trailing kana."""
    surface = t["surface"]
    if all(is_kana(c) for c in surface):
        return kata_to_hira(surface)
    # The phonetic form is exact kana unless it contains a long-vowel mark.
    pron = t.get("phonological_surface_form", "")
    if pron and pron != "*" and "ー" not in pron:
        return kata_to_hira(pron)
    lemma_reading = kata_to_hira(t.get("reading", ""))
    base = t.get("orthographic_base_form", "")
    if not lemma_reading or lemma_reading == "*":
        return kata_to_hira(pron)
    if surface == base:
        return lemma_reading
    s_stem, s_tail = trailing_kana(surface)
    b_stem, b_tail = trailing_kana(base)
    b_tail_h = kata_to_hira(b_tail)
    if s_stem and s_stem == b_stem and lemma_reading.endswith(b_tail_h):
        stem_reading = lemma_reading[: len(lemma_reading) - len(b_tail_h)]
        return stem_reading + kata_to_hira(s_tail)
    if all(is_kana(c) for c in surface):
        return kata_to_hira(surface)
    return kata_to_hira(t.get("phonological_surface_form", ""))


# name prefix -> (lemma candidates, surface reading, variant key)
MAPPINGS = {
    "ipadic": (
        lambda t: [t.get("base_form")],
        lambda t: kata_to_hira(t.get("reading", "")),
        lambda t: t.get("base_form"),
    ),
    "unidic": (
        lambda t: [t.get("orthographic_base_form"), t.get("lexeme")],
        unidic_surface_reading,
        lambda t: t.get("lexeme"),
    ),
    "sudachi": (
        lambda t: [t.get("dictionary_form"), t.get("normalized_form")],
        lambda t: kata_to_hira(t.get("reading_form", "")),
        lambda t: t.get("normalized_form"),
    ),
}


def load_gold():
    items, cat = [], None
    for line in (HERE / "data" / "gold-ja.tsv").read_text(encoding="utf-8").splitlines():
        if line.startswith("#cat "):
            cat = line[5:].strip()
            continue
        if not line.strip() or line.startswith("#"):
            continue
        text, checks = line.split("\t")
        parsed = []
        for c in checks.split("; "):
            surface, rhs = c.split("=", 1)
            if rhs.startswith("@"):
                parsed.append((surface, None, None, rhs[1:]))
                continue
            lemmas, readings = rhs.split("/")
            parsed.append((surface, None if lemmas == "*" else lemmas.split("|"), readings.split("|"), None))
        items.append((cat, text, parsed))
    return items


def evaluate(path):
    name = Path(path).stem
    lemma_of, reading_of, key_of = MAPPINGS[name.split("-")[0]]
    outputs = [json.loads(l) for l in Path(path).read_text(encoding="utf-8").splitlines()]
    gold = load_gold()
    assert len(outputs) == len(gold), (path, len(outputs), len(gold))

    stats = defaultdict(lambda: defaultdict(int))
    groups = defaultdict(list)
    failures = []
    for (cat, text, checks), out in zip(gold, outputs):
        assert out["text"] == text, (out["text"], text)
        toks = out["tokens"]
        for surface, lemmas, readings, group in checks:
            exact = [t for t in toks if t["surface"] == surface]
            for bucket in (cat, "ALL"):
                stats[bucket]["checks"] += 1
            if not exact:
                merged = any(surface in t["surface"] for t in toks)
                kind = "merged" if merged else "split"
                for bucket in (cat, "ALL"):
                    stats[bucket][kind] += 1
                segs = " | ".join(t["surface"] for t in toks)
                failures.append(f"[{cat}] {kind}: {surface} in {text} -> {segs}")
                if group:
                    groups[group].append((surface, f"<{kind}>"))
                continue
            t = exact[0]
            for bucket in (cat, "ALL"):
                stats[bucket]["segmented"] += 1
            if group:
                groups[group].append((surface, key_of(t)))
                continue
            if lemmas is not None:
                ok = any(c in lemmas for c in lemma_of(t) if c)
                for bucket in (cat, "ALL"):
                    stats[bucket]["lemma_n"] += 1
                    stats[bucket]["lemma_ok"] += ok
                if not ok:
                    failures.append(f"[{cat}] lemma: {surface} in {text} -> {lemma_of(t)} (want {lemmas})")
            r = reading_of(t)
            ok = r in readings
            for bucket in (cat, "ALL"):
                stats[bucket]["reading_n"] += 1
                stats[bucket]["reading_ok"] += ok
            if not ok:
                failures.append(f"[{cat}] reading: {surface} in {text} -> {r} (want {readings})")

    unified = 0
    for g, members in groups.items():
        keys = {k for _, k in members}
        ok = len(keys) == 1 and not next(iter(keys)).startswith("<")
        unified += ok
        if not ok:
            failures.append(f"[variants] group {g} not unified: {members}")
    stats["ALL"]["groups"] = len(groups)
    stats["ALL"]["groups_ok"] = unified
    (HERE / "out" / f"failures-{name}.txt").write_text("\n".join(failures) + "\n", encoding="utf-8")
    return name, stats


def pct(a, b):
    return f"{100 * a / b:5.1f}%" if b else "   - "


def main():
    results = [evaluate(p) for p in sys.argv[1:]]
    print("| Tokenizer | Segmented as expected | Lemma correct | Surface reading correct | Variant groups unified |")
    print("|---|---|---|---|---|")
    for name, s in results:
        a = s["ALL"]
        print(
            f"| {name} | {a['segmented']}/{a['checks']} ({pct(a['segmented'], a['checks']).strip()}) "
            f"| {a['lemma_ok']}/{a['lemma_n']} ({pct(a['lemma_ok'], a['lemma_n']).strip()}) "
            f"| {a['reading_ok']}/{a['reading_n']} ({pct(a['reading_ok'], a['reading_n']).strip()}) "
            f"| {a['groups_ok']}/{a['groups']} |"
        )
    cats = [c for c in results[0][1] if c != "ALL"]
    print()
    print("Per category, lemma / reading correct where the token was segmented as expected:")
    print()
    print("| Category | " + " | ".join(n for n, _ in results) + " |")
    print("|---|" + "---|" * len(results))
    for c in cats:
        cells = []
        for _, s in results:
            x = s[c]
            cells.append(f"{pct(x['lemma_ok'], x['lemma_n']).strip()} / {pct(x['reading_ok'], x['reading_n']).strip()} (seg {x['segmented']}/{x['checks']})")
        print(f"| {c} | " + " | ".join(cells) + " |")


if __name__ == "__main__":
    main()
