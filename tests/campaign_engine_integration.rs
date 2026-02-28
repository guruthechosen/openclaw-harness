use openclaw_harness::campaign::{
    CampaignConstraints, CampaignEngine, MissionAiPlanner, MissionDraft, MissionRule,
    UserBehaviourStats,
};
use rusqlite::Connection;

struct MockPlanner;

impl MissionAiPlanner for MockPlanner {
    fn propose(
        &self,
        _conn: &Connection,
        stats: &UserBehaviourStats,
        _history: &[openclaw_harness::campaign::BehaviourRecord],
        _constraints: &CampaignConstraints,
    ) -> anyhow::Result<MissionDraft> {
        let required_count = if stats.success_rate >= 0.7 { 6 } else { 3 };
        let diff = if stats.success_rate >= 0.7 { 0.72 } else { 0.45 };
        let expected_prob = if stats.success_rate >= 0.7 { 0.58 } else { 0.74 };
        let expected_hours = if stats.avg_duration_minutes <= 20.0 { 1.5 } else { 2.5 };

        Ok(MissionDraft {
            title: format!("Adaptive mission for {}", stats.user_id),
            description: "AI-generated mission based on behaviour analysis".to_string(),
            rule: MissionRule {
                mission_type: "count_event".to_string(),
                required_count,
                event_type: "checkout".to_string(),
                window_hours: 48,
            },
            difficulty_score: diff,
            expected_completion_probability: expected_prob,
            expected_hours,
            recommended_points: 180,
            analysis: format!(
                "Analysed {} events, success_rate={:.2}, avg_duration={:.1}",
                stats.total_events, stats.success_rate, stats.avg_duration_minutes
            ),
        })
    }
}

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE Behaviours (
            user_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            success INTEGER NOT NULL,
            duration_minutes INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .unwrap();

    for i in 0..20 {
        let success = if i < 15 { 1 } else { 0 };
        conn.execute(
            "INSERT INTO Behaviours (user_id, event_type, success, duration_minutes, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            rusqlite::params!["user-1", "checkout", success, 12],
        )
        .unwrap();
    }

    conn
}

#[test]
fn integration_generates_dynamic_mission_and_clamps_points() {
    let conn = setup_db();
    let engine = CampaignEngine::new(MockPlanner);
    let constraints = CampaignConstraints {
        max_points_per_mission: 100,
        min_completion_probability: 0.35,
        max_expected_hours: 3.0,
    };

    let mission = engine
        .generate_mission(&conn, "user-1", &constraints)
        .unwrap();

    assert_eq!(mission.user_id, "user-1");
    assert_eq!(mission.rule.event_type, "checkout");
    assert!(mission.rule.required_count >= 3);
    assert_eq!(mission.recommended_points, 180);
    assert_eq!(mission.final_points, 100);
    assert!(mission.clamped);
    assert!(mission.analysis.contains("success_rate"));
}

#[test]
fn integration_rejects_unrealistic_mission() {
    struct AggressivePlanner;
    impl MissionAiPlanner for AggressivePlanner {
        fn propose(
            &self,
            _conn: &Connection,
            _stats: &UserBehaviourStats,
            _history: &[openclaw_harness::campaign::BehaviourRecord],
            _constraints: &CampaignConstraints,
        ) -> anyhow::Result<MissionDraft> {
            Ok(MissionDraft {
                title: "Too hard".to_string(),
                description: "Should fail feasibility".to_string(),
                rule: MissionRule {
                    mission_type: "count_event".to_string(),
                    required_count: 20,
                    event_type: "purchase".to_string(),
                    window_hours: 24,
                },
                difficulty_score: 0.95,
                expected_completion_probability: 0.1,
                expected_hours: 6.0,
                recommended_points: 999,
                analysis: "Too hard for user".to_string(),
            })
        }
    }

    let conn = setup_db();
    let engine = CampaignEngine::new(AggressivePlanner);
    let constraints = CampaignConstraints {
        max_points_per_mission: 100,
        min_completion_probability: 0.35,
        max_expected_hours: 3.0,
    };

    let err = engine
        .generate_mission(&conn, "user-1", &constraints)
        .unwrap_err();

    assert!(err
        .to_string()
        .contains("expected completion probability too low"));
}
