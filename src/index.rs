//! Tantivy full-text index in .smug/tantivy, refreshed incrementally. Turkish-friendly: ascii-folded, lower-cased.
use crate::{store::State, util::*};
use std::{collections::HashSet, fs, path::Path, time::UNIX_EPOCH};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::QueryParser,
    schema::{Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, STORED, STRING},
    snippet::SnippetGenerator,
    tokenizer::{AsciiFoldingFilter, LowerCaser, SimpleTokenizer, TextAnalyzer},
    Index, IndexWriter, TantivyDocument, Term,
};

pub struct Hit {
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

fn open(root: &Path) -> Res<(Index, Field, Field)> {
    let mut b = Schema::builder();
    let path = b.add_text_field("path", STRING | STORED);
    let opts = TextOptions::default().set_stored().set_indexing_options(TextFieldIndexing::default().set_tokenizer("smug").set_index_option(IndexRecordOption::WithFreqsAndPositions));
    let body = b.add_text_field("body", opts);
    let dir = root.join(".smug/tantivy");
    fs::create_dir_all(&dir)?;
    let idx = Index::open_or_create(MmapDirectory::open(&dir)?, b.build())?;
    idx.tokenizers().register("smug", TextAnalyzer::builder(SimpleTokenizer::default()).filter(LowerCaser).filter(AsciiFoldingFilter).build());
    Ok((idx, path, body))
}

/// Re-indexes only notes whose size/mtime changed; drops deleted ones. Returns how many changed.
pub fn refresh(root: &Path, st: &State) -> Res<usize> {
    let (idx, path_f, body_f) = open(root)?;
    let known = st.docs()?;
    let pats = patterns();
    let mut writer: IndexWriter = idx.writer_with_num_threads(1, 50_000_000)?;
    let (mut seen, mut set, mut changed) = (HashSet::new(), vec![], 0);
    for f in md_files(root) {
        let r = rel(root, &f);
        let m = fs::metadata(&f)?;
        let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
        let sig = format!("{}:{t}", m.len());
        seen.insert(r.clone());
        if known.get(&r) == Some(&sig) {
            continue;
        }
        changed += 1;
        writer.delete_term(Term::from_field_text(path_f, &r));
        let d = fs::read(&f)?;
        if hits(&pats, &d).is_empty() {
            let mut doc = TantivyDocument::default();
            doc.add_text(path_f, &r);
            doc.add_text(body_f, String::from_utf8_lossy(&d));
            writer.add_document(doc)?;
        } // secret-shaped notes are never indexed
        set.push((r, sig));
    }
    let gone: Vec<String> = known.keys().filter(|k| !seen.contains(*k)).cloned().collect();
    for g in &gone {
        changed += 1;
        writer.delete_term(Term::from_field_text(path_f, g));
    }
    if changed > 0 {
        writer.commit()?;
        st.docs_apply(&set, &gone)?;
    }
    Ok(changed)
}

pub fn retrieve(root: &Path, st: &State, q: &str, k: usize) -> Res<Vec<Hit>> {
    refresh(root, st)?;
    let clean: String = q.chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
    if clean.trim().is_empty() || k == 0 {
        return Ok(vec![]);
    }
    let (idx, path_f, body_f) = open(root)?;
    let reader = idx.reader()?;
    let s = reader.searcher();
    let query = QueryParser::for_index(&idx, vec![body_f]).parse_query(&clean)?;
    let top = s.search(&query, &TopDocs::with_limit(k).order_by_score())?;
    let sg = SnippetGenerator::create(&s, &*query, body_f)?;
    let mut out = vec![];
    for (score, addr) in top {
        let doc: TantivyDocument = s.doc(addr)?;
        let path = doc.get_first(path_f).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let snippet = sg.snippet_from_doc(&doc).fragment().replace('\n', " ");
        out.push(Hit { path, score, snippet });
    }
    Ok(out)
}
