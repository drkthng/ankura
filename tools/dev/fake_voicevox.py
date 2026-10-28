"""A stand-in VOICEVOX engine for development and the e2e test: one speaker, silent audio.

Answers GET /speakers, POST /audio_query and POST /synthesis like VOICEVOX's HTTP API, and
logs the texts it was asked to speak (GET /spoken).
"""

import io
import json
import sys
import wave
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

SPOKEN = []


def silence(seconds=0.4, rate=24000):
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(b"\0\0" * int(seconds * rate))
    return buf.getvalue()


class Handler(BaseHTTPRequestHandler):
    def reply(self, data, ctype):
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path.startswith("/speakers"):
            speakers = [{"name": "四国めたん", "speaker_uuid": "fake", "styles": [{"name": "ノーマル", "id": 2}, {"name": "あまあま", "id": 0}]}]
            self.reply(json.dumps(speakers, ensure_ascii=False).encode(), "application/json")
        elif self.path.startswith("/spoken"):
            self.reply(json.dumps(SPOKEN, ensure_ascii=False).encode(), "application/json")
        else:
            self.send_error(404)

    def do_POST(self):
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else b""
        url = urlparse(self.path)
        if url.path == "/audio_query":
            SPOKEN.append(parse_qs(url.query)["text"][0])
            self.reply(json.dumps({"accent_phrases": [], "speedScale": 1.0}).encode(), "application/json")
        elif url.path == "/synthesis":
            json.loads(body or b"{}")
            self.reply(silence(), "audio/wav")
        else:
            self.send_error(404)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 50021
    print(f"fake VOICEVOX on http://127.0.0.1:{port}")
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
