"""End-to-end smoke test of the desktop app through WebDriver (development only, Linux).

Needs: a debug build with the frontend embedded (`npx tauri build --debug --no-bundle`), tauri-driver
(`cargo install tauri-driver`), WebKitWebDriver (`apt install webkit2gtk-driver`), Xvfb, the Sudachi
core system.dic and a Yomitan JMdict zip (see jamdict_to_yomitan.py).

    python3 tools/dev/e2e_smoke.py --sudachi /path/system.dic --dict /path/JMdict.zip --out shots/

Starts the fake AnkiConnect, Xvfb and tauri-driver, uses a throwaway data directory, walks through
setup → Anki mapping → sync → reading → word popover, checks results, and saves screenshots.
"""
import argparse
import base64
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
# Talk to localhost directly, whatever proxy the environment configures.
OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))
REPO = HERE.parent.parent

STORY = """駅の朝

毎朝、私は七時の電車に乗って会社へ行く。駅はいつも人が多くて、静かだとは言えない。
今日は雨が降っていた。家を出るとき、傘を忘れたことに気づいたが、時間がなかったので、そのまま急いで駅へ向かった。
電車の中で、隣に座った小さい女の子が、窓の外を見て笑っていた。彼女の手には白い猫のぬいぐるみがあった。
「雨の日も、悪くないね」と、女の子はお母さんに話していた。
私は少しだけ遅れて会社に着いたが、なぜか一日中、気分がよかった。"""


class Driver:
    def __init__(self, url, app):
        self.url = url
        caps = {"capabilities": {"alwaysMatch": {"browserName": "wry", "tauri:options": {"application": str(app)}}}}
        # tauri-driver opens its port before its WebKitWebDriver backend is ready; retry briefly.
        for attempt in range(20):
            try:
                self.sid = self.req("POST", "/session", caps)["sessionId"]
                break
            except (urllib.error.URLError, ConnectionError):
                if attempt == 19:
                    raise
                time.sleep(0.5)

    def req(self, method, path, body=None):
        data = json.dumps(body).encode() if body is not None else None
        r = urllib.request.Request(self.url + path, data=data, method=method, headers={"Content-Type": "application/json"})
        with OPENER.open(r, timeout=120) as resp:
            return json.loads(resp.read())["value"]

    def s(self, method, path, body=None):
        return self.req(method, f"/session/{self.sid}{path}", body)

    def js(self, script, *args):
        return self.s("POST", "/execute/sync", {"script": script, "args": list(args)})

    def js_async(self, script, *args):
        return self.s("POST", "/execute/async", {"script": script, "args": list(args)})

    def find(self, xpath, timeout=30):
        end = time.time() + timeout
        while True:
            try:
                el = self.s("POST", "/element", {"using": "xpath", "value": xpath})
                return list(el.values())[0]
            except urllib.error.HTTPError:
                if time.time() > end:
                    raise AssertionError(f"element not found: {xpath}")
                time.sleep(0.3)

    def click(self, xpath, timeout=30):
        self.s("POST", f"/element/{self.find(xpath, timeout)}/click", {})

    def type(self, xpath, text):
        self.s("POST", f"/element/{self.find(xpath)}/value", {"text": text})

    def text(self, xpath, timeout=30):
        return self.s("GET", f"/element/{self.find(xpath, timeout)}/text")

    def shot(self, path):
        Path(path).write_bytes(base64.b64decode(self.s("GET", "/screenshot")))
        print("screenshot", path)

    def invoke(self, cmd, args=None):
        res = self.js_async(
            "const [cmd, args, done] = arguments;"
            "window.__TAURI_INTERNALS__.invoke(cmd, args).then(v => done({ok: v}), e => done({err: String(e)}));",
            cmd,
            args or {},
        )
        if "err" in res:
            raise AssertionError(f"{cmd}: {res['err']}")
        return res["ok"]

    def quit(self):
        try:
            self.req("DELETE", f"/session/{self.sid}")
        except Exception:
            pass


def wait_port(port, timeout=20):
    import socket

    end = time.time() + timeout
    while time.time() < end:
        try:
            socket.create_connection(("127.0.0.1", port), 0.5).close()
            return
        except OSError:
            time.sleep(0.2)
    raise RuntimeError(f"port {port} never opened")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--sudachi", required=True)
    ap.add_argument("--dict", required=True)
    ap.add_argument("--out", default="e2e-shots")
    ap.add_argument("--app", default=str(REPO / "target/debug/ankura"))
    a = ap.parse_args()
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    data = tempfile.mkdtemp(prefix="ankura-e2e-")
    env = dict(
        os.environ,
        DISPLAY=":99",
        XDG_DATA_HOME=data,
        ANKURA_SUDACHI_DIC=a.sudachi,
        LANG="ja_JP.UTF-8",
        ANKURA_LLM_URL="http://127.0.0.1:8766",
        ANKURA_API_KEY="test",
    )
    procs = [
        subprocess.Popen(["Xvfb", ":99", "-screen", "0", "1280x860x24"], stderr=subprocess.DEVNULL),
        subprocess.Popen([sys.executable, str(HERE / "fake_ankiconnect.py"), "8765"], stdout=subprocess.DEVNULL),
        subprocess.Popen([sys.executable, str(HERE / "fake_llm.py"), "8766"], stdout=subprocess.DEVNULL),
    ]
    time.sleep(1)
    procs.append(subprocess.Popen(["tauri-driver", "--port", "4444"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
    wait_port(8765)
    wait_port(8766)
    wait_port(4444)
    d = None
    try:
        d = Driver("http://127.0.0.1:4444", a.app)
        d.find("//nav//button[normalize-space()='Library']")
        d.shot(out / "01-library-setup.png")

        info = d.invoke("import_dictionary", {"path": a.dict})
        assert info["terms"] > 100_000, info
        status = d.invoke("app_status")
        assert status["tokenizer_ready"], status

        # Anki: connect (automatic), suggest fields, preview, save, sync.
        d.click("//nav//button[normalize-space()='Anki']")
        d.find("//span[contains(., 'Connected')]")
        d.click("//div[contains(@class,'nt')][.//strong[text()='日本語コア']]//button[normalize-space()='Suggest fields']")
        preview = d.text("//div[contains(@class,'nt')][.//strong[text()='日本語コア']]//div[contains(@class,'preview')]/p")
        print("preview:", preview)
        d.shot(out / "02-anki-mapping.png")
        d.click("//div[contains(@class,'nt')][.//strong[text()='日本語コア']]//button[normalize-space()='Save']")
        d.find("//div[contains(@class,'nt')][.//strong[text()='日本語コア']]//span[normalize-space()='mapped']")
        # Sentence cards: Suggest fields should find the marked word and the reading field.
        nt = "//div[contains(@class,'nt')][.//strong[text()='Satzkarten']]"
        d.click(nt + "//button[normalize-space()='Suggest fields']")
        reason = d.text(nt + "//p[contains(@class,'reason')]")
        print("suggestion:", reason)
        assert "bold" in reason and "Reading" in reason, reason
        sentence_preview = d.text(nt + "//div[contains(@class,'preview')]")
        print("sentence preview:", sentence_preview.replace("\n", " | ")[:300])
        assert "長篠" in sentence_preview and "向かう" in sentence_preview, sentence_preview
        d.js("document.evaluate(arguments[0], document, null, 9, null).singleNodeValue.scrollIntoView({block: 'start'})", nt)
        time.sleep(0.3)
        d.shot(out / "02b-sentence-mapping.png")
        d.click(nt + "//button[normalize-space()='Save']")
        d.find(nt + "//span[normalize-space()='mapped']")
        d.click("//button[normalize-space()='Sync now']")
        report = d.text("//p[contains(., 'Read ') and contains(., 'cards')]", timeout=60)
        per_type = d.text("//table[contains(@class,'per-type')]")
        print("per note type:", per_type.replace("\n", " | "))
        assert "Satzkarten" in per_type, per_type
        print("sync:", report)
        assert "48 cards" in report and "FSRS is on" in report, report
        d.shot(out / "03-anki-synced.png")

        # Library: paste and read.
        d.click("//nav//button[normalize-space()='Library']")
        d.type("//input[@placeholder='Title (optional)']", "駅の朝")
        d.type("//textarea", STORY)
        d.click("//button[normalize-space()='Start reading']")
        headline = d.text("//div[contains(@class,'headline')]", timeout=60)
        print("coverage:", headline)
        time.sleep(3.5)  # let exposures register
        d.shot(out / "04-reader.png")

        known = d.js("return [...document.querySelectorAll('.tok.st-known')].map(e => e.textContent.trim())")
        unknown = d.js("return [...document.querySelectorAll('.tok.st-unknown')].map(e => e.textContent.trim())")
        fading = d.js("return [...document.querySelectorAll('.tok.st-fading')].map(e => e.textContent.trim())")
        print("known:", known[:12], "\nunknown:", unknown[:12], "\nfading:", fading)
        assert any("猫" in k for k in known), known
        # From the sentence cards: marked words count as known.
        assert any("気分" in k for k in known) and any("向か" in k for k in known), known
        assert any("降" in f for f in fading), fading  # 降る has recall 0.6

        # Popover on an unknown word, then "I don't know this".
        d.click("//span[contains(@class,'tok') and contains(@class,'st-unknown') and contains(., 'ぬいぐるみ')]")
        d.find("//div[@role='dialog']//ol[contains(@class,'senses')]")
        d.shot(out / "05-popover.png")
        d.click("//div[@role='dialog']//button[normalize-space()='I don’t know this' or normalize-space()=\"I don't know this\"]")

        # Mark another word known and check the colour and the score change.
        before = d.text("//div[contains(@class,'headline')]/strong")
        d.click("//span[contains(@class,'tok') and contains(@class,'st-unknown') and contains(., '毎朝')]")
        d.click("//div[@role='dialog']//button[normalize-space()='I know this']")
        time.sleep(0.5)
        after = d.text("//div[contains(@class,'headline')]/strong")
        print("coverage before/after marking 毎朝:", before, after)
        assert after != before
        d.shot(out / "06-after-mark.png")

        # Furigana everywhere, then vertical text.
        d.js("const s = document.querySelector('select'); s.value = 'all'; s.dispatchEvent(new Event('change'));")
        time.sleep(0.4)
        d.shot(out / "07-furigana-all.png")
        d.click("//label[contains(., 'Vertical')]/input")
        time.sleep(0.4)
        d.shot(out / "08-vertical.png")

        time.sleep(5.5)  # one flush interval
        texts = d.invoke("list_texts")
        assert len(texts) == 1, texts
        opened = d.invoke("open_text", {"id": texts[0]["id"]})
        tok = next(t for p in opened["doc"]["paragraphs"] for t in p if t["surface"] == "ぬいぐるみ")
        stats = d.invoke("word_info", {"key": tok["key"], "surface": tok["dictionary_form"]})
        print("ぬいぐるみ stats:", stats["stats"])
        assert stats["stats"]["lookups"] >= 2, stats["stats"]  # lookup + dont_know
        assert stats["stats"]["exposures"] >= 1, stats["stats"]
        # Write for me: pick the looked-up word, write, and read the result.
        d.click("//nav//button[normalize-space()='Write for me']")
        d.click("//button[contains(@class,'chip') and contains(., 'ぬいぐるみ')]")
        d.type("//input[contains(@class,'typed')]", "傘")
        d.click("//button[contains(@class,'chip') and normalize-space()='a train journey']")
        d.shot(out / "09-write.png")
        d.click("//button[normalize-space()='Write it']")
        written = d.text("//p[contains(@class,'written')]", timeout=90)
        print("generated:", written)
        assert "ぬいぐるみ" in written and "傘" in written, written
        examples = d.text("//section[contains(@class,'examples')]")
        assert "We talked about" in examples, examples
        time.sleep(0.5)
        d.shot(out / "10-generated.png")
        assert len(d.invoke("list_texts")) == 2

        # Copy & paste through claude.ai: a weak reply gets a rewrite request, a good one opens.
        sys.path.insert(0, str(HERE))
        from fake_llm import draft

        d.click("//nav//button[normalize-space()='Write for me']")
        d.click("//button[@role='radio' and contains(., 'Copy')]")
        d.type("//input[contains(@class,'typed')]", "傘")
        d.click("//button[normalize-space()='Get the request']")
        d.find("//summary[contains(., 'Show the request')]")
        prompt = d.js("return document.querySelector('.chat details textarea').value")
        assert "Target words: 傘" in prompt and '"paragraphs"' in prompt, prompt[:300]
        paste = (
            "const [v] = arguments; const t = document.querySelector('textarea.answer');"
            "t.value = v; t.dispatchEvent(new Event('input', {bubbles: true}));"
        )
        weak = {"title": "経済", "paragraphs": ["経済学者が量子力学と傘の相関を分析した。"], "examples": []}
        d.js(paste, "Sure! Here it is:\n```json\n" + json.dumps(weak, ensure_ascii=False) + "\n```")
        d.click("//button[normalize-space()='Check it']")
        verdict = d.text("//p[contains(@class,'verdict')]")
        print("chat verdict:", verdict)
        assert "95%" in verdict, verdict
        d.find("//button[contains(., 'Copy the rewrite request')]")
        d.shot(out / "11-chat-rewrite.png")
        d.js(paste, "```json\n" + json.dumps(draft(prompt), ensure_ascii=False) + "\n```")
        d.click("//button[normalize-space()='Check it']")
        if d.js("return !!document.querySelector('p.verdict') && !document.querySelector('p.written')"):
            print("good reply still under 95%:", d.text("//p[contains(@class,'verdict')]"))
            d.click("//button[contains(., 'Use this version anyway')]")
        written = d.text("//p[contains(@class,'written')]", timeout=30)
        assert "傘" in written, written
        time.sleep(0.5)
        d.shot(out / "12-chat-saved.png")
        assert len(d.invoke("list_texts")) == 3
        print("OK")
    except Exception:
        if d:
            d.shot(out / "failure.png")
            print("page text at failure:\n", d.js("return document.querySelector('main').innerText")[-1500:])
        raise
    finally:
        if d:
            d.quit()
        for p in reversed(procs):
            p.terminate()


if __name__ == "__main__":
    main()
