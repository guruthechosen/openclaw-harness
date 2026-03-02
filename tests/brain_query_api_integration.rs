use axum::{http::StatusCode, Json};
use openclaw_harness::web::routes::{
    get_brain_graph_v2, query_brain_v2, search_brain_v2, BrainQueryRequest, BrainSearchRequest,
};
use serde_json::json;

fn write_ontology_fixture(base: &std::path::Path) {
    let v2 = base.join("ontology").join("v2");
    std::fs::create_dir_all(&v2).unwrap();

    let nodes = [
        json!({"id":"b1","kind":"Bottleneck","title":"risk-hit x3: npm run build"}),
        json!({"id":"p1","kind":"TaskPattern","title":"repeat x5: npm run build"}),
        json!({"id":"a1","kind":"AutomationOpportunity","title":"automate build flow"}),
        json!({"id":"s1","kind":"Skill","title":"openclaw exec mastery score=8"}),
    ];
    let nodes_jsonl = nodes
        .iter()
        .map(|n| serde_json::to_string(n).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(v2.join("nodes.jsonl"), nodes_jsonl).unwrap();

    let edges_jsonl =
        serde_json::to_string(&json!({"from":"p1","to":"a1","rel":"pattern_of"})).unwrap()
            + "\n";
    std::fs::write(v2.join("edges.jsonl"), edges_jsonl).unwrap();

    std::fs::write(
        v2.join("insights.json"),
        serde_json::to_string_pretty(&json!({"decisions_detected": 6})).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn api_brain_query_recommendations_returns_scored_priorities() {
    let tmp = tempfile::tempdir().unwrap();
    write_ontology_fixture(tmp.path());
    std::env::set_var("SAFEBOT_DATA_DIR", tmp.path());

    let Json(resp) = query_brain_v2(Json(BrainQueryRequest {
        query_type: "recommendations".to_string(),
        limit: Some(5),
    }))
    .await
    .unwrap();

    assert!(resp.ok);
    assert!(!resp.results.is_empty());
    let first = resp.results.first().unwrap()["score"].as_u64().unwrap();
    let last = resp.results.last().unwrap()["score"].as_u64().unwrap();
    assert!(first >= last);
}

#[tokio::test]
async fn api_brain_graph_and_search_work() {
    let tmp = tempfile::tempdir().unwrap();
    write_ontology_fixture(tmp.path());
    std::env::set_var("SAFEBOT_DATA_DIR", tmp.path());

    let Json(graph) = get_brain_graph_v2().await.unwrap();
    assert!(graph.ok);
    assert_eq!(graph.nodes.len(), 4);
    assert_eq!(graph.edges.len(), 1);

    let Json(search) = search_brain_v2(Json(BrainSearchRequest {
        keyword: "build".to_string(),
        kinds: Some(vec!["TaskPattern".to_string()]),
        limit: Some(10),
    }))
    .await
    .unwrap();

    assert!(search.ok);
    assert_eq!(search.results.len(), 1);
    assert_eq!(search.results[0]["kind"], "TaskPattern");
}

#[tokio::test]
async fn api_brain_search_empty_keyword_returns_bad_request() {
    let result = search_brain_v2(Json(BrainSearchRequest {
        keyword: "   ".to_string(),
        kinds: None,
        limit: None,
    }))
    .await;

    assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
}
