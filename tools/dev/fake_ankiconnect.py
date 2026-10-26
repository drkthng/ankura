"""A stand-in for AnkiConnect with a small sample collection (development only).

    python3 tools/dev/fake_ankiconnect.py [port]      # default 8765

Answers the actions Ankura uses: version, modelNamesAndIds, modelFieldNames, findNotes, findCards,
notesInfo, getIntervals. Searches support what Ankura sends: mid:N, OR, parentheses, is:new,
is:learn, is:suspended, is:buried, prop:s>=X, prop:r>=X, and deck:NAME (prefix with *).
"""
import json
import re
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

VOCAB = ("日本語コア", 1001, ["Vocab", "Reading", "Meaning", "Sentence"])
KANJI = ("Kanji (RTK)", 1002, ["Kanji", "Keyword"])
# Sentence cards: the word is marked in the sentence, the reading field starts with the word's reading.
SENTENCES = ("Satzkarten", 1003, ["Original sentence", "Reading", "Definition"])
SENTENCE_NOTES = [
    ("<b>長篠</b>の戦いは日本の歴史で有名です。", "ながしの<br>ながしのがたたかいはにっぽんのれきしでゆうめいです。", "Nagashino (place name)", 150.0),
    ("今日はなんだか<b>気分</b>がいい。", "きぶん<br>きょうはなんだかきぶんがいい。", "mood", 120.0),
    ("彼は急いで駅へ<i>向かった</i>。", "むかった<br>かれはいそいでえきへむかった。", "to head towards", 90.0),
    ("<u>少し</u>だけ待ってください。", "すこし<br>すこしだけまってください。", "a little", 200.0),
]

# (vocab field, reading field, meaning, state) where state is new / learn / s:<stability>:<recall> /
# ivl:<days> / suspended. Formatting varies on purpose, as in an old collection.
WORDS = [
    ("猫", "ねこ", "cat", "s:240:0.97"),
    ("<b>犬</b>", "いぬ", "dog", "s:180:0.95"),
    ("駅", "えき", "station", "s:120:0.93"),
    ("電車", "でんしゃ", "train", "s:90:0.92"),
    ("乗る", "のる", "to ride", "s:60:0.9"),
    ("降りる", "おりる", "to get off", "s:12:0.88"),
    ("朝", "あさ", "morning", "s:300:0.98"),
    ("毎日", "まいにち", "every day", "s:200:0.96"),
    ("会社", "かいしゃ", "company", "s:150:0.95"),
    ("行く", "いく", "to go", "s:400:0.99"),
    ("来る", "くる", "to come", "s:380:0.99"),
    ("見る", "みる", "to see", "s:350:0.98"),
    ("食べる", "たべる", "to eat", "s:320:0.98"),
    ("飲む", "のむ", "to drink", "s:30:0.72"),
    ("小さい", "ちいさい", "small", "s:100:0.94"),
    ("大きい", "おおきい", "big", "s:110:0.95"),
    ("白い", "しろい", "white", "s:9:0.85"),
    ("黒い", "くろい", "black", "learn"),
    ("窓", "まど", "window", "s:45:0.9"),
    ("外", "そと", "outside", "s:70:0.93"),
    ("雨", "あめ", "rain", "s:160:0.96"),
    ("降る", "ふる", "to fall (rain)", "s:25:0.6"),
    ("傘", "かさ", "umbrella", "new"),
    ("忘れる", "わすれる", "to forget", "s:40:0.88"),
    ("気づく", "きづく", "to notice", "learn"),
    ("静か", "しずか", "quiet", "s:80:0.94"),
    ("座る", "すわる", "to sit", "s:55:0.91"),
    ("隣", "となり", "next to", "s:14:0.86"),
    ("顔", "かお", "face", "s:130:0.95"),
    ("笑う", "わらう", "to laugh", "s:65:0.92"),
    ("友達", "ともだち", "friend", "s:210:0.97"),
    ("話す", "はなす", "to speak", "s:190:0.96"),
    ("分かる、解る", "わかる", "to understand", "s:260:0.97"),
    ("今日", "きょう", "today", "s:330:0.98"),
    ("天気", "てんき", "weather", "s:140:0.95"),
    ("時間", "じかん", "time", "s:170:0.96"),
    (" 帰[かえ]る", "", "to go home", "s:85:0.93"),
    ("家", "いえ", "house", "s:220:0.97"),
    ("待つ", "まつ", "to wait", "s:75:0.9"),
    ("急ぐ", "いそぐ", "to hurry", "new"),
    ("遅れる", "おくれる", "to be late", "ivl:30"),
    ("道", "みち", "road", "s:95:0.94"),
    ("花", "はな", "flower", "suspended"),
    ("人気[ひとけ]", "", "sign of life", "new"),
]


def build():
    notes, nid, cid = [], 1_600_000_000_000, 1_700_000_000_000
    for vocab, reading, meaning, state in WORDS:
        nid += 1
        cid += 1
        card = {"id": cid, "deck": "日本語::コア", "flags": set(), "stability": None, "recall": None, "interval": 0}
        if state == "new":
            card["flags"].add("new")
        elif state == "learn":
            card["flags"].add("learn")
            card["interval"] = -600
        elif state == "suspended":
            card["flags"].add("suspended")
            card["interval"] = 50
            card["stability"], card["recall"] = 60.0, 0.9
        elif state.startswith("ivl:"):
            card["interval"] = int(state[4:])
        else:
            _, s, r = state.split(":")
            card["stability"], card["recall"] = float(s), float(r)
            card["interval"] = int(float(s) * 0.9)
        notes.append({"id": nid, "model": VOCAB, "fields": [vocab, reading, meaning, ""], "cards": [card]})
    for sentence, reading, meaning, s in SENTENCE_NOTES:
        nid += 1
        cid += 1
        card = {"id": cid, "deck": "日本語::文", "flags": set(), "stability": s, "recall": 0.95, "interval": int(s)}
        notes.append({"id": nid, "model": SENTENCES, "fields": [sentence, reading, meaning], "cards": [card]})
    for kanji, keyword, s in [("猫", "cat", 200.0), ("駅", "station", 90.0)]:
        nid += 1
        cid += 1
        card = {"id": cid, "deck": "Kanji", "flags": set(), "stability": s, "recall": 0.95, "interval": int(s)}
        notes.append({"id": nid, "model": KANJI, "fields": [kanji, keyword], "cards": [card]})
    return notes


NOTES = build()
# Notes added through addNote (served at GET /added for tests).
ADDED = []


def tokens(q):
    return re.findall(r"\(|\)|[^\s()]+", q)


def parse(ts):
    """Tiny recursive-descent parser: terms joined by implicit AND, OR groups, parentheses."""

    def expr(i):
        node, i = conj(i)
        while i < len(ts) and ts[i] == "OR":
            rhs, i = conj(i + 1)
            node = ("or", node, rhs)
        return node, i

    def conj(i):
        node = None
        while i < len(ts) and ts[i] not in (")", "OR"):
            if ts[i] == "(":
                sub, i = expr(i + 1)
                i += 1  # ')'
            else:
                sub, i = ("term", ts[i]), i + 1
            node = sub if node is None else ("and", node, sub)
        return node or ("true",), i

    return expr(0)[0]


def match(node, note, card):
    kind = node[0]
    if kind == "true":
        return True
    if kind == "and":
        return match(node[1], note, card) and match(node[2], note, card)
    if kind == "or":
        return match(node[1], note, card) or match(node[2], note, card)
    t = node[1]
    neg = t.startswith("-")
    t = t.lstrip("-")
    if t.startswith("mid:"):
        ok = note["model"][1] == int(t[4:])
    elif t.startswith("is:"):
        flag = t[3:]
        ok = flag in card["flags"] if flag != "review" else not ({"new", "learn"} & card["flags"])
    elif t.startswith("prop:s>="):
        ok = card["stability"] is not None and card["stability"] >= float(t[8:])
    elif t.startswith("prop:r>="):
        ok = card["recall"] is not None and card["recall"] >= float(t[8:])
    elif t.startswith("prop:due<="):
        ok = "review" in card["flags"] or not ({"new", "learn"} & card["flags"])
        ok = ok and -card.get("overdue", 0) <= int(t[10:])
    elif t.startswith("deck:"):
        pat = t[5:].strip('"')
        ok = card["deck"].startswith(pat[:-1]) if pat.endswith("*") else card["deck"] == pat
    else:
        raise ValueError(f"unsupported search term {t!r}")
    return ok != neg


def act(action, p):
    if action == "version":
        return 6
    if action == "modelNamesAndIds":
        return {m[0]: m[1] for m in (VOCAB, KANJI, SENTENCES)}
    if action == "modelFieldNames":
        return next(m[2] for m in (VOCAB, KANJI, SENTENCES) if m[0] == p["modelName"])
    if action in ("findNotes", "findCards"):
        tree = parse(tokens(p["query"]))
        hits = [(n, c) for n in NOTES for c in n["cards"] if match(tree, n, c)]
        if action == "findNotes":
            return sorted({n["id"] for n, _ in hits})
        return [c["id"] for _, c in hits]
    if action == "notesInfo":
        want = set(p["notes"])
        return [
            {
                "noteId": n["id"],
                "modelName": n["model"][0],
                "tags": [],
                "fields": {f: {"value": v, "order": i} for i, (f, v) in enumerate(zip(n["model"][2], n["fields"]))},
                "cards": [c["id"] for c in n["cards"]],
            }
            for n in NOTES
            if n["id"] in want
        ]
    if action == "deckNames":
        return sorted({c["deck"] for n in NOTES for c in n["cards"]} | {"Default"})
    if action == "canAddNotesWithErrorDetail":
        out = []
        for note in p["notes"]:
            first = next(iter(note["fields"].values()), "")
            dup = any(n["model"][0] == note["modelName"] and n["fields"][0] == first for n in NOTES + ADDED)
            out.append({"canAdd": False, "error": "cannot create note because it is a duplicate"} if dup else {"canAdd": True})
        return out
    if action == "addNote":
        note = p["note"]
        model = next(m for m in (VOCAB, KANJI, SENTENCES) if m[0] == note["modelName"])
        nid = 1_800_000_000_000 + len(ADDED)
        ADDED.append({"id": nid, "model": model, "fields": [note["fields"].get(f, "") for f in model[2]],
                      "deck": note["deckName"], "tags": note["tags"], "cards": []})
        return nid
    if action == "getIntervals":
        by_id = {c["id"]: c for n in NOTES for c in n["cards"]}
        return [by_id[i]["interval"] for i in p["cards"]]
    raise ValueError(f"unsupported action {action}")


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        data = json.dumps(ADDED, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")  # no pooled connections the server has closed
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        try:
            reply = {"result": act(body["action"], body.get("params", {})), "error": None}
        except Exception as e:  # report like AnkiConnect does
            reply = {"result": None, "error": str(e)}
        data = json.dumps(reply, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")  # no pooled connections the server has closed
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
    print(f"fake AnkiConnect on http://127.0.0.1:{port} ({len(NOTES)} notes)")
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
