use anyhow::Context;
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviourRecord {
    pub user_id: String,
    pub event_type: String,
    pub success: bool,
    pub duration_minutes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviourStats {
    pub user_id: String,
    pub total_events: u64,
    pub success_count: u64,
    pub success_rate: f32,
    pub avg_duration_minutes: f32,
    pub unique_event_types: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignConstraints {
    pub max_points_per_mission: u32,
    pub min_completion_probability: f32,
    pub max_expected_hours: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRule {
    pub mission_type: String,
    pub required_count: u32,
    pub event_type: String,
    pub window_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionDraft {
    pub title: String,
    pub description: String,
    pub rule: MissionRule,
    pub difficulty_score: f32,
    pub expected_completion_probability: f32,
    pub expected_hours: f32,
    pub recommended_points: u32,
    pub analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionPlan {
    pub user_id: String,
    pub title: String,
    pub description: String,
    pub rule: MissionRule,
    pub difficulty_score: f32,
    pub expected_completion_probability: f32,
    pub expected_hours: f32,
    pub recommended_points: u32,
    pub final_points: u32,
    pub analysis: String,
    pub clamped: bool,
}

pub trait MissionAiPlanner {
    fn propose(
        &self,
        conn: &Connection,
        stats: &UserBehaviourStats,
        history: &[BehaviourRecord],
        constraints: &CampaignConstraints,
    ) -> anyhow::Result<MissionDraft>;
}

/// Production LLM planner.
///
/// Env vars:
/// - `SAFEBOT_LLM_API_KEY` (required)
/// - `SAFEBOT_LLM_BASE_URL` (optional, default https://api.openai.com/v1)
/// - `SAFEBOT_LLM_MODEL` (optional, default gpt-4o-mini)
pub struct LlmAiPlanner {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    max_attempts: u32,
}

impl LlmAiPlanner {
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("SAFEBOT_LLM_API_KEY")
            .context("missing SAFEBOT_LLM_API_KEY for LLM planner")?;
        let base_url = std::env::var("SAFEBOT_LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model =
            std::env::var("SAFEBOT_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
            api_key,
            base_url,
            model,
            max_attempts: 3,
        })
    }

    fn build_prompt(
        &self,
        stats: &UserBehaviourStats,
        history: &[BehaviourRecord],
        constraints: &CampaignConstraints,
    ) -> String {
        let history_json = serde_json::to_string(history).unwrap_or_else(|_| "[]".to_string());
        let stats_json = serde_json::to_string(stats).unwrap_or_else(|_| "{}".to_string());
        let constraints_json =
            serde_json::to_string(constraints).unwrap_or_else(|_| "{}".to_string());

        format!(
            "You are an adaptive campaign planner. Analyze user behavior deeply and produce ONLY JSON.\n\
             Constraints must be respected, especially max_points_per_mission.\n\
             Return schema exactly:\n\
             {{\n\
               \"title\": string,\n\
               \"description\": string,\n\
               \"rule\": {{\"mission_type\": string, \"required_count\": number, \"event_type\": string, \"window_hours\": number}},\n\
               \"difficulty_score\": number (0..1),\n\
               \"expected_completion_probability\": number (0..1),\n\
               \"expected_hours\": number,\n\
               \"recommended_points\": number,\n\
               \"analysis\": string\n\
             }}\n\
             No markdown. No extra keys.\n\
             User stats: {stats_json}\n\
             Campaign constraints: {constraints_json}\n\
             Recent behavior records (up to 200): {history_json}"
        )
    }

    fn repair_prompt(&self, broken: &str, err: &str) -> String {
        format!(
            "Repair this output into strict valid JSON matching required mission schema only.\n\
             Keep the intent and values, but fix structure/types/ranges.\n\
             Validation error: {err}\n\
             Broken output:\n{broken}"
        )
    }

    fn call_chat(&self, prompt: &str) -> anyhow::Result<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0.2,
            "response_format": {"type":"json_object"},
            "messages": [
                {"role":"system","content":"You generate strict JSON for adaptive campaigns."},
                {"role":"user","content":prompt}
            ]
        });

        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()?
            .error_for_status()?;

        let v: serde_json::Value = resp.json()?;
        let content = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("LLM response missing content"))?;
        Ok(content.to_string())
    }
}

impl MissionAiPlanner for LlmAiPlanner {
    fn propose(
        &self,
        conn: &Connection,
        stats: &UserBehaviourStats,
        history: &[BehaviourRecord],
        constraints: &CampaignConstraints,
    ) -> anyhow::Result<MissionDraft> {
        ensure_audit_table(conn)?;

        let initial_prompt = self.build_prompt(stats, history, constraints);
        let mut prompt = initial_prompt.clone();
        let mut last_err = String::new();

        for attempt in 1..=self.max_attempts {
            let raw = self
                .call_chat(&prompt)
                .with_context(|| format!("LLM call failed at attempt {attempt}"))?;
            match validate_mission_draft_json(&raw, constraints) {
                Ok(draft) => {
                    write_audit_log(
                        conn,
                        &stats.user_id,
                        attempt as i64,
                        "ok",
                        &prompt,
                        &raw,
                        None,
                    )?;
                    return Ok(draft);
                }
                Err(e) => {
                    last_err = e.to_string();
                    write_audit_log(
                        conn,
                        &stats.user_id,
                        attempt as i64,
                        "repair_needed",
                        &prompt,
                        &raw,
                        Some(&last_err),
                    )?;
                    prompt = self.repair_prompt(&raw, &last_err);
                }
            }
        }

        anyhow::bail!("LLM planner failed after retries: {last_err}")
    }
}

pub struct CampaignEngine<P: MissionAiPlanner> {
    planner: P,
}

impl<P: MissionAiPlanner> CampaignEngine<P> {
    pub fn new(planner: P) -> Self {
        Self { planner }
    }

    pub fn generate_mission(
        &self,
        conn: &Connection,
        user_id: &str,
        constraints: &CampaignConstraints,
    ) -> anyhow::Result<MissionPlan> {
        let history = load_behaviours(conn, user_id)?;
        let stats = compute_stats(user_id, &history);

        let draft = self.planner.propose(conn, &stats, &history, constraints)?;

        if draft.expected_completion_probability < constraints.min_completion_probability {
            anyhow::bail!("mission rejected: expected completion probability too low")
        }
        if draft.expected_hours > constraints.max_expected_hours {
            anyhow::bail!("mission rejected: expected hours exceeds user capacity")
        }

        let final_points =
            clamp_points(draft.recommended_points, constraints.max_points_per_mission);
        let clamped = final_points != draft.recommended_points;

        Ok(MissionPlan {
            user_id: user_id.to_string(),
            title: draft.title,
            description: draft.description,
            rule: draft.rule,
            difficulty_score: draft.difficulty_score,
            expected_completion_probability: draft.expected_completion_probability,
            expected_hours: draft.expected_hours,
            recommended_points: draft.recommended_points,
            final_points,
            analysis: draft.analysis,
            clamped,
        })
    }
}

pub fn clamp_points(recommended: u32, max_cap: u32) -> u32 {
    recommended.min(max_cap)
}

fn ensure_audit_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mission_generation_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            attempt INTEGER NOT NULL,
            status TEXT NOT NULL,
            prompt TEXT NOT NULL,
            raw_output TEXT NOT NULL,
            error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;
    Ok(())
}

fn write_audit_log(
    conn: &Connection,
    user_id: &str,
    attempt: i64,
    status: &str,
    prompt: &str,
    raw_output: &str,
    error: Option<&str>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO mission_generation_audit (user_id, attempt, status, prompt, raw_output, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![user_id, attempt, status, prompt, raw_output, error],
    )?;
    Ok(())
}

fn validate_mission_draft_json(
    raw: &str,
    constraints: &CampaignConstraints,
) -> anyhow::Result<MissionDraft> {
    let mut draft: MissionDraft =
        serde_json::from_str(raw).with_context(|| "mission JSON parse failed")?;

    if !(0.0..=1.0).contains(&draft.difficulty_score) {
        anyhow::bail!("difficulty_score out of range")
    }
    if !(0.0..=1.0).contains(&draft.expected_completion_probability) {
        anyhow::bail!("expected_completion_probability out of range")
    }
    if draft.rule.required_count == 0 {
        anyhow::bail!("required_count must be > 0")
    }
    if draft.rule.window_hours == 0 {
        anyhow::bail!("window_hours must be > 0")
    }

    // Hard guardrail: never allow recommendation above human cap.
    if draft.recommended_points > constraints.max_points_per_mission {
        draft.recommended_points = constraints.max_points_per_mission;
    }

    Ok(draft)
}

fn load_behaviours(conn: &Connection, user_id: &str) -> anyhow::Result<Vec<BehaviourRecord>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, event_type, success, duration_minutes, created_at
         FROM Behaviours
         WHERE user_id = ?1
         ORDER BY created_at DESC
         LIMIT 200",
    )?;

    let rows = stmt.query_map(params![user_id], |row| {
        Ok(BehaviourRecord {
            user_id: row.get(0)?,
            event_type: row.get(1)?,
            success: row.get::<_, i64>(2)? == 1,
            duration_minutes: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;

    Ok(rows.filter_map(Result::ok).collect())
}

fn compute_stats(user_id: &str, history: &[BehaviourRecord]) -> UserBehaviourStats {
    if history.is_empty() {
        return UserBehaviourStats {
            user_id: user_id.to_string(),
            total_events: 0,
            success_count: 0,
            success_rate: 0.0,
            avg_duration_minutes: 0.0,
            unique_event_types: 0,
        };
    }

    let total_events = history.len() as u64;
    let success_count = history.iter().filter(|h| h.success).count() as u64;
    let success_rate = success_count as f32 / total_events as f32;
    let avg_duration_minutes = history
        .iter()
        .map(|h| h.duration_minutes as f32)
        .sum::<f32>()
        / total_events as f32;

    let mut types = std::collections::HashSet::new();
    for h in history {
        types.insert(h.event_type.clone());
    }

    UserBehaviourStats {
        user_id: user_id.to_string(),
        total_events,
        success_count,
        success_rate,
        avg_duration_minutes,
        unique_event_types: types.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_points() {
        assert_eq!(clamp_points(120, 100), 100);
        assert_eq!(clamp_points(80, 100), 80);
    }

    #[test]
    fn test_validate_mission_json() {
        let constraints = CampaignConstraints {
            max_points_per_mission: 100,
            min_completion_probability: 0.35,
            max_expected_hours: 3.0,
        };

        let raw = r#"{
          "title":"A",
          "description":"B",
          "rule":{"mission_type":"count_event","required_count":3,"event_type":"checkout","window_hours":48},
          "difficulty_score":0.5,
          "expected_completion_probability":0.6,
          "expected_hours":2.0,
          "recommended_points":180,
          "analysis":"x"
        }"#;

        let draft = validate_mission_draft_json(raw, &constraints).unwrap();
        assert_eq!(draft.recommended_points, 100);
    }
}
