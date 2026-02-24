use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: String,
    pub snippet: String,
}

#[derive(Debug)]
pub struct Indexer {
    db_path: PathBuf,
}

impl Indexer {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    pub fn init(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("falha ao abrir db: {}", self.db_path.display()))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS files (
              path TEXT PRIMARY KEY,
              hash TEXT NOT NULL,
              updated_at INTEGER NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(
              path UNINDEXED,
              content
            );
            "#,
        )?;

        Ok(())
    }

    pub fn index_workspace(&self, workspace: &Path) -> Result<usize> {
        self.init()?;
        let conn = Connection::open(&self.db_path)?;
        let mut updated = 0usize;

        for entry in WalkDir::new(workspace).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }

            let abs = entry.path();
            let rel = match abs.strip_prefix(workspace) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if should_skip(rel) {
                continue;
            }

            let content = match fs::read_to_string(abs) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let hash = hash_content(&content);
            let rels = rel.to_string_lossy().to_string();

            let old_hash: Option<String> = conn
                .query_row(
                    "SELECT hash FROM files WHERE path = ?1",
                    params![rels],
                    |row| row.get(0),
                )
                .ok();

            if old_hash.as_deref() == Some(hash.as_str()) {
                continue;
            }

            conn.execute("DELETE FROM docs_fts WHERE path = ?1", params![rels])?;
            conn.execute(
                "INSERT INTO docs_fts(path, content) VALUES(?1, ?2)",
                params![rels, content],
            )?;
            conn.execute(
                r#"
                INSERT INTO files(path, hash, updated_at)
                VALUES(?1, ?2, strftime('%s','now'))
                ON CONFLICT(path) DO UPDATE SET
                  hash = excluded.hash,
                  updated_at = excluded.updated_at
                "#,
                params![rels, hash],
            )?;

            updated += 1;
        }

        Ok(updated)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
        self.init()?;
        let conn = Connection::open(&self.db_path)?;

        let mut stmt = conn.prepare(
            "SELECT path, snippet(docs_fts, 1, '[', ']', ' … ', 16) FROM docs_fts WHERE docs_fts MATCH ?1 LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(SearchHit {
                path: row.get(0)?,
                snippet: row.get(1)?,
            })
        })?;

        let mut hits = Vec::new();
        for row in rows {
            hits.push(row?);
        }

        Ok(hits)
    }
}

fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn should_skip(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("/.git/") || s.contains("/target/") || s.ends_with(".png") || s.ends_with(".jpg")
}
