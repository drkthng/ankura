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


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        prompt = body["messages"][-1]["content"]
        props = list(body["output_config"]["format"]["schema"]["properties"])
        if props == ["sentence"]:
            answer = card_sentence(prompt)
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
