use crate::asr::models::{model_dir, url_for, ModelSpec};
use crate::asr::sha256::sha256_file;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROGRESS_EVERY: u64 = 64 * 1024;
const READ_CHUNK: usize = 256 * 1024;

#[derive(PartialEq, Eq, Debug)]
pub enum Resume {
    Complete,
    Range(u64),
    Restart,
}

pub fn resume_from(part_len: u64, total: u64) -> Resume {
    if part_len == 0 || part_len > total {
        Resume::Restart
    } else if part_len == total {
        Resume::Complete
    } else {
        Resume::Range(part_len)
    }
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn verify_or_remove(path: &Path, want_sha256: &str) -> Result<(), String> {
    let got = sha256_file(path)
        .map_err(|e| format!("Could not read {} to check it: {e}", name_of(path)))?;
    if got == want_sha256 {
        return Ok(());
    }
    let _ = std::fs::remove_file(path);
    Err(format!("{} failed its integrity check (expected {want_sha256}, got {got}). The download was discarded; try again.", name_of(path)))
}

fn finish(part: &Path, target: &Path, spec: &ModelSpec) -> Result<(), String> {
    verify_or_remove(part, spec.sha256)?;
    std::fs::rename(part, target)
        .map_err(|e| format!("Could not move {} into place: {e}", spec.file))
}

pub fn download_model(
    spec: &ModelSpec,
    on_progress: &dyn Fn(u64, u64),
    cancel: &dyn Fn() -> bool,
) -> Result<PathBuf, String> {
    let dir = model_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let target = dir.join(spec.file);
    if target.exists() && verify_or_remove(&target, spec.sha256).is_ok() {
        on_progress(spec.bytes, spec.bytes);
        return Ok(target);
    }
    let part = dir.join(format!("{}.part", spec.file));
    let part_len = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    let mut done = match resume_from(part_len, spec.bytes) {
        Resume::Complete => {
            finish(&part, &target, spec)?;
            on_progress(spec.bytes, spec.bytes);
            return Ok(target);
        }
        Resume::Range(n) => n,
        Resume::Restart => 0,
    };

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .build();
    let url = url_for(spec);
    let mut req = agent.get(&url);
    if done > 0 {
        req = req.set("Range", &format!("bytes={done}-"));
    }
    let resp = match req.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(_, _)) if done > 0 => {
            done = 0;
            agent
                .get(&url)
                .call()
                .map_err(|e| format!("Could not reach {url}: {e}"))?
        }
        Err(e) => return Err(format!("Could not reach {url}: {e}")),
    };
    if done > 0 && resp.status() != 206 {
        done = 0;
    }

    let mut file = if done > 0 {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(&part)
            .map_err(|e| format!("Could not reopen {}: {e}", name_of(&part)))?;
        f.seek(SeekFrom::Start(done))
            .map_err(|e| format!("Could not seek {}: {e}", name_of(&part)))?;
        f
    } else {
        std::fs::File::create(&part)
            .map_err(|e| format!("Could not create {}: {e}", name_of(&part)))?
    };

    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; READ_CHUNK];
    let mut since = 0u64;
    loop {
        if cancel() {
            let _ = file.flush();
            return Err("The model download was cancelled.".into());
        }
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("The download stopped early: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("Could not write {}: {e}", name_of(&part)))?;
        done += n as u64;
        since += n as u64;
        if since >= PROGRESS_EVERY {
            since = 0;
            on_progress(done, spec.bytes);
        }
    }
    file.flush()
        .map_err(|e| format!("Could not finish writing {}: {e}", name_of(&part)))?;
    drop(file);
    finish(&part, &target, spec)?;
    on_progress(spec.bytes, spec.bytes);
    Ok(target)
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
