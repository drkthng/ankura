// Spike S3: dump Sudachi tokens as JSON lines, same shape as the Lindera binary.
// Usage: sudachi <system.dic> [A|B|C] < sentences.txt > tokens.jsonl
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

use serde_json::json;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::Mode;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::MorphemeList;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dic = PathBuf::from(std::env::args().nth(1).expect("path to system.dic"));
    let mode = match std::env::args().nth(2).as_deref() {
        Some("A") => Mode::A,
        Some("B") => Mode::B,
        _ => Mode::C,
    };
    let t0 = Instant::now();
    let config = Config::new(None, None, Some(dic))?;
    let dict = JapaneseDictionary::from_cfg(&config)?;
    eprintln!("load_ms={}", t0.elapsed().as_millis());

    let mut tokenizer = StatefulTokenizer::new(&dict, mode);
    let mut morphemes = MorphemeList::empty(&dict);
    let mut out = io::BufWriter::new(io::stdout().lock());
    let t1 = Instant::now();
    let mut chars = 0usize;
    for line in io::stdin().lock().lines() {
        let line = line?;
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        chars += text.chars().count();
        tokenizer.reset().push_str(text);
        tokenizer.do_tokenize()?;
        morphemes.collect_results(&mut tokenizer)?;
        let toks: Vec<_> = morphemes
            .iter()
            .map(|m| {
                json!({
                    "surface": m.surface().to_string(),
                    "dictionary_form": m.dictionary_form(),
                    "normalized_form": m.normalized_form(),
                    "reading_form": m.reading_form(),
                    "part_of_speech": m.part_of_speech().join(","),
                })
            })
            .collect();
        writeln!(out, "{}", json!({ "text": text, "tokens": toks }))?;
    }
    eprintln!("segment_ms={} chars={}", t1.elapsed().as_millis(), chars);
    Ok(())
}
