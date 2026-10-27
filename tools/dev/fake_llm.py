"""A stand-in for the Anthropic Messages API, for testing AI features without an API key (dev only).

    python3 tools/dev/fake_llm.py [port]          # default 8766
    ANKURA_LLM_URL=http://127.0.0.1:8766 ANKURA_API_KEY=test npm run tauri dev

Answers by the JSON schema Ankura asks for: a short text built from the target words and words the
fake AnkiConnect collection knows, a card sentence, or a card's fields.
"""
import json
import re
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def draft(prompt):
    m = re.search(r"Target words: (.+)", prompt)
    targets = [t for t in re.split(r"[、,]", m.group(1)) if t] if m else ["猫"]
    paragraphs = ["今日は朝から雨が降っていた。", "駅で友達と会った。" + "".join(f"{t}の話をした。" for t in targets), "家に帰って、猫と静かな時間を過ごした。"]
    return {
        "title": "雨の日",
        "paragraphs": paragraphs,
        "examples": [{"word": t, "sentence": f"{t}の話をした。", "translation": f"We talked about {t}."} for t in targets],
    }


def card_sentence(prompt):
    m = re.search(r"Target word: (\S+)", prompt)
    return {"sentence": f"今日、{m.group(1) if m else '猫'}を見た。"}


def card_fields(prompt, props):
    order = re.search(r"^Fields, in order: (.+)$", prompt, re.M)
    props = order.group(1).split(", ") if order else props
    fact = lambda name: (re.search(rf"^{name}: (.+)$", prompt, re.M) or [None, ""])[1]
    word, sentence = fact("Word"), fact("Sentence")
    written = fact("The word as written in the sentence") or word
    meaning = (re.search(r"^\s*1\. (.+)$", prompt, re.M) or [None, ""])[1]
    reading = fact("Reading of the word") + "<br>" + fact(re.escape("Sentence reading (from a tokenizer)"))
    out = {}
    for i, f in enumerate(props):
        if i == 0:
            out[f] = sentence.replace(written, f"<b>{written}</b>", 1)
        elif "reading" in f.lower():
            out[f] = reading
        else:
            out[f] = meaning
    return out


def practice_items(prompt):
    """Practice-test items (ADR 0015) around the given words; the key is always option 0."""
    n = int((re.search(r"Number of items: (\d+)", prompt) or [None, "4"])[1])
    words = re.findall(r"^\d+\. (\S+?)（", prompt, re.M)
    t = (re.search(r"Type: (\S+?)\.", prompt) or [None, ""])[1]
    items = []
    for i in range(n):
        w = words[i] if i < len(words) else f"文法{i}"
        if t == "漢字読み":
            items.append({"word": w, "sentence": f"きのう、{w}について友だちと話しました。", "underlined": w, "options": [], "answer": 0, "explanation": ""})
        elif t in ("表記", "言い換え類義"):
            items.append({"word": w, "sentence": f"これは{w}の話です。", "underlined": w,
                          "options": [w, "別の言葉", "ちがう言葉", "ほかの言葉"], "answer": 0, "explanation": f"{w} fits; the others don't."})
        elif t == "用法":
            items.append({"word": w, "sentence": w, "underlined": "",
                          "options": [f"毎日{w}を使います。", f"{w}が走りました。", f"{w}に寝ます。", f"{w}を飲みました。"], "answer": 0,
                          "explanation": "Only the first uses the word naturally."})
        else:
            items.append({"word": w, "sentence": f"毎朝、駅（　）歩いて行きます。#{i}", "underlined": "",
                          "options": ["まで", "から", "より", "ほど"], "answer": 0, "explanation": "まで marks where you walk to."})
    return {"practice_items": items}


def practice_passages(prompt):
    k = int((re.search(r"Number of texts: (\d+)", prompt) or [None, "2"])[1])
    q = int((re.search(r"Questions per text: (\d+)", prompt) or [None, "1"])[1])
    grammar = "Type: 文章の文法" in prompt
    out = []
    for t in range(k):
        if grammar:
            text = "私は毎朝、駅まで歩きます。" + "".join(f"［{i}］、電車に乗ります。" for i in range(1, q + 1))
            qs = [{"question": f"［{i}］", "options": ["それから", "しかし", "つまり", "たとえば"], "answer": 0,
                   "explanation": "それから: and then."} for i in range(1, q + 1)]
        else:
            text = f"田中さんは毎朝七時に起きて、八時の電車に乗ります。会社まで三十分かかります。#{t}"
            qs = [{"question": f"田中さんは何時に起きますか。#{i}", "options": ["七時", "八時", "六時", "九時"], "answer": 0,
                   "explanation": "The first sentence says 七時."} for i in range(q)]
        out.append({"text": text, "text2": "田中さんの妹は九時に起きます。", "questions": qs})
    return {"practice_passages": out}


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        prompt = body["messages"][-1]["content"]
        props = list(body["output_config"]["format"]["schema"]["properties"])
        if props == ["sentence"]:
            answer = card_sentence(prompt)
        elif props == ["items"]:
            n = int((re.search(r"Number of items: (\d+)", prompt) or [None, "12"])[1])
            answer = {"items": [
                {"question": f"毎日、駅（　）歩いて行きます。#{i}", "options": ["まで", "から", "より", "ほど"], "answer": 0} for i in range(n)
            ]}
        elif props == ["passages"]:
            answer = {"passages": [
                {"text": f"田中さんは毎朝七時に起きて、八時の電車に乗ります。会社まで三十分かかります。#{i}",
                 "questions": [{"question": "田中さんは何時に起きますか。", "options": ["七時", "八時", "六時", "九時"], "answer": 0},
                               {"question": "会社までどのぐらいかかりますか。", "options": ["三十分", "一時間", "十分", "二時間"], "answer": 0}]}
                for i in range(3)
            ]}
        elif props == ["practice_items"]:
            answer = practice_items(prompt)
        elif props == ["order_items"]:
            n = int((re.search(r"Number of items: (\d+)", prompt) or [None, "4"])[1])
            answer = {"order_items": [{"before": "私は", "fragments": ["駅", "の", "前", "で"], "after": f"友だちを待ちます。#{i}",
                                       "explanation": "の links 駅 and 前; で marks the place."} for i in range(n)]}
        elif props == ["practice_passages"]:
            answer = practice_passages(prompt)
        elif props == ["answers"]:
            # The checker: right on grammar and word items; reading questions only with the
            # passage. For ★ items the key is the fragment 前 (options are shuffled).
            questions = []
            for line in prompt.splitlines():
                if re.match(r"^\d+\. ", line):
                    questions.append([])
                elif questions and re.match(r"^\s+\d\) ", line):
                    questions[-1].append(line.split(") ", 1)[1])
            sees = any(m in prompt for m in ("Passage:", "（", "【", "★", "使い方"))
            answer = {"answers": [
                {"index": i, "choice": opts.index("前") if "前" in opts else (0 if sees else 1), "ambiguous": False}
                for i, opts in enumerate(questions)
            ]}
        elif props == ["topics"]:
            answer = {"topics": ["Railway travel", "Morning routines"]}
        elif props == ["kanji"]:
            # Primitive analysis: every story uses 口, under alternating names (a clash to resolve).
            kanji = re.findall(r"^(\S) \| ", prompt, re.M)
            answer = {"kanji": [
                {"kanji": k, "primitives": [{"component": "口", "name": ["mouth", "opening"][i % 2], "rtk_name": "mouth"}]}
                for i, k in enumerate(kanji)
            ]}
        elif props == ["stories"]:
            blocks = re.findall(r"^Kanji: (\S+) .*?\nStory: (.*)$", prompt, re.M)
            answer = {"stories": [{"kanji": k, "story": story.replace("A ", "A mouth-shaped ", 1)} for k, story in blocks]}
        elif set(props) == {"story", "primitives"}:
            kanji = (re.search(r"^Kanji: (\S+)$", prompt, re.M) or [None, "?"])[1]
            keyword = (re.search(r"^Keyword: (.+)$", prompt, re.M) or [None, "?"])[1]
            answer = {"story": f"A new story: the <b>{keyword}</b> sits on a mat.", "primitives": [{"component": kanji, "name": keyword}]}
        elif set(props) == {"simple", "translation"}:
            answer = {"simple": "今日は雨だった。", "translation": "It was raining today."}
        elif set(props) == {"title", "paragraphs", "examples"}:
            answer = draft(prompt)
        else:
            answer = card_fields(prompt, props)
        reply = {
            "id": "msg_fake",
            "type": "message",
            "role": "assistant",
            "model": body.get("model"),
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": json.dumps(answer, ensure_ascii=False)}],
            "usage": {"input_tokens": 0, "output_tokens": 0},
        }
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
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8766
    print(f"fake LLM on http://127.0.0.1:{port}")
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
