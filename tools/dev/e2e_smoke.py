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
import http.client
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
        # tauri-driver sometimes reuses a connection to WebKitWebDriver that was reset ("Connection
        # reset by peer" in tauri-driver.log) and drops our request; it never reached the page.
        for attempt in range(3):
            try:
                with OPENER.open(r, timeout=120) as resp:
                    return json.loads(resp.read())["value"]
            except (http.client.RemoteDisconnected, ConnectionResetError):
                if attempt == 2:
                    raise
                print(f"driver dropped {method} {path}; retrying")
                time.sleep(1)

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
    # Audio for the listening test: a PulseAudio server with a null sink, if PulseAudio is there.
    if subprocess.run(["pactl", "info"], capture_output=True).returncode != 0 and subprocess.run(["which", "pulseaudio"], capture_output=True).returncode == 0:
        subprocess.run(["pulseaudio", "-D", "--exit-idle-time=-1", "-n", "--load=module-null-sink sink_name=null", "--load=module-native-protocol-unix"], capture_output=True)
    audio = subprocess.run(["pactl", "info"], capture_output=True).returncode == 0
    procs = [
        subprocess.Popen(["Xvfb", ":99", "-screen", "0", "1280x860x24"], stderr=subprocess.DEVNULL),
        subprocess.Popen([sys.executable, str(HERE / "fake_ankiconnect.py"), "8765"], stdout=subprocess.DEVNULL),
        subprocess.Popen([sys.executable, str(HERE / "fake_llm.py"), "8766"], stdout=subprocess.DEVNULL),
        subprocess.Popen([sys.executable, str(HERE / "fake_voicevox.py"), "50021"], stdout=subprocess.DEVNULL),
    ]
    time.sleep(1)
    # The app's and the driver's output, for when the session drops.
    driver_log = open(out / "tauri-driver.log", "w")
    procs.append(subprocess.Popen(["tauri-driver", "--port", "4444"], env=env, stdout=driver_log, stderr=subprocess.STDOUT))
    wait_port(8765)
    wait_port(8766)
    wait_port(50021)
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
        # Kanji notes (ADR 0013): the note type's hint sets it up, saved straight away with the
        # suggested fields.
        d.click("//div[contains(@class,'nt')][.//strong[text()='Kanji (RTK)']]//button[normalize-space()='Use as kanji notes']")
        km = "//div[contains(@class,'kanji-map')][.//strong[text()='Kanji (RTK)']]"
        d.find(km + "//span[normalize-space()='mapped']")
        picked = d.js("return [...document.querySelectorAll('.kanji-map select')].map(s => s.value)")
        print("kanji fields:", picked)
        assert picked == ["Kanji", "Keyword", "Story"], picked
        assert len(d.invoke("kanji_mappings")) == 1
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
        for _ in range(25):  # the mark is saved, then the colours and the score update
            after = d.text("//div[contains(@class,'headline')]/strong")
            if after != before:
                break
            time.sleep(0.2)
        print("coverage before/after marking 毎朝:", before, after)
        assert after != before
        d.shot(out / "06-after-mark.png")

        # Listening (ADR 0017): VOICEVOX (the fake) reads the text sentence by sentence, the
        # current sentence highlighted; the audio is kept on disk.
        d.click("//button[contains(normalize-space(), 'Listen')]")
        d.find("//*[contains(@class,'speaking')]", timeout=30)
        voice = d.text("//p[starts-with(normalize-space(), 'Voice:')]")
        d.shot(out / "06b-listening.png")
        for _ in range(120):  # silent clips play to the end (or the webview can't play audio here)
            state = d.js("return document.querySelector('.reader-page .error')?.textContent || (document.querySelector('.speaking') ? 'playing' : 'done')")
            if state != "playing":
                break
            time.sleep(0.25)
        spoken = json.loads(OPENER.open("http://127.0.0.1:50021/spoken").read())
        cached = list(Path(data).glob("*/audio/voicevox/2/*.wav"))
        print("listening:", voice, "| end:", state, "| spoken:", len(spoken), spoken[:2], "| cached:", len(cached))
        assert voice == "Voice: VOICEVOX:四国めたん" and spoken and spoken[0] in STORY.replace("\n", "") and cached, (voice, spoken, cached)
        if audio:  # with a sound server the whole text plays, sentence by sentence
            assert state == "done" and len(spoken) >= 6, (state, spoken)
        d.js("[...document.querySelectorAll('button')].find(b => b.title === 'Stop')?.click()")

        # Furigana everywhere, then vertical text.
        d.js("const s = document.querySelector('.controls select'); s.value = 'all'; s.dispatchEvent(new Event('change'));")
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
        # Vertical text: the example sentences sit below it, not over it.
        vertical = "//label[contains(., 'Vertical')]/input"
        if not d.js("return document.querySelector('.text').classList.contains('vertical')"):
            d.click(vertical)
            time.sleep(0.4)
        gap = d.js(
            "return document.querySelector('.examples').getBoundingClientRect().top"
            " - document.querySelector('.text').getBoundingClientRect().bottom"
        )
        d.shot(out / "10b-generated-vertical.png")
        assert gap >= 0, f"examples overlap the vertical text by {-gap}px"
        d.click(vertical)
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
        # Wait for the check: either the reader opens, or a second verdict (try 3) appears.
        for _ in range(60):
            if d.js("return !!document.querySelector('p.written') || document.body.innerText.includes('Try 3')"):
                break
            time.sleep(0.5)
        if not d.js("return !!document.querySelector('p.written')"):
            print("good reply still under 95%:", d.text("//p[contains(@class,'verdict')]"))
            d.click("//button[contains(., 'Use this version anyway')]")
        written = d.text("//p[contains(@class,'written')]", timeout=30)
        assert "傘" in written, written
        time.sleep(0.5)
        d.shot(out / "12-chat-saved.png")
        assert len(d.invoke("list_texts")) == 3

        # Interests: suggested from reading, saved, then offered as ideas and picked for texts.
        d.click("//nav//button[normalize-space()='Write for me']")
        d.click("//button[normalize-space()='Tell Ankura what you like']")
        d.click("//button[normalize-space()='Suggest from my reading']")
        d.click("//button[contains(@class,'chip') and normalize-space()='+ Railway travel']", timeout=60)
        d.click("//button[normalize-space()='Save interests']")
        d.find("//div[contains(@class,'ideas')]//button[normalize-space()='Railway travel']")
        assert d.invoke("pick_topic") == "Railway travel"
        print("interests: ok")

        # Anki cards (ADR 0012): choose where they go, card a word and a selected phrase, add both.
        d.click("//nav//button[normalize-space()='Settings']")
        d.find("//select[@id='cnt']/option[@value='Satzkarten']")
        choose = "const [id, v] = arguments; const s = document.getElementById(id); s.value = v; s.dispatchEvent(new Event('change'));"
        d.js(choose, "cnt", "Satzkarten")
        d.js(choose, "cdk", "日本語::文")
        d.click("//section[.//h2[normalize-space()='Anki cards']]//button[normalize-space()='Save']")
        d.find("//section[.//h2[normalize-space()='Anki cards']]//span[normalize-space()='Saved.']")

        d.click("//nav//button[normalize-space()='Library']")
        d.click("//button[contains(@class,'open')][contains(., '駅の朝')]")
        # Correct a reading in this text (毎朝 まいあさ → まいちょう), then reset it.
        ruby = "return [...document.querySelectorAll('.tok')].find(e => e.textContent.startsWith('毎朝'))?.querySelector('rt')?.textContent"
        d.click("//span[contains(@class,'tok') and starts-with(normalize-space(.), '毎朝')]")
        d.click("//div[@role='dialog']//button[contains(@class,'fix')]")
        d.click("//div[@role='dialog']//section[contains(@class,'fixer')]//button[normalize-space()='まいちょう']")
        d.click("//div[@role='dialog']//button[normalize-space()='Use this reading']")
        for _ in range(20):
            if d.js(ruby) == "まいちょう":
                break
            time.sleep(0.3)
        assert d.js(ruby) == "まいちょう", d.js(ruby)
        d.click("//span[contains(@class,'tok') and starts-with(normalize-space(.), '毎朝')]")
        d.click("//div[@role='dialog']//button[contains(@class,'fix')]")
        d.click("//div[@role='dialog']//button[normalize-space()='Reset']")
        for _ in range(20):
            if d.js(ruby) == "まいあさ":
                break
            time.sleep(0.3)
        assert d.js(ruby) == "まいあさ", d.js(ruby)
        print("reading fix: ok")
        d.click("//span[contains(@class,'tok') and normalize-space(.)='傘' or contains(@class,'tok') and starts-with(normalize-space(.), '傘')]")
        d.click("//div[@role='dialog']//button[normalize-space()='Simplify this sentence']")
        simple = d.text("//div[@role='dialog']//p[contains(@class,'simple')]", timeout=60)
        d.click("//div[@role='dialog']//button[normalize-space()='Show translation']")
        print("simplified:", simple, "/", d.text("//div[@role='dialog']//section[contains(@class,'simplify')]/p[2]"))
        assert "雨" in simple, simple
        d.shot(out / "12b-simplified.png")
        d.click("//div[@role='dialog']//button[normalize-space()='with this sentence']")
        msg = d.text("//div[@role='dialog']//span[contains(@class,'card-msg')]")
        assert "card list" in msg, msg
        d.js("document.querySelector('.backdrop').click()")
        # Select 気分がよかった (skipping furigana) and let go of the mouse.
        d.js(
            "const target = '気分がよかった'; const art = document.querySelector('article.text');"
            "const w = document.createTreeWalker(art, NodeFilter.SHOW_TEXT, {acceptNode: n => n.parentElement.closest('rt') ? 2 : 1});"
            "const nodes = []; let all = ''; for (let n; (n = w.nextNode());) { nodes.push([n, all.length]); all += n.data; }"
            "const at = (i) => { for (let k = nodes.length - 1; k >= 0; k--) if (nodes[k][1] <= i) return [nodes[k][0], i - nodes[k][1]]; };"
            "const i = all.indexOf(target); const r = document.createRange();"
            "r.setStart(...at(i)); r.setEnd(...at(i + target.length));"
            "const sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(r);"
            "window.dispatchEvent(new MouseEvent('mouseup'));"
        )
        d.find("//div[@role='dialog']//ol[contains(@class,'senses')]")
        lemma = d.js("return document.querySelector('[role=dialog] .lemma').textContent")  # visible or not
        print("phrase:", lemma)
        assert lemma.startswith("気分が"), lemma
        d.shot(out / "13-phrase.png")
        d.click("//div[@role='dialog']//button[normalize-space()='with a new sentence']")
        msg = d.text("//div[@role='dialog']//span[contains(@class,'card-msg')]", timeout=60)
        assert "card list" in msg, msg
        # Marking the phrase known makes the reader show it as one word.
        d.click("//div[@role='dialog']//button[normalize-space()='I know this']")
        merged = "//span[contains(@class,'tok') and contains(@class,'st-marked_known') and contains(., '気分') and contains(., 'かった')]"
        print("merged phrase:", d.text(merged))

        # Kanji pop-up: 猫's card, a new mnemonic saved to Anki (story field only), then undone.
        d.click("//span[contains(@class,'tok') and contains(., '猫')]")
        d.click("//div[@role='dialog']//button[contains(@class,'kchip') and normalize-space()='猫']")
        d.find("//div[@role='dialog']//span[contains(@class,'kw') and normalize-space()='cat']")
        time.sleep(0.4)  # the pop-up moves to fit the kanji panel; a click mid-move hits the backdrop
        d.click("//div[@role='dialog']//button[normalize-space()='Show story']")
        d.find("//div[@role='dialog']//p[contains(@class,'story')]")
        story = d.js("return document.querySelector('[role=dialog] p.story').textContent")  # may be scrolled out of view
        assert "crouches" in story, story
        d.click("//div[@role='dialog']//button[normalize-space()='New mnemonic']")
        d.find("//div[@role='dialog']//div[contains(@class,'compare')]", timeout=60)
        d.shot(out / "13b-kanji.png")
        d.click("//div[@role='dialog']//button[normalize-space()='Save to Anki']")
        saved = d.text("//div[@role='dialog']//p[contains(@class,'ok')]", timeout=30)
        updates = json.loads(OPENER.open("http://127.0.0.1:8765/updates").read())
        print("kanji saved:", saved, "| updates:", updates)
        assert [list(u["fields"]) for u in updates if "fields" in u] == [["Story"]], updates
        assert any(u.get("addTags", {}).get("tags") == "ankura-edited" for u in updates), updates
        d.click("//div[@role='dialog']//button[normalize-space()=\"Undo Ankura's change\"]")
        d.find("//div[@role='dialog']//p[contains(@class,'ok') and contains(., 'old story')]")
        updates = json.loads(OPENER.open("http://127.0.0.1:8765/updates").read())
        assert "crouches" in [u for u in updates if "fields" in u][-1]["fields"]["Story"], updates
        d.js("document.querySelector('.backdrop').click()")

        # Consistent primitive names (ADR 0013 point 5): analyse, approve one name, update a story.
        d.click("//nav//button[normalize-space()='Kanji']")
        d.click("//button[starts-with(normalize-space(), 'Analyse 2')]")
        d.find("//div[contains(@class,'prim') and contains(@class,'clash')]//span[normalize-space()='口']", timeout=60)
        names = d.js("return [...document.querySelectorAll('.prim.clash .name')].map(l => [l.textContent.trim(), l.querySelector('input').checked])")
        print("primitive names:", names)
        assert names == [["mouth ×1", True], ["opening ×1", False]], names
        d.click("//button[normalize-space()='Save list']")
        d.click("//button[starts-with(normalize-space(), 'Prepare new versions')]")
        d.find("//div[contains(@class,'rw')]", timeout=60)
        d.shot(out / "13c-primitives.png")
        d.click("//button[normalize-space()='Save 1 to Anki']")
        d.find("//div[contains(@class,'rw')]//p[contains(., 'saved')]", timeout=30)
        updates = json.loads(OPENER.open("http://127.0.0.1:8765/updates").read())
        assert "mouth-shaped" in [u for u in updates if "fields" in u][-1]["fields"]["Story"], updates

        d.click("//nav//button[normalize-space()='Cards']")
        d.find("//section[contains(@class,'draft')][2]//textarea", timeout=60)  # both filled in
        print("field names:", d.invoke("card_field_names"))
        labels = d.js("return [...document.querySelectorAll('section.draft')][0].querySelector('.fields label').textContent")
        assert labels == "Original sentence", labels  # Anki's field order, not alphabetical
        fields = d.js("return [...document.querySelectorAll('section.draft')].map(s => s.querySelector('textarea').value)")
        print("first fields:", fields)
        assert any("<b>傘</b>" in f for f in fields), fields
        d.shot(out / "14-cards.png")
        d.click("//button[normalize-space()='Add 2 to Anki']")
        for _ in range(60):  # until both have left the list
            if d.js("return document.querySelectorAll('section.draft').length === 0"):
                break
            time.sleep(0.5)
        d.find("//h2[normalize-space()='Added to Anki']")
        added = json.loads(OPENER.open("http://127.0.0.1:8765/added").read())
        print("added to Anki:", [(n["fields"][0], n["deck"], n["tags"]) for n in added])
        assert len(added) == 2 and all(n["tags"] == ["ankura"] and n["deck"] == "日本語::文" for n in added), added
        d.shot(out / "15-cards-added.png")

        # Level check (ADR 0014): answer everything, read the result.
        d.click("//nav//button[normalize-space()='Level check']")
        d.click("//button[normalize-space()='Start']")
        seen = set()
        for _ in range(400):
            state = d.js(
                "if (document.querySelector('.headline')) return 'done';"
                "const k = [...document.querySelectorAll('button')].find(b => b.textContent.startsWith('Know it'));"
                "if (k) { k.click(); return 'vocab'; }"
                "const o = document.querySelector('.options button');"
                "if (o) { const s = document.querySelector('.page p.muted.small')?.textContent ?? ''; o.click(); return s.split(' · ')[0]; }"
                "return 'wait';"
            )
            seen.add(state)
            if state == "done":
                break
            time.sleep(0.25 if state == "wait" else 0.05)
        print("level check sections:", sorted(seen))
        headline = d.text("//p[contains(@class,'headline')]")
        print("level check:", headline)
        assert "CEFR" in headline and {"vocab", "2. Kanji readings", "3. Grammar", "3. Grammar, part 2", "4. Reading"} <= seen, seen
        d.shot(out / "15b-level-check.png")

        # Practice test (ADR 0015): a half N5 test, prepared by the fake AI; answer with the
        # first option (some right, some wrong), hand in both parts, review, retry the missed.
        d.click("//nav//button[normalize-space()='Practice test']")
        d.click("//button[normalize-space()='N5']")
        d.click("//button[starts-with(normalize-space(), 'Half')]")
        d.click("//button[normalize-space()='Prepare the test']")
        d.find("//nav[contains(@class,'grid')]", timeout=120)
        d.shot(out / "15c-practice-question.png")
        parts = []
        for _ in range(600):
            state = d.js(
                "if (document.querySelector('.headline')) return 'done';"
                "const yes = [...document.querySelectorAll('button')].find(b => b.textContent.trim() === 'Hand in anyway');"
                "if (yes) { yes.click(); return 'confirm'; }"
                "const open = document.querySelector('.qn:not(.done)');"
                "if (open) { open.click(); return 'pick'; }"
                "const hand = [...document.querySelectorAll('header.bar button.primary')].find(b => b.textContent.startsWith('Hand in'));"
                "if (hand) { const p = document.querySelector('header.bar strong')?.textContent; hand.click(); return 'hand:' + p; }"
                "return 'wait';"
            )
            if state == "pick":
                time.sleep(0.05)
                d.js("document.querySelector('.options button')?.click()")
            elif state.startswith("hand:"):
                parts.append(state[5:])
            if state == "done":
                break
            time.sleep(0.1)
        print("practice parts handed in:", parts)
        assert len(parts) == 2, parts
        verdict = d.text("//p[contains(@class,'headline')]")
        sections = d.js("return [...document.querySelectorAll('.sec strong')].map(e => e.textContent)")
        print("practice result:", verdict, sections)
        assert verdict in ("Likely pass", "Borderline", "Not yet") and len(sections) == 1 and sections[0].endswith("/120"), (verdict, sections)
        d.shot(out / "15d-practice-result.png")
        d.click("//button[normalize-space()='Review answers']")
        d.find("//section[contains(@class,'rev')]")
        d.shot(out / "15e-practice-review.png")
        d.click("(//section[contains(@class,'rev')])[1]//button[normalize-space()='Report a bad question']")
        d.find("(//section[contains(@class,'rev')])[1]//button[normalize-space()='Reported · undo']")
        card = d.js("const b = [...document.querySelectorAll('section.rev button')].find(b => b.textContent.startsWith('Anki card for')); if (b) { b.click(); return b.textContent; } return null;")
        if card:
            d.find("//button[normalize-space()='Card drafted ✓']", timeout=60)
        print("practice card:", card)
        d.click("//button[normalize-space()='← Results']")
        d.click("//button[starts-with(normalize-space(), 'Retry the')]")
        d.find("//header[contains(@class,'bar')]//strong[normalize-space()='Missed questions']")
        tests = d.invoke("practice_intro")["tests"]
        print("practice tests:", [(t["kind"], t["items"], t["answered"]) for t in tests])
        assert [t["kind"] for t in tests] == ["retry", "practice"], tests
        d.click("//button[normalize-space()='Pause']")
        d.find("//button[normalize-space()='Continue']")

        # The question bank (ADR 0016): with earlier questions allowed, a second half N5 test
        # comes from the bank at once, without AI requests; every request was logged.
        mem = d.invoke("memory_stats")
        print("memory:", mem)
        assert mem["requests"] > 0 and mem["bank"] >= 24, mem
        d.click("//button[normalize-space()='N5']")
        d.click("//button[starts-with(normalize-space(), 'Half')]")
        d.click("//button[normalize-space()='Bank first']")
        d.click("//label[contains(., 'Include questions from earlier tests')]/input")
        for _ in range(40):
            plan = d.js("return document.querySelector('p.plan strong')?.textContent ?? ''")
            if "no AI requests" in plan:
                break
            time.sleep(0.25)
        print("plan:", plan)
        assert "no AI requests" in plan, plan
        d.shot(out / "15f-practice-bank.png")
        d.click("//button[normalize-space()='Prepare the test']")
        d.find("//nav[contains(@class,'grid')]", timeout=30)
        after = d.invoke("memory_stats")
        assert after["requests"] == mem["requests"] and after["reused"]["practice_items"] >= 24, after
        d.click("//button[normalize-space()='Pause']")

        # Stats: characters read (logged as paragraphs counted as read) and the streak.
        d.click("//nav//button[normalize-space()='Stats']")
        today = d.text("//div[contains(@class,'tile')][.//span[normalize-space()='Read today']]")
        print("stats:", today.replace("\n", " "))
        st = d.invoke("reading_stats", {"today": time.strftime("%Y-%m-%d")})
        assert st["chars_today"] > 0 and st["streak"] == 1, st
        d.js("document.querySelectorAll('rect.hit')[29].dispatchEvent(new MouseEvent('mouseenter'))")
        d.find("//div[contains(@class,'tip')]")
        d.shot(out / "16-stats.png")
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
