//! The one safe way to change a note: secret refusal, symlink refusal, hash check, intent/commit journal, atomic rename.
use crate::{store::State, util::*};
use std::{fs, io::Write, path::Path};

/// Outer Err = refused or failed. Inner Err = conflict (someone changed the file); Ok = new sha256.
pub fn write_note(st: &State, rel: &Path, data: &[u8], expect: Option<&str>) -> Res<Result<String, String>> {
    // the secret scan treats a NUL in the first 8 KB as "binary" and skips the file: a note must not be able to use that to hide a secret
    if data.iter().take(8192).any(|&b| b == 0) {
        return Err("content has NUL bytes: notes are text".into());
    }
    if !hits(&patterns(), data).is_empty() {
        return Err("content looks like a secret; put it in .secrets/ by hand".into());
    }
    no_symlinks(&st.root, rel)?;
    let path = st.root.join(rel);
    let cur = fs::read(&path).ok().map(|d| sha(&d));
    if let Some(e) = expect {
        let ok = match (&cur, e) {
            (None, "new") => true,
            (Some(h), x) => h == x,
            _ => false,
        };
        if !ok {
            return Ok(Err(format!("CONFLICT {} changed (now {})", rel.display(), cur.as_deref().unwrap_or("missing"))));
        }
    }
    let r = rel.to_string_lossy().replace('\\', "/");
    let new = sha(data);
    let id = st.append("intent", &r, &new, None)?;
    fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension(format!("smug-tmp-{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    st.append("commit", &r, &new, Some(id))?;
    Ok(Ok(new))
}
