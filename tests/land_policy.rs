use abacus::land::{
    Candidate, CompositionResult, DecisionInput, EnqueueOutcome, LandDecision, LocalLeg, decide,
    enumerate_candidates, parse_enqueue_result,
};

#[test]
fn closed_lane_pr_flows_from_candidate_discovery_to_enqueue_admission() {
    let open_prs = r#"[{"headRefName":"lane/ab-a"}]"#;
    let closed_beads = r#"{"issues":[{"id":"ab-a","status":"closed"}],"total":1}"#;

    let candidates = enumerate_candidates(open_prs, closed_beads).unwrap();
    assert_eq!(
        candidates,
        [Candidate {
            bead_id: "ab-a".into(),
            branch: "lane/ab-a".into(),
        }]
    );

    let decision = decide(DecisionInput::Admission {
        composition: CompositionResult::Clean,
        local_leg: LocalLeg::Pass,
        admitted_head_sha: "abc123".into(),
    });
    assert_eq!(
        decision,
        LandDecision::Enqueue {
            admitted_head_sha: "abc123".into(),
        }
    );

    let enqueue = parse_enqueue_result((
        0,
        "✓ Pull request owner/repo#1 will be added to the merge queue for trunk when ready".into(),
        String::new(),
    ));
    assert_eq!(enqueue.unwrap(), EnqueueOutcome::Admitted);
}
