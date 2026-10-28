// Listening (ADR 0017): the computer's own voices (Web Speech API in the webview), or VOICEVOX
// when it runs next to Ankura. `say` resolves when the text has been spoken (or was stopped).
import { invoke } from "@tauri-apps/api/core";

export type Engine = "auto" | "system" | "voicevox";

export interface SpeechSettings {
  engine: Engine;
  system_voice: string;
  style: number | null;
  speed: number;
  voicevox_url: string;
}

export interface VoicevoxSpeaker {
  name: string;
  styles: { name: string; id: number }[];
}

let settings: SpeechSettings | null = null;
let speakers: VoicevoxSpeaker[] | null = null;
/** VOICEVOX audio plays through Web Audio: exact end events, and pause = suspend. */
let ctx: AudioContext | null = null;
let source: AudioBufferSourceNode | null = null;
let finish: (() => void) | null = null;

const synth = (): SpeechSynthesis | null => (typeof speechSynthesis === "undefined" ? null : speechSynthesis);

/** The computer's Japanese voices (they load asynchronously in some webviews). */
export async function systemVoices(): Promise<SpeechSynthesisVoice[]> {
  const s = synth();
  if (!s) return [];
  const ja = () => s.getVoices().filter((v) => v.lang.toLowerCase().replace("_", "-").startsWith("ja"));
  if (s.getVoices().length) return ja();
  await new Promise<void>((done) => {
    const t = setTimeout(done, 1500);
    s.addEventListener("voiceschanged", () => (clearTimeout(t), done()), { once: true });
  });
  return ja();
}

/** VOICEVOX's voices, or null when it isn't running. */
export async function voicevoxSpeakers(): Promise<VoicevoxSpeaker[] | null> {
  try {
    return await invoke<VoicevoxSpeaker[]>("voicevox_speakers");
  } catch {
    return null;
  }
}

/** Loads the settings and checks for VOICEVOX; call again after the settings change. */
export async function load(): Promise<SpeechSettings> {
  settings = await invoke<SpeechSettings>("speech_settings");
  speakers = settings.engine === "system" ? null : await voicevoxSpeakers();
  return settings;
}

export interface Voice {
  engine: "voicevox" | "system" | "none";
  /** What to show: the VOICEVOX character (its terms ask for "VOICEVOX:name") or the system voice. */
  label: string;
}

/** The voice `say` will use. */
export async function current(): Promise<Voice> {
  if (!settings) await load();
  const s = settings!;
  if (s.engine !== "system" && speakers?.length) {
    const style = styleId();
    const sp = speakers.find((x) => x.styles.some((st) => st.id === style)) ?? speakers[0];
    return { engine: "voicevox", label: `VOICEVOX:${sp.name}` };
  }
  if (s.engine === "voicevox") return { engine: "none", label: "VOICEVOX isn't running" };
  const v = await pickSystemVoice();
  return v ? { engine: "system", label: v.name } : { engine: "none", label: "no Japanese voice on this computer" };
}

/** Who speaks a line of a listening question (ADR 0018). */
export type Role = "narrator" | "woman" | "man";

// VOICEVOX characters by voice, for the woman and the man of a dialogue.
const WOMEN = ["四国めたん", "春日部つむぎ", "冥鳴ひまり", "九州そら", "もち子さん", "WhiteCUL", "小夜/SAYO", "櫻歌ミコ", "波音リツ", "中国うさぎ"];
const MEN = ["玄野武宏", "青山龍星", "白上虎太郎", "剣崎雌雄", "雀松朱司", "麒ヶ島宗麟", "ちび式じい"];
const MALE_SYSTEM = /ichiro|keita|otoya|hattori|daichi|naoki/i;

function roleStyle(role: Role): number {
  const narrator = styleId();
  if (role === "narrator") return narrator;
  const names = role === "woman" ? WOMEN : MEN;
  for (const name of names) {
    const sp = speakers?.find((x) => x.name === name);
    const st = sp?.styles.find((x) => x.name === "ノーマル") ?? sp?.styles[0];
    if (st && st.id !== narrator) return st.id;
  }
  return narrator;
}

async function roleVoice(role: Role): Promise<{ voice: SpeechSynthesisVoice | null; pitch: number }> {
  const narrator = await pickSystemVoice();
  if (role === "narrator") return { voice: narrator, pitch: 1 };
  const voices = await systemVoices();
  const fits = voices.filter((v) => (role === "man") === MALE_SYSTEM.test(v.name) && v.name !== narrator?.name);
  // One voice only: tell the speakers apart by pitch.
  return fits[0] ? { voice: fits[0], pitch: 1 } : { voice: narrator, pitch: role === "man" ? 0.7 : 1.2 };
}

function styleId(): number {
  const ids = (speakers ?? []).flatMap((s) => s.styles.map((st) => st.id));
  return settings?.style != null && ids.includes(settings.style) ? settings.style : (ids[0] ?? 0);
}

async function pickSystemVoice(): Promise<SpeechSynthesisVoice | null> {
  const voices = await systemVoices();
  return voices.find((v) => v.name === settings?.system_voice) ?? voices.find((v) => v.localService) ?? voices[0] ?? null;
}

export function speed(): number {
  return settings?.speed ?? 1;
}

export function setSpeed(x: number) {
  if (settings) settings.speed = x;
  invoke("set_setting", { key: "tts_speed", value: String(x) }).catch(() => {});
}

/** Speaks a text; resolves when it ends or is stopped, rejects when no voice can speak it. */
export async function say(text: string, role: Role = "narrator"): Promise<void> {
  stop();
  const v = await current();
  if (v.engine === "none") throw new Error(`Can't speak: ${v.label}. Choose a voice in Settings → Listening.`);
  if (v.engine === "voicevox") {
    const raw = await invoke<ArrayBuffer | Uint8Array | number[]>("voicevox_speak", { text, style: roleStyle(role), speed: speed() });
    // Tauri hands raw responses over as an ArrayBuffer on some platforms, a number array on others.
    const bytes =
      raw instanceof ArrayBuffer ? raw : raw instanceof Uint8Array ? raw.slice().buffer : new Uint8Array(raw).buffer;
    ctx ??= new AudioContext();
    if (ctx.state === "suspended") await ctx.resume();
    let buffer: AudioBuffer;
    try {
      buffer = await ctx.decodeAudioData(bytes);
    } catch (e) {
      throw new Error(`the audio couldn't be played (${e})`);
    }
    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.connect(ctx.destination);
    source = src;
    try {
      await new Promise<void>((done) => {
        finish = done;
        src.onended = () => done();
        src.start();
      });
    } finally {
      if (source === src) source = null;
      finish = null;
    }
    return;
  }
  const s = synth()!;
  const u = new SpeechSynthesisUtterance(text);
  u.lang = "ja-JP";
  u.rate = speed();
  const { voice, pitch } = await roleVoice(role);
  if (voice) u.voice = voice;
  u.pitch = pitch;
  await new Promise<void>((done, fail) => {
    finish = done;
    u.onend = () => done();
    u.onerror = (e) => (e.error === "canceled" || e.error === "interrupted" ? done() : fail(new Error(`speech: ${e.error}`)));
    s.speak(u);
  });
  finish = null;
}

export function pause() {
  if (source) ctx?.suspend();
  synth()?.pause();
}

export function resume() {
  ctx?.resume();
  synth()?.resume();
}

/** Stops speaking; a pending `say` resolves. */
export function stop() {
  if (source) {
    const s = source;
    source = null;
    s.onended = null;
    try {
      s.stop();
    } catch {}
  }
  if (ctx?.state === "suspended") ctx.resume();
  synth()?.cancel();
  const f = finish;
  finish = null;
  f?.();
}
