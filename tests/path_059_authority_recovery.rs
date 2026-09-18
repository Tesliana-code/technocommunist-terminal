// =====================================================================
// PATH-059A — RECOVERED AUTHORITY STATE / HISTORY COMPLETENESS BOUNDARY
// =====================================================================
//
// FIND-036 relocates authenticity trust to an external authority record.
// This PATH asks whether a surviving recovered record also proves that no
// later authoritative transition existed and was lost.
//
// Ground truth is test-oracle state only. Recovery code cannot observe it.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Path059AuthorityRecord {
    epoch: u64,
    lineage: u64,
    transition_id: u64,
    parent_transition_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path059RecoveryVerdict {
    RecoveredLatest(Path059AuthorityRecord),
    Unresolved,
}

fn path_059_naive_recover_latest(
    recovered: Option<Path059AuthorityRecord>,
) -> Path059RecoveryVerdict {
    match recovered {
        Some(record) => Path059RecoveryVerdict::RecoveredLatest(record),
        None => Path059RecoveryVerdict::Unresolved,
    }
}

fn path_059_ground_truth_recovery_verdict(
    recovered: Option<Path059AuthorityRecord>,
    actual_latest: Option<Path059AuthorityRecord>,
) -> Path059RecoveryVerdict {
    match (recovered, actual_latest) {
        (Some(recovered), Some(actual)) if recovered == actual => {
            Path059RecoveryVerdict::RecoveredLatest(recovered)
        }
        _ => Path059RecoveryVerdict::Unresolved,
    }
}

#[test]
fn path_059_a_complete_recovery_control_accepts_actual_latest_record() {
    let epoch_three = Path059AuthorityRecord {
        epoch: 3,
        lineage: 7,
        transition_id: 30,
        parent_transition_id: Some(20),
    };

    assert_eq!(
        path_059_naive_recover_latest(Some(epoch_three)),
        Path059RecoveryVerdict::RecoveredLatest(epoch_three),
    );

    assert_eq!(
        path_059_ground_truth_recovery_verdict(
            Some(epoch_three),
            Some(epoch_three),
        ),
        Path059RecoveryVerdict::RecoveredLatest(epoch_three),
    );
}

#[test]
fn path_059_b_surviving_stale_record_is_naively_misclassified_as_latest() {
    let recovered_three = Path059AuthorityRecord {
        epoch: 3,
        lineage: 7,
        transition_id: 30,
        parent_transition_id: Some(20),
    };

    let actual_latest_four = Path059AuthorityRecord {
        epoch: 4,
        lineage: 7,
        transition_id: 40,
        parent_transition_id: Some(30),
    };

    assert_eq!(
        path_059_naive_recover_latest(Some(recovered_three)),
        Path059RecoveryVerdict::RecoveredLatest(recovered_three),
    );

    assert_eq!(
        path_059_ground_truth_recovery_verdict(
            Some(recovered_three),
            Some(actual_latest_four),
        ),
        Path059RecoveryVerdict::Unresolved,
    );
}

#[test]
fn path_059_c_complete_and_lossy_histories_are_observationally_identical() {
    let recovered_three = Path059AuthorityRecord {
        epoch: 3,
        lineage: 7,
        transition_id: 30,
        parent_transition_id: Some(20),
    };

    let lost_four = Path059AuthorityRecord {
        epoch: 4,
        lineage: 7,
        transition_id: 40,
        parent_transition_id: Some(30),
    };

    let world_complete_observable = Some(recovered_three);
    let world_lossy_observable = Some(recovered_three);

    assert_eq!(world_complete_observable, world_lossy_observable);

    let observed_complete =
        path_059_naive_recover_latest(world_complete_observable);
    let observed_lossy =
        path_059_naive_recover_latest(world_lossy_observable);

    assert_eq!(observed_complete, observed_lossy);

    let correct_complete =
        path_059_ground_truth_recovery_verdict(
            world_complete_observable,
            Some(recovered_three),
        );

    let correct_lossy =
        path_059_ground_truth_recovery_verdict(
            world_lossy_observable,
            Some(lost_four),
        );

    assert_ne!(correct_complete, correct_lossy);
    assert_eq!(observed_complete, correct_complete);
    assert_ne!(observed_lossy, correct_lossy);
}

#[test]
fn path_059_d_no_surviving_authority_record_stays_unresolved() {
    assert_eq!(
        path_059_naive_recover_latest(None),
        Path059RecoveryVerdict::Unresolved,
    );
}


// =====================================================================
// PATH-059B — REPLICA AGREEMENT / CORRELATED HISTORY LOSS
// =====================================================================
//
// Agreement between recovered copies is evidence that the copies agree.
// It does not by itself prove that their shared recovered history is complete.

fn path_059_recover_from_agreeing_replicas(
    a: Option<Path059AuthorityRecord>,
    b: Option<Path059AuthorityRecord>,
) -> Path059RecoveryVerdict {
    match (a, b) {
        (Some(a), Some(b)) if a == b => {
            Path059RecoveryVerdict::RecoveredLatest(a)
        }
        _ => Path059RecoveryVerdict::Unresolved,
    }
}

#[test]
fn path_059_e_agreeing_replicas_control_when_shared_record_is_actual_latest() {
    let epoch_three = Path059AuthorityRecord {
        epoch: 3,
        lineage: 7,
        transition_id: 30,
        parent_transition_id: Some(20),
    };

    assert_eq!(
        path_059_recover_from_agreeing_replicas(
            Some(epoch_three),
            Some(epoch_three),
        ),
        Path059RecoveryVerdict::RecoveredLatest(epoch_three),
    );
}

#[test]
fn path_059_f_agreeing_replicas_can_both_be_stale_after_correlated_loss() {
    let recovered_three = Path059AuthorityRecord {
        epoch: 3,
        lineage: 7,
        transition_id: 30,
        parent_transition_id: Some(20),
    };

    let actual_latest_four = Path059AuthorityRecord {
        epoch: 4,
        lineage: 7,
        transition_id: 40,
        parent_transition_id: Some(30),
    };

    let observed = path_059_recover_from_agreeing_replicas(
        Some(recovered_three),
        Some(recovered_three),
    );

    assert_eq!(
        observed,
        Path059RecoveryVerdict::RecoveredLatest(recovered_three),
    );

    assert_eq!(
        path_059_ground_truth_recovery_verdict(
            Some(recovered_three),
            Some(actual_latest_four),
        ),
        Path059RecoveryVerdict::Unresolved,
    );
}

#[test]
fn path_059_g_complete_and_correlated_loss_worlds_have_identical_replica_views() {
    let recovered_three = Path059AuthorityRecord {
        epoch: 3,
        lineage: 7,
        transition_id: 30,
        parent_transition_id: Some(20),
    };

    let world_complete =
        (Some(recovered_three), Some(recovered_three));
    let world_correlated_loss =
        (Some(recovered_three), Some(recovered_three));

    assert_eq!(world_complete, world_correlated_loss);

    assert_eq!(
        path_059_recover_from_agreeing_replicas(
            world_complete.0,
            world_complete.1,
        ),
        path_059_recover_from_agreeing_replicas(
            world_correlated_loss.0,
            world_correlated_loss.1,
        ),
    );
}



#[test]
fn path_059_h_same_replica_view_can_require_different_correct_recovery_verdicts() {
    let recovered_three = Path059AuthorityRecord {
        epoch: 3,
        lineage: 7,
        transition_id: 30,
        parent_transition_id: Some(20),
    };

    let lost_four = Path059AuthorityRecord {
        epoch: 4,
        lineage: 7,
        transition_id: 40,
        parent_transition_id: Some(30),
    };

    let world_complete_view =
        (Some(recovered_three), Some(recovered_three));
    let world_lossy_view =
        (Some(recovered_three), Some(recovered_three));

    assert_eq!(world_complete_view, world_lossy_view);

    let observed_complete = path_059_recover_from_agreeing_replicas(
        world_complete_view.0,
        world_complete_view.1,
    );

    let observed_lossy = path_059_recover_from_agreeing_replicas(
        world_lossy_view.0,
        world_lossy_view.1,
    );

    assert_eq!(observed_complete, observed_lossy);

    let correct_complete = path_059_ground_truth_recovery_verdict(
        Some(recovered_three),
        Some(recovered_three),
    );

    let correct_lossy = path_059_ground_truth_recovery_verdict(
        Some(recovered_three),
        Some(lost_four),
    );

    assert_ne!(correct_complete, correct_lossy);
    assert_eq!(observed_complete, correct_complete);
    assert_ne!(observed_lossy, correct_lossy);
}


// --- PATH-059 INSERTION POINT ---
