//! State: redb (events, doc signatures, feedback) + an append-only journal that lives in git.
//! The journal is the durable truth; `.smug/state.redb` can be deleted and is rebuilt from it.
use crate::util::*;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io::Write, path::{Path, PathBuf}};

const EVENTS: TableDefinition<u64, &str> = TableDefinition::new("events");
const DOCS: TableDefinition<&str, &str> = TableDefinition::new("docs");
const FEEDBACK: TableDefinition<&str, i64> = TableDefinition::new("feedback");

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub id: u64,
    pub ts: i64,
    pub kind: String,
    pub path: String,
    pub hash: String,
    pub r#ref: Option<u64>,
}

pub struct State {
    pub root: PathBuf,
    db: Database,
    _lock: Lock,
}

fn journal(root: &Path) -> PathBuf {
    root.join("history/events.jsonl")
}

impl State {
    pub fn open(root: &Path) -> Res<State> {
        if !is_vault(root) {
            return Err(format!("{} is not a smug vault. Run `smug init <dir>`, or point smug at one with --vault <dir>.", root.display()).into());
        }
        let lock = Lock::take(root, 8000)?;
        let path = root.join(".smug/state.redb");
        let db = match Database::create(&path) {
            Ok(d) => d,
            Err(_) => {
                // a damaged state file is disposable: set it aside, start clean, the journal and the Markdown rebuild everything
                let _ = fs::rename(&path, root.join(".smug/state.redb.corrupt"));
                Database::create(&path)?
            }
        };
        let w = db.begin_write()?;
        {
            w.open_table(EVENTS)?;
            w.open_table(DOCS)?;
            w.open_table(FEEDBACK)?;
        }
        w.commit()?;
        let s = State { root: root.to_path_buf(), db, _lock: lock };
        s.replay()?;
        Ok(s)
    }

    /// Copies journal lines the database has not seen (a crash between the two writes, or a deleted .smug/).
    fn replay(&self) -> Res<()> {
        let Ok(text) = fs::read_to_string(journal(&self.root)) else { return Ok(()) };
        let last = self.events()?.last().map(|e| e.id).unwrap_or(0);
        let w = self.db.begin_write()?;
        {
            let mut t = w.open_table(EVENTS)?;
            for l in text.lines() {
                if let Ok(e) = serde_json::from_str::<Event>(l) {
                    if e.id > last {
                        t.insert(e.id, l)?;
                    }
                }
            }
        }
        w.commit()?;
        Ok(())
    }

    pub fn events(&self) -> Res<Vec<Event>> {
        let r = self.db.begin_read()?;
        let t = r.open_table(EVENTS)?;
        let mut v = vec![];
        for e in t.iter()? {
            let (_, s) = e?;
            if let Ok(ev) = serde_json::from_str(s.value()) {
                v.push(ev);
            }
        }
        Ok(v)
    }

    pub fn append(&self, kind: &str, path: &str, hash: &str, r: Option<u64>) -> Res<u64> {
        let id = self.events()?.last().map(|e| e.id).unwrap_or(0) + 1;
        let ev = Event { id, ts: now(), kind: kind.into(), path: path.into(), hash: hash.into(), r#ref: r };
        let line = serde_json::to_string(&ev)?;
        fs::create_dir_all(self.root.join("history"))?;
        let mut f = fs::OpenOptions::new().create(true).append(true).open(journal(&self.root))?;
        writeln!(f, "{line}")?;
        f.sync_all()?;
        let w = self.db.begin_write()?;
        {
            w.open_table(EVENTS)?.insert(id, line.as_str())?;
        }
        w.commit()?;
        Ok(id)
    }

    /// Intents that never got a commit/abort/recovered event.
    pub fn dangling(&self) -> Res<Vec<Event>> {
        let ev = self.events()?;
        let done: std::collections::HashSet<u64> = ev.iter().filter_map(|e| e.r#ref).collect();
        Ok(ev.into_iter().filter(|e| e.kind == "intent" && !done.contains(&e.id)).collect())
    }

    pub fn docs(&self) -> Res<HashMap<String, String>> {
        let r = self.db.begin_read()?;
        let t = r.open_table(DOCS)?;
        let mut m = HashMap::new();
        for e in t.iter()? {
            let (k, v) = e?;
            m.insert(k.value().to_string(), v.value().to_string());
        }
        Ok(m)
    }

    pub fn docs_apply(&self, set: &[(String, String)], del: &[String]) -> Res<()> {
        let w = self.db.begin_write()?;
        {
            let mut t = w.open_table(DOCS)?;
            for (k, v) in set {
                t.insert(k.as_str(), v.as_str())?;
            }
            for k in del {
                t.remove(k.as_str())?;
            }
        }
        w.commit()?;
        Ok(())
    }

    pub fn docs_clear(&self) -> Res<()> {
        let w = self.db.begin_write()?;
        w.delete_table(DOCS)?;
        w.open_table(DOCS)?;
        w.commit()?;
        Ok(())
    }

    pub fn feedback(&self) -> Res<HashMap<String, i64>> {
        let r = self.db.begin_read()?;
        let t = r.open_table(FEEDBACK)?;
        let mut m = HashMap::new();
        for e in t.iter()? {
            let (k, v) = e?;
            m.insert(k.value().to_string(), v.value());
        }
        Ok(m)
    }

    pub fn add_feedback(&self, path: &str, delta: i64) -> Res<i64> {
        let cur = self.feedback()?.get(path).copied().unwrap_or(0) + delta;
        let w = self.db.begin_write()?;
        {
            w.open_table(FEEDBACK)?.insert(path, cur)?;
        }
        w.commit()?;
        Ok(cur)
    }
}
