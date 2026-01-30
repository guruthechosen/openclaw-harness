//! SQLite database for storing action logs and analysis results

use super::{AgentAction, AnalysisResult, AgentType, ActionType, RiskLevel};
use rusqlite::{Connection, params};
use std::path::Path;
use tracing::info;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Initialize database schema
    fn initialize(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS actions (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                agent TEXT NOT NULL,
                action_type TEXT NOT NULL,
                content TEXT NOT NULL,
                target TEXT,
                session_id TEXT,
                metadata TEXT
            );

            CREATE TABLE IF NOT EXISTS analysis_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                matched_rules TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                recommendation TEXT NOT NULL,
                explanation TEXT NOT NULL,
                FOREIGN KEY (action_id) REFERENCES actions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_actions_timestamp ON actions(timestamp);
            CREATE INDEX IF NOT EXISTS idx_actions_agent ON actions(agent);
            CREATE INDEX IF NOT EXISTS idx_analysis_risk ON analysis_results(risk_level);
            "#,
        )?;

        info!("Database initialized");
        Ok(())
    }

    /// Store an action
    pub fn store_action(&self, action: &AgentAction) -> anyhow::Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO actions (id, timestamp, agent, action_type, content, target, session_id, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                action.id,
                action.timestamp.to_rfc3339(),
                action.agent.to_string(),
                format!("{:?}", action.action_type),
                action.content,
                action.target,
                action.session_id,
                action.metadata.as_ref().map(|m| m.to_string()),
            ],
        )?;

        Ok(())
    }

    /// Store an analysis result
    pub fn store_analysis(&self, result: &AnalysisResult) -> anyhow::Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO analysis_results (action_id, timestamp, matched_rules, risk_level, recommendation, explanation)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                result.action.id,
                chrono::Utc::now().to_rfc3339(),
                result.matched_rules.join(","),
                format!("{:?}", result.risk_level),
                format!("{:?}", result.recommendation),
                result.explanation,
            ],
        )?;

        Ok(())
    }

    /// Get recent actions
    pub fn get_recent_actions(&self, limit: usize) -> anyhow::Result<Vec<AgentAction>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, timestamp, agent, action_type, content, target, session_id, metadata
            FROM actions
            ORDER BY timestamp DESC
            LIMIT ?1
            "#,
        )?;

        let actions = stmt
            .query_map([limit], |row| {
                Ok(AgentAction {
                    id: row.get(0)?,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .unwrap_or_default()
                        .with_timezone(&chrono::Utc),
                    agent: parse_agent_type(&row.get::<_, String>(2)?),
                    action_type: parse_action_type(&row.get::<_, String>(3)?),
                    content: row.get(4)?,
                    target: row.get(5)?,
                    session_id: row.get(6)?,
                    metadata: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(actions)
    }

    /// Get statistics
    pub fn get_stats(&self) -> anyhow::Result<Stats> {
        let total_actions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM actions",
            [],
            |row| row.get(0),
        )?;

        let blocked: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM analysis_results WHERE recommendation = 'CriticalAlert'",
            [],
            |row| row.get(0),
        )?;

        let warnings: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM analysis_results WHERE risk_level = 'Warning'",
            [],
            |row| row.get(0),
        )?;

        Ok(Stats {
            total_actions,
            blocked,
            warnings,
        })
    }

    /// Clean up old entries
    pub fn cleanup(&self, retention_days: u32) -> anyhow::Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
        
        let deleted = self.conn.execute(
            "DELETE FROM actions WHERE timestamp < ?1",
            [cutoff.to_rfc3339()],
        )?;

        info!("Cleaned up {} old action records", deleted);
        Ok(deleted)
    }
}

fn parse_agent_type(s: &str) -> AgentType {
    match s.to_lowercase().as_str() {
        "moltbot" => AgentType::Moltbot,
        "claude_code" => AgentType::ClaudeCode,
        "cursor" => AgentType::Cursor,
        "ralph" => AgentType::Ralph,
        _ => AgentType::Unknown,
    }
}

fn parse_action_type(s: &str) -> ActionType {
    match s {
        "Exec" => ActionType::Exec,
        "FileRead" => ActionType::FileRead,
        "FileWrite" => ActionType::FileWrite,
        "FileDelete" => ActionType::FileDelete,
        "HttpRequest" => ActionType::HttpRequest,
        "BrowserAction" => ActionType::BrowserAction,
        "MessageSend" => ActionType::MessageSend,
        "GitOperation" => ActionType::GitOperation,
        _ => ActionType::Unknown,
    }
}

#[derive(Debug)]
pub struct Stats {
    pub total_actions: i64,
    pub blocked: i64,
    pub warnings: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{AgentType, ActionType};

    #[test]
    fn test_database_operations() {
        let db = Database::open_in_memory().unwrap();

        let action = AgentAction {
            id: "test-1".to_string(),
            timestamp: chrono::Utc::now(),
            agent: AgentType::Moltbot,
            action_type: ActionType::Exec,
            content: "ls -la".to_string(),
            target: None,
            session_id: None,
            metadata: None,
        };

        db.store_action(&action).unwrap();

        let actions = db.get_recent_actions(10).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "test-1");
    }
}
