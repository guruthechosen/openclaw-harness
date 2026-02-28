use openclaw_harness::brain::{
    build_ontology_from_db, build_ontology_v2_from_db, persist_ontology, persist_ontology_v2,
};
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE actions (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            agent TEXT NOT NULL,
            action_type TEXT NOT NULL,
            content TEXT NOT NULL,
            target TEXT,
            session_id TEXT,
            metadata TEXT
        );

        CREATE TABLE analysis_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action_id TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            matched_rules TEXT NOT NULL,
            risk_level TEXT NOT NULL,
            recommendation TEXT NOT NULL,
            explanation TEXT NOT NULL
        );
        "#,
    )
    .unwrap();

    conn.execute(
        "INSERT INTO actions (id, timestamp, agent, action_type, content, target, session_id, metadata)
         VALUES ('a1', datetime('now'), 'openclaw', 'Exec', 'npm run build', '/Volumes/formac/proj/safebot/ui/src/main.tsx', 's1', NULL)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO analysis_results (action_id, timestamp, matched_rules, risk_level, recommendation, explanation)
         VALUES ('a1', datetime('now'), 'dangerous_rm', 'Warning', 'Alert', 'test')",
        [],
    )
    .unwrap();

    conn
}

#[test]
fn integration_builds_and_persists_ontology() {
    let conn = setup_db();
    let (nodes, edges) = build_ontology_from_db(&conn).unwrap();

    assert!(nodes.iter().any(|n| n.kind == "User"));
    assert!(nodes.iter().any(|n| n.kind == "Session"));
    assert!(nodes.iter().any(|n| n.kind == "Tool"));
    assert!(nodes.iter().any(|n| n.kind == "Command"));
    assert!(nodes.iter().any(|n| n.kind == "File"));
    assert!(nodes.iter().any(|n| n.kind == "Project"));
    assert!(nodes.iter().any(|n| n.kind == "Incident"));

    assert!(edges.iter().any(|e| e.rel == "used_tool"));
    assert!(edges.iter().any(|e| e.rel == "ran_command"));
    assert!(edges.iter().any(|e| e.rel == "worked_on"));

    let tmp = tempfile::tempdir().unwrap();
    let summary = persist_ontology(tmp.path(), &nodes, &edges).unwrap();
    assert!(summary.nodes > 0);
    assert!(summary.edges > 0);
    assert!(tmp.path().join("ontology/v1/nodes.jsonl").exists());
    assert!(tmp.path().join("ontology/v1/edges.jsonl").exists());
    assert!(tmp.path().join("ontology/v1/summary.json").exists());
}

#[test]
fn integration_builds_and_persists_ontology_v2_semantics() {
    let conn = setup_db();

    // add repeated and decision-like actions for semantic layer
    conn.execute(
        "INSERT INTO actions (id, timestamp, agent, action_type, content, target, session_id, metadata)
         VALUES ('a2', datetime('now'), 'openclaw', 'Exec', 'fix build issue', '/Volumes/formac/proj/safebot/src/lib.rs', 's1', NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO actions (id, timestamp, agent, action_type, content, target, session_id, metadata)
         VALUES ('a3', datetime('now'), 'openclaw', 'Exec', 'npm run build', '/Volumes/formac/proj/safebot/ui/src/App.tsx', 's1', NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO actions (id, timestamp, agent, action_type, content, target, session_id, metadata)
         VALUES ('a4', datetime('now'), 'openclaw', 'Exec', 'npm run build', '/Volumes/formac/proj/safebot/ui/src/main.tsx', 's1', NULL)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO analysis_results (action_id, timestamp, matched_rules, risk_level, recommendation, explanation)
         VALUES ('a3', datetime('now'), 'rule', 'Warning', 'Alert', 'test')",
        [],
    )
    .unwrap();

    let (nodes, edges, insights) = build_ontology_v2_from_db(&conn).unwrap();
    assert!(nodes.iter().any(|n| n.kind == "TaskPattern"));
    assert!(nodes.iter().any(|n| n.kind == "Decision"));
    assert!(nodes.iter().any(|n| n.kind == "Skill"));
    assert!(insights.repeated_patterns >= 1);

    let tmp = tempfile::tempdir().unwrap();
    let summary = persist_ontology_v2(tmp.path(), &nodes, &edges, &insights).unwrap();
    assert!(summary.nodes > 0);
    assert!(summary.edges > 0);
    assert!(tmp.path().join("ontology/v2/nodes.jsonl").exists());
    assert!(tmp.path().join("ontology/v2/edges.jsonl").exists());
    assert!(tmp.path().join("ontology/v2/insights.json").exists());
    assert!(tmp.path().join("ontology/v2/summary.json").exists());
}
