use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRecord {
    pub id: i64,
    pub repo: String,
    pub pr_number: i64,
    pub provider: String,
    pub head_sha: String,
    pub verdict: String,
    pub summary: String,
    pub inline_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewStats {
    pub total_reviews: i64,
    pub approved: i64,
    pub request_changes: i64,
    pub commented: i64,
    pub avg_inline_comments: f64,
}

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> anyhow::Result<Self> {
        let mut conn = Connection::open(db_path)?;
        Self::init_schema(&mut conn)?;
        info!("Database initialized at {}", db_path);
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &mut Connection) -> SqliteResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS reviews (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo TEXT NOT NULL,
                pr_number INTEGER NOT NULL,
                provider TEXT NOT NULL,
                head_sha TEXT NOT NULL,
                verdict TEXT NOT NULL,
                summary TEXT NOT NULL,
                inline_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reviews_repo ON reviews(repo)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reviews_created ON reviews(created_at)",
            [],
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn save_review(
        &self,
        repo: &str,
        pr_number: i64,
        provider: &str,
        head_sha: &str,
        verdict: &str,
        summary: &str,
        inline_count: i64,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        let created_at = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO reviews (repo, pr_number, provider, head_sha, verdict, summary, inline_count, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![repo, pr_number, provider, head_sha, verdict, summary, inline_count, created_at],
        )?;

        let id = conn.last_insert_rowid();
        info!("Saved review {} for {}/{}", id, repo, pr_number);
        Ok(id)
    }

    pub async fn get_recent_reviews(&self, limit: i64) -> anyhow::Result<Vec<ReviewRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, repo, pr_number, provider, head_sha, verdict, summary, inline_count, created_at
             FROM reviews
             ORDER BY created_at DESC
             LIMIT ?1"
        )?;

        let reviews = stmt.query_map(params![limit], |row| {
            let created_at_str: String = row.get(8)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(ReviewRecord {
                id: row.get(0)?,
                repo: row.get(1)?,
                pr_number: row.get(2)?,
                provider: row.get(3)?,
                head_sha: row.get(4)?,
                verdict: row.get(5)?,
                summary: row.get(6)?,
                inline_count: row.get(7)?,
                created_at,
            })
        })?;

        let mut result = Vec::new();
        for review in reviews {
            result.push(review?);
        }

        Ok(result)
    }

    pub async fn get_stats(&self) -> anyhow::Result<ReviewStats> {
        let conn = self.conn.lock().unwrap();

        let total_reviews: i64 = conn.query_row(
            "SELECT COUNT(*) FROM reviews",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let approved: i64 = conn.query_row(
            "SELECT COUNT(*) FROM reviews WHERE verdict = 'Approve'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let request_changes: i64 = conn.query_row(
            "SELECT COUNT(*) FROM reviews WHERE verdict = 'RequestChanges'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let commented: i64 = conn.query_row(
            "SELECT COUNT(*) FROM reviews WHERE verdict = 'Comment'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let avg_inline: f64 = conn.query_row(
            "SELECT COALESCE(AVG(inline_count), 0.0) FROM reviews",
            [],
            |row| row.get(0),
        ).unwrap_or(0.0);

        Ok(ReviewStats {
            total_reviews,
            approved,
            request_changes,
            commented,
            avg_inline_comments: avg_inline,
        })
    }

    pub async fn get_reviews_by_repo(&self, repo: &str) -> anyhow::Result<Vec<ReviewRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, repo, pr_number, provider, head_sha, verdict, summary, inline_count, created_at
             FROM reviews
             WHERE repo = ?1
             ORDER BY created_at DESC"
        )?;

        let reviews = stmt.query_map(params![repo], |row| {
            let created_at_str: String = row.get(8)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(ReviewRecord {
                id: row.get(0)?,
                repo: row.get(1)?,
                pr_number: row.get(2)?,
                provider: row.get(3)?,
                head_sha: row.get(4)?,
                verdict: row.get(5)?,
                summary: row.get(6)?,
                inline_count: row.get(7)?,
                created_at,
            })
        })?;

        let mut result = Vec::new();
        for review in reviews {
            result.push(review?);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_operations() {
        let db = Database::new(":memory:").unwrap();

        let id = db.save_review(
            "test/repo",
            1,
            "github",
            "abc123",
            "Approve",
            "Looks good!",
            0,
        ).await.unwrap();

        assert_eq!(id, 1);

        let reviews = db.get_recent_reviews(10).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].repo, "test/repo");
        assert_eq!(reviews[0].verdict, "Approve");

        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total_reviews, 1);
        assert_eq!(stats.approved, 1);

        db.save_review(
            "test/repo",
            2,
            "github",
            "def456",
            "RequestChanges",
            "Needs work",
            3,
        ).await.unwrap();

        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total_reviews, 2);
        assert_eq!(stats.request_changes, 1);
        assert_eq!(stats.avg_inline_comments, 1.5);
    }
}
