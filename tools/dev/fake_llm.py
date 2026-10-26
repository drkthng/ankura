"""A stand-in for the Anthropic Messages API, for testing "Write for me" without an API key (dev only).

    python3 tools/dev/fake_llm.py [port]          # default 8766
    ANKURA_LLM_URL=http://127.0.0.1:8766 ANKURA_API_KEY=test npm run tauri dev

Reads the target words from the prompt and answers with a short text built from them and words the
fake AnkiConnect collection knows, in the JSON shape Ankura asks for.
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


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        prompt = body["messages"][-1]["content"]
        reply = {
            "id": "msg_fake",
            "type": "message",
            "role": "assistant",
            "model": body.get("model"),
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": json.dumps(draft(prompt), ensure_ascii=False)}],
            "usage": {"input_tokens": 0, "output_tokens": 0},
        }
        data = json.dumps(reply, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8766
    print(f"fake LLM on http://127.0.0.1:{port}")
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
