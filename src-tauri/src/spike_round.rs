//! Read only newly appended round-end events; never retain or forward game logs.
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::PathBuf, time::SystemTime};

const MAX_READ: u64 = 64 * 1024;
const MAX_LINE: usize = 4096;

pub struct RoundLog {
    path: PathBuf,
    cursor: Option<u64>,
    created: Option<SystemTime>,
    pending: Vec<u8>,
}

impl RoundLog {
    pub fn new(path: PathBuf) -> Self {
        let mut log = Self { path, cursor:None, created:None, pending:Vec::new() };
        // Establish the initial EOF before audio detection starts.
        log.poll();
        log
    }

    pub fn poll(&mut self) -> bool {
        self.read_new().unwrap_or(false)
    }

    fn read_new(&mut self) -> std::io::Result<bool> {
        let mut file = File::open(&self.path)?;
        let meta = file.metadata()?;
        let len = meta.len();
        let created = meta.created().ok();
        let Some(mut offset) = self.cursor else {
            self.cursor = Some(len);
            self.created = created;
            return Ok(false);
        };
        if len < offset || created != self.created {
            // Truncated/replaced file: start a fresh stream.
            offset = 0;
            self.pending.clear();
        }
        self.created = created;
        if len.saturating_sub(offset) > MAX_READ {
            // A delayed/backlogged log is not a trustworthy live event.
            self.cursor = Some(len);
            self.pending.clear();
            return Ok(false);
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        file.take(len.saturating_sub(offset)).read_to_end(&mut bytes)?;
        self.cursor = Some(offset + bytes.len() as u64);
        let mut ended = false;
        for byte in bytes {
            if byte == b'\n' {
                ended |= is_round_end(&self.pending);
                self.pending.clear();
            } else {
                self.pending.push(byte);
                if self.pending.len() > MAX_LINE { self.pending.clear(); }
            }
        }
        Ok(ended)
    }
}

fn is_round_end(line: &[u8]) -> bool {
    let Ok(line) = std::str::from_utf8(line) else { return false; };
    let Some((prefix, suffix)) = line.split_once("LogShooterGameState: Warning: AShooterGameState::OnRoundEnded for round '") else { return false; };
    let Some(round) = suffix.trim_end().strip_suffix('\'') else { return false; };
    prefix.starts_with('[') && prefix.trim_end().ends_with(']')
        && !round.is_empty() && round.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    const EVENT: &str = "[2026.10.11-19.01.19:503][816]LogShooterGameState: Warning: AShooterGameState::OnRoundEnded for round '3'\r\n";
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self { Self(std::env::temp_dir().join(format!("spike-round-{}.log", uuid::Uuid::new_v4()))) }
        fn append(&self, text: &str) {
            std::fs::OpenOptions::new().create(true).append(true).open(&self.0).unwrap().write_all(text.as_bytes()).unwrap();
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); } }

    #[test]
    fn skips_history_detects_new_end_once_and_waits_for_complete_line() {
        let f = Fixture::new(); f.append(EVENT);
        let mut log = RoundLog::new(f.0.clone());
        assert!(!log.poll());
        f.append(&EVENT[..60]); assert!(!log.poll());
        f.append(&EVENT[60..]); assert!(log.poll()); assert!(!log.poll());
    }
    #[test]
    fn missing_log_and_unrelated_lines_are_not_round_ends() {
        let f = Fixture::new(); let mut log = RoundLog::new(f.0.clone());
        assert!(!log.poll());
        f.append(EVENT); assert!(!log.poll()); // first successful open skips history
        f.append("unrelated log\nOnRoundEnded\n"); assert!(!log.poll());
        f.append(EVENT); assert!(log.poll());
    }
    #[test]
    fn handles_truncation_and_replacement() {
        let f = Fixture::new(); f.append(&"x".repeat(1000));
        let mut log = RoundLog::new(f.0.clone());
        std::fs::write(&f.0, EVENT).unwrap(); assert!(log.poll());
        std::fs::remove_file(&f.0).unwrap(); assert!(!log.poll());
        std::fs::write(&f.0, "").unwrap(); assert!(!log.poll());
        f.append(EVENT); assert!(log.poll());
    }
    #[test]
    fn ignores_backlog_and_rejects_malformed_event() {
        let f = Fixture::new(); f.append(""); let mut log = RoundLog::new(f.0.clone());
        f.append(&"x".repeat(MAX_READ as usize)); f.append(EVENT); assert!(!log.poll());
        f.append(EVENT); assert!(log.poll());
        assert!(!is_round_end(EVENT.replace("'3'", "'unknown'").as_bytes()));
        assert!(!is_round_end(b"chat: AShooterGameState::OnRoundEnded for round '3'"));
    }
}
