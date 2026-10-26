// Spike S3: dump Lindera tokens as JSON lines.
// Usage: s3tok <dictionary-dir> < sentences.txt > tokens.jsonl
use std::borrow::Cow;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Instant;

use lindera::dictionary::load_fs_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use serde_json::{json, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dict_dir = std::env::args().nth(1).expect("dictionary directory");
    let t0 = Instant::now();
    let dictionary = load_fs_dictionary(Path::new(&dict_dir))?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    eprintln!("load_ms={}", t0.elapsed().as_millis());

    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout().lock());
    let t1 = Instant::now();
    let mut chars = 0usize;
    for line in stdin.lock().lines() {
        let line = line?;
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        chars += text.chars().count();
        let mut tokens = segmenter.segment(Cow::Borrowed(text))?;
        let toks: Vec<Value> = tokens
            .iter_mut()
            .map(|t| {
                let mut v = t.as_value();
                if let Value::Object(ref mut m) = v {
                    m.insert("surface".into(), json!(t.surface.as_ref()));
                }
                v
            })
            .collect();
        writeln!(out, "{}", json!({ "text": text, "tokens": toks }))?;
    }
    eprintln!("segment_ms={} chars={}", t1.elapsed().as_millis(), chars);
    Ok(())
}
