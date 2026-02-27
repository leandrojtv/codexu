use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::process::Command;

pub fn ripgrep_search(query: &str) -> Result<Vec<String>> {
    let output = Command::new("rg")
        .args(["--line-number", "--with-filename", query, "."])
        .output()
        .context("falha ao executar ripgrep")?;

    if !output.status.success() {
        // rg retorna 1 quando não há matches; não tratamos como erro fatal.
        if output.status.code() == Some(1) {
            return Ok(vec![]);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ripgrep retornou erro: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

pub fn sqlite_fts_search(db_path: &str, query: &str, limit: usize) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("falha ao abrir sqlite db: {db_path}"))?;

    let mut stmt = conn.prepare(
        "SELECT path, snippet(docs_fts, 1, '[', ']', ' … ', 16) FROM docs_fts WHERE docs_fts MATCH ?1 LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![query, limit as i64], |row| {
        let path: String = row.get(0)?;
        let snippet: String = row.get(1)?;
        Ok(format!("{path}: {snippet}"))
    })?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }

    Ok(out)
}
