// =====================================================================
// PATH-060A — BOUNDED COMPLETENESS WITNESS / CURRENTNESS BOUNDARY
// =====================================================================
//
// FIND-037 showed that surviving records and replica agreement do not prove
// authority-history completeness.
//
// PATH-060A grants a stronger test-local witness:
//
//     history complete through epoch N
//
// The witness is assumed truthful about that bounded cut.
// This experiment asks whether bounded completeness also proves that N is
// still the current latest authoritative state.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Path060AuthorityRecord {
    epoch: u64,
    transition_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Path060CompletenessWitness {
    complete_through_epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path060LatestVerdict {
    CurrentLatest(Path060AuthorityRecord),
    Unresolved,
}

fn path_060_naive_latest_from_bounded_completeness(
    recovered: Path060AuthorityRecord,
    witness: Option<Path060CompletenessWitness>,
) -> Path060LatestVerdict {
    match witness {
        Some(witness)
            if witness.complete_through_epoch == recovered.epoch =>
        {
            Path060LatestVerdict::CurrentLatest(recovered)
        }
        _ => Path060LatestVerdict::Unresolved,
    }
}

fn path_060_ground_truth_latest_verdict(
    recovered: Path060AuthorityRecord,
    actual_latest: Path060AuthorityRecord,
) -> Path060LatestVerdict {
    if recovered == actual_latest {
        Path060LatestVerdict::CurrentLatest(recovered)
    } else {
        Path060LatestVerdict::Unresolved
    }
}

#[test]
fn path_060_a_complete_cut_control_when_no_later_transition_exists() {
    let epoch_three = Path060AuthorityRecord {
        epoch: 3,
        transition_id: 30,
    };

    let witness = Path060CompletenessWitness {
        complete_through_epoch: 3,
    };

    assert_eq!(
        path_060_naive_latest_from_bounded_completeness(
            epoch_three,
            Some(witness),
        ),
        Path060LatestVerdict::CurrentLatest(epoch_three),
    );

    assert_eq!(
        path_060_ground_truth_latest_verdict(
            epoch_three,
            epoch_three,
        ),
        Path060LatestVerdict::CurrentLatest(epoch_three),
    );
}

#[test]
fn path_060_b_truthful_but_old_complete_cut_does_not_prove_current_latestness() {
    let recovered_three = Path060AuthorityRecord {
        epoch: 3,
        transition_id: 30,
    };

    let witness = Path060CompletenessWitness {
        complete_through_epoch: 3,
    };

    let later_four = Path060AuthorityRecord {
        epoch: 4,
        transition_id: 40,
    };

    assert_eq!(
        path_060_naive_latest_from_bounded_completeness(
            recovered_three,
            Some(witness),
        ),
        Path060LatestVerdict::CurrentLatest(recovered_three),
    );

    assert_eq!(
        path_060_ground_truth_latest_verdict(
            recovered_three,
            later_four,
        ),
        Path060LatestVerdict::Unresolved,
    );
}

#[test]
fn path_060_c_same_bounded_witness_can_require_different_currentness_verdicts() {
    let recovered_three = Path060AuthorityRecord {
        epoch: 3,
        transition_id: 30,
    };

    let witness = Path060CompletenessWitness {
        complete_through_epoch: 3,
    };

    let later_four = Path060AuthorityRecord {
        epoch: 4,
        transition_id: 40,
    };

    let complete_world_observable =
        (recovered_three, Some(witness));

    let later_append_world_observable =
        (recovered_three, Some(witness));

    assert_eq!(
        complete_world_observable,
        later_append_world_observable,
    );

    let observed_complete =
        path_060_naive_latest_from_bounded_completeness(
            complete_world_observable.0,
            complete_world_observable.1,
        );

    let observed_later_append =
        path_060_naive_latest_from_bounded_completeness(
            later_append_world_observable.0,
            later_append_world_observable.1,
        );

    assert_eq!(observed_complete, observed_later_append);

    let correct_complete =
        path_060_ground_truth_latest_verdict(
            recovered_three,
            recovered_three,
        );

    let correct_later_append =
        path_060_ground_truth_latest_verdict(
            recovered_three,
            later_four,
        );

    assert_ne!(correct_complete, correct_later_append);
    assert_eq!(observed_complete, correct_complete);
    assert_ne!(observed_later_append, correct_later_append);
}

#[test]
fn path_060_d_missing_completeness_witness_stays_unresolved() {
    let recovered_three = Path060AuthorityRecord {
        epoch: 3,
        transition_id: 30,
    };

    assert_eq!(
        path_060_naive_latest_from_bounded_completeness(
            recovered_three,
            None,
        ),
        Path060LatestVerdict::Unresolved,
    );
}


// =====================================================================
// PATH-060B — POINT-IN-TIME CURRENTNESS / USE-TIME BOUNDARY
// =====================================================================
//
// This candidate grants a stronger witness:
//
//     epoch N was current when observed at time T
//
// The witness is assumed truthful about that observation.
// The experiment asks whether point-in-time currentness proves currentness
// when the recovery decision is later used.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Path060CurrentnessWitness {
    observed_latest_epoch: u64,
    observation_id: u64,
}

fn path_060_naive_latest_from_currentness_witness(
    recovered: Path060AuthorityRecord,
    completeness: Option<Path060CompletenessWitness>,
    currentness: Option<Path060CurrentnessWitness>,
) -> Path060LatestVerdict {
    match (completeness, currentness) {
        (Some(completeness), Some(currentness))
            if completeness.complete_through_epoch == recovered.epoch
                && currentness.observed_latest_epoch == recovered.epoch =>
        {
            Path060LatestVerdict::CurrentLatest(recovered)
        }
        _ => Path060LatestVerdict::Unresolved,
    }
}

#[test]
fn path_060_e_currentness_witness_control_when_no_later_append_occurs() {
    let epoch_three = Path060AuthorityRecord {
        epoch: 3,
        transition_id: 30,
    };

    let completeness = Path060CompletenessWitness {
        complete_through_epoch: 3,
    };

    let currentness = Path060CurrentnessWitness {
        observed_latest_epoch: 3,
        observation_id: 100,
    };

    assert_eq!(
        path_060_naive_latest_from_currentness_witness(
            epoch_three,
            Some(completeness),
            Some(currentness),
        ),
        Path060LatestVerdict::CurrentLatest(epoch_three),
    );

    assert_eq!(
        path_060_ground_truth_latest_verdict(
            epoch_three,
            epoch_three,
        ),
        Path060LatestVerdict::CurrentLatest(epoch_three),
    );
}

#[test]
fn path_060_f_truthful_currentness_witness_can_become_stale_before_use() {
    let recovered_three = Path060AuthorityRecord {
        epoch: 3,
        transition_id: 30,
    };

    let later_four = Path060AuthorityRecord {
        epoch: 4,
        transition_id: 40,
    };

    let completeness = Path060CompletenessWitness {
        complete_through_epoch: 3,
    };

    // Truthful at observation 100: epoch 3 was current then.
    let currentness = Path060CurrentnessWitness {
        observed_latest_epoch: 3,
        observation_id: 100,
    };

    assert_eq!(
        path_060_naive_latest_from_currentness_witness(
            recovered_three,
            Some(completeness),
            Some(currentness),
        ),
        Path060LatestVerdict::CurrentLatest(recovered_three),
    );

    // Ground truth at decision-use time: epoch 4 now exists.
    assert_eq!(
        path_060_ground_truth_latest_verdict(
            recovered_three,
            later_four,
        ),
        Path060LatestVerdict::Unresolved,
    );
}

#[test]
fn path_060_g_same_truthful_witnesses_can_require_different_use_time_verdicts() {
    let recovered_three = Path060AuthorityRecord {
        epoch: 3,
        transition_id: 30,
    };

    let later_four = Path060AuthorityRecord {
        epoch: 4,
        transition_id: 40,
    };

    let completeness = Path060CompletenessWitness {
        complete_through_epoch: 3,
    };

    let currentness = Path060CurrentnessWitness {
        observed_latest_epoch: 3,
        observation_id: 100,
    };

    let no_append_observable =
        (recovered_three, Some(completeness), Some(currentness));

    let later_append_observable =
        (recovered_three, Some(completeness), Some(currentness));

    assert_eq!(no_append_observable, later_append_observable);

    let observed_no_append =
        path_060_naive_latest_from_currentness_witness(
            no_append_observable.0,
            no_append_observable.1,
            no_append_observable.2,
        );

    let observed_later_append =
        path_060_naive_latest_from_currentness_witness(
            later_append_observable.0,
            later_append_observable.1,
            later_append_observable.2,
        );

    assert_eq!(observed_no_append, observed_later_append);

    let correct_no_append =
        path_060_ground_truth_latest_verdict(
            recovered_three,
            recovered_three,
        );

    let correct_later_append =
        path_060_ground_truth_latest_verdict(
            recovered_three,
            later_four,
        );

    assert_ne!(correct_no_append, correct_later_append);
    assert_eq!(observed_no_append, correct_no_append);
    assert_ne!(observed_later_append, correct_later_append);
}


// --- PATH-060 INSERTION POINT ---
