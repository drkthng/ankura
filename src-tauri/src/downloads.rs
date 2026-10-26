//! First-run downloads: SudachiDict core (via PyPI's JSON API, which always names the latest
//! release) and JMdict in Yomitan format (yomidevs/jmdict-yomitan release).

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

const SUDACHI_PYPI: &str = "https://pypi.org/pypi/SudachiDict-core/json";
const JMDICT_URL: &str = "https://github.com/yomidevs/jmdict-yomitan/releases/latest/download/JMdict_english.zip";

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder().tls_config(tls).build().new_agent()
}

#[derive(Clone, Serialize)]
struct Progress<'a> {
    what: &'a str,
    done: u64,
    total: Option<u64>,
}

fn download(app: &AppHandle, what: &str, url: &str, dest: &Path) -> Result<(), String> {
    let mut resp = agent().get(url).call().map_err(|e| format!("download of {what} failed: {e}"))?;
    let total = resp.body().content_length();
    let mut reader = resp.body_mut().as_reader();
    let tmp = dest.with_extension("part");
    let mut out = File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 16];
    let mut done = 0u64;
    let mut last_emit = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("download of {what} failed: {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if done - last_emit > 1 << 20 {
            last_emit = done;
            let _ = app.emit("download-progress", Progress { what, done, total });
        }
    }
    let _ = app.emit("download-progress", Progress { what, done, total });
    fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// Downloads the SudachiDict core wheel and extracts `system.dic` into `dir`.
pub fn sudachi_core(app: &AppHandle, dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let meta: Value = agent()
        .get(SUDACHI_PYPI)
        .call()
        .map_err(|e| format!("can't reach PyPI: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    let wheel = meta["urls"]
        .as_array()
        .and_then(|u| u.iter().find(|f| f["packagetype"] == "bdist_wheel"))
        .and_then(|f| f["url"].as_str())
        .ok_or("PyPI lists no SudachiDict-core wheel")?
        .to_string();
    let version = meta["info"]["version"].as_str().unwrap_or("unknown");
    let wheel_path = dir.join("sudachidict-core.whl");
    download(app, &format!("SudachiDict core {version}"), &wheel, &wheel_path)?;

    let mut zip = zip::ZipArchive::new(File::open(&wheel_path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let name = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().and_then(|f| f.name().ok().map(|n| n.into_owned())))
        .find(|n| n.ends_with("resources/system.dic"))
        .ok_or("the wheel contains no system.dic")?;
    let dest = dir.join("system.dic");
    let mut src = zip.by_name(&name).map_err(|e| e.to_string())?;
    let mut out = File::create(&dest).map_err(|e| e.to_string())?;
    io::copy(&mut src, &mut out).map_err(|e| e.to_string())?;
    drop(src);
    let _ = fs::remove_file(&wheel_path);
    Ok(dest)
}

/// Downloads JMdict (English, Yomitan format) and returns the zip path.
pub fn jmdict(app: &AppHandle, dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let dest = dir.join("JMdict_english.zip");
    download(app, "JMdict", JMDICT_URL, &dest)?;
    Ok(dest)
}
