use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyNode {
    pub id: String,
    pub kind: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyEdge {
    pub from: String,
    pub to: String,
    pub rel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyBuildSummary {
    pub nodes: usize,
    pub edges: usize,
}

#[derive(Debug, Clone)]
struct ActionRow {
    id: String,
    agent: String,
    action_type: String,
    content: String,
    target: Option<String>,
    session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainInsights {
    pub repeated_patterns: usize,
    pub decisions_detected: usize,
    pub bottlenecks_detected: usize,
    pub skills_inferred: usize,
}

pub fn build_ontology_from_db(conn: &Connection) -> anyhow::Result<(Vec<OntologyNode>, Vec<OntologyEdge>)> {
    let actions = load_actions(conn)?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_seen = HashSet::new();
    let mut edge_seen = HashSet::new();

    // track mapping to link incidents
    let mut action_to_session: HashMap<String, String> = HashMap::new();
    let mut action_to_command: HashMap<String, String> = HashMap::new();

    for a in actions {
        let user_id = format!("user:{}", a.agent);
        push_node(
            &mut nodes,
            &mut node_seen,
            OntologyNode {
                id: user_id.clone(),
                kind: "User".to_string(),
                title: a.agent.clone(),
            },
        );

        let session_val = a.session_id.clone().unwrap_or_else(|| "unknown".to_string());
        let session = format!("session:{}", session_val);
        push_node(
            &mut nodes,
            &mut node_seen,
            OntologyNode {
                id: session.clone(),
                kind: "Session".to_string(),
                title: session_val,
            },
        );
        push_edge(
            &mut edges,
            &mut edge_seen,
            OntologyEdge {
                from: user_id.clone(),
                to: session.clone(),
                rel: "did".to_string(),
            },
        );

        let tool = format!("tool:{}", a.action_type.to_lowercase());
        push_node(
            &mut nodes,
            &mut node_seen,
            OntologyNode {
                id: tool.clone(),
                kind: "Tool".to_string(),
                title: a.action_type.clone(),
            },
        );
        push_edge(
            &mut edges,
            &mut edge_seen,
            OntologyEdge {
                from: session.clone(),
                to: tool.clone(),
                rel: "used_tool".to_string(),
            },
        );

        action_to_session.insert(a.id.clone(), session.clone());

        if a.action_type.eq_ignore_ascii_case("Exec") {
            let command_id = format!("command:{}", hash_short(&a.content));
            push_node(
                &mut nodes,
                &mut node_seen,
                OntologyNode {
                    id: command_id.clone(),
                    kind: "Command".to_string(),
                    title: a.content.clone(),
                },
            );
            push_edge(
                &mut edges,
                &mut edge_seen,
                OntologyEdge {
                    from: session.clone(),
                    to: command_id.clone(),
                    rel: "ran_command".to_string(),
                },
            );
            action_to_command.insert(a.id.clone(), command_id);
        }

        if let Some(t) = a.target {
            if t.starts_with('/') {
                let file_id = format!("file:{}", t);
                push_node(
                    &mut nodes,
                    &mut node_seen,
                    OntologyNode {
                        id: file_id.clone(),
                        kind: "File".to_string(),
                        title: t.clone(),
                    },
                );
                push_edge(
                    &mut edges,
                    &mut edge_seen,
                    OntologyEdge {
                        from: session.clone(),
                        to: file_id,
                        rel: "touched_file".to_string(),
                    },
                );

                let project = project_from_path(&t);
                let proj_id = format!("project:{}", project);
                push_node(
                    &mut nodes,
                    &mut node_seen,
                    OntologyNode {
                        id: proj_id.clone(),
                        kind: "Project".to_string(),
                        title: project,
                    },
                );
                push_edge(
                    &mut edges,
                    &mut edge_seen,
                    OntologyEdge {
                        from: session,
                        to: proj_id,
                        rel: "worked_on".to_string(),
                    },
                );
            }
        }
    }

    // incidents + links
    let mut stmt2 = conn.prepare(
        "SELECT action_id, risk_level, matched_rules FROM analysis_results WHERE risk_level IN ('Warning','Critical') ORDER BY id DESC LIMIT 2000",
    )?;
    let rows2 = stmt2.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;

    for row in rows2 {
        let (action_id, risk, rules) = row?;
        let incident_id = format!("incident:{}:{}", risk.to_lowercase(), action_id);
        push_node(
            &mut nodes,
            &mut node_seen,
            OntologyNode {
                id: incident_id.clone(),
                kind: "Incident".to_string(),
                title: format!("{}:{}", risk, rules),
            },
        );

        if let Some(sess) = action_to_session.get(&action_id) {
            push_edge(
                &mut edges,
                &mut edge_seen,
                OntologyEdge {
                    from: sess.clone(),
                    to: incident_id.clone(),
                    rel: "triggered_incident".to_string(),
                },
            );
        }
        if let Some(cmd) = action_to_command.get(&action_id) {
            push_edge(
                &mut edges,
                &mut edge_seen,
                OntologyEdge {
                    from: incident_id,
                    to: cmd.clone(),
                    rel: "incident_on_command".to_string(),
                },
            );
        }
    }

    Ok((nodes, edges))
}

pub fn build_ontology_v2_from_db(
    conn: &Connection,
) -> anyhow::Result<(Vec<OntologyNode>, Vec<OntologyEdge>, BrainInsights)> {
    let (mut nodes, mut edges) = build_ontology_from_db(conn)?;
    let mut node_seen: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let mut edge_seen: HashSet<String> = edges
        .iter()
        .map(|e| format!("{}|{}|{}", e.from, e.to, e.rel))
        .collect();

    let actions = load_actions(conn)?;

    // 1) TaskPattern from repeated commands
    let mut command_counts: HashMap<String, u32> = HashMap::new();
    for a in &actions {
        if a.action_type.eq_ignore_ascii_case("Exec") {
            *command_counts.entry(a.content.clone()).or_default() += 1;
        }
    }

    let mut repeated_patterns = 0usize;
    for (cmd, count) in command_counts.iter().filter(|(_, c)| **c >= 3) {
        repeated_patterns += 1;
        let pattern_id = format!("pattern:{}", hash_short(cmd));
        let command_id = format!("command:{}", hash_short(cmd));
        push_node(
            &mut nodes,
            &mut node_seen,
            OntologyNode {
                id: pattern_id.clone(),
                kind: "TaskPattern".to_string(),
                title: format!("repeat x{}: {}", count, cmd),
            },
        );
        push_edge(
            &mut edges,
            &mut edge_seen,
            OntologyEdge {
                from: pattern_id,
                to: command_id,
                rel: "pattern_of".to_string(),
            },
        );
    }

    // 2) Decisions from intent-like commands
    let decision_keywords = ["fix", "refactor", "implement", "optimize", "deploy", "rollback"];
    let mut decisions_detected = 0usize;
    for a in actions.iter().filter(|a| a.action_type.eq_ignore_ascii_case("Exec")) {
        let lower = a.content.to_lowercase();
        if decision_keywords.iter().any(|k| lower.contains(k)) {
            decisions_detected += 1;
            let decision_id = format!("decision:{}", hash_short(&format!("{}:{}", a.id, a.content)));
            let cmd_id = format!("command:{}", hash_short(&a.content));
            push_node(
                &mut nodes,
                &mut node_seen,
                OntologyNode {
                    id: decision_id.clone(),
                    kind: "Decision".to_string(),
                    title: a.content.clone(),
                },
            );
            push_edge(
                &mut edges,
                &mut edge_seen,
                OntologyEdge {
                    from: decision_id,
                    to: cmd_id,
                    rel: "derived_from".to_string(),
                },
            );
        }
    }

    // 3) Bottlenecks from incident-heavy commands
    let mut stmt = conn.prepare(
        "SELECT a.content, COUNT(*) as c
         FROM analysis_results r
         JOIN actions a ON a.id = r.action_id
         WHERE r.risk_level IN ('Warning','Critical')
         GROUP BY a.content
         HAVING c >= 2
         ORDER BY c DESC
         LIMIT 20",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u32)))?;

    let mut bottlenecks_detected = 0usize;
    for row in rows {
        let (cmd, count) = row?;
        bottlenecks_detected += 1;
        let bottleneck_id = format!("bottleneck:{}", hash_short(&cmd));
        let cmd_id = format!("command:{}", hash_short(&cmd));
        push_node(
            &mut nodes,
            &mut node_seen,
            OntologyNode {
                id: bottleneck_id.clone(),
                kind: "Bottleneck".to_string(),
                title: format!("risk-hit x{}: {}", count, cmd),
            },
        );
        push_edge(
            &mut edges,
            &mut edge_seen,
            OntologyEdge {
                from: bottleneck_id,
                to: cmd_id,
                rel: "caused_by".to_string(),
            },
        );
    }

    // 4) Skill inference from tool usage minus incidents (per user)
    let mut usage_by_user_tool: HashMap<(String, String), u32> = HashMap::new();
    for a in &actions {
        let key = (a.agent.clone(), a.action_type.to_lowercase());
        *usage_by_user_tool.entry(key).or_default() += 1;
    }

    let mut risk_by_user_tool: HashMap<(String, String), u32> = HashMap::new();
    let mut stmt_risk = conn.prepare(
        "SELECT a.agent, LOWER(a.action_type), COUNT(*)
         FROM analysis_results r
         JOIN actions a ON a.id = r.action_id
         WHERE r.risk_level IN (Warning,Critical)
         GROUP BY a.agent, LOWER(a.action_type)",
    )?;
    let risk_rows = stmt_risk.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)? as u32,
        ))
    })?;
    for row in risk_rows {
        let (agent, tool, c) = row?;
        risk_by_user_tool.insert((agent, tool), c);
    }

    let mut skills_inferred = 0usize;
    for ((agent, tool), cnt) in usage_by_user_tool {
        let risk = risk_by_user_tool
            .get(&(agent.clone(), tool.clone()))
            .copied()
            .unwrap_or(0);
        let score = (cnt as i32 - risk as i32).max(0);
        if score > 0 {
            skills_inferred += 1;
            let skill_id = format!("skill:{}:{}", agent, tool);
            push_node(
                &mut nodes,
                &mut node_seen,
                OntologyNode {
                    id: skill_id.clone(),
                    kind: "Skill".to_string(),
                    title: format!("{} {} mastery score={}", agent, tool, score),
                },
            );
            push_edge(
                &mut edges,
                &mut edge_seen,
                OntologyEdge {
                    from: format!("user:{}", agent),
                    to: skill_id,
                    rel: "has_skill".to_string(),
                },
            );
        }
    }

    let insights = BrainInsights {
        repeated_patterns,
        decisions_detected,
        bottlenecks_detected,
        skills_inferred,
    };

    Ok((nodes, edges, insights))
}

pub fn persist_ontology(
    base_dir: &Path,
    nodes: &[OntologyNode],
    edges: &[OntologyEdge],
) -> anyhow::Result<OntologyBuildSummary> {
    let dir = base_dir.join("ontology").join("v1");
    fs::create_dir_all(&dir)?;

    let nodes_jsonl = nodes
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n")
        + "\n";
    let edges_jsonl = edges
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n")
        + "\n";

    fs::write(dir.join("nodes.jsonl"), nodes_jsonl)?;
    fs::write(dir.join("edges.jsonl"), edges_jsonl)?;

    let summary = OntologyBuildSummary {
        nodes: nodes.len(),
        edges: edges.len(),
    };
    fs::write(dir.join("summary.json"), serde_json::to_string_pretty(&summary)?)?;
    Ok(summary)
}

pub fn persist_ontology_v2(
    base_dir: &Path,
    nodes: &[OntologyNode],
    edges: &[OntologyEdge],
    insights: &BrainInsights,
) -> anyhow::Result<OntologyBuildSummary> {
    let dir = base_dir.join("ontology").join("v2");
    fs::create_dir_all(&dir)?;

    let nodes_jsonl = nodes
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n")
        + "\n";
    let edges_jsonl = edges
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n")
        + "\n";

    fs::write(dir.join("nodes.jsonl"), nodes_jsonl)?;
    fs::write(dir.join("edges.jsonl"), edges_jsonl)?;
    fs::write(dir.join("insights.json"), serde_json::to_string_pretty(insights)?)?;

    let summary = OntologyBuildSummary {
        nodes: nodes.len(),
        edges: edges.len(),
    };
    fs::write(dir.join("summary.json"), serde_json::to_string_pretty(&summary)?)?;
    Ok(summary)
}

fn load_actions(conn: &Connection) -> anyhow::Result<Vec<ActionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, agent, action_type, content, target, session_id
         FROM actions
         ORDER BY timestamp DESC
         LIMIT 5000",
    )?;

    let rows = stmt.query_map([], |r| {
        Ok(ActionRow {
            id: r.get(0)?,
            agent: r.get(1)?,
            action_type: r.get(2)?,
            content: r.get(3)?,
            target: r.get(4)?,
            session_id: r.get(5)?,
        })
    })?;

    Ok(rows.filter_map(Result::ok).collect())
}

fn push_node(nodes: &mut Vec<OntologyNode>, seen: &mut HashSet<String>, node: OntologyNode) {
    if seen.insert(node.id.clone()) {
        nodes.push(node);
    }
}

fn push_edge(edges: &mut Vec<OntologyEdge>, seen: &mut HashSet<String>, edge: OntologyEdge) {
    let key = format!("{}|{}|{}", edge.from, edge.to, edge.rel);
    if seen.insert(key) {
        edges.push(edge);
    }
}

fn project_from_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 4 {
        format!("/{}/{}/{}/{}", parts[0], parts[1], parts[2], parts[3])
    } else {
        path.to_string()
    }
}

fn hash_short(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_from_path() {
        assert_eq!(
            project_from_path("/Volumes/formac/proj/safebot/src/main.rs"),
            "/Volumes/formac/proj/safebot"
        );
    }
}
