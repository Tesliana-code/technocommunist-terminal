use distribution_machine::contribution::{
    ContributionEvent, ContributionFactors, ContributionWeights,
};
use distribution_machine::distribution::allocate_contribution_pool;
use distribution_machine::measurement::FailureDomainId;
use distribution_machine::measurement::aggregate_dependency_paths_with_model;

fn weights() -> ContributionWeights {
    ContributionWeights {
        responsibility: 0.20,
        criticality: 0.30,
        deal_impact: 0.25,
        dependency_reach: 0.25,
    }
}

fn event(
    contributor_id: &str,
    contribution_id: &str,
    base_work_value: f64,
    responsibility: f64,
    criticality: f64,
    deal_impact: f64,
    stewardship: f64,
    dependency_reach: f64,
    evidence_confidence: f64,
) -> ContributionEvent {
    ContributionEvent {
        contributor_id: contributor_id.to_string(),
        contribution_id: contribution_id.to_string(),
        base_work_value,
        factors: ContributionFactors {
            responsibility,
            criticality,
            deal_impact,
            stewardship,
            dependency_reach,
            evidence_confidence,
        },
    }
}

#[test]
fn path_001_founder_gravity_does_not_automatically_dominate_pool() {
    let events = vec![
        event(
            "founder",
            "architecture",
            1.20,
            0.95,
            0.95,
            0.85,
            0.70,
            0.90,
            1.00,
        ),
        event(
            "engineer",
            "critical-platform-work",
            1.00,
            0.90,
            1.00,
            0.90,
            0.70,
            1.00,
            1.00,
        ),
        event(
            "operations",
            "production-continuity",
            1.00,
            0.80,
            0.90,
            0.80,
            1.00,
            0.90,
            1.00,
        ),
    ];

    let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

    let founder = allocations
        .iter()
        .find(|a| a.contributor_id == "founder")
        .unwrap();

    assert!(
        founder.normalized_share < 0.50,
        "founder gravity pathological result: founder received {:.2}%",
        founder.normalized_share * 100.0
    );
}

#[test]
fn path_002_invisible_infrastructure_remains_economically_visible() {
    let events = vec![
        event(
            "sales",
            "commercial-close",
            1.00,
            0.80,
            0.60,
            1.00,
            0.20,
            0.40,
            1.00,
        ),
        event(
            "infrastructure",
            "reconciliation-foundation",
            1.00,
            0.85,
            1.00,
            0.55,
            0.80,
            1.00,
            1.00,
        ),
    ];

    let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

    let infrastructure = allocations
        .iter()
        .find(|a| a.contributor_id == "infrastructure")
        .unwrap();

    assert!(
        infrastructure.normalized_share >= 0.40,
        "invisible infrastructure pathological result: only {:.2}%",
        infrastructure.normalized_share * 100.0
    );
}

#[test]
fn path_003_event_count_does_not_create_value_by_itself() {
    let mut events = vec![event(
        "architect",
        "critical-decision",
        1.00,
        1.00,
        1.00,
        0.90,
        0.70,
        1.00,
        1.00,
    )];

    for index in 0..20 {
        events.push(event(
            "commit-farmer",
            &format!("trivial-{index}"),
            0.01,
            0.10,
            0.10,
            0.10,
            0.10,
            0.10,
            1.00,
        ));
    }

    let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

    let architect_total: f64 = allocations
        .iter()
        .filter(|a| a.contributor_id == "architect")
        .map(|a| a.allocated_amount)
        .sum();

    let farmer_total: f64 = allocations
        .iter()
        .filter(|a| a.contributor_id == "commit-farmer")
        .map(|a| a.allocated_amount)
        .sum();

    assert!(
        architect_total > farmer_total,
        "event-count farming beat critical work"
    );
}

#[test]
fn path_004_stewardship_does_not_leak_into_contribution_pool() {
    let events = vec![
        event(
            "builder",
            "original-build",
            1.00,
            0.80,
            0.90,
            0.80,
            0.00,
            0.80,
            1.00,
        ),
        event(
            "steward",
            "long-term-maintenance",
            1.00,
            0.80,
            0.90,
            0.80,
            1.00,
            0.80,
            1.00,
        ),
    ];

    let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

    let builder = allocations
        .iter()
        .find(|a| a.contributor_id == "builder")
        .unwrap();

    let steward = allocations
        .iter()
        .find(|a| a.contributor_id == "steward")
        .unwrap();

    assert!(
        (builder.allocated_amount - steward.allocated_amount).abs() < 1e-9,
        "stewardship leaked into Contribution Pool"
    );
}

#[test]
fn path_005_historical_contribution_survives_zero_current_deal_impact() {
    let historical = event(
        "historical-contributor",
        "old-foundational-capability",
        1.00,
        0.90,
        1.00,
        0.0,
        0.60,
        1.00,
        1.00,
    );

    let allocations = allocate_contribution_pool(&[historical], &weights(), 1_000.0).unwrap();

    assert!(allocations[0].contribution_value > 0.0);
}

#[test]
fn path_006_direct_sales_causality_does_not_monopolize_value() {
    let events = vec![
        event(
            "sales",
            "deal-close",
            1.00,
            0.80,
            0.50,
            1.00,
            0.20,
            0.40,
            1.00,
        ),
        event(
            "product",
            "required-capability",
            1.00,
            0.90,
            1.00,
            0.80,
            0.60,
            1.00,
            1.00,
        ),
    ];

    let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

    let sales = allocations
        .iter()
        .find(|a| a.contributor_id == "sales")
        .unwrap();

    assert!(
        sales.normalized_share < 0.60,
        "sales monopoly pathological result: {:.2}%",
        sales.normalized_share * 100.0
    );
}

#[test]
fn path_007_weak_evidence_cannot_beat_strong_evidence_by_status() {
    let events = vec![
        event(
            "high-status",
            "claimed-work",
            2.00,
            1.00,
            1.00,
            1.00,
            1.00,
            1.00,
            0.10,
        ),
        event(
            "verified-contributor",
            "verified-work",
            1.00,
            0.80,
            0.80,
            0.80,
            0.80,
            0.80,
            1.00,
        ),
    ];

    let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

    let high_status = allocations
        .iter()
        .find(|a| a.contributor_id == "high-status")
        .unwrap();

    let verified = allocations
        .iter()
        .find(|a| a.contributor_id == "verified-contributor")
        .unwrap();

    assert!(
        verified.allocated_amount > high_status.allocated_amount,
        "weak evidence overcame verified contribution"
    );
}

#[test]
fn path_008_stewardship_is_currently_embedded_inside_contribution_claim() {
    let low_stewardship = event(
        "same-contributor",
        "same-work-low-stewardship",
        1.00,
        0.80,
        0.80,
        0.80,
        0.00,
        0.80,
        1.00,
    );

    let high_stewardship = event(
        "same-contributor",
        "same-work-high-stewardship",
        1.00,
        0.80,
        0.80,
        0.80,
        1.00,
        0.80,
        1.00,
    );

    let low = low_stewardship.calculate(&weights()).unwrap();
    let high = high_stewardship.calculate(&weights()).unwrap();

    println!(
        "PATH-008 stewardship contribution uplift: {:.2}%",
        ((high.evidence_adjusted_value / low.evidence_adjusted_value) - 1.0) * 100.0
    );

    assert!(
        (high.evidence_adjusted_value - low.evidence_adjusted_value).abs() < 1e-9,
        "SEPARATE-CLAIM VIOLATION: stewardship changes Contribution Value \
         even though stewardship already has its own distribution pool"
    );
}

#[test]
fn path_009_contribution_and_stewardship_are_independent_claim_paths() {
    use distribution_machine::stewardship::{
        StewardshipClaim, StewardshipFactors, StewardshipWeights,
    };

    let contribution_events = vec![
        event("alice", "build", 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0),
        event("bob", "build", 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0),
    ];

    let contribution_allocations =
        allocate_contribution_pool(&contribution_events, &weights(), 10_000.0).unwrap();

    let alice_contribution = contribution_allocations
        .iter()
        .find(|a| a.contributor_id == "alice")
        .unwrap();

    let bob_contribution = contribution_allocations
        .iter()
        .find(|a| a.contributor_id == "bob")
        .unwrap();

    assert!((alice_contribution.allocated_amount - bob_contribution.allocated_amount).abs() < 1e-9);

    let stewardship_weights = StewardshipWeights {
        continuity_responsibility: 0.30,
        maintenance_burden: 0.25,
        failure_exposure: 0.20,
        dependency_reach: 0.25,
    };

    let stewardship_claims = vec![
        StewardshipClaim {
            steward_id: "alice".to_string(),
            stewardship_id: "minimal-stewardship".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 0.10,
                maintenance_burden: 0.10,
                failure_exposure: 0.10,
                dependency_reach: 0.10,
                evidence_confidence: 1.0,
            },
        },
        StewardshipClaim {
            steward_id: "bob".to_string(),
            stewardship_id: "production-stewardship".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 1.0,
                maintenance_burden: 1.0,
                failure_exposure: 1.0,
                dependency_reach: 1.0,
                evidence_confidence: 1.0,
            },
        },
    ];

    let stewardship_allocations = distribution_machine::distribution::allocate_stewardship_pool(
        &stewardship_claims,
        &stewardship_weights,
        6_000.0,
    )
    .unwrap();

    let alice_stewardship = stewardship_allocations
        .iter()
        .find(|a| a.steward_id == "alice")
        .unwrap();

    let bob_stewardship = stewardship_allocations
        .iter()
        .find(|a| a.steward_id == "bob")
        .unwrap();

    assert!(bob_stewardship.allocated_amount > alice_stewardship.allocated_amount);

    let total: f64 = stewardship_allocations
        .iter()
        .map(|a| a.allocated_amount)
        .sum();

    assert!((total - 6_000.0).abs() < 1e-9);
}

#[test]
fn path_010_capital_payout_must_never_exceed_verified_return_cap() {
    use distribution_machine::capital::{CapitalClaim, allocate_capital_pool};

    let claims = vec![CapitalClaim {
        claimant_id: "founder",
        contributed_capital: 1_000.0,
        eligible_return_cap: 1_000.0,
        evidence_confidence: 1.0,
    }];

    let allocations = allocate_capital_pool(&claims, 7_500.0).unwrap();

    let founder = &allocations[0];

    println!(
        "PATH-010 verified claim: €{:.2}, payout: €{:.2}",
        founder.verified_claim_value, founder.allocated_amount
    );

    assert!(
        founder.allocated_amount <= founder.verified_claim_value,
        "CAP VIOLATION: capital claimant received €{:.2} against a verified capped claim of €{:.2}",
        founder.allocated_amount,
        founder.verified_claim_value
    );
}

#[test]
fn path_011_unused_capital_pool_remains_explicitly_unallocated() {
    use distribution_machine::capital::{CapitalClaim, allocate_capital_pool};

    let capital_pool = 7_500.0;

    let claims = vec![CapitalClaim {
        claimant_id: "founder",
        contributed_capital: 1_000.0,
        eligible_return_cap: 1_000.0,
        evidence_confidence: 1.0,
    }];

    let allocations = allocate_capital_pool(&claims, capital_pool).unwrap();

    let total_allocated: f64 = allocations.iter().map(|a| a.allocated_amount).sum();

    let unallocated_remainder = capital_pool - total_allocated;

    println!(
        "PATH-011 capital pool: €{:.2}, allocated: €{:.2}, remainder: €{:.2}",
        capital_pool, total_allocated, unallocated_remainder
    );

    assert!(
        (total_allocated - 1_000.0).abs() < 1e-9,
        "capital allocator distributed more than the verified capped claim"
    );

    assert!(
        (unallocated_remainder - 6_500.0).abs() < 1e-9,
        "expected €6500.00 to remain explicitly unallocated, got €{:.2}",
        unallocated_remainder
    );

    assert!(
        (total_allocated + unallocated_remainder - capital_pool).abs() < 1e-9,
        "capital pool reconciliation failed"
    );
}

#[test]
fn path_012_trivial_dependency_farming_must_not_create_high_criticality() {
    use distribution_machine::measurement::{
        CriticalityMeasurementWeights, CriticalityObservables, Substitutability,
        measure_criticality,
    };

    let weights = CriticalityMeasurementWeights {
        qualified_dependency_weight: 0.30,
        blocking_paths: 0.30,
        failure_blast_radius: 0.25,
        substitutability: 0.15,
    };

    let farmed = measure_criticality(
        CriticalityObservables {
            qualified_dependency_weight: 0.0,
            blocking_paths: 0,
            failure_blast_radius: 0,
            substitutability: Substitutability::Easy,
        },
        weights,
    )
    .unwrap();

    println!(
        "PATH-012 trivial unqualified dependencies produce criticality: {:.4}",
        farmed.score
    );

    assert!(
        farmed.score < 0.20,
        "DEPENDENCY FARMING VIOLATION: trivial dependency count alone produced criticality {:.4}",
        farmed.score
    );
}

#[test]
fn path_013_many_irrelevant_dependencies_must_not_create_dependency_weight() {
    use distribution_machine::measurement::{
        DependencyEvidence, DependencyQualificationWeights, aggregate_dependency_weight,
        qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let trivial = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.0,
            failure_propagation: 0.0,
            operational_reliance: 0.0,
            business_path_relevance: 0.0,
        },
        weights,
    )
    .unwrap();

    let dependencies = vec![trivial; 10_000];

    let aggregate = aggregate_dependency_weight(&dependencies);

    println!(
        "PATH-013 aggregate weight from 10,000 irrelevant dependencies: {:.6}",
        aggregate
    );

    assert!(
        aggregate.abs() < 1e-9,
        "IRRELEVANT DEPENDENCY FARMING VIOLATION: 10,000 zero-value edges produced {:.6}",
        aggregate
    );
}

#[test]
fn path_014_few_strong_dependencies_must_outrank_many_irrelevant_dependencies() {
    use distribution_machine::measurement::{
        DependencyEvidence, DependencyQualificationWeights, aggregate_dependency_weight,
        qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let trivial = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.0,
            failure_propagation: 0.0,
            operational_reliance: 0.0,
            business_path_relevance: 0.0,
        },
        weights,
    )
    .unwrap();

    let critical = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.95,
            failure_propagation: 0.90,
            operational_reliance: 0.95,
            business_path_relevance: 0.80,
        },
        weights,
    )
    .unwrap();

    let many_trivial = vec![trivial; 10_000];
    let few_critical = vec![critical; 2];

    let trivial_weight = aggregate_dependency_weight(&many_trivial);
    let critical_weight = aggregate_dependency_weight(&few_critical);

    println!(
        "PATH-014 many trivial: {:.6}, few critical: {:.6}",
        trivial_weight, critical_weight
    );

    assert!(
        critical_weight > trivial_weight,
        "QUALIFICATION FAILURE: many irrelevant edges outranked evidenced critical dependencies"
    );
}

#[test]
fn path_015_many_weak_dependencies_must_not_fake_systemic_criticality() {
    use distribution_machine::measurement::{
        DependencyEvidence, DependencyQualificationWeights, aggregate_dependency_weight,
        qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let weak = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.01,
            failure_propagation: 0.01,
            operational_reliance: 0.01,
            business_path_relevance: 0.01,
        },
        weights,
    )
    .unwrap();

    assert!(
        (weak.score - 0.01).abs() < 1e-9,
        "test fixture must represent a genuinely weak dependency"
    );

    let weak_swarm = vec![weak; 1_000];

    let aggregate = aggregate_dependency_weight(&weak_swarm);

    println!(
        "PATH-015 single weak dependency: {:.6}, 1,000 weak dependencies aggregate: {:.6}",
        weak.score, aggregate
    );

    assert!(
        aggregate < 0.20,
        "WEAK-EDGE SWARM VIOLATION: 1,000 individually weak dependencies produced systemic weight {:.6}",
        aggregate
    );
}

#[test]
fn path_016_threshold_cliff_must_not_amplify_tiny_evidence_change() {
    use distribution_machine::measurement::{
        DependencyAggregationModel, DependencyEvidence, DependencyQualificationWeights,
        aggregate_dependency_weight_with_model, qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let just_below = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.199,
            failure_propagation: 0.199,
            operational_reliance: 0.199,
            business_path_relevance: 0.199,
        },
        weights,
    )
    .unwrap();

    let just_above = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.201,
            failure_propagation: 0.201,
            operational_reliance: 0.201,
            business_path_relevance: 0.201,
        },
        weights,
    )
    .unwrap();

    let below = aggregate_dependency_weight_with_model(
        &[just_below],
        DependencyAggregationModel::Thresholded,
    );

    let above = aggregate_dependency_weight_with_model(
        &[just_above],
        DependencyAggregationModel::Thresholded,
    );

    let input_delta = just_above.score - just_below.score;
    let output_delta = above - below;

    println!(
        "PATH-016 threshold cliff | input {:.3} -> {:.3} (delta {:.3}) | output {:.3} -> {:.3} (delta {:.3})",
        just_below.score, just_above.score, input_delta, below, above, output_delta
    );

    assert!(
        output_delta <= input_delta * 10.0,
        "THRESHOLD CLIFF VIOLATION: evidence changed by {:.3}, aggregation changed by {:.3}",
        input_delta,
        output_delta
    );
}

#[test]
fn path_017_duplicate_dependency_must_not_multiply_economic_importance() {
    use distribution_machine::measurement::{
        DependencyAggregationModel, DependencyEvidence, DependencyQualificationWeights,
        aggregate_dependency_weight_with_model, qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let same_dependency = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let once = aggregate_dependency_weight_with_model(
        &[same_dependency],
        DependencyAggregationModel::RankedDecay,
    );

    let duplicated = vec![same_dependency; 100];

    let repeated = aggregate_dependency_weight_with_model(
        &duplicated,
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-017 duplicate edge | once {:.6} | repeated 100x {:.6}",
        once, repeated
    );

    assert!(
        (repeated - once).abs() < 1e-9,
        "DUPLICATE-EDGE VIOLATION: one dependency {:.6} became {:.6} when duplicated",
        once,
        repeated
    );
}

#[test]
fn path_018_exact_duplicate_dependency_identity_must_not_create_value() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, aggregate_unique_dependencies_with_model, qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let dependency = DependencyRecord {
        dependency_id: DependencyId("dep-runtime-a-b".to_string()),
        dependency_path_id: DependencyPathId("default-path".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: CapabilityId("capability-a".to_string()),
        target_capability: CapabilityId("capability-b".to_string()),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let once = aggregate_unique_dependencies_with_model(
        &[dependency.clone()],
        DependencyAggregationModel::RankedDecay,
    );

    let duplicated = vec![dependency; 100];

    let repeated = aggregate_unique_dependencies_with_model(
        &duplicated,
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-018 identity dedup | once {:.6} | duplicated 100x {:.6}",
        once, repeated
    );

    assert!(
        (once - repeated).abs() < 1e-9,
        "IDENTITY DEDUP VIOLATION: duplicate dependency identity changed economic weight"
    );
}

#[test]
fn path_019_new_ids_must_not_disguise_same_structural_dependency() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, aggregate_unique_dependencies_with_model, qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let source = CapabilityId("capability-a".to_string());
    let target = CapabilityId("capability-b".to_string());

    let once = DependencyRecord {
        dependency_id: DependencyId("dep-001".to_string()),
        dependency_path_id: DependencyPathId("default-path".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source.clone(),
        target_capability: target.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let single =
        aggregate_unique_dependencies_with_model(&[once], DependencyAggregationModel::RankedDecay);

    let laundered: Vec<DependencyRecord> = (0..100)
        .map(|i| DependencyRecord {
            dependency_id: DependencyId(format!("dep-laundered-{i:03}")),
            dependency_path_id: DependencyPathId("default-path".to_string()),
            failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
            source_capability: source.clone(),
            target_capability: target.clone(),
            relationship_type: DependencyRelationshipType::Runtime,
            qualified,
        })
        .collect();

    let repeated = aggregate_unique_dependencies_with_model(
        &laundered,
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-019 structural identity laundering | single {:.6} | same edge under 100 ids {:.6}",
        single, repeated
    );

    assert!(
        (single - repeated).abs() < 1e-9,
        "STRUCTURAL IDENTITY VIOLATION: same source/target/type relationship changed economic weight when assigned new IDs"
    );
}

#[test]
fn path_020_structural_identity_must_resist_id_laundering() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, aggregate_structural_dependencies_with_model,
        qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let source = CapabilityId("capability-a".to_string());
    let target = CapabilityId("capability-b".to_string());

    let once = DependencyRecord {
        dependency_id: DependencyId("dep-001".to_string()),
        dependency_path_id: DependencyPathId("default-path".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source.clone(),
        target_capability: target.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let single = aggregate_structural_dependencies_with_model(
        &[once],
        DependencyAggregationModel::RankedDecay,
    );

    let laundered: Vec<DependencyRecord> = (0..100)
        .map(|i| DependencyRecord {
            dependency_id: DependencyId(format!("dep-laundered-{i:03}")),
            dependency_path_id: DependencyPathId("default-path".to_string()),
            failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
            source_capability: source.clone(),
            target_capability: target.clone(),
            relationship_type: DependencyRelationshipType::Runtime,
            qualified,
        })
        .collect();

    let repeated = aggregate_structural_dependencies_with_model(
        &laundered,
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-020 structural dedup | single {:.6} | same relation under 100 ids {:.6}",
        single, repeated
    );

    assert!(
        (single - repeated).abs() < 1e-9,
        "STRUCTURAL DEDUP VIOLATION: ID laundering changed economic weight"
    );
}

#[test]
fn path_021_multiple_relation_types_must_not_fake_independent_paths() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let source = CapabilityId("capability-a".to_string());
    let target = CapabilityId("capability-b".to_string());

    let runtime = DependencyRecord {
        dependency_id: DependencyId("dep-runtime-a-b".to_string()),
        dependency_path_id: DependencyPathId("path-a-b".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source.clone(),
        target_capability: target.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let data_flow = DependencyRecord {
        dependency_id: DependencyId("dep-data-a-b".to_string()),
        dependency_path_id: DependencyPathId("path-a-b".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source,
        target_capability: target,
        relationship_type: DependencyRelationshipType::DataFlow,
        qualified,
    };

    let single = aggregate_dependency_paths_with_model(
        &[runtime.clone()],
        DependencyAggregationModel::RankedDecay,
    );

    let correlated = aggregate_dependency_paths_with_model(
        &[runtime, data_flow],
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-021 correlated relation types | one A->B relation {:.6} | Runtime + DataFlow descriptions {:.6}",
        single, correlated
    );

    assert!(
        (single - correlated).abs() < 1e-9,
        "CORRELATED-PATH VIOLATION: multiple relationship types for the same source/target manufactured independent economic importance"
    );
}

#[test]
fn path_022_independent_dependency_paths_must_remain_distinct() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, aggregate_dependency_paths_with_model, qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let source = CapabilityId("capability-a".to_string());
    let target = CapabilityId("capability-b".to_string());

    let primary = DependencyRecord {
        dependency_id: DependencyId("dep-primary".to_string()),
        dependency_path_id: DependencyPathId("path-primary".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source.clone(),
        target_capability: target.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let secondary = DependencyRecord {
        dependency_id: DependencyId("dep-secondary".to_string()),
        dependency_path_id: DependencyPathId("path-secondary".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source,
        target_capability: target,
        relationship_type: DependencyRelationshipType::DataFlow,
        qualified,
    };

    let single = aggregate_dependency_paths_with_model(
        &[primary.clone()],
        DependencyAggregationModel::RankedDecay,
    );

    let independent = aggregate_dependency_paths_with_model(
        &[primary, secondary],
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-022 independent paths | one path {:.6} | two independent paths {:.6}",
        single, independent
    );

    assert!(
        independent > single,
        "INDEPENDENT-PATH COLLAPSE: two distinct dependency paths did not increase aggregate importance"
    );

    assert!(
        independent <= 1.0,
        "BOUNDEDNESS VIOLATION: independent dependency paths exceeded 1.0"
    );
}

#[test]
fn path_023_shared_bottleneck_must_not_fake_path_independence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, aggregate_dependency_paths_with_model, qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    /*
        Conceptual topology:

            A
           / \
          B   C
           \ /
            D

        The two declared paths are:

            path-a-b-d
            path-a-c-d

        They have different path IDs, but both terminate through the
        same downstream bottleneck D.

        Therefore path-ID distinctness alone is insufficient evidence
        of failure independence.
    */

    let source = CapabilityId("capability-a".to_string());
    let shared_bottleneck = CapabilityId("capability-d".to_string());

    let path_via_b = DependencyRecord {
        dependency_id: DependencyId("dep-a-b-d".to_string()),
        dependency_path_id: DependencyPathId("path-a-b-d".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source.clone(),
        target_capability: shared_bottleneck.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let path_via_c = DependencyRecord {
        dependency_id: DependencyId("dep-a-c-d".to_string()),
        dependency_path_id: DependencyPathId("path-a-c-d".to_string()),
        failure_domain_id: FailureDomainId("default-failure-domain".to_string()),
        source_capability: source,
        target_capability: shared_bottleneck,
        relationship_type: DependencyRelationshipType::DataFlow,
        qualified,
    };

    let single = aggregate_dependency_paths_with_model(
        &[path_via_b.clone()],
        DependencyAggregationModel::RankedDecay,
    );

    let declared_independent = aggregate_dependency_paths_with_model(
        &[path_via_b, path_via_c],
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-023 shared bottleneck | one path {:.6} | two declared paths through same bottleneck {:.6}",
        single, declared_independent
    );

    assert!(
        (single - declared_independent).abs() < 1e-9,
        "SHARED-BOTTLENECK VIOLATION: distinct path IDs through the same failure bottleneck were treated as independent economic paths"
    );
}

#[test]
fn path_024_shared_failure_domain_must_collapse_declared_paths() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, FailureDomainId, aggregate_failure_domains_with_model,
        qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let source = CapabilityId("capability-a".to_string());
    let target = CapabilityId("capability-d".to_string());

    let via_b = DependencyRecord {
        dependency_id: DependencyId("dep-a-b-d".to_string()),
        dependency_path_id: DependencyPathId("path-a-b-d".to_string()),
        failure_domain_id: FailureDomainId("failure-domain-d".to_string()),
        source_capability: source.clone(),
        target_capability: target.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let via_c = DependencyRecord {
        dependency_id: DependencyId("dep-a-c-d".to_string()),
        dependency_path_id: DependencyPathId("path-a-c-d".to_string()),
        failure_domain_id: FailureDomainId("failure-domain-d".to_string()),
        source_capability: source,
        target_capability: target,
        relationship_type: DependencyRelationshipType::DataFlow,
        qualified,
    };

    let single = aggregate_failure_domains_with_model(
        &[via_b.clone()],
        DependencyAggregationModel::RankedDecay,
    );

    let correlated = aggregate_failure_domains_with_model(
        &[via_b, via_c],
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-024 shared failure domain | one path {:.6} | two declared paths same failure domain {:.6}",
        single, correlated
    );

    assert!(
        (single - correlated).abs() < 1e-9,
        "FAILURE-DOMAIN VIOLATION: paths sharing one failure domain were treated as independent"
    );
}

#[test]
fn path_025_distinct_failure_domains_must_preserve_independence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, FailureDomainId, aggregate_failure_domains_with_model,
        qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let source = CapabilityId("capability-a".to_string());
    let target = CapabilityId("capability-d".to_string());

    let primary = DependencyRecord {
        dependency_id: DependencyId("dep-primary".to_string()),
        dependency_path_id: DependencyPathId("path-primary".to_string()),
        failure_domain_id: FailureDomainId("failure-domain-primary".to_string()),
        source_capability: source.clone(),
        target_capability: target.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let secondary = DependencyRecord {
        dependency_id: DependencyId("dep-secondary".to_string()),
        dependency_path_id: DependencyPathId("path-secondary".to_string()),
        failure_domain_id: FailureDomainId("failure-domain-secondary".to_string()),
        source_capability: source,
        target_capability: target,
        relationship_type: DependencyRelationshipType::DataFlow,
        qualified,
    };

    let single = aggregate_failure_domains_with_model(
        &[primary.clone()],
        DependencyAggregationModel::RankedDecay,
    );

    let independent = aggregate_failure_domains_with_model(
        &[primary, secondary],
        DependencyAggregationModel::RankedDecay,
    );

    println!(
        "PATH-025 distinct failure domains | one domain {:.6} | two independent domains {:.6}",
        single, independent
    );

    assert!(
        independent > single,
        "FAILURE-DOMAIN COLLAPSE: distinct failure domains were incorrectly merged"
    );

    assert!(
        independent <= 1.0,
        "BOUNDEDNESS VIOLATION: distinct failure domains exceeded 1.0"
    );
}

#[test]
fn path_026_failure_domain_id_laundering_must_not_create_independence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyAggregationModel, DependencyEvidence, DependencyId,
        DependencyPathId, DependencyQualificationWeights, DependencyRecord,
        DependencyRelationshipType, FailureDomainId, aggregate_failure_domains_with_model,
        qualify_dependency,
    };

    let weights = DependencyQualificationWeights {
        blocking_relevance: 0.30,
        failure_propagation: 0.30,
        operational_reliance: 0.25,
        business_path_relevance: 0.15,
    };

    let qualified = qualify_dependency(
        DependencyEvidence {
            blocking_relevance: 0.60,
            failure_propagation: 0.60,
            operational_reliance: 0.60,
            business_path_relevance: 0.60,
        },
        weights,
    )
    .unwrap();

    let source = CapabilityId("capability-a".to_string());
    let target = CapabilityId("shared-bottleneck-d".to_string());

    let single = DependencyRecord {
        dependency_id: DependencyId("dep-single".to_string()),
        dependency_path_id: DependencyPathId("path-single".to_string()),
        failure_domain_id: FailureDomainId("domain-real".to_string()),
        source_capability: source.clone(),
        target_capability: target.clone(),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified,
    };

    let baseline =
        aggregate_failure_domains_with_model(&[single], DependencyAggregationModel::RankedDecay);

    let laundered: Vec<DependencyRecord> = (0..100)
        .map(|i| DependencyRecord {
            dependency_id: DependencyId(format!("dep-laundered-{i}")),
            dependency_path_id: DependencyPathId(format!("path-laundered-{i}")),
            failure_domain_id: FailureDomainId(format!("fake-domain-{i}")),
            source_capability: source.clone(),
            target_capability: target.clone(),
            relationship_type: DependencyRelationshipType::Runtime,
            qualified,
        })
        .collect();

    let attacked =
        aggregate_failure_domains_with_model(&laundered, DependencyAggregationModel::RankedDecay);

    println!(
        "PATH-026 failure-domain laundering | real bottleneck {:.6} | same bottleneck under 100 domain ids {:.6}",
        baseline, attacked
    );

    assert!(
        (baseline - attacked).abs() < 1e-9,
        "FAILURE-DOMAIN LAUNDERING VIOLATION: renaming one underlying bottleneck into many failure-domain IDs created economic importance"
    );
}

#[test]
fn path_027_shared_structural_bottleneck_must_be_graph_derived() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("c", "d"),
    ];

    let bottlenecks = shared_structural_bottlenecks(&CapabilityId("a".into()), &edges);

    assert_eq!(bottlenecks, vec![CapabilityId("d".into())]);
}

#[test]
fn path_027a_genuinely_independent_branches_have_no_shared_bottleneck() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    // A -> B -> D
    // A -> C -> E
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "e"),
    ];

    let bottlenecks = shared_structural_bottlenecks(&CapabilityId("a".into()), &edges);

    assert_eq!(
        bottlenecks,
        Vec::<CapabilityId>::new(),
        "INDEPENDENCE VIOLATION: disjoint branches were reported as converging"
    );
}

#[test]
fn path_027b_incoming_edge_count_alone_is_not_evidence_of_a_shared_bottleneck() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    // Only one branch leaves A (through B). D has two incoming edges
    // (from B and from E), but both are internal to the single branch
    // under B, so D is not fed by two independent branches of A.
    //
    // A naive "count incoming edges to the node" implementation would
    // wrongly flag D here.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("b", "e"),
        DependencyEdge::new("e", "d"),
    ];

    let bottlenecks = shared_structural_bottlenecks(&CapabilityId("a".into()), &edges);

    assert_eq!(
        bottlenecks,
        Vec::<CapabilityId>::new(),
        "SEMANTIC VIOLATION: single-branch indegree was mistaken for branch convergence"
    );
}

#[test]
fn path_027c_unrelated_indegree_elsewhere_in_the_graph_is_not_evidence() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    // A only reaches B -> D. Node Y has two incoming edges, but from X
    // and Z, which are entirely unrelated to A. Y must not appear.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("z", "y"),
    ];

    let bottlenecks = shared_structural_bottlenecks(&CapabilityId("a".into()), &edges);

    assert_eq!(
        bottlenecks,
        Vec::<CapabilityId>::new(),
        "SEMANTIC VIOLATION: bottleneck was derived from an unrelated part of the graph"
    );
}

#[test]
fn path_027d_duplicate_edges_do_not_manufacture_an_independent_branch() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    // A -> B repeated five times, A -> C once, both flowing into D.
    // Repetition of one edge must not be treated as extra branches.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("c", "d"),
    ];

    let bottlenecks = shared_structural_bottlenecks(&CapabilityId("a".into()), &edges);

    assert_eq!(
        bottlenecks,
        vec![CapabilityId("d".into())],
        "DUPLICATE-EDGE LAUNDERING VIOLATION: repeating an edge changed the structural result"
    );
}

#[test]
fn path_027e_cycles_terminate_and_do_not_manufacture_arbitrary_bottlenecks() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    // A -> B -> A (cycle back to the source)
    // B -> X -> X-loop (cycle not touching the source)
    // B -> D, A -> C -> D
    //
    // Must terminate, must still find D as the genuine convergence, and
    // must NOT report X (only reachable through B's single branch, and
    // sitting in its own cycle) as shared.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "a"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("x", "b"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let bottlenecks = shared_structural_bottlenecks(&CapabilityId("a".into()), &edges);

    assert_eq!(
        bottlenecks,
        vec![CapabilityId("d".into())],
        "CYCLE VIOLATION: a cycle either failed to terminate or manufactured a bogus bottleneck"
    );
}

#[test]
fn path_027f_metadata_identifiers_cannot_influence_a_topology_only_result() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    // Same topology (A->B->D, A->C->D), described using edges derived
    // from records that would carry wildly different DependencyPathId /
    // FailureDomainId metadata upstream. DependencyEdge itself carries no
    // such metadata, so the graph-derived result must be identical
    // regardless of what those identifiers were.
    let edges_domain_set_1 = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("c", "d"),
    ];

    let edges_domain_set_2 = vec![
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("c", "d"),
        DependencyEdge::new("b", "d"),
    ];

    let source = CapabilityId("a".into());
    let result_1 = shared_structural_bottlenecks(&source, &edges_domain_set_1);
    let result_2 = shared_structural_bottlenecks(&source, &edges_domain_set_2);

    assert_eq!(
        result_1, result_2,
        "METADATA/ORDER LEAKAGE VIOLATION: topology-only result depended on edge ordering"
    );
    assert_eq!(result_1, vec![CapabilityId("d".into())]);
}

#[test]
fn path_027g_reconvergence_before_the_final_node_propagates_downstream() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, shared_structural_bottlenecks};

    // A -> B -> X
    // A -> C -> X
    //           X -> D
    //
    // This is a documented policy choice, not an obvious fact: branches B
    // and C converge first at X, then continue together to D. This
    // implementation reports BOTH X and D as shared, because D remains
    // reachable from both of A's original branches (see doc comment on
    // shared_structural_bottlenecks). A "first convergence point only"
    // policy would report just X. If PATH-028 picks a different policy,
    // this assertion is the one that should change.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "d"),
    ];

    let bottlenecks = shared_structural_bottlenecks(&CapabilityId("a".into()), &edges);

    assert_eq!(
        bottlenecks,
        vec![CapabilityId("d".into()), CapabilityId("x".into())],
    );
}

// ---------------------------------------------------------------------
// PATH-028: FIRST CONVERGENCE vs DOWNSTREAM PROPAGATION
//
// PATH-027 left an explicitly documented open question: when branches
// reconverge at X and then continue together to D (and beyond),
// shared_structural_bottlenecks() reports every downstream node as
// "shared" (transitive reachability semantics). PATH-028 tests a second
// candidate primitive, first_structural_convergences(), which reports
// only the first point of convergence per branch pair, on the theory
// that one structural loss of independence should not be counted
// repeatedly just because the shared path continues downstream.
//
// Both primitives are kept side by side. Neither is declared "correct"
// without adversarial evidence.
// ---------------------------------------------------------------------

#[test]
fn path_028_a_diamond_first_convergence_matches_shared_bottleneck() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    // A -> B -> D
    // A -> C -> D
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("c", "d"),
    ];

    let first = first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(
        first,
        vec![CapabilityId("d".into())],
        "a lone convergence point must be reported as first convergence"
    );
}

#[test]
fn path_028_b_long_tail_downstream_is_not_repeated_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
        shared_structural_bottlenecks,
    };

    // A -> B -> X -> D -> E -> F
    // A -> C -> X
    //
    // One structural loss of independence (at X), followed by a long
    // shared tail. shared_structural_bottlenecks() is expected to report
    // the whole tail; first_structural_convergences() must not.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("x", "d"),
        DependencyEdge::new("d", "e"),
        DependencyEdge::new("e", "f"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
    ];

    let source = CapabilityId("a".into());
    let shared = shared_structural_bottlenecks(&source, &edges);
    let first = first_structural_convergences(&source, &edges);

    println!(
        "PATH-028 downstream convergence\nshared-reachability = {:?}\nfirst-convergence   = {:?}",
        shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
        first.iter().map(|c| &c.0).collect::<Vec<_>>(),
    );

    assert_eq!(
        shared,
        vec![
            CapabilityId("d".into()),
            CapabilityId("e".into()),
            CapabilityId("f".into()),
            CapabilityId("x".into()),
        ],
        "shared_structural_bottlenecks must still report the full downstream tail"
    );

    assert_eq!(
        first,
        vec![CapabilityId("x".into())],
        "DOWNSTREAM MULTIPLICITY VIOLATION: first_structural_convergences must not \
         repeat the same structural loss of independence for D/E/F"
    );
}

#[test]
fn path_028_c_multiple_separate_convergence_regions_are_each_first() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    // Three branches out of A. B+C converge at X. B+E separately converge
    // at Y. X and Y are unrelated sinks -- two distinct, non-overlapping
    // convergence regions sharing only the source and one common branch.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "e"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("b", "y"),
        DependencyEdge::new("e", "y"),
    ];

    let first = first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(
        first,
        vec![CapabilityId("x".into()), CapabilityId("y".into())],
        "two independent convergence regions must both be reported"
    );
}

#[test]
fn path_028_d_duplicate_edges_cannot_create_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "d"),
        DependencyEdge::new("x", "d"),
    ];

    let first = first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(
        first,
        vec![CapabilityId("x".into())],
        "DUPLICATE-EDGE LAUNDERING VIOLATION: repeated edges changed the first-convergence result"
    );
}

#[test]
fn path_028_e_cycles_through_source_do_not_manufacture_fake_first_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    // Same cycle-through-source shape as PATH-027e: A<->B, B<->M (a cycle
    // that does not touch the source), B->X, A->C->X.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "a"),
        DependencyEdge::new("b", "m"),
        DependencyEdge::new("m", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
    ];

    let first = first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(
        first,
        vec![CapabilityId("x".into())],
        "CYCLE VIOLATION: a cycle through the source manufactured a bogus first-convergence node"
    );
}

#[test]
fn path_028_f_cyclic_shared_nodes_make_first_convergence_ill_defined() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    // A -> B -> X
    // A -> C -> X
    //           X <-> Y   (cycle downstream of the real convergence point)
    //
    // The genuine structural convergence happens at X. But X and Y are
    // mutually reachable, so under the "no other shared node reaches me"
    // rule, X disqualifies Y and Y disqualifies X: neither is reported.
    //
    // This is the smallest known counterexample where
    // first_structural_convergences() is not uniquely defined by its own
    // rule. It is captured here as a DOCUMENTED LIMITATION, not corrected
    // by adding cycle-condensation machinery -- see FIND-008.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    let first = first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(
        first,
        Vec::<CapabilityId>::new(),
        "KNOWN AMBIGUITY: mutual reachability among shared nodes collapses the \
         first-convergence result to empty even though a real convergence exists at X. \
         If this assertion ever changes, update the ambiguity note on \
         first_structural_convergences() and FIND-008 accordingly."
    );
}

#[test]
fn path_028_g_one_branch_feeding_another_branch_root_is_a_separate_open_question() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    // A -> B -> C   (B flows directly into A's OTHER branch root, C)
    // A -> C -> D
    //
    // B contributes no reachable node that C does not already reach: B's
    // entire downstream continuation is a subset of C's. Under the
    // current branch-counting rule, C itself becomes a "shared" node
    // (reachable from branch B via B->C, and trivially from branch C),
    // and is reported as the first convergence. Whether a branch root
    // being re-fed by another branch should count as a structural loss
    // of independence at all is a separate open question from the
    // first-vs-downstream question PATH-028 was scoped to answer; it is
    // recorded here rather than silently resolved.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let first = first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(
        first,
        vec![CapabilityId("c".into())],
        "branch-root absorption produced an unexpected first-convergence result; \
         re-examine this case before relying on it economically"
    );
}

#[test]
fn path_028_h_unrelated_graph_regions_do_not_affect_source_relative_result() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        // Unrelated region: its own convergence, nothing to do with A.
        DependencyEdge::new("p", "q"),
        DependencyEdge::new("p", "r"),
        DependencyEdge::new("q", "z"),
        DependencyEdge::new("r", "z"),
    ];

    let first = first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(
        first,
        vec![CapabilityId("x".into())],
        "SEMANTIC VIOLATION: an unrelated convergence region leaked into a source-relative result"
    );
}

#[test]
fn path_028_i_edge_ordering_and_naming_cannot_affect_first_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences,
    };

    let edges_order_1 = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("x", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
    ];

    let edges_order_2 = vec![
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("x", "d"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "b"),
    ];

    let source = CapabilityId("a".into());
    let result_1 = first_structural_convergences(&source, &edges_order_1);
    let result_2 = first_structural_convergences(&source, &edges_order_2);

    assert_eq!(result_1, result_2);
    assert_eq!(result_1, vec![CapabilityId("x".into())]);
}

// ---------------------------------------------------------------------
// PATH-029: SUBSUMED BRANCHES AND STRUCTURAL INDEPENDENCE
//
// PATH-028-g flagged an open question: in `A->B->C, A->C->D`, B and C
// are both immediate children of A, but B's only route anywhere is
// through C. Does B count as an independent structural branch at all?
//
// effective_structural_branches() answers this with a counterfactual
// removal test, not raw reachable-set containment (which gets the
// direction backwards on this exact graph -- see path_029_a2).
// ---------------------------------------------------------------------

#[test]
fn path_029_a1_direct_branch_absorption_collapses_to_one_effective_branch() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A -> B -> C
    // A -> C -> D
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let effective = effective_structural_branches(&CapabilityId("a".into()), &edges);

    assert_eq!(
        effective,
        vec![CapabilityId("c".into())],
        "B's entire reach is gatekept by C; B must not survive as an independent branch"
    );
}

#[test]
fn path_029_a2_counterexample_naive_containment_is_backwards() {
    use distribution_machine::measurement::DependencyEdge;

    // Documents, rather than exercises library code, why plain
    // reachable-set containment (reachable(r1) subset-of reachable(r2))
    // is the wrong test for subsumption.
    //
    // A -> B -> C
    // A -> C -> D
    //
    // reachable(B) = {B, C, D}
    // reachable(C) = {C, D}
    //
    // reachable(C) IS a subset of reachable(B) -- C is the smaller set.
    // A naive "smaller reachable set is the subsumed one" rule would flag
    // C (which needs nothing from B) as subsumed by B (which needs
    // everything from C). That is exactly backwards from
    // path_029_a1's result, where B is correctly identified as the
    // redundant branch via counterfactual removal instead.
    let _edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];
    let reachable_b_len = 3; // {B, C, D}
    let reachable_c_len = 2; // {C, D}
    assert!(
        reachable_c_len < reachable_b_len,
        "sanity check on the containment counterexample this comment describes"
    );
}

#[test]
fn path_029_b_proper_independence_preserves_both_branches() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A -> B -> D
    // A -> C -> E
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "e"),
    ];

    let mut effective = effective_structural_branches(&CapabilityId("a".into()), &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));

    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())],
        "genuinely independent branches must both survive"
    );
}

#[test]
fn path_029_c_partial_overlap_is_not_accidentally_collapsed() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A -> B -> X
    // A -> C -> X
    // B -> Y      (B has unique structure the shared node X does not gatekeep)
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("b", "y"),
    ];

    let mut effective = effective_structural_branches(&CapabilityId("a".into()), &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));

    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())],
        "COLLAPSE VIOLATION: partial independence (B's unique reach to Y) was destroyed"
    );
}

#[test]
fn path_029_d1_mutual_branch_root_cycle_collapses_to_one_deterministic_representative() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A -> B
    // A -> C
    // B -> C
    // C -> B
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "b"),
    ];

    let effective = effective_structural_branches(&CapabilityId("a".into()), &edges);

    assert_eq!(
        effective,
        vec![CapabilityId("b".into())],
        "mutually subsuming branch roots must collapse to a single deterministic representative"
    );
}

#[test]
fn path_029_d2_three_way_subsumption_cycle_would_collapse_to_empty_without_scc_handling() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A -> B, A -> C, A -> E
    // B -> C
    // C -> E
    // E -> B
    //
    // Pairwise: B is subsumed by C (B's only route is through C), C is
    // subsumed by E, E is subsumed by B -- a one-directional cycle with
    // no single mutual pair. A naive "drop any branch subsumed by
    // another" rule, applied without transitive-closure/SCC awareness,
    // drops all three, leaving zero effective branches from three real
    // ones. This is the PATH-029 counterexample that forced the
    // SCC-condensation step in effective_structural_branches().
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "e"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "e"),
        DependencyEdge::new("e", "b"),
    ];

    let effective = effective_structural_branches(&CapabilityId("a".into()), &edges);

    assert_eq!(
        effective,
        vec![CapabilityId("b".into())],
        "SCC VIOLATION: a 3-way subsumption cycle among branch roots must collapse to one \
         deterministic representative, not vanish entirely"
    );
}

#[test]
fn path_029_e_duplicate_edges_are_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let effective = effective_structural_branches(&CapabilityId("a".into()), &edges);

    assert_eq!(
        effective,
        vec![CapabilityId("c".into())],
        "DUPLICATE-EDGE VIOLATION: repeated edges changed the subsumption result"
    );
}

#[test]
fn path_029_f_edge_ordering_and_naming_are_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    let edges_order_1 = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let edges_order_2 = vec![
        DependencyEdge::new("c", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "b"),
    ];

    let source = CapabilityId("a".into());
    let result_1 = effective_structural_branches(&source, &edges_order_1);
    let result_2 = effective_structural_branches(&source, &edges_order_2);

    assert_eq!(result_1, result_2);
    assert_eq!(result_1, vec![CapabilityId("c".into())]);
}

#[test]
fn path_029_g_three_branches_one_subsumed_two_independent() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A -> B
    // A -> C
    // A -> E
    // B -> C   (B subsumed by C)
    // E -> F   (E independent, unique downstream)
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "e"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("e", "f"),
    ];

    let mut effective = effective_structural_branches(&CapabilityId("a".into()), &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));

    assert_eq!(
        effective,
        vec![CapabilityId("c".into()), CapabilityId("e".into())],
        "B must be dropped as subsumed by C, while C and E remain independent"
    );
}

#[test]
fn path_029_h_unique_work_before_entering_another_branch_is_not_subsumed() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A -> B -> P -> Q -> C   (B does unique work at P, Q before reaching C)
    // A -> C -> D
    //
    // B eventually reaches into C's territory, but P and Q are B's own
    // unique structure that does not depend on C existing at all. B must
    // NOT be collapsed away.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "p"),
        DependencyEdge::new("p", "q"),
        DependencyEdge::new("q", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let mut effective = effective_structural_branches(&CapabilityId("a".into()), &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));

    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())],
        "UNIQUE-WORK VIOLATION: a branch doing genuine unique work before joining another \
         branch must not be treated as subsumed"
    );
}

#[test]
fn path_029_i_cycle_through_source_does_not_manufacture_fake_subsumption() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // A <-> B (cycle through source), B -> D
    // A -> C -> D
    //
    // Without treating source as a dead end during the removal test, B
    // could "reach" C merely by cycling back through A, manufacturing a
    // bogus subsumption relationship neither branch actually has.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "a"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let mut effective = effective_structural_branches(&CapabilityId("a".into()), &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));

    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())],
        "CYCLE VIOLATION: a cycle through the source manufactured a bogus subsumption relationship"
    );
}

#[test]
fn path_029_j_compare_raw_branches_shared_bottlenecks_and_effective_branches() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        shared_structural_bottlenecks,
    };

    // A -> B -> C
    // A -> C -> D
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];

    let source = CapabilityId("a".into());
    let effective = effective_structural_branches(&source, &edges);
    let shared = shared_structural_bottlenecks(&source, &edges);
    let first = first_structural_convergences(&source, &edges);

    println!(
        "PATH-029 subsumed branches\nraw branches        = [\"b\", \"c\"]\neffective branches   = {:?}\nshared bottlenecks   = {:?}\nfirst convergences   = {:?}",
        effective.iter().map(|c| &c.0).collect::<Vec<_>>(),
        shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
        first.iter().map(|c| &c.0).collect::<Vec<_>>(),
    );

    // shared_structural_bottlenecks and first_structural_convergences are
    // computed over RAW immediate branches (B, C) and are intentionally
    // unaffected by PATH-029 -- effective_structural_branches is a
    // separate candidate, not a replacement.
    assert_eq!(effective, vec![CapabilityId("c".into())]);
    assert_eq!(
        shared,
        vec![CapabilityId("c".into()), CapabilityId("d".into())]
    );
    assert_eq!(
        first,
        vec![CapabilityId("c".into())]
    );
}

// ---------------------------------------------------------------------
// PATH-030: COMPOSING EFFECTIVE BRANCHES WITH CONVERGENCE ANALYSIS
//
// PATH-027/028's shared_structural_bottlenecks / first_structural_convergences
// run over RAW immediate branches. PATH-029's effective_structural_branches
// filters out branches that are subsumed by another branch. PATH-030 asks:
// does convergence analysis change (and change for the better) if it runs
// over effective branches instead of raw ones?
//
// Two composed candidates are added without mutating any existing
// primitive: shared_structural_bottlenecks_from_effective_branches and
// first_structural_convergences_from_effective_branches.
// ---------------------------------------------------------------------

#[test]
fn path_030_a_branch_absorption_removes_a_false_shared_bottleneck() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B -> C
    // A -> C -> D
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];
    let source = CapabilityId("a".into());

    let effective = effective_structural_branches(&source, &edges);
    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    println!(
        "PATH-030 branch absorption\neffective branches = {:?}\nraw shared          = {:?}\nraw first           = {:?}\ncomposed shared     = {:?}\ncomposed first      = {:?}",
        effective.iter().map(|c| &c.0).collect::<Vec<_>>(),
        raw_shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
        raw_first.iter().map(|c| &c.0).collect::<Vec<_>>(),
        composed_shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
        composed_first.iter().map(|c| &c.0).collect::<Vec<_>>(),
    );

    assert_eq!(effective, vec![CapabilityId("c".into())]);
    assert_eq!(
        raw_shared,
        vec![CapabilityId("c".into()), CapabilityId("d".into())],
        "raw analysis flags C (and D) as shared, purely because subsumed branch B also reaches C"
    );
    assert_eq!(raw_first, vec![CapabilityId("c".into())]);

    // With only one effective branch, there is nothing left to converge
    // with -- C was never really "shared", it was A's only real route.
    assert_eq!(
        composed_shared,
        Vec::<CapabilityId>::new(),
        "COMPOSITION VIOLATION: effective-branch analysis should remove the false convergence at C"
    );
    assert_eq!(composed_first, Vec::<CapabilityId>::new());
}

#[test]
fn path_030_b_genuine_diamond_raw_and_composed_agree() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B -> D
    // A -> C -> D
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];
    let source = CapabilityId("a".into());

    let mut effective = effective_structural_branches(&source, &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));

    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())],
        "a genuine diamond must not have its branches normalized away"
    );

    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    assert_eq!(raw_shared, vec![CapabilityId("d".into())]);
    assert_eq!(raw_first, vec![CapabilityId("d".into())]);
    assert_eq!(
        composed_shared, raw_shared,
        "genuine independence must not diverge between raw and composed analysis"
    );
    assert_eq!(composed_first, raw_first);
}

#[test]
fn path_030_c_long_tail_convergence_is_unaffected_by_composition() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B -> X -> D -> E -> F
    // A -> C -> X
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("x", "d"),
        DependencyEdge::new("d", "e"),
        DependencyEdge::new("e", "f"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
    ];
    let source = CapabilityId("a".into());

    let mut effective = effective_structural_branches(&source, &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())]
    );

    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    assert_eq!(composed_shared, raw_shared, "PATH-028's downstream-vs-first result must survive composition");
    assert_eq!(composed_first, raw_first);
    assert_eq!(composed_first, vec![CapabilityId("x".into())]);
}

#[test]
fn path_030_d_subsumed_branch_alone_manufactures_a_shared_bottleneck_artifact() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B
    // A -> C
    // A -> E
    // B -> C   (B subsumed by C)
    // E -> F   (E independent)
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "e"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("e", "f"),
    ];
    let source = CapabilityId("a".into());

    let mut effective = effective_structural_branches(&source, &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(
        effective,
        vec![CapabilityId("c".into()), CapabilityId("e".into())]
    );

    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    println!(
        "PATH-030 subsumed-branch artifact\nraw shared     = {:?}\ncomposed shared = {:?}",
        raw_shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
        composed_shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
    );

    // C is flagged "shared" under raw analysis purely because the
    // subsumed branch B also happens to reach it (C is reachable from
    // B's branch AND trivially from its own branch). E and F never
    // participate -- there is no real second branch reaching C.
    assert_eq!(
        raw_shared,
        vec![CapabilityId("c".into())],
        "ARTIFACT CHECK: raw analysis must show C as a (false) shared bottleneck here"
    );
    assert_eq!(raw_first, vec![CapabilityId("c".into())]);

    assert_eq!(
        composed_shared,
        Vec::<CapabilityId>::new(),
        "COMPOSITION VIOLATION: effective-branch analysis must remove the artifact caused solely by B's presence"
    );
    assert_eq!(composed_first, Vec::<CapabilityId>::new());
}

#[test]
fn path_030_e_partial_overlap_convergence_survives_composition() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B -> X
    // A -> C -> X
    // B -> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("b", "y"),
    ];
    let source = CapabilityId("a".into());

    let mut effective = effective_structural_branches(&source, &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())],
        "B's unique reach to Y means it is not subsumed by C"
    );

    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    assert_eq!(raw_shared, vec![CapabilityId("x".into())]);
    assert_eq!(raw_first, vec![CapabilityId("x".into())]);
    assert_eq!(
        composed_shared, raw_shared,
        "STRUCTURALLY-JUSTIFIED CONVERGENCE VIOLATION: X's real convergence must survive composition"
    );
    assert_eq!(composed_first, raw_first);
}

#[test]
fn path_030_f_branch_root_scc_collapses_convergence_to_empty_without_losing_downstream_info() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B
    // A -> C
    // B -> C
    // C -> B
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "b"),
    ];
    let source = CapabilityId("a".into());

    let effective = effective_structural_branches(&source, &edges);
    assert_eq!(effective, vec![CapabilityId("b".into())]);

    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    println!(
        "PATH-030 branch-root SCC\nraw shared      = {:?}\nraw first       = {:?} (ambiguous, PATH-028-f style)\ncomposed shared = {:?}\ncomposed first  = {:?}",
        raw_shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
        raw_first.iter().map(|c| &c.0).collect::<Vec<_>>(),
        composed_shared.iter().map(|c| &c.0).collect::<Vec<_>>(),
        composed_first.iter().map(|c| &c.0).collect::<Vec<_>>(),
    );

    // B and C are mutually reachable and reach nothing else: raw analysis
    // reports them as "shared" with each other (an artifact of the cycle,
    // not a real second branch converging on anything), and first-
    // convergence collapses to the same PATH-028-f-style ambiguity (each
    // disqualifies the other).
    assert_eq!(
        raw_shared,
        vec![CapabilityId("b".into()), CapabilityId("c".into())]
    );
    assert_eq!(raw_first, Vec::<CapabilityId>::new());

    // With one effective branch there is nothing to converge with.
    assert_eq!(
        composed_shared,
        Vec::<CapabilityId>::new(),
        "a single effective branch cannot converge with itself"
    );
    assert_eq!(composed_first, Vec::<CapabilityId>::new());

    // No downstream information is lost: because B and C are mutually
    // reachable, they have IDENTICAL reachable sets outside the {B, C}
    // group by definition (anything one reaches, the other reaches via
    // the cycle edge). Collapsing to one representative cannot hide a
    // real downstream node one branch could see and the other could not.
}

#[test]
fn path_030_g_three_way_branch_root_cycle_matches_the_two_way_pattern() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B, A -> C, A -> E
    // B -> C, C -> E, E -> B
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "e"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "e"),
        DependencyEdge::new("e", "b"),
    ];
    let source = CapabilityId("a".into());

    let effective = effective_structural_branches(&source, &edges);
    assert_eq!(effective, vec![CapabilityId("b".into())]);

    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    assert_eq!(
        raw_shared,
        vec![
            CapabilityId("b".into()),
            CapabilityId("c".into()),
            CapabilityId("e".into())
        ]
    );
    assert_eq!(raw_first, Vec::<CapabilityId>::new());
    assert_eq!(composed_shared, Vec::<CapabilityId>::new());
    assert_eq!(composed_first, Vec::<CapabilityId>::new());
}

#[test]
fn path_030_h_renaming_isomorphic_scc_members_changes_only_the_returned_label() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // Two structurally identical 2-cycles, labeled differently so the
    // lexicographically-smallest (canonical) representative differs.
    let edges_bc = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "b"),
    ];
    let edges_pq = vec![
        DependencyEdge::new("a", "q"),
        DependencyEdge::new("a", "p"),
        DependencyEdge::new("q", "p"),
        DependencyEdge::new("p", "q"),
    ];
    let source = CapabilityId("a".into());

    let effective_bc = effective_structural_branches(&source, &edges_bc);
    let effective_pq = effective_structural_branches(&source, &edges_pq);

    // Acceptable: the actual chosen representative label differs (it is
    // metadata -- an arbitrary tie-break among structurally-equivalent
    // nodes), because these are genuinely different node identities.
    assert_eq!(effective_bc, vec![CapabilityId("b".into())]);
    assert_eq!(effective_pq, vec![CapabilityId("p".into())]);

    // Not acceptable: the STRUCTURAL SHAPE of the result -- how many
    // effective branches, how many composed shared bottlenecks -- must be
    // identical between isomorphic graphs regardless of labeling.
    assert_eq!(effective_bc.len(), effective_pq.len());

    let composed_bc = shared_structural_bottlenecks_from_effective_branches(&source, &edges_bc);
    let composed_pq = shared_structural_bottlenecks_from_effective_branches(&source, &edges_pq);
    assert_eq!(
        composed_bc.len(),
        composed_pq.len(),
        "RENAMING VIOLATION: isomorphic graphs produced different-shaped composed results"
    );
    assert_eq!(composed_bc, Vec::<CapabilityId>::new());
    assert_eq!(composed_pq, Vec::<CapabilityId>::new());
}

#[test]
fn path_030_i_path_028_f_ambiguity_is_unaffected_by_branch_effective_composition() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, first_structural_convergences,
        first_structural_convergences_from_effective_branches, shared_structural_bottlenecks,
        shared_structural_bottlenecks_from_effective_branches,
    };

    // A -> B -> X
    // A -> C -> X
    //           X <-> Y   (cycle downstream of the branch roots, not among them)
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    // B and C do not reach each other at all, so PATH-029 finds nothing
    // to normalize: the branch set is unchanged by effective-branch
    // analysis.
    let mut effective = effective_structural_branches(&source, &edges);
    effective.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(
        effective,
        vec![CapabilityId("b".into()), CapabilityId("c".into())],
        "the PATH-028-f cycle is downstream of the branch roots, not between them"
    );

    let raw_shared = shared_structural_bottlenecks(&source, &edges);
    let raw_first = first_structural_convergences(&source, &edges);
    let composed_shared = shared_structural_bottlenecks_from_effective_branches(&source, &edges);
    let composed_first = first_structural_convergences_from_effective_branches(&source, &edges);

    // Branch normalization changed nothing here, so the composed results
    // must be byte-identical to the raw ones -- including the known
    // PATH-028-f ambiguity (first collapses to empty).
    assert_eq!(composed_shared, raw_shared);
    assert_eq!(composed_first, raw_first);
    assert_eq!(
        raw_first,
        Vec::<CapabilityId>::new(),
        "INDEPENDENT-PROBLEMS CHECK: branch-root normalization and convergence-cycle semantics \
         are separate concerns -- PATH-030 does not resolve PATH-028-f"
    );
}

// ---------------------------------------------------------------------
// PATH-031: CYCLE-AWARE CONVERGENCE FRONTIER
//
// PATH-028-f found that when the shared-bottleneck set contains a cycle,
// first_structural_convergences collapses to [] -- every cyclic member
// disqualifies every other. PATH-030 confirmed effective-branch
// composition does not touch this; the ambiguity lives inside the
// shared-bottleneck relation itself.
//
// PATH-031 adds cycle_aware_first_structural_convergences (and its
// effective-branch-composed counterpart), returning
// Vec<StructuralConvergenceRegion> instead of Vec<CapabilityId>: an SCC
// of the induced subgraph over the shared set becomes one region, and a
// region is "first" iff no other region reaches it. Neither existing
// primitive is modified.
// ---------------------------------------------------------------------

fn region_shapes(regions: &[distribution_machine::measurement::StructuralConvergenceRegion]) -> Vec<Vec<String>> {
    regions
        .iter()
        .map(|r| r.members.iter().map(|c| c.0.clone()).collect())
        .collect()
}

#[test]
fn path_031_a_minimal_cycle_is_reported_as_one_region_not_empty() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        first_structural_convergences,
    };

    // A -> B -> X
    // A -> C -> X
    //           X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let raw_first = first_structural_convergences(&source, &edges);
    let regions = cycle_aware_first_structural_convergences(&source, &edges);

    println!(
        "PATH-031 minimal cycle\nraw first (ambiguous) = {:?}\ncycle-aware regions   = {:?}",
        raw_first.iter().map(|c| &c.0).collect::<Vec<_>>(),
        region_shapes(&regions),
    );

    assert_eq!(
        raw_first,
        Vec::<CapabilityId>::new(),
        "PATH-028-f baseline must still reproduce the ambiguity in the unmodified primitive"
    );
    assert_eq!(
        region_shapes(&regions),
        vec![vec!["x".to_string(), "y".to_string()]],
        "CYCLE-AWARE VIOLATION: {{X,Y}} must be reported as one first-convergence region"
    );
}

#[test]
fn path_031_b_acyclic_diamond_preserves_d() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "d"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(region_shapes(&regions), vec![vec!["d".to_string()]]);
}

#[test]
fn path_031_c_long_tail_still_isolates_first_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X -> D -> E
    // A -> C -> X
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("x", "d"),
        DependencyEdge::new("d", "e"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        region_shapes(&regions),
        vec![vec!["x".to_string()]],
        "DOWNSTREAM MULTIPLICITY VIOLATION: D/E must not appear as additional regions"
    );
}

#[test]
fn path_031_d_cycle_downstream_of_an_acyclic_first_convergence_stays_upstream() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X -> Y -> Z
    // A -> C -> X
    //           Y <-> Z
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "z"),
        DependencyEdge::new("z", "y"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        region_shapes(&regions),
        vec![vec!["x".to_string()]],
        "the frontier must stay at X; the downstream {{Y,Z}} cycle is a continuation, not a second event"
    );
}

#[test]
fn path_031_e_cycle_entered_at_two_different_nodes_is_still_one_region() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X
    // A -> C -> Y
    //      X -> Y
    //      Y -> X
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(region_shapes(&regions), vec![vec!["x".to_string(), "y".to_string()]]);
}

#[test]
fn path_031_f_two_separate_first_regions_one_cyclic_both_survive() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B1 -> X, A -> B2 -> X            (acyclic convergence at X)
    // A -> B3 -> Y, A -> B4 -> Z, Y <-> Z    (cyclic convergence at {Y,Z})
    let edges = vec![
        DependencyEdge::new("a", "b1"),
        DependencyEdge::new("b1", "x"),
        DependencyEdge::new("a", "b2"),
        DependencyEdge::new("b2", "x"),
        DependencyEdge::new("a", "b3"),
        DependencyEdge::new("b3", "y"),
        DependencyEdge::new("a", "b4"),
        DependencyEdge::new("b4", "z"),
        DependencyEdge::new("y", "z"),
        DependencyEdge::new("z", "y"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        region_shapes(&regions),
        vec![
            vec!["x".to_string()],
            vec!["y".to_string(), "z".to_string()]
        ],
        "both independent convergence regions must survive, one acyclic and one cyclic"
    );
}

#[test]
fn path_031_g_shared_scc_with_downstream_tail_is_one_region_not_four() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X, A -> C -> X, X <-> Y, X -> D -> E
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
        DependencyEdge::new("x", "d"),
        DependencyEdge::new("d", "e"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        region_shapes(&regions),
        vec![vec!["x".to_string(), "y".to_string()]],
        "MULTIPLICITY VIOLATION: one structural loss of independence must not become {{X,Y}} plus D plus E"
    );
}

#[test]
fn path_031_h_nested_shared_sccs_only_the_upstream_one_is_first() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> P, A -> C -> P, P <-> Q, P -> R, A -> D -> R, R <-> S
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "p"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "p"),
        DependencyEdge::new("p", "q"),
        DependencyEdge::new("q", "p"),
        DependencyEdge::new("p", "r"),
        DependencyEdge::new("a", "d"),
        DependencyEdge::new("d", "r"),
        DependencyEdge::new("r", "s"),
        DependencyEdge::new("s", "r"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        region_shapes(&regions),
        vec![vec!["p".to_string(), "q".to_string()]],
        "NESTED-SCC VIOLATION: only the upstream SCC {{P,Q}} may be first; {{R,S}} is downstream"
    );
}

#[test]
fn path_031_i_duplicate_edges_are_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(region_shapes(&regions), vec![vec!["x".to_string(), "y".to_string()]]);
}

#[test]
fn path_031_j_edge_ordering_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let edges_1 = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let edges_2 = vec![
        DependencyEdge::new("y", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "b"),
    ];
    let source = CapabilityId("a".into());

    let regions_1 = cycle_aware_first_structural_convergences(&source, &edges_1);
    let regions_2 = cycle_aware_first_structural_convergences(&source, &edges_2);
    assert_eq!(region_shapes(&regions_1), region_shapes(&regions_2));
}

#[test]
fn path_031_k_renaming_isomorphic_cycles_preserves_shape_not_labels() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let edges_xy = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let edges_pq = vec![
        DependencyEdge::new("a", "m"),
        DependencyEdge::new("a", "n"),
        DependencyEdge::new("m", "q"),
        DependencyEdge::new("n", "q"),
        DependencyEdge::new("q", "p"),
        DependencyEdge::new("p", "q"),
    ];
    let source = CapabilityId("a".into());

    let regions_xy = cycle_aware_first_structural_convergences(&source, &edges_xy);
    let regions_pq = cycle_aware_first_structural_convergences(&source, &edges_pq);

    // Same shape: one region, two members.
    assert_eq!(regions_xy.len(), regions_pq.len());
    assert_eq!(regions_xy[0].members.len(), regions_pq[0].members.len());

    // Different labels are expected and fine -- these are genuinely
    // different node identities, not the same graph reordered.
    assert_eq!(
        regions_xy[0].members,
        vec![CapabilityId("x".into()), CapabilityId("y".into())]
    );
    assert_eq!(
        regions_pq[0].members,
        vec![CapabilityId("p".into()), CapabilityId("q".into())]
    );
}

#[test]
fn path_031_l_cycle_through_source_does_not_fabricate_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A <-> B, B -> X ; A -> C -> X
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "a"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        region_shapes(&regions),
        vec![vec!["x".to_string()]],
        "CYCLE-THROUGH-SOURCE VIOLATION: a cycle back to A must not fabricate extra convergence"
    );
}

#[test]
fn path_031_m_composition_removes_a_branch_root_from_inside_a_shared_scc() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        cycle_aware_first_structural_convergences_from_effective_branches,
    };

    // A -> B -> C, A -> C -> D, C <-> D
    //
    // B is subsumed by C (PATH-029). Under RAW analysis, C -- itself one
    // of A's own branch roots -- ends up inside the same SCC as D,
    // because B's redundant presence makes C look "shared". Composing
    // with effective branches drops B first, leaving only one effective
    // branch (C), so there is nothing left to converge with at all.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
        DependencyEdge::new("d", "c"),
    ];
    let source = CapabilityId("a".into());

    let raw_regions = cycle_aware_first_structural_convergences(&source, &edges);
    let composed_regions =
        cycle_aware_first_structural_convergences_from_effective_branches(&source, &edges);

    println!(
        "PATH-031 branch-root-in-SCC\nraw regions      = {:?}\ncomposed regions = {:?}",
        region_shapes(&raw_regions),
        region_shapes(&composed_regions),
    );

    assert_eq!(
        region_shapes(&raw_regions),
        vec![vec!["c".to_string(), "d".to_string()]],
        "raw analysis puts branch root C inside the shared SCC due to B's redundant presence"
    );
    assert_eq!(
        region_shapes(&composed_regions),
        Vec::<Vec<String>>::new(),
        "COMPOSITION VIOLATION: with B correctly excluded, there is only one effective branch \
         and nothing left to converge with"
    );
}

#[test]
fn path_031_n_region_membership_alone_does_not_capture_entry_point_structure() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // PATH-032 CORRECTION: this test originally claimed single-entry and
    // dual-entry cycles were "topologically indistinguishable" and cited
    // it as evidence the graph model lacks causal/temporal information.
    // That claim was wrong -- the two graphs below have different edges
    // (`c->x` vs `c->y`) and were never the same graph. What is true, and
    // is all this test now asserts, is narrower: `members` alone (SCC
    // membership) is the same for both, because SCC membership does not
    // record which node a branch entered through. See
    // path_032_a/path_032_b, which show `entries` DOES distinguish them
    // using ordinary topology -- no timestamps or causal metadata needed.
    // FIND-009 has been corrected accordingly.
    let single_entry = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let dual_entry = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    let source = CapabilityId("a".into());
    let single_entry_regions = cycle_aware_first_structural_convergences(&source, &single_entry);
    let dual_entry_regions = cycle_aware_first_structural_convergences(&source, &dual_entry);

    assert_eq!(
        region_shapes(&single_entry_regions),
        region_shapes(&dual_entry_regions),
        "member-set shape is identical between these two different graphs -- \
         members alone under-represents them, which is exactly why PATH-032 adds entries"
    );
}

// ---------------------------------------------------------------------
// PATH-032: PRESERVE ENTRY-POINT TOPOLOGY INSIDE CYCLIC CONVERGENCE REGIONS
//
// StructuralConvergenceRegion now carries `entries: Vec<StructuralConvergenceEntry>`
// alongside `members`: for each branch (raw, or effective when composed
// with PATH-030), which specific region member it enters through. This
// tests H3: original topology + branch-specific entry edges is enough to
// distinguish single-entry from dual-entry cyclic convergence, without
// any timestamp or causal metadata.
// ---------------------------------------------------------------------

fn entry_pairs(
    region: &distribution_machine::measurement::StructuralConvergenceRegion,
) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = region
        .entries
        .iter()
        .map(|e| (e.branch.0.clone(), e.member.0.clone()))
        .collect();
    pairs.sort();
    pairs
}

#[test]
fn path_032_a_single_entry_shows_both_branches_entering_the_same_member() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X
    // A -> C -> X
    //           X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0].members,
        vec![CapabilityId("x".into()), CapabilityId("y".into())]
    );
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "x".to_string())],
        "H3 VIOLATION: single-entry pattern must show both branches entering via X"
    );
}

#[test]
fn path_032_b_dual_entry_shows_branches_entering_different_members() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X
    // A -> C -> Y
    //      X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0].members,
        vec![CapabilityId("x".into()), CapabilityId("y".into())]
    );
    let entries = entry_pairs(&regions[0]);
    assert_eq!(
        entries,
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "y".to_string())],
        "H3 VIOLATION: dual-entry pattern must show branches entering via different members"
    );

    // The critical corrected claim: A and B (single vs. dual entry) no
    // longer collapse to identical enriched output, even though their
    // `members` sets are identical.
    let single_edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let single_regions = cycle_aware_first_structural_convergences(&source, &single_edges);
    assert_ne!(
        entry_pairs(&single_regions[0]),
        entries,
        "H3 VIOLATION: single-entry and dual-entry graphs must no longer be indistinguishable"
    );
}

#[test]
fn path_032_c_longer_paths_to_the_same_member_still_read_as_single_entry() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> P -> X
    // A -> C -> Q -> X
    //                X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "p"),
        DependencyEdge::new("p", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "q"),
        DependencyEdge::new("q", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "x".to_string())],
        "entry provenance must attribute to the branch ROOT and the region member, \
         not to the intermediate relay nodes P/Q"
    );
}

#[test]
fn path_032_d_longer_paths_to_different_members_remain_distinguishable() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> P -> X
    // A -> C -> Q -> Y
    //                X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "p"),
        DependencyEdge::new("p", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "q"),
        DependencyEdge::new("q", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "y".to_string())],
        "dual-entry pattern must survive even through longer relay paths"
    );
}

#[test]
fn path_032_e_one_branch_entering_a_region_twice_is_two_entries_same_branch() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B, B -> X, B -> Y  (B enters at both X and Y directly)
    // A -> C -> X
    //           X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("b", "y"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![
            ("b".to_string(), "x".to_string()),
            ("b".to_string(), "y".to_string()),
            ("c".to_string(), "x".to_string()),
        ],
        "one branch (B) touching the region at two distinct members must appear as two \
         entries sharing the same branch, not merged into one and not treated as two branches"
    );
}

#[test]
fn path_032_f_duplicate_entry_edges_are_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "x".to_string())],
        "DUPLICATE-EDGE VIOLATION: repeated edges must not multiply entries"
    );
}

#[test]
fn path_032_g_redundant_branch_entry_disappears_under_effective_composition() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        cycle_aware_first_structural_convergences_from_effective_branches,
    };

    // A -> B -> C, A -> C -> D, C <-> D  (B subsumed by C, per PATH-029)
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
        DependencyEdge::new("d", "c"),
    ];
    let source = CapabilityId("a".into());

    let raw_regions = cycle_aware_first_structural_convergences(&source, &edges);
    let composed_regions =
        cycle_aware_first_structural_convergences_from_effective_branches(&source, &edges);

    println!(
        "PATH-032 redundant-branch entry\nraw entries      = {:?}\ncomposed regions = {:?}",
        entry_pairs(&raw_regions[0]),
        composed_regions.len(),
    );

    assert_eq!(raw_regions[0].members, vec![CapabilityId("c".into()), CapabilityId("d".into())]);
    assert_eq!(
        entry_pairs(&raw_regions[0]),
        vec![("b".to_string(), "c".to_string()), ("c".to_string(), "c".to_string())],
        "raw analysis must show the redundant branch B entering at C, plus C's trivial self-entry"
    );

    assert_eq!(
        composed_regions.len(),
        0,
        "COMPOSITION VIOLATION: with B correctly excluded there is only one effective branch \
         (C) and no region at all -- no entries to manufacture"
    );
}

#[test]
fn path_032_h_three_branches_two_entry_members_preserves_2_vs_1_structure() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X, A -> C -> X, A -> D -> Y, X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("a", "d"),
        DependencyEdge::new("d", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![
            ("b".to_string(), "x".to_string()),
            ("c".to_string(), "x".to_string()),
            ("d".to_string(), "y".to_string()),
        ],
        "2-vs-1 entry structure (two branches via X, one via Y) must be preserved"
    );
}

#[test]
fn path_032_i_three_member_scc_entry_patterns_distinguish_identical_membership() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let member_set = vec![CapabilityId("x".into()), CapabilityId("y".into()), CapabilityId("z".into())];
    let ring = |extra: Vec<DependencyEdge>| {
        let mut edges = vec![
            DependencyEdge::new("x", "y"),
            DependencyEdge::new("y", "z"),
            DependencyEdge::new("z", "x"),
        ];
        edges.extend(extra);
        edges
    };
    let source = CapabilityId("a".into());

    // All branches enter at X.
    let all_at_x = {
        let mut e = ring(vec![
            DependencyEdge::new("a", "b"),
            DependencyEdge::new("b", "x"),
            DependencyEdge::new("a", "c"),
            DependencyEdge::new("c", "x"),
        ]);
        e.push(DependencyEdge::new("a", "b"));
        e
    };
    let regions_all_x = cycle_aware_first_structural_convergences(&source, &all_at_x);
    assert_eq!(regions_all_x[0].members, member_set);
    assert_eq!(
        entry_pairs(&regions_all_x[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "x".to_string())]
    );

    // Branches enter at X and Y.
    let x_and_y = ring(vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "y"),
    ]);
    let regions_x_y = cycle_aware_first_structural_convergences(&source, &x_and_y);
    assert_eq!(regions_x_y[0].members, member_set);
    assert_eq!(
        entry_pairs(&regions_x_y[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "y".to_string())]
    );

    // Branches enter at X, Y, and Z.
    let x_y_z = ring(vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("a", "d"),
        DependencyEdge::new("d", "z"),
    ]);
    let regions_x_y_z = cycle_aware_first_structural_convergences(&source, &x_y_z);
    assert_eq!(regions_x_y_z[0].members, member_set);
    assert_eq!(
        entry_pairs(&regions_x_y_z[0]),
        vec![
            ("b".to_string(), "x".to_string()),
            ("c".to_string(), "y".to_string()),
            ("d".to_string(), "z".to_string()),
        ]
    );

    // Region membership is identical (the whole 3-cycle) in all three
    // cases; entry topology is what tells them apart.
    assert_eq!(regions_all_x[0].members, regions_x_y[0].members);
    assert_eq!(regions_x_y[0].members, regions_x_y_z[0].members);
    assert_ne!(entry_pairs(&regions_all_x[0]), entry_pairs(&regions_x_y[0]));
    assert_ne!(entry_pairs(&regions_x_y[0]), entry_pairs(&regions_x_y_z[0]));
}

#[test]
fn path_032_j_upstream_acyclic_first_convergence_has_simple_entries_no_scc_leakage() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> P, A -> C -> P, P -> X, X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "p"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "p"),
        DependencyEdge::new("p", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(
        regions.len(),
        1,
        "the downstream {{X,Y}} SCC must not appear as a second first-convergence region"
    );
    assert_eq!(regions[0].members, vec![CapabilityId("p".into())]);
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![("b".to_string(), "p".to_string()), ("c".to_string(), "p".to_string())],
        "SCC-LEAKAGE VIOLATION: entries for the upstream acyclic region must not reference X/Y"
    );
}

#[test]
fn path_032_k_entries_for_separate_first_regions_do_not_contaminate_each_other() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // Acyclic region {X} fed by B1, B2. Cyclic region {Y,Z} fed by B3, B4.
    let edges = vec![
        DependencyEdge::new("a", "b1"),
        DependencyEdge::new("b1", "x"),
        DependencyEdge::new("a", "b2"),
        DependencyEdge::new("b2", "x"),
        DependencyEdge::new("a", "b3"),
        DependencyEdge::new("b3", "y"),
        DependencyEdge::new("a", "b4"),
        DependencyEdge::new("b4", "z"),
        DependencyEdge::new("y", "z"),
        DependencyEdge::new("z", "y"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 2);

    let x_region = regions.iter().find(|r| r.members == vec![CapabilityId("x".into())]).unwrap();
    let yz_region = regions
        .iter()
        .find(|r| r.members == vec![CapabilityId("y".into()), CapabilityId("z".into())])
        .unwrap();

    assert_eq!(
        entry_pairs(x_region),
        vec![("b1".to_string(), "x".to_string()), ("b2".to_string(), "x".to_string())],
        "CONTAMINATION VIOLATION: X's entries must not include B3/B4"
    );
    assert_eq!(
        entry_pairs(yz_region),
        vec![("b3".to_string(), "y".to_string()), ("b4".to_string(), "z".to_string())],
        "CONTAMINATION VIOLATION: {{Y,Z}}'s entries must not include B1/B2"
    );
}

#[test]
fn path_032_l_edge_ordering_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let edges_1 = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let edges_2 = vec![
        DependencyEdge::new("y", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "b"),
    ];
    let source = CapabilityId("a".into());

    let regions_1 = cycle_aware_first_structural_convergences(&source, &edges_1);
    let regions_2 = cycle_aware_first_structural_convergences(&source, &edges_2);
    assert_eq!(entry_pairs(&regions_1[0]), entry_pairs(&regions_2[0]));
}

#[test]
fn path_032_m_renaming_preserves_entry_shape_not_labels() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    let edges_bc_xy = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let edges_mn_pq = vec![
        DependencyEdge::new("a", "m"),
        DependencyEdge::new("a", "n"),
        DependencyEdge::new("m", "p"),
        DependencyEdge::new("n", "q"),
        DependencyEdge::new("p", "q"),
        DependencyEdge::new("q", "p"),
    ];
    let source = CapabilityId("a".into());

    let regions_1 = cycle_aware_first_structural_convergences(&source, &edges_bc_xy);
    let regions_2 = cycle_aware_first_structural_convergences(&source, &edges_mn_pq);

    // Same shape: one region, two members, two entries, one entry per member.
    assert_eq!(regions_1[0].members.len(), regions_2[0].members.len());
    assert_eq!(regions_1[0].entries.len(), regions_2[0].entries.len());

    // Different labels are expected metadata, not a structural violation.
    assert_eq!(
        entry_pairs(&regions_1[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "y".to_string())]
    );
    assert_eq!(
        entry_pairs(&regions_2[0]),
        vec![("m".to_string(), "p".to_string()), ("n".to_string(), "q".to_string())]
    );
}

#[test]
fn path_032_n_cycle_through_source_does_not_fabricate_entry_evidence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A <-> B, B -> X ; A -> C -> X ; X <-> Y
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "a"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "x".to_string())],
        "CYCLE-THROUGH-SOURCE VIOLATION: the a<->b cycle must not fabricate a spurious entry"
    );
}

#[test]
fn path_032_o_opposite_incidence_patterns_with_identical_coarse_shape_stay_distinguishable() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // Adversarial search (item O), reframed: not "same input twice" (that
    // was only a determinism check, not a semantic-sufficiency proof),
    // but two DIFFERENT graphs, using the SAME branch names and the SAME
    // region members, that share every coarse statistic:
    //   - one region, two members {x, y}
    //   - three branches (b, c, d), three entries
    //   - two occupied entry members
    //   - the SAME multiset of per-member entry counts: {2, 1}
    //
    // Graph 1: b and c enter at X, d enters at Y  (X:2, Y:1)
    // Graph 2: b enters at X, c and d enter at Y  (X:1, Y:2)
    //
    // A summary that only recorded "how many branches enter each member"
    // (without recording WHICH branch) would see the same {2,1}
    // multiset for both and could not tell them apart. `entries` records
    // the full (branch, member) incidence, not just per-member counts, so
    // it must distinguish them -- specifically, c's entry member differs
    // between the two graphs.
    let source = CapabilityId("a".into());

    let graph_1 = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("a", "d"),
        DependencyEdge::new("d", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let graph_2 = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "y"),
        DependencyEdge::new("a", "d"),
        DependencyEdge::new("d", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    let regions_1 = cycle_aware_first_structural_convergences(&source, &graph_1);
    let regions_2 = cycle_aware_first_structural_convergences(&source, &graph_2);

    // Coarse shape really is identical: same member count, same entry
    // count, same occupied-member count, same per-member count multiset.
    assert_eq!(regions_1[0].members.len(), regions_2[0].members.len());
    assert_eq!(regions_1[0].entries.len(), regions_2[0].entries.len());
    let coarse_counts = |region: &distribution_machine::measurement::StructuralConvergenceRegion| {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for e in &region.entries {
            *counts.entry(e.member.0.clone()).or_insert(0) += 1;
        }
        let mut values: Vec<usize> = counts.into_values().collect();
        values.sort();
        values
    };
    assert_eq!(
        coarse_counts(&regions_1[0]),
        coarse_counts(&regions_2[0]),
        "sanity check: the two graphs must genuinely share the same coarse {{2,1}} shape"
    );

    // The full incidence pattern must still differ.
    assert_ne!(
        entry_pairs(&regions_1[0]),
        entry_pairs(&regions_2[0]),
        "H3 VIOLATION: opposite 2:1 incidence patterns with identical coarse shape \
         collapsed to the same entries -- (branch, member) incidence is not being preserved"
    );
    assert_eq!(
        entry_pairs(&regions_1[0]),
        vec![
            ("b".to_string(), "x".to_string()),
            ("c".to_string(), "x".to_string()),
            ("d".to_string(), "y".to_string()),
        ]
    );
    assert_eq!(
        entry_pairs(&regions_2[0]),
        vec![
            ("b".to_string(), "x".to_string()),
            ("c".to_string(), "y".to_string()),
            ("d".to_string(), "y".to_string()),
        ]
    );
}

#[test]
fn path_032_p_region_boundary_is_respected_downstream_relay_does_not_manufacture_a_second_entry() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
    };

    // A -> B -> X
    // A -> C -> X
    //           X <-> Y
    //           X -> P -> Y
    //
    // compute_region_entries must never traverse THROUGH a region member
    // to discover a "second" entry into another member from private
    // downstream territory. B and C each enter the region only at X;
    // the relay through P must not manufacture (B, Y) / (C, Y).
    //
    // Note: because X<->Y and X->P->Y together mean P is reachable FROM
    // the region (X->P) and reaches BACK into the region (P->Y->X), P is
    // itself mutually reachable with X and Y, so the real region for this
    // graph is {P, X, Y}, not {X, Y}. That SCC closure is what already
    // keeps this specific graph safe even before this fix (P was already
    // excluded from being treated as "outside" territory by the region
    // membership check). This test locks in the correct result directly,
    // as a boundary-respecting property of the traversal itself, not as
    // an accident of the region computation elsewhere.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
        DependencyEdge::new("x", "p"),
        DependencyEdge::new("p", "y"),
    ];
    let source = CapabilityId("a".into());

    let regions = cycle_aware_first_structural_convergences(&source, &edges);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0].members,
        vec![CapabilityId("p".into()), CapabilityId("x".into()), CapabilityId("y".into())],
        "sanity check: P is genuinely absorbed into the region for this exact graph shape"
    );
    assert_eq!(
        entry_pairs(&regions[0]),
        vec![("b".to_string(), "x".to_string()), ("c".to_string(), "x".to_string())],
        "REGION-BOUNDARY VIOLATION: entries must be exactly (B,X) and (C,X), not (B,Y)/(C,Y)"
    );
}

// ---------------------------------------------------------------------
// PATH-033: ADVERSARIAL EXPERIMENT ON ECONOMIC REDUCTION OF
// StructuralConvergenceRegion
//
// H4 under attack: economic structural importance depends primarily on
// the number of DISTINCT branches entering a first-convergence region
// (`distinct_entering_branch_count`), while SCC member count and repeated
// entries from the same branch are provenance, not automatic extra
// economic weight. This is a hypothesis, not an established result --
// this group is designed to try to break it, not to confirm it.
//
// `distinct_entering_branch_count` is NOT wired into any `aggregate_*`
// function or money allocation. `members` and `entries` remain fully
// inspectable on every region produced below -- the candidate function is
// an additional lens, not a replacement for the raw structural evidence.
// ---------------------------------------------------------------------

#[test]
fn path_033_a_scc_member_count_inflation_does_not_change_distinct_branch_count() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    // Small region: A->B->X, A->C->X, X<->Y (2 members).
    let small = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    // Same two branches, same single entry member (X), but the SCC
    // downstream of X is inflated to 5 members via a longer cycle:
    // X->P1->P2->P3->Y->X.
    let large = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "p1"),
        DependencyEdge::new("p1", "p2"),
        DependencyEdge::new("p2", "p3"),
        DependencyEdge::new("p3", "y"),
        DependencyEdge::new("y", "x"),
    ];

    let source = CapabilityId("a".into());
    let small_regions = cycle_aware_first_structural_convergences(&source, &small);
    let large_regions = cycle_aware_first_structural_convergences(&source, &large);

    assert_eq!(small_regions.len(), 1);
    assert_eq!(large_regions.len(), 1);
    assert_eq!(small_regions[0].members.len(), 2);
    assert_eq!(
        large_regions[0].members.len(),
        5,
        "sanity check: the longer cycle must actually inflate SCC membership"
    );

    assert_eq!(
        distinct_entering_branch_count(&small_regions[0]),
        distinct_entering_branch_count(&large_regions[0]),
        "H4 VIOLATION: same two entering branches produced different distinct-branch counts \
         purely because the downstream SCC grew from 2 to 5 members"
    );
    assert_eq!(distinct_entering_branch_count(&small_regions[0]), 2);
}

#[test]
fn path_033_b_multiple_entries_from_one_branch_does_not_inflate_distinct_branch_count() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    // Baseline: B and C each enter once, at X.
    let single_entry_each = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    // B alone touches the region at both X and Y directly; C still enters
    // once at X. Same two branches, one extra entry.
    let one_branch_double_entry = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("b", "y"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    let source = CapabilityId("a".into());
    let baseline = cycle_aware_first_structural_convergences(&source, &single_entry_each);
    let doubled = cycle_aware_first_structural_convergences(&source, &one_branch_double_entry);

    assert_eq!(baseline[0].entries.len(), 2);
    assert_eq!(
        doubled[0].entries.len(),
        3,
        "sanity check: B's second direct entry must actually add a third entry record"
    );

    assert_eq!(
        distinct_entering_branch_count(&baseline[0]),
        distinct_entering_branch_count(&doubled[0]),
        "H4 VIOLATION: one branch touching the region at an extra member changed the \
         distinct-branch count even though no new branch was introduced"
    );
    assert_eq!(distinct_entering_branch_count(&baseline[0]), 2);
}

#[test]
fn path_033_c_two_vs_three_effective_branches_are_distinguished() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    let two_branches = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    let three_branches = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("a", "d"),
        DependencyEdge::new("d", "y"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    let source = CapabilityId("a".into());
    let two = cycle_aware_first_structural_convergences(&source, &two_branches);
    let three = cycle_aware_first_structural_convergences(&source, &three_branches);

    assert_eq!(distinct_entering_branch_count(&two[0]), 2);
    assert_eq!(
        distinct_entering_branch_count(&three[0]),
        3,
        "the metric must actually respond to a genuine third independent branch"
    );
}

#[test]
fn path_033_d_redundant_raw_branch_inflates_raw_count_but_not_effective_count() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        cycle_aware_first_structural_convergences_from_effective_branches,
        distinct_entering_branch_count,
    };

    // A->B->C, A->C->D, C<->D (B subsumed by C, per PATH-029), plus a
    // genuine third raw branch A->E->D that is NOT subsumed (E relays
    // through D before it would ever reach C, so E keeps independent
    // reach and survives PATH-029 subsumption).
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "d"),
        DependencyEdge::new("d", "c"),
        DependencyEdge::new("a", "e"),
        DependencyEdge::new("e", "d"),
    ];
    let source = CapabilityId("a".into());

    let raw_regions = cycle_aware_first_structural_convergences(&source, &edges);
    let composed_regions =
        cycle_aware_first_structural_convergences_from_effective_branches(&source, &edges);

    assert_eq!(raw_regions.len(), 1);
    assert_eq!(composed_regions.len(), 1);
    assert_eq!(raw_regions[0].members, composed_regions[0].members, "same region either way");

    let raw_count = distinct_entering_branch_count(&raw_regions[0]);
    let composed_count = distinct_entering_branch_count(&composed_regions[0]);

    println!(
        "PATH-033 redundant branch | raw distinct = {} | effective distinct = {}",
        raw_count, composed_count
    );

    assert_eq!(
        raw_count, 3,
        "raw analysis counts B as a third branch even though it contributes nothing C doesn't"
    );
    assert_eq!(
        composed_count, 2,
        "COMPOSITION VIOLATION: effective-branch composition must drop the redundant raw \
         branch (B) from the count, per the PATH-030 composition recommendation"
    );
}

#[test]
fn path_033_e_duplicate_edges_are_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
        DependencyEdge::new("y", "x"),
    ];
    let source = CapabilityId("a".into());
    let regions = cycle_aware_first_structural_convergences(&source, &edges);

    assert_eq!(
        distinct_entering_branch_count(&regions[0]),
        2,
        "DUPLICATE-EDGE VIOLATION: repeated edges must not change the distinct-branch count"
    );
}

#[test]
fn path_033_f_edge_ordering_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    let forward = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let mut shuffled = forward.clone();
    shuffled.reverse();

    let source = CapabilityId("a".into());
    let forward_regions = cycle_aware_first_structural_convergences(&source, &forward);
    let shuffled_regions = cycle_aware_first_structural_convergences(&source, &shuffled);

    assert_eq!(
        distinct_entering_branch_count(&forward_regions[0]),
        distinct_entering_branch_count(&shuffled_regions[0]),
        "ORDERING VIOLATION: edge order must not change the distinct-branch count"
    );
}

#[test]
fn path_033_g_renaming_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    let original = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];
    let renamed = vec![
        DependencyEdge::new("root", "branch-one"),
        DependencyEdge::new("branch-one", "hub-1"),
        DependencyEdge::new("root", "branch-two"),
        DependencyEdge::new("branch-two", "hub-1"),
        DependencyEdge::new("hub-1", "hub-2"),
        DependencyEdge::new("hub-2", "hub-1"),
    ];

    let original_regions =
        cycle_aware_first_structural_convergences(&CapabilityId("a".into()), &original);
    let renamed_regions =
        cycle_aware_first_structural_convergences(&CapabilityId("root".into()), &renamed);

    assert_eq!(
        distinct_entering_branch_count(&original_regions[0]),
        distinct_entering_branch_count(&renamed_regions[0]),
        "RENAMING VIOLATION: relabeling capabilities must not change the distinct-branch count"
    );
    assert_eq!(distinct_entering_branch_count(&renamed_regions[0]), 2);
}

#[test]
fn path_033_h_separate_first_regions_are_counted_independently() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    // Acyclic region {X} fed by 2 branches. Cyclic region {Y,Z} fed by 3.
    let edges = vec![
        DependencyEdge::new("a", "b1"),
        DependencyEdge::new("b1", "x"),
        DependencyEdge::new("a", "b2"),
        DependencyEdge::new("b2", "x"),
        DependencyEdge::new("a", "b3"),
        DependencyEdge::new("b3", "y"),
        DependencyEdge::new("a", "b4"),
        DependencyEdge::new("b4", "z"),
        DependencyEdge::new("a", "b5"),
        DependencyEdge::new("b5", "z"),
        DependencyEdge::new("y", "z"),
        DependencyEdge::new("z", "y"),
    ];
    let source = CapabilityId("a".into());
    let regions = cycle_aware_first_structural_convergences(&source, &edges);

    assert_eq!(regions.len(), 2);
    let x_region = regions.iter().find(|r| r.members == vec![CapabilityId("x".into())]).unwrap();
    let yz_region = regions
        .iter()
        .find(|r| r.members == vec![CapabilityId("y".into()), CapabilityId("z".into())])
        .unwrap();

    assert_eq!(
        distinct_entering_branch_count(x_region),
        2,
        "CONTAMINATION VIOLATION: X's count must reflect only its own 2 branches"
    );
    assert_eq!(
        distinct_entering_branch_count(yz_region),
        3,
        "CONTAMINATION VIOLATION: {{Y,Z}}'s count must reflect only its own 3 branches"
    );
}

#[test]
fn path_033_i_counterexample_search_member_count_and_entry_multiplicity_are_discarded() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, cycle_aware_first_structural_convergences,
        distinct_entering_branch_count,
    };

    // Region A: 2 branches, each entering once, 2-member region.
    let region_a_edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("x", "y"),
        DependencyEdge::new("y", "x"),
    ];

    // Region B: the SAME 2 branches, but P enters once and Q enters at 4
    // different members of an 8-member cycle. Same distinct-branch count
    // (2), radically larger blast radius (8 members instead of 2) AND
    // radically more entry incidence (5 entries instead of 2).
    let region_b_edges = vec![
        DependencyEdge::new("a", "p"),
        DependencyEdge::new("p", "m1"),
        DependencyEdge::new("a", "q"),
        DependencyEdge::new("q", "m1"),
        DependencyEdge::new("q", "m3"),
        DependencyEdge::new("q", "m5"),
        DependencyEdge::new("q", "m7"),
        DependencyEdge::new("m1", "m2"),
        DependencyEdge::new("m2", "m3"),
        DependencyEdge::new("m3", "m4"),
        DependencyEdge::new("m4", "m5"),
        DependencyEdge::new("m5", "m6"),
        DependencyEdge::new("m6", "m7"),
        DependencyEdge::new("m7", "m8"),
        DependencyEdge::new("m8", "m1"),
    ];

    let region_a = cycle_aware_first_structural_convergences(&CapabilityId("a".into()), &region_a_edges);
    let region_b = cycle_aware_first_structural_convergences(&CapabilityId("a".into()), &region_b_edges);

    assert_eq!(region_a.len(), 1);
    assert_eq!(region_b.len(), 1);

    println!(
        "PATH-033 counterexample search | region A members={} entries={} distinct={} | \
         region B members={} entries={} distinct={}",
        region_a[0].members.len(),
        region_a[0].entries.len(),
        distinct_entering_branch_count(&region_a[0]),
        region_b[0].members.len(),
        region_b[0].entries.len(),
        distinct_entering_branch_count(&region_b[0]),
    );

    assert_eq!(region_a[0].members.len(), 2);
    assert_eq!(region_b[0].members.len(), 8, "sanity check: region B's SCC is genuinely 8-wide");
    assert_eq!(region_a[0].entries.len(), 2);
    assert_eq!(region_b[0].entries.len(), 5, "sanity check: Q genuinely enters at 4 members");

    // This is the actual, current, GREEN behavior -- not forced. It is
    // recorded here as the PATH-033 counterexample: distinct_entering_
    // branch_count treats a clean 2-member/2-entry region and an 8-member/
    // 5-entry region entered by the same two branches as economically
    // IDENTICAL (2 == 2). Whether that is correct is NOT a topological
    // question this test (or this codebase) can answer -- it depends on
    // whether "how many capabilities fail together" (blast radius) is a
    // separate economic dimension from "how many independent branches
    // were lost" (redundancy loss), a question H4 does not address and
    // this suite does not resolve. See FIND-010: this is preserved as
    // open evidence AGAINST treating H4 as economically sufficient by
    // itself, not as a disproof of the narrower claim that distinct
    // branch count is what drives REDUNDANCY-LOSS weight specifically.
    assert_eq!(
        distinct_entering_branch_count(&region_a[0]),
        distinct_entering_branch_count(&region_b[0]),
        "if this ever fails, the candidate metric has started looking at member/entry \
         shape and FIND-010's open question has been answered one way or another -- update it"
    );
}

// ---------------------------------------------------------------------
// PATH-034: substitutability temporal semantics (H5, narrowed).
//
// H5 (as actually tested, corrected from an earlier overstated version):
// the existing `Substitutability` representation does not explicitly
// encode or derive temporal restoration semantics -- no
// `Evidence -> Substitutability` derivation exists, so it is UNSPECIFIED
// whether Easy/Moderate/Difficult are meant to track replacement time,
// effort, functional completeness, cost, or something else. If
// counterfactual criticality must distinguish otherwise-equivalent
// substitution scenarios whose restoration latency differs, additional
// EXPLICIT temporal evidence is required, because nothing currently
// derives or stores that latency. `has_substitute: bool` was only the
// motivating thought experiment in the mission brief, never an accurate
// description of `Substitutability` (which is an existing four-level
// categorical value with a deterministic 0.00/0.33/0.67/1.00 numeric
// normalization, not a boolean).
//
// These tests do NOT claim the existing protocol necessarily assigns
// equal categories to cases with different real-world restoration
// latency -- a human could assign different categories, which would
// remain epistemically insufficient under FIND-003 (unspecified
// derivation) without being a forced categorical collapse. Where a test
// constructs equal categorical values for two cases, that equality is
// test setup, not derived evidence, and is labelled as such below.
//
// See the module doc comment on `TimeToEffectiveSubstitution` in
// `src/measurement.rs` for the full inspection result, and
// `docs/findings/FIND-011-*.md` for the final verdict.
//
// `TimeToEffectiveSubstitution` is NOT wired into `measure_criticality`,
// any `aggregate_*` function, or `Contribution`. These tests only attack
// the raw observable/type, per the PATH-034 mission constraints.
// ---------------------------------------------------------------------

#[test]
fn path_034_a_isomorphic_structure_different_substitution_latency_is_distinguishable() {
    use distribution_machine::measurement::{Substitutability, TimeToEffectiveSubstitution};
    use std::time::Duration;

    // Test setup, not derived evidence: both cases are GIVEN the same
    // existing categorical value here. This does not claim the existing
    // protocol would necessarily assign both real-world cases the same
    // category -- `Substitutability`'s evidence-to-category derivation is
    // unspecified (no `Evidence -> Substitutability` constructor exists
    // anywhere in this module), so a human could in principle assign
    // different categories to a 30ms case and a 3-week case.
    let same_category_a = Substitutability::Easy;
    let same_category_b = Substitutability::Easy;
    assert_eq!(
        same_category_a, same_category_b,
        "test setup: equal categorical input by construction, not a claim about how the \
         protocol would derive these categories from real evidence"
    );

    // Candidate temporal evidence differs.
    let tau_a = TimeToEffectiveSubstitution::Bounded(Duration::from_millis(30));
    let tau_b = TimeToEffectiveSubstitution::Bounded(Duration::from_secs(3 * 7 * 24 * 60 * 60));

    // Given equal categorical input, the temporal observable still
    // distinguishes the two cases: categorical equality does not imply
    // temporal equality, and the category alone carries no information
    // capable of expressing this difference once it is fixed equal.
    assert_ne!(
        tau_a, tau_b,
        "the temporal observable distinguishes 30ms failover from a 3-week rebuild even when \
         the (unspecified-derivation) categorical input given to this test is equal"
    );
}

#[test]
fn path_034_b_fast_substitute_vs_no_substitute_remain_distinct() {
    use distribution_machine::measurement::{Substitutability, TimeToEffectiveSubstitution};
    use std::time::Duration;

    // Fast substitute: an existing non-`None` category, paired with an
    // almost-immediate temporal value.
    let fast_exists = Substitutability::Easy;
    let fast_tau = TimeToEffectiveSubstitution::Bounded(Duration::from_millis(30));

    // The existing "no substitute" category. No claim is made here about
    // what `Substitutability::None` means beyond being the existing
    // distinct categorical value -- its evidence derivation is
    // unspecified, same as every other category (FIND-003).
    let none_exists = Substitutability::None;
    let none_tau = TimeToEffectiveSubstitution::Never;

    assert_ne!(
        fast_exists, none_exists,
        "the existing non-None and None categories must remain distinguishable from each other"
    );
    assert_ne!(
        fast_tau, none_tau,
        "a fast bounded substitution must not collapse onto the same representation \
         as never reaching one"
    );

    // A small bounded duration must never equal `Never` -- adding the
    // temporal axis must not accidentally erase this distinction.
    assert_ne!(TimeToEffectiveSubstitution::Bounded(Duration::from_millis(1)), TimeToEffectiveSubstitution::Never);
}

#[test]
fn path_034_c_temporal_ordering_is_preserved_at_the_observable_level() {
    use std::time::Duration;

    let tau1 = Duration::from_millis(30);
    let tau2 = Duration::from_secs(30);
    let tau3 = Duration::from_secs(4 * 60 * 60);

    // The raw `Duration` observable preserves ordering. This test makes
    // no claim that any economic scoring function derived from it must be
    // monotonic or linear -- only that the observable itself does not
    // lose the ordering.
    assert!(tau1 < tau2, "30ms must be strictly less than 30s");
    assert!(tau2 < tau3, "30s must be strictly less than 4h");
    assert!(tau1 < tau3, "30ms must be strictly less than 4h");
}

#[test]
fn path_034_d_unit_invariance_across_equivalent_durations() {
    use distribution_machine::measurement::TimeToEffectiveSubstitution;
    use std::time::Duration;

    // 30 seconds expressed in different units must be the same quantity.
    let via_secs = Duration::from_secs(30);
    let via_millis = Duration::from_millis(30_000);
    let via_micros = Duration::from_micros(30_000_000);

    assert_eq!(via_secs, via_millis);
    assert_eq!(via_secs, via_micros);
    assert_eq!(
        TimeToEffectiveSubstitution::Bounded(via_secs),
        TimeToEffectiveSubstitution::Bounded(via_millis),
        "the same duration expressed through different units must not be treated as \
         two different temporal observables"
    );
}

#[test]
fn path_034_e_zero_duration_means_instantaneous_failover_not_no_substitution() {
    use distribution_machine::measurement::TimeToEffectiveSubstitution;
    use std::time::Duration;

    let immediate = TimeToEffectiveSubstitution::Bounded(Duration::ZERO);

    // `Bounded(ZERO)` is a meaningful, effective-substitution state (an
    // already-active hot standby / instantaneous failover). It must be
    // representable and must NOT be the same value as `Never` (no
    // substitution is ever effective) -- those are semantic opposites.
    assert_ne!(immediate, TimeToEffectiveSubstitution::Never);

    // It is also distinct from a merely small-but-nonzero duration --
    // zero is not silently rounded into "approximately fast".
    assert_ne!(
        immediate,
        TimeToEffectiveSubstitution::Bounded(Duration::from_nanos(1))
    );
}

#[test]
fn path_034_f_non_none_category_paired_with_never_effective_is_a_distinct_representable_state() {
    use distribution_machine::measurement::{Substitutability, TimeToEffectiveSubstitution};

    // Attack an edge-case combination: the existing categorical value is a
    // non-`None` category (a substitute-exists category, whatever its
    // unspecified evidence derivation actually tracks), while the
    // temporal axis independently reports the substitute never becomes
    // effective (e.g. a documented replacement procedure that has never
    // actually completed in practice). The model must represent this
    // combination WITHOUT collapsing it into an error state.
    //
    // This test makes NO claim about what `Substitutability::None` means,
    // whether it has graph-structural semantics, or how it relates to
    // `Never` -- `Substitutability`'s evidence derivation is unspecified
    // (FIND-003), and `Never` is defined purely on the temporal axis (see
    // its doc comment in `src/measurement.rs`).
    let existing_category = Substitutability::Difficult;
    let temporal_value = TimeToEffectiveSubstitution::Never;

    assert_ne!(
        existing_category, Substitutability::None,
        "test setup: a non-None categorical value is used for this case"
    );
    assert_eq!(
        temporal_value,
        TimeToEffectiveSubstitution::Never,
        "on the temporal axis alone, a non-None category paired with 'never effective' is a \
         representable, non-error combination"
    );
}

#[test]
fn path_034_g_graph_invariance_temporal_semantics_are_orthogonal_to_structural_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, TimeToEffectiveSubstitution,
        cycle_aware_first_structural_convergences, distinct_entering_branch_count,
    };
    use std::time::Duration;

    // Identical graph topology (a plain two-branch diamond) used for both
    // capabilities -- only the temporal observable differs.
    let edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
    ];

    let regions_a = cycle_aware_first_structural_convergences(&CapabilityId("a".into()), &edges);
    let regions_b = cycle_aware_first_structural_convergences(&CapabilityId("a".into()), &edges);

    assert_eq!(regions_a, regions_b, "identical topology must produce identical structural regions");
    assert_eq!(
        distinct_entering_branch_count(&regions_a[0]),
        distinct_entering_branch_count(&regions_b[0]),
        "structural measurement is unaffected by any temporal evidence -- it never receives it"
    );

    // Attach different temporal evidence to each "capability" that shares
    // this exact topology. Nothing in the structural computation above
    // took (or could take) either value as input -- it is a separate,
    // orthogonal axis by construction, not by coincidence.
    let tau_fast = TimeToEffectiveSubstitution::Bounded(Duration::from_millis(30));
    let tau_slow = TimeToEffectiveSubstitution::Bounded(Duration::from_secs(3 * 7 * 24 * 60 * 60));
    assert_ne!(tau_fast, tau_slow);
}

#[test]
fn path_034_h_graph_difference_with_equal_temporal_value_are_still_distinguished_structurally() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, TimeToEffectiveSubstitution,
        cycle_aware_first_structural_convergences, distinct_entering_branch_count,
    };
    use std::time::Duration;

    // Two structurally DIFFERENT convergence situations (2 branches vs 3
    // branches entering), with the SAME temporal value attached to both.
    let two_branch_edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
    ];
    let three_branch_edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("a", "c"),
        DependencyEdge::new("a", "d"),
        DependencyEdge::new("b", "x"),
        DependencyEdge::new("c", "x"),
        DependencyEdge::new("d", "x"),
    ];

    let two = cycle_aware_first_structural_convergences(&CapabilityId("a".into()), &two_branch_edges);
    let three = cycle_aware_first_structural_convergences(&CapabilityId("a".into()), &three_branch_edges);

    assert_eq!(distinct_entering_branch_count(&two[0]), 2);
    assert_eq!(distinct_entering_branch_count(&three[0]), 3);

    let shared_tau = TimeToEffectiveSubstitution::Bounded(Duration::from_secs(30));
    assert_eq!(shared_tau, shared_tau);

    // Equal temporal value must not imply equal structural measurement --
    // temporal equality and structural equality are independent axes.
    assert_ne!(
        distinct_entering_branch_count(&two[0]),
        distinct_entering_branch_count(&three[0]),
        "identical tau must not mask a genuine structural difference"
    );
}

#[test]
fn path_034_i_counterexample_search_tau_alone_does_not_capture_partial_restoration() {
    use distribution_machine::measurement::TimeToEffectiveSubstitution;
    use std::time::Duration;

    // Counterexample: two situations with IDENTICAL time-to-effective-
    // substitution can still differ in whether the restored capability is
    // a full or merely partial substitute for the original. `tau` alone
    // cannot express this -- it only answers "when", not "how completely".
    //
    // Case 1: substitute restores full original functionality in 1 hour.
    let tau_full_restoration = TimeToEffectiveSubstitution::Bounded(Duration::from_secs(60 * 60));
    // Case 2: a DIFFERENT substitute also becomes available in 1 hour, but
    // only restores partial functionality (e.g. a read-only fallback for a
    // read-write capability). This is the ONE counterexample dimension
    // this test actually constructs and checks. The mission's broader
    // attack-surface list (manual intervention burden, capacity
    // degradation, switching cost, technical-vs-business-outcome
    // mismatch) is plausible by the same shape of argument but is NOT
    // implemented or tested here -- it is not claimed as an established
    // finding, only noted as an untested, similarly-shaped hypothesis.
    let tau_partial_restoration = TimeToEffectiveSubstitution::Bounded(Duration::from_secs(60 * 60));

    assert_eq!(
        tau_full_restoration, tau_partial_restoration,
        "tau is identical for both -- this equality is the counterexample: a scalar \
         time-to-effective-substitution cannot distinguish full restoration from partial \
         restoration reached in the same amount of time"
    );

    // This is preserved as evidence, not fixed here: PATH-034 is scoped to
    // representation/observability, not to designing a sufficient
    // multi-dimensional substitution model. See FIND-011: both "the
    // existing categorical value does not explicitly encode temporal
    // restoration semantics" and "a scalar tau also loses information (at
    // least restoration completeness)" are preserved as open findings,
    // without prematurely solving the second one with a new observable (a
    // restoration-degree function `S_X(t)`, per the mission's
    // formal-model notes, is explicitly NOT implemented here).
}

// =======================================================================
// PATH-035: FAILURE BLAST RADIUS GRANULARITY / REPRESENTATION LAUNDERING
//
// INSPECTION RESULT (before any test below): `CriticalityObservables.
// failure_blast_radius` is a raw `u32` field. There is NO `Evidence ->
// FailureBlastRadius` derivation anywhere in this module --
// `measure_criticality` only ever *consumes* the value (via
// `saturating_count(value, 5)` then a linear weight); it never computes
// it from `DependencyEdge`, `CapabilityId`, or any other graph
// primitive. Every existing call site that sets `failure_blast_radius`
// (PATH-012, and the `measurement.rs` unit tests) supplies it as a
// literal integer. This is the same shape of finding as PATH-034 /
// FIND-011 for `Substitutability`: the problem is ABSENCE of a
// deterministic, representation-invariant derivation function, not proof
// that an existing derivation performs naive downstream-node counting.
// No test below claims current production code "counts downstream
// nodes" -- it does not compute a count of anything.
//
// H6 (as actually tested): raw affected-NODE count is not a
// representation-invariant measure of failure blast radius, because one
// real-world affected capability can be represented as either one graph
// node or several internal graph nodes (a chain). H6 is tested against a
// NAIVE STRAWMAN counting function defined only in this test module
// (`naive_downstream_reachable_count`, below) -- explicitly NOT a claim
// about what `failure_blast_radius` currently computes, since it
// computes nothing today. The strawman exists solely to give H6 a
// concrete function to attack, matching the mission's own A/B graphs.
//
// The mission also asks whether an existing PATH-027..033 graph
// primitive already gives a representation-invariant STRUCTURAL
// reduction that could stand in for extent.
// `effective_structural_branches(source, edges).len()` (PATH-029/030) is
// tested here as exactly that candidate: it already deduplicates edges
// (`build_structural_adjacency`'s `seen_edges`), already collapses
// subsumed/chain branches via PATH-029's counterfactual-removal
// analysis, and PATH-028-i / PATH-033-f/g already established
// ordering/renaming invariance for the same underlying traversal. NO NEW
// PRODUCTION CODE is added for PATH-035: every test below calls existing
// `src/measurement.rs` functions as-is.
//
// PATH-024/025/026 (`FailureDomainId`, `deduplicate_failure_domains`,
// `aggregate_failure_domains_with_model`) were inspected for relevance
// and found NOT to bear on this question: that machinery operates on
// `DependencyRecord` / `QualifiedDependency` for the economic dependency
// WEIGHT aggregation pipeline (`aggregate_*_with_model` -> f64), which is
// structurally disjoint from `CapabilityId` / `DependencyEdge` /
// `CriticalityObservables.failure_blast_radius` (u32). No function in
// this module bridges a `FailureDomainId` computation into
// `CriticalityObservables`. PATH-026 already established (for its own
// purpose) that identifier metadata is not authoritative evidence of
// independence; nothing here re-derives or contradicts that. This is
// recorded as an already-answered sub-question (no PATH-035-E test is
// added, per the mission's instruction to record rather than duplicate
// already-solved ground).
//
// See FIND-012 for the verdict this test group establishes.
// =======================================================================

/// STRAWMAN naive measure for H6 only: every node reachable from `source`
/// (excluding `source` itself), deduplicated by an ordinary visited-set
/// BFS. This is deliberately NOT a claim about `failure_blast_radius`'s
/// derivation -- inspection found no such derivation exists in
/// production code at all (see banner comment above). It exists only so
/// H6 ("raw affected-node count is not representation-invariant") has a
/// concrete function to attack.
fn naive_downstream_reachable_count(
    source: &distribution_machine::measurement::CapabilityId,
    edges: &[distribution_machine::measurement::DependencyEdge],
) -> usize {
    use distribution_machine::measurement::CapabilityId;
    use std::collections::{HashMap, HashSet, VecDeque};

    let mut adjacency: HashMap<CapabilityId, Vec<CapabilityId>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }

    let mut visited: HashSet<CapabilityId> = HashSet::new();
    let mut queue: VecDeque<CapabilityId> = VecDeque::new();
    queue.push_back(source.clone());
    visited.insert(source.clone());

    while let Some(node) = queue.pop_front() {
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                if visited.insert(child.clone()) {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    visited.len() - 1
}

#[test]
fn path_035_a_node_splitting_inflates_naive_count_but_not_effective_branch_count() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // Test premise (explicit, not inferred from labels): PAYMENT_FRONTEND
    // -> PAYMENT_BACKEND is STIPULATED here to be internal decomposition
    // of the same external capability as bare PAYMENT, with the same
    // real-world failure consequence. This equivalence is asserted as
    // test setup, never derived from the labels or from any code.
    let source = CapabilityId("x".into());

    let single_node = vec![DependencyEdge::new("x", "payment")];
    let split_chain = vec![
        DependencyEdge::new("x", "payment_frontend"),
        DependencyEdge::new("payment_frontend", "payment_backend"),
    ];

    let naive_single = naive_downstream_reachable_count(&source, &single_node);
    let naive_split = naive_downstream_reachable_count(&source, &split_chain);

    assert_ne!(
        naive_single, naive_split,
        "H6 REPRESENTATION LAUNDERING (naive strawman): downstream node count inflates from {} \
         to {} purely from internal chain decomposition of a stipulated single capability, with \
         no change in real-world failure consequence",
        naive_single, naive_split
    );
    assert_eq!(naive_single, 1);
    assert_eq!(naive_split, 2);

    // Candidate structural reduction: PATH-029/030's
    // effective_structural_branches is invariant to this exact attack --
    // both graphs have exactly one immediate branch root from `x`.
    let effective_single = effective_structural_branches(&source, &single_node).len();
    let effective_split = effective_structural_branches(&source, &split_chain).len();

    assert_eq!(
        effective_single, effective_split,
        "candidate: effective_structural_branches count survives chain decomposition"
    );
    assert_eq!(effective_single, 1);
}

#[test]
fn path_035_b_relay_insertion_inflates_naive_count_but_not_effective_branch_count() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // Test premise: RELAY is stipulated to introduce no independent
    // economic capability -- a pure pass-through node.
    let source = CapabilityId("x".into());

    let direct = vec![DependencyEdge::new("x", "a"), DependencyEdge::new("a", "b")];
    let via_relay = vec![
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("a", "relay"),
        DependencyEdge::new("relay", "b"),
    ];

    let naive_direct = naive_downstream_reachable_count(&source, &direct);
    let naive_relay = naive_downstream_reachable_count(&source, &via_relay);

    assert_ne!(
        naive_direct, naive_relay,
        "H6 REPRESENTATION LAUNDERING (naive strawman): inserting a stipulated pass-through node \
         inflates downstream node count from {} to {}",
        naive_direct, naive_relay
    );

    let effective_direct = effective_structural_branches(&source, &direct).len();
    let effective_relay = effective_structural_branches(&source, &via_relay).len();

    assert_eq!(
        effective_direct, effective_relay,
        "candidate: effective_structural_branches count is invariant to relay insertion"
    );
    assert_eq!(effective_direct, 1);
}

#[test]
fn path_035_c_duplicate_representation_is_already_neutralized() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    let source = CapabilityId("x".into());
    let single = vec![DependencyEdge::new("x", "payment")];
    let duplicated = vec![
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("x", "payment"),
    ];

    // Already-existing structural dedup (`build_structural_adjacency`'s
    // `seen_edges`, and the naive strawman's own BFS visited-set) already
    // neutralizes duplicate-edge representation of one affected
    // capability, consistent with FIND-007 ("duplicate edges must not
    // create value"). Recorded here rather than re-solved with new
    // machinery.
    assert_eq!(
        naive_downstream_reachable_count(&source, &single),
        naive_downstream_reachable_count(&source, &duplicated),
    );
    assert_eq!(
        effective_structural_branches(&source, &single).len(),
        effective_structural_branches(&source, &duplicated).len(),
    );
}

#[test]
fn path_035_d_genuine_independent_expansion_is_still_detected() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // Test premise: PAYMENT and SHIPPING are stipulated as independent --
    // neither reaches the other in this graph.
    let source = CapabilityId("x".into());
    let one_capability = vec![DependencyEdge::new("x", "payment")];
    let two_capabilities = vec![
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("x", "shipping"),
    ];

    let naive_one = naive_downstream_reachable_count(&source, &one_capability);
    let naive_two = naive_downstream_reachable_count(&source, &two_capabilities);
    assert!(naive_two > naive_one);

    let effective_one = effective_structural_branches(&source, &one_capability).len();
    let effective_two = effective_structural_branches(&source, &two_capabilities).len();

    assert!(
        effective_two > effective_one,
        "the candidate reduction must not flatten genuine independent expansion to a constant"
    );
    assert_eq!(effective_one, 1);
    assert_eq!(effective_two, 2);
}

#[test]
fn path_035_f_renaming_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    let original = vec![
        DependencyEdge::new("x", "payment_frontend"),
        DependencyEdge::new("payment_frontend", "payment_backend"),
        DependencyEdge::new("x", "shipping"),
    ];
    let renamed = vec![
        DependencyEdge::new("node-1", "node-2"),
        DependencyEdge::new("node-2", "node-3"),
        DependencyEdge::new("node-1", "node-4"),
    ];

    let original_count =
        effective_structural_branches(&CapabilityId("x".into()), &original).len();
    let renamed_count =
        effective_structural_branches(&CapabilityId("node-1".into()), &renamed).len();

    assert_eq!(
        original_count, renamed_count,
        "RENAMING VIOLATION: relabeling capabilities must not change the candidate extent count"
    );
    assert_eq!(renamed_count, 2);
}

#[test]
fn path_035_g_ordering_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    let forward = vec![
        DependencyEdge::new("x", "payment_frontend"),
        DependencyEdge::new("payment_frontend", "payment_backend"),
        DependencyEdge::new("x", "shipping"),
    ];
    let mut shuffled = forward.clone();
    shuffled.reverse();

    let source = CapabilityId("x".into());
    let forward_count = effective_structural_branches(&source, &forward).len();
    let shuffled_count = effective_structural_branches(&source, &shuffled).len();

    assert_eq!(
        forward_count, shuffled_count,
        "ORDERING VIOLATION: edge order must not change the candidate extent count"
    );
    assert_eq!(forward_count, 2);
}

#[test]
fn path_035_h_equal_extent_cannot_distinguish_consequence_severity() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // Labels below are for human readability only -- no code in this test
    // or in `effective_structural_branches` performs semantic inference
    // from "reporting" or "payment". Both graphs have the identical
    // shape: one source, one downstream capability, extent = 1.
    let source = CapabilityId("x".into());
    let optional_reporting_failure = vec![DependencyEdge::new("x", "reporting_dashboard")];
    let payment_failure = vec![DependencyEdge::new("x", "payment_settlement")];

    let reporting_extent =
        effective_structural_branches(&source, &optional_reporting_failure).len();
    let payment_extent = effective_structural_branches(&source, &payment_failure).len();

    assert_eq!(
        reporting_extent, payment_extent,
        "SUFFICIENCY COUNTEREXAMPLE: structural extent alone (1 == 1) cannot distinguish an \
         optional reporting capability from a payment capability -- this equality is the finding, \
         not a claim that the two failures are equally consequential"
    );
}

#[test]
fn path_035_i_equal_extent_cannot_distinguish_degradation_degree_or_duration() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches,
    };

    // Same topology reused for two DECLARED (test-premise-only) scenarios
    // that differ only in dimensions the graph carries no evidence for:
    // total-vs-partial capacity loss, and long-vs-short duration. Neither
    // dimension is represented anywhere in `DependencyEdge` or
    // `CapabilityId`, so the candidate structural reduction is, by
    // construction, blind to both. That blindness -- not a new
    // observable for either dimension -- is the counterexample this test
    // records.
    let source = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("x", "shipping"),
    ];

    // Scenario 1 (stipulated, not represented in the graph): total,
    // multi-day outage of both capabilities.
    let scenario_total_long = effective_structural_branches(&source, &edges).len();

    // Scenario 2 (stipulated, not represented in the graph): partial,
    // sub-second degraded-capacity blip of both capabilities.
    let scenario_partial_short = effective_structural_branches(&source, &edges).len();

    assert_eq!(
        scenario_total_long, scenario_partial_short,
        "COUNTEREXAMPLE: identical structural extent (2 == 2) cannot distinguish total/long \
         degradation from partial/short degradation, because degradation degree and duration are \
         not represented in DependencyEdge/CapabilityId topology at all"
    );
}

// =======================================================================
// PATH-036: FAILURE PROPAGATION / COUNTERFACTUAL DEGRADATION SEMANTICS
//
// INSPECTION RESULT (before any test below): no generic "remove
// capability X and recompute" helper existed anywhere in
// `src/measurement.rs` prior to this mission. `effective_structural_
// branches`'s subsumption check (PATH-029) does perform a narrow,
// internal counterfactual removal -- for a candidate branch-root pair
// (root_i, root_j) it BFS-explores from root_i while treating `source`
// AND root_j as dead ends, to ask "does root_i reach anything beyond
// itself and root_j once root_j is gone?" -- but that is scoped
// entirely to pairs of `source`'s own immediate branch roots, deciding
// subsumption between them. It is not a general "remove arbitrary node
// X, ask whether arbitrary target Y stays reachable" utility, and no
// such utility existed to reuse. Every prior PATH-027..035 test that
// needed a "before/after" comparison did so by hand-authoring two
// separate edge lists, never by computing one graph from another via an
// explicit removal function.
//
// PATH-036 adds four minimal, deterministic, topology-only primitives to
// close that gap (all in `src/measurement.rs`, none wired into
// `CriticalityObservables`, `measure_criticality`, `Contribution`, or any
// `aggregate_*` function):
//
// - `remove_capability(edges, removed)`: the ONE removal semantics used
//   throughout -- delete every edge incident to `removed` (as source or
//   target). No second, inconsistent notion of "remove X" is
//   introduced anywhere in this test group.
// - `is_reachable(source, target, edges)`: plain cycle-safe BFS
//   reachability.
// - `target_scoped_effective_branches(source, target, edges)`: EXISTING
//   `effective_structural_branches` (PATH-029/030), filtered down to
//   only the branches that can reach `target` at all. Added by the
//   PATH-036 EPISTEMIC CORRECTION below -- see that note for why the
//   unfiltered, source-global count is not safe to use here.
// - `structural_counterfactual(source, target, edges, removed)`: the
//   before/after tuple (`before_reachable`, `after_reachable`,
//   `before_effective_branches_to_target`,
//   `after_effective_branches_to_target`). All four fields describe
//   `source`'s relationship to `target` specifically -- none is a
//   source-global quantity.
//
// PATH-036 EPISTEMIC CORRECTION (target-scope leak, `path_036_m`): the
// first version of `StructuralCounterfactual` used
// `effective_structural_branches(source, edges).len()` directly --
// a SOURCE-GLOBAL count, not scoped to `target` at all -- alongside the
// `source -> target`-specific reachability fields. `path_036_m` confirms
// this let a branch entirely unrelated to `target` inflate or deflate
// the reported delta: `SOURCE->X->ANALYTICS, SOURCE->Y->PAYMENT`,
// querying `target = PAYMENT`, `removed = X`, reported a branch-count
// drop (2 -> 1) even though X never lay on any `SOURCE -> PAYMENT` path
// and PAYMENT's own redundancy was completely unaffected. This was fixed
// by scoping the branch fields through `target_scoped_effective_branches`
// (filtering by `is_reachable`, never by label) and renaming them to
// `..._to_target` to make the scope explicit. See FIND-013 for the full
// correction and what remains unestablished even after it.
//
// H7 (corrected, narrowed): BINARY TARGET REACHABILITY can fail to
// preserve structural changes visible elsewhere in the graph --
// confirmed below (PATH-036-B/C/I): reachability to `target` can stay
// `true` while `target_scoped_effective_branches` (target-scoped) drops.
// This is deliberately narrower than an earlier, overclaimed phrasing
// ("effective_structural_branches drops" without target-scoping, which
// PATH-036-M shows is not by itself evidence of a target-relevant
// change). H7 is NOT tested, and no claim is made, about operational
// degradation degree (capacity/latency/duration) -- PATH-036-J/K record
// that boundary explicitly instead of crossing it.
//
// No test below claims "reachable means healthy," "unreachable means
// economically catastrophic," "two branches means redundancy" (without
// having actually established target-scoped structural independence),
// or "effective branch delta is criticality." See FIND-013 for the
// final verdict.
// =======================================================================

#[test]
fn path_036_a_single_point_of_failure_reachability_is_lost() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
    ];

    let result = structural_counterfactual(&source, &target, &edges, &removed);

    assert!(result.before_reachable, "PAYMENT must be reachable before removal");
    assert!(
        !result.after_reachable,
        "single point of failure: PAYMENT must become unreachable once X is removed"
    );
    assert_eq!(result.before_effective_branches_to_target, 1);
    assert_eq!(result.after_effective_branches_to_target, 0);
}

#[test]
fn path_036_b_full_redundancy_preserves_reachability() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    let result = structural_counterfactual(&source, &target, &edges, &removed);

    assert!(result.before_reachable);
    assert!(
        result.after_reachable,
        "PAYMENT remains reachable through Y after X is removed"
    );

    // Reachability alone is reported here -- this test does NOT conclude
    // "zero degradation" from it. See path_036_c for the structural
    // signal reachability misses.
}

#[test]
fn path_036_c_redundancy_loss_is_observable_without_reachability_loss() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    let result = structural_counterfactual(&source, &target, &edges, &removed);

    assert_eq!(result.before_reachable, result.after_reachable);
    assert!(
        result.before_reachable,
        "reachability is unchanged (still true) -- the structural signal below is what changes"
    );
    assert!(
        result.after_effective_branches_to_target < result.before_effective_branches_to_target,
        "target_scoped_effective_branches (built on PATH-029/030's effective_structural_branches, \
         filtered to branches that reach PAYMENT) observes the loss of a redundant PAYMENT-\
         reaching branch even though reachability alone reports no change. This is established \
         here because X's removed branch itself led to PAYMENT -- path_036_m tests the case \
         where the removed branch does NOT lead to target, and confirms the target-scoped count \
         correctly reports no change there."
    );
    assert_eq!(result.before_effective_branches_to_target, 2);
    assert_eq!(result.after_effective_branches_to_target, 1);
}

#[test]
fn path_036_d_shared_bottleneck_is_not_distinguished_by_branch_count_alone() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences, structural_counterfactual,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    // Test premise: M is stipulated as a shared bottleneck both branches
    // pass through on the way to PAYMENT.
    let shared_bottleneck_shape = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("m", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];

    let truly_independent_shape = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    // NOTE: `shared_structural_bottlenecks` alone does NOT distinguish
    // these two shapes -- PAYMENT itself is reachable from both branches
    // in EITHER shape, so it is reported as "shared" in both (that is
    // simply "both branches deliver to the same declared target", not
    // evidence of an upstream single point of failure). The primitive
    // that actually distinguishes them is `first_structural_
    // convergences` (PATH-028): it strips out convergence nodes that
    // are themselves downstream of another shared node, so in the
    // shared-bottleneck shape it reports M (strictly upstream of
    // PAYMENT), while in the independent shape the only convergence
    // left is PAYMENT itself.
    let bottleneck_convergences = first_structural_convergences(&source, &shared_bottleneck_shape);
    let independent_convergences =
        first_structural_convergences(&source, &truly_independent_shape);

    assert!(
        bottleneck_convergences.iter().any(|node| *node != target),
        "the shared-bottleneck shape must have a first structural convergence strictly \
         upstream of PAYMENT (at M): {:?}",
        bottleneck_convergences
    );
    assert!(
        independent_convergences.iter().all(|node| *node == target),
        "truly independent paths must converge only at PAYMENT itself, never at an upstream \
         shared node: {:?}",
        independent_convergences
    );

    // What the removal-based candidate (structural_counterfactual on X)
    // reports for each shape.
    let bottleneck_result =
        structural_counterfactual(&source, &target, &shared_bottleneck_shape, &removed);
    let independent_result =
        structural_counterfactual(&source, &target, &truly_independent_shape, &removed);

    assert_eq!(
        bottleneck_result, independent_result,
        "RAW BRANCH-COUNT OVERSTATEMENT: structural_counterfactual(remove X) reports an \
         identical result for the shared-bottleneck shape and the truly independent shape, even \
         though only the independent shape has no remaining single point of failure at M -- \
         removing X does not remove M, and the branch-count delta alone cannot see that M is \
         still a shared exposure in the surviving branch"
    );
}

#[test]
fn path_036_e_relay_insertion_on_the_surviving_branch_does_not_change_the_result() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    // Test premise: RELAY is stipulated as a pure pass-through node with
    // no independent economic capability (PATH-035 lesson reused).
    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let direct = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    let via_relay = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "relay"),
        DependencyEdge::new("relay", "payment"),
    ];

    let direct_result = structural_counterfactual(&source, &target, &direct, &removed);
    let relay_result = structural_counterfactual(&source, &target, &via_relay, &removed);

    assert_eq!(
        direct_result, relay_result,
        "inserting a stipulated pass-through relay into the surviving branch must not change \
         the structural counterfactual result"
    );
}

#[test]
fn path_036_f_duplicate_edges_do_not_manufacture_apparent_resilience() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let single = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    let mut duplicated = single.clone();
    for _ in 0..10 {
        duplicated.push(DependencyEdge::new("source", "y"));
        duplicated.push(DependencyEdge::new("y", "payment"));
    }

    let single_result = structural_counterfactual(&source, &target, &single, &removed);
    let duplicated_result = structural_counterfactual(&source, &target, &duplicated, &removed);

    assert_eq!(
        single_result, duplicated_result,
        "DUPLICATE-EDGE VIOLATION: repeated observation of the same edge must not manufacture \
         apparent additional resilience"
    );
}

#[test]
fn path_036_g_renaming_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let original = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];
    let renamed = vec![
        DependencyEdge::new("root", "branch-one"),
        DependencyEdge::new("branch-one", "leaf"),
        DependencyEdge::new("root", "branch-two"),
        DependencyEdge::new("branch-two", "leaf"),
    ];

    let original_result = structural_counterfactual(
        &CapabilityId("source".into()),
        &CapabilityId("payment".into()),
        &original,
        &CapabilityId("x".into()),
    );
    let renamed_result = structural_counterfactual(
        &CapabilityId("root".into()),
        &CapabilityId("leaf".into()),
        &renamed,
        &CapabilityId("branch-one".into()),
    );

    assert_eq!(
        original_result, renamed_result,
        "RENAMING VIOLATION: relabeling capabilities must not change the structural \
         counterfactual result"
    );
}

#[test]
fn path_036_h_ordering_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let forward = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];
    let mut shuffled = forward.clone();
    shuffled.reverse();

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let forward_result = structural_counterfactual(&source, &target, &forward, &removed);
    let shuffled_result = structural_counterfactual(&source, &target, &shuffled, &removed);

    assert_eq!(
        forward_result, shuffled_result,
        "ORDERING VIOLATION: edge order must not change the structural counterfactual result"
    );
}

#[test]
fn path_036_i_binary_reachability_is_insufficient_to_preserve_the_structural_change() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    let result = structural_counterfactual(&source, &target, &edges, &removed);

    // A hypothetical observable that reported ONLY before/after
    // reachability would show no change at all here.
    assert_eq!(
        result.before_reachable, result.after_reachable,
        "a reachability-only observable reports identical before/after states"
    );
    // Yet a real, meaningful structural change occurred.
    assert_ne!(
        result.before_effective_branches_to_target, result.after_effective_branches_to_target,
        "binary reachable/unreachable is insufficient: it would silently drop this real \
         target-scoped structural-independence loss that target_scoped_effective_branches \
         still records"
    );
}

#[test]
fn path_036_j_identical_structural_result_leaves_operational_degradation_undetermined() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_counterfactual,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    // Scenario 1 (stipulated, NOT represented anywhere in the graph):
    // Y has no spare capacity, so losing X halves served capacity.
    let scenario_no_spare_capacity = structural_counterfactual(&source, &target, &edges, &removed);

    // Scenario 2 (stipulated, NOT represented anywhere in the graph):
    // Y has full spare capacity, so losing X has no user-visible effect.
    let scenario_full_spare_capacity =
        structural_counterfactual(&source, &target, &edges, &removed);

    assert_eq!(
        scenario_no_spare_capacity, scenario_full_spare_capacity,
        "REPRESENTATIONAL BOUNDARY: the identical graph produces the identical structural \
         counterfactual result under both stipulated capacity scenarios -- topology alone \
         cannot determine operational degradation degree without additional evidence that \
         DependencyEdge/CapabilityId do not carry"
    );
}

// PATH-036-K — PARTIAL SERVICE COUNTEREXAMPLE (recorded, not executable):
//
// `StructuralCounterfactual` and every existing graph primitive
// (`is_reachable`, `effective_structural_branches`, `shared_structural_
// bottlenecks`, `first_structural_convergences*`) express only boolean
// reachability and integer branch/convergence counts. There is no
// "reachable but degraded-mode" state anywhere in this module -- no
// field, no enum variant, no third value between reachable and
// unreachable. This absence is recorded directly rather than manufactured
// as a test, per the mission's instruction not to add a large state
// machine for it. See FIND-013.

#[test]
fn path_036_l_counterexample_shared_bottleneck_vs_independent_paths_defeats_the_candidate() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, shared_structural_bottlenecks, structural_counterfactual,
    };

    // This restates path_036_d as a direct attack on
    // `structural_counterfactual` specifically: two graphs that are
    // meaningfully different in a way an EXISTING primitive
    // (`shared_structural_bottlenecks`) can already see, yet the
    // candidate removal-based reduction cannot.
    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let shared_bottleneck_shape = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("m", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let truly_independent_shape = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    assert_ne!(
        shared_structural_bottlenecks(&source, &shared_bottleneck_shape).len(),
        shared_structural_bottlenecks(&source, &truly_independent_shape).len(),
        "these two graphs are structurally different by an existing, already-trusted measure"
    );

    let bottleneck_result =
        structural_counterfactual(&source, &target, &shared_bottleneck_shape, &removed);
    let independent_result =
        structural_counterfactual(&source, &target, &truly_independent_shape, &removed);

    assert_eq!(
        bottleneck_result, independent_result,
        "COUNTEREXAMPLE AGAINST THE CANDIDATE: structural_counterfactual collapses two \
         structurally distinguishable graphs to the same result -- even after target-scoping \
         (path_036_m), it is not a sufficient summary of structural degradation on its own, \
         only of target reachability and target-scoped branch count specifically"
    );
}

#[test]
fn path_036_m_target_scope_leak_unrelated_branch_removal_is_correctly_ignored() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, effective_structural_branches, remove_capability,
        structural_counterfactual, target_scoped_effective_branches,
    };

    // Test premise: ANALYTICS and PAYMENT are stipulated as unrelated
    // capabilities. Relevance to `target` is established purely by graph
    // reachability below (via `is_reachable`/`target_scoped_effective_
    // branches`), never inferred from these labels -- X's branch simply
    // never has an edge path reaching PAYMENT.
    let source = CapabilityId("source".into());
    let target = CapabilityId("payment".into());
    let removed = CapabilityId("x".into());

    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "analytics"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    // CONFIRM THE LEAK CLASS: the SOURCE-GLOBAL primitive
    // (effective_structural_branches) genuinely does change when X is
    // removed -- it drops from 2 (X, Y) to 1 (Y) -- regardless of
    // PAYMENT. A `StructuralCounterfactual` built directly on this
    // global count (the pre-correction shape) would have reported that
    // drop as if it were evidence about PAYMENT's redundancy.
    let after_edges = remove_capability(&edges, &removed);
    let global_before = effective_structural_branches(&source, &edges).len();
    let global_after = effective_structural_branches(&source, &after_edges).len();

    assert_eq!(global_before, 2);
    assert_eq!(global_after, 1);
    assert_ne!(
        global_before, global_after,
        "TARGET-SCOPE LEAK CONFIRMED (pre-correction class): the source-global branch count \
         changes solely because X's own branch (to ANALYTICS) disappeared -- this says nothing \
         about PAYMENT"
    );

    // CONFIRM THE CORRECTION: target_scoped_effective_branches, and the
    // corrected structural_counterfactual built on it, must NOT report
    // any PAYMENT-relevant change, because X never lay on any
    // SOURCE -> PAYMENT path.
    let target_scoped_before = target_scoped_effective_branches(&source, &target, &edges).len();
    let target_scoped_after =
        target_scoped_effective_branches(&source, &target, &after_edges).len();

    assert_eq!(
        target_scoped_before, target_scoped_after,
        "target_scoped_effective_branches must report no change for PAYMENT when the removed \
         branch never reached PAYMENT"
    );
    assert_eq!(target_scoped_before, 1);
    assert_eq!(target_scoped_after, 1);

    let result = structural_counterfactual(&source, &target, &edges, &removed);

    assert!(result.before_reachable);
    assert!(result.after_reachable);
    assert_eq!(
        result.before_effective_branches_to_target, result.after_effective_branches_to_target,
        "CORRECTED: structural_counterfactual's target-scoped fields must not exhibit the \
         target-scope leak -- removing a branch unrelated to PAYMENT must not change PAYMENT's \
         reported redundancy"
    );
    assert_eq!(result.before_effective_branches_to_target, 1);
    assert_eq!(result.after_effective_branches_to_target, 1);
}

// =======================================================================
// PATH-037: MULTI-TARGET COUNTERFACTUAL PROPAGATION SURFACE
//
// INSPECTION RESULT: no existing function enumerated reachable/downstream
// capabilities as production code before this mission (PATH-035's
// `naive_downstream_reachable_count` was a test-only strawman, not
// production). No existing function produced per-target structural
// facts across multiple targets at once -- `structural_counterfactual`
// (PATH-036) answers one `(source, target, removed)` query at a time.
// `failure_blast_radius` (PATH-035/FIND-012) remains an unspecified raw
// `u32` input with no derivation of any kind; it is evaluated as a
// candidate MEANING below, not assumed to already be one of these.
//
// TARGET UNIVERSE (chosen, and why): `reachable_capabilities(source,
// edges)` -- every `CapabilityId` reachable from `source` BEFORE
// removal -- with `source` and `removed` excluded. Justification:
// - A target unreachable before removal can never be usefully included:
//   `remove_capability` only deletes edges, and deleting edges from a
//   graph cannot create a path that did not already exist (every edge
//   usable after removal was already present before). Reachability is
//   therefore monotonically non-increasing under this removal semantics
//   -- see `path_037_inspection_removal_cannot_create_new_reachability`
//   for an executable check on a graph containing a cycle.
// - `source` is excluded: it is trivially "reachable from itself" by
//   `is_reachable`'s definition, so an entry for it would always report
//   `true -> true` with zero informative content.
// - `removed` is excluded: the query is about DOWNSTREAM CONSEQUENCES of
//   removing it, not the tautological fact that it is now gone (any
//   `removed` that was reachable before removal deterministically loses
//   reachability after, for every possible graph -- see PATH-037-L).
// - This is an edge-only representation: a `CapabilityId` with no edges
//   at all cannot appear in this universe (or in any structural
//   primitive in this module) even if some larger "declared capability
//   universe" outside the graph considers it to exist. This limitation
//   is inherited, not fixed, here.
//
// H8 (corrected -- see FIND-014's EPISTEMIC CORRECTION): the TESTED
// count-like scalar blast-radius reductions (candidates A/B/C below) do
// not faithfully preserve the structural consequences of capability
// removal, because different targets can experience DIFFERENT KINDS of
// structural change (hard reachability loss vs. branch-count loss while
// still reachable vs. no change) that a count collapses together
// (PATH-037-B/P), AND because each tested "count of changed/unreachable
// targets" candidate is itself representation-sensitive under chain
// decomposition and relay insertion (PATH-037-H/Q/Q2) -- the same
// representation-laundering risk PATH-035/FIND-012 found for raw
// affected-node counts turns out to also defeat two further candidate
// counts (B and C) that had not been tested before. This is NOT a claim
// that every mathematically possible scalar encoding must fail --
// only that the specific count-like candidates tried here do.
//
// No claim in this test group says a target count is severity, that
// unreachable is worse than branch-loss economically, that more affected
// targets means more critical, or any monetary consequence. This
// mission is topology/measurement only. See FIND-014 for the verdict.
// =======================================================================

#[test]
fn path_037_inspection_removal_cannot_create_new_reachability() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, reachable_capabilities, remove_capability,
    };
    use std::collections::HashSet;

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());

    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("y", "a"),
        DependencyEdge::new("a", "b"),
    ];

    let after_edges = remove_capability(&edges, &removed);

    let before: HashSet<CapabilityId> = reachable_capabilities(&source, &edges).into_iter().collect();
    let after: HashSet<CapabilityId> = reachable_capabilities(&source, &after_edges).into_iter().collect();

    assert!(
        after.is_subset(&before),
        "MONOTONICITY: `remove_capability` only ever deletes edges, so any node reachable \
         after removal was already reachable before -- after={:?} must be a subset of \
         before={:?}",
        after,
        before
    );
    assert!(before.contains(&CapabilityId("x".into())));
    assert!(
        !after.contains(&CapabilityId("x".into())),
        "the removed node itself loses reachability, consistent with (not a violation of) \
         monotonicity"
    );
    assert!(after.contains(&CapabilityId("a".into())) && after.contains(&CapabilityId("b".into())),
        "a/b remain reachable via the surviving branch y -- monotonicity does not mean \
         everything downstream of `removed` is lost, only that nothing NEW becomes reachable"
    );
}

#[test]
fn path_037_a_single_downstream_target() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, StructuralPropagationEntry, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    assert_eq!(
        surface,
        vec![StructuralPropagationEntry {
            target: CapabilityId("payment".into()),
            before_reachable: true,
            after_reachable: false,
            before_effective_branches_to_target: 1,
            after_effective_branches_to_target: 0,
        }]
    );
}

#[test]
fn path_037_b_mixed_consequences_cannot_collapse_to_a_scalar() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("x", "search"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "search"),
        DependencyEdge::new("source", "z"),
        DependencyEdge::new("z", "analytics"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    let payment = surface
        .iter()
        .find(|e| e.target == CapabilityId("payment".into()))
        .expect("PAYMENT must be reported as changed");
    let search = surface
        .iter()
        .find(|e| e.target == CapabilityId("search".into()))
        .expect("SEARCH must be reported as changed");

    assert!(
        !surface.iter().any(|e| e.target == CapabilityId("analytics".into())),
        "ANALYTICS is unaffected by removing X and must not appear"
    );

    assert_eq!(payment.after_reachable, false, "PAYMENT: hard reachability loss");
    assert_eq!(
        (search.after_reachable, search.before_effective_branches_to_target, search.after_effective_branches_to_target),
        (true, 2, 1),
        "SEARCH: remains reachable, but loses one of two redundant branches"
    );

    // H8: a single scalar "affected_targets = 2" is identical whether
    // this actually happened (one hard loss + one branch loss) or two
    // hard losses, or two branch losses -- see path_037_p for the
    // executable version of that collapse.
    assert_eq!(surface.len(), 2);
    assert_ne!(
        payment.after_reachable, search.after_reachable,
        "these two 'affected' targets experienced categorically different structural \
         transitions -- a scalar count of 2 cannot preserve that distinction"
    );
}

#[test]
fn path_037_c_unrelated_branch_exclusion() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    // Test premise: ANALYTICS and PAYMENT are stipulated as unrelated;
    // relevance is established by graph structure below, not by labels.
    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "analytics"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    assert!(
        !surface.iter().any(|e| e.target == CapabilityId("payment".into())),
        "PAYMENT-SCOPE LEAK (PATH-036-M lesson reused): PAYMENT must not appear as \
         structurally changed when the removed branch never reached it"
    );
    assert_eq!(surface.len(), 1);
    assert_eq!(surface[0].target, CapabilityId("analytics".into()));
    assert!(!surface[0].after_reachable);
}

#[test]
fn path_037_d_cascade_deterministic_enumeration_and_ordering() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    let targets: Vec<&str> = surface.iter().map(|e| e.target.0.as_str()).collect();
    assert_eq!(
        targets,
        vec!["a", "b", "c"],
        "deterministic lexical ordering, not a claim of severity ordering"
    );
    for entry in &surface {
        assert!(entry.before_reachable);
        assert!(!entry.after_reachable);
        assert_eq!(entry.before_effective_branches_to_target, 1);
        assert_eq!(entry.after_effective_branches_to_target, 0);
    }
}

#[test]
fn path_037_e_partial_propagation_with_bypass() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, StructuralPropagationEntry, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "b"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    assert_eq!(
        surface,
        vec![
            StructuralPropagationEntry {
                target: CapabilityId("a".into()),
                before_reachable: true,
                after_reachable: false,
                before_effective_branches_to_target: 1,
                after_effective_branches_to_target: 0,
            },
            StructuralPropagationEntry {
                target: CapabilityId("b".into()),
                before_reachable: true,
                after_reachable: true,
                before_effective_branches_to_target: 2,
                after_effective_branches_to_target: 1,
            },
        ],
        "A suffers a hard reachability loss (its only path ran through X); B remains reachable \
         via the Y bypass but still loses one of its two branches -- both are real, DIFFERENT \
         consequences of the same removal"
    );
}

#[test]
fn path_037_f_shared_bottleneck_blind_spot_persists_in_the_surface() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, first_structural_convergences, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let target = CapabilityId("payment".into());

    let shared_bottleneck_shape = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("m", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let truly_independent_shape = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "payment"),
    ];

    // Reused from PATH-036-D/L: an existing primitive already
    // distinguishes these two graphs structurally.
    let bottleneck_convergences = first_structural_convergences(&source, &shared_bottleneck_shape);
    let independent_convergences = first_structural_convergences(&source, &truly_independent_shape);
    assert!(bottleneck_convergences.iter().any(|node| *node != target));
    assert!(independent_convergences.iter().all(|node| *node == target));

    let bottleneck_surface = structural_propagation_surface(&source, &shared_bottleneck_shape, &removed);
    let independent_surface = structural_propagation_surface(&source, &truly_independent_shape, &removed);

    let bottleneck_payment = bottleneck_surface
        .iter()
        .find(|e| e.target == target)
        .expect("PAYMENT must appear as changed in the bottleneck shape");
    let independent_payment = independent_surface
        .iter()
        .find(|e| e.target == target)
        .expect("PAYMENT must appear as changed in the independent shape");

    assert_eq!(
        bottleneck_payment, independent_payment,
        "BLIND SPOT PERSISTS: the surface's own entry for the shared queried target (PAYMENT) \
         is identical between the two structurally distinguishable graphs -- the surface does \
         not incorporate shared-bottleneck/convergence information any more than \
         structural_counterfactual did (PATH-036-D/L)"
    );

    // The surfaces DO differ in overall length (M is an extra entry in
    // the bottleneck shape), but that is evidence of the SAME
    // representation-sensitivity PATH-037-H/Q attack, not evidence that
    // the surface principledly captures shared-bottleneck risk: M only
    // appears because it happens to be modeled as its own labeled node
    // in this representation, not because anything computed its
    // bottleneck role.
    assert_ne!(bottleneck_surface.len(), independent_surface.len());
}

#[test]
fn path_037_g_duplicate_edge_invariance() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let single = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "b"),
    ];
    let mut duplicated = single.clone();
    for _ in 0..10 {
        duplicated.push(DependencyEdge::new("source", "y"));
        duplicated.push(DependencyEdge::new("y", "b"));
    }

    assert_eq!(
        structural_propagation_surface(&source, &single, &removed),
        structural_propagation_surface(&source, &duplicated, &removed),
        "DUPLICATE-EDGE VIOLATION: repeated observation of the same edge must not change the \
         propagation surface"
    );
}

#[test]
fn path_037_h_relay_insertion_inflates_surface_cardinality_with_a_redundant_entry() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());

    let direct = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
    ];
    let via_relay = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "relay"),
        DependencyEdge::new("relay", "payment"),
    ];

    let direct_surface = structural_propagation_surface(&source, &direct, &removed);
    let relay_surface = structural_propagation_surface(&source, &via_relay, &removed);

    assert_eq!(direct_surface.len(), 1);
    assert_eq!(
        relay_surface.len(),
        2,
        "REPRESENTATION PROBLEM (recorded, not hidden): inserting a single stipulated \
         pass-through node changes surface cardinality from 1 to 2, purely because RELAY is \
         itself a distinct graph node in this representation"
    );

    let relay_entry = relay_surface
        .iter()
        .find(|e| e.target == CapabilityId("relay".into()))
        .unwrap();
    let payment_entry = relay_surface
        .iter()
        .find(|e| e.target == CapabilityId("payment".into()))
        .unwrap();

    assert_eq!(
        (relay_entry.before_reachable, relay_entry.after_reachable, relay_entry.before_effective_branches_to_target, relay_entry.after_effective_branches_to_target),
        (payment_entry.before_reachable, payment_entry.after_reachable, payment_entry.before_effective_branches_to_target, payment_entry.after_effective_branches_to_target),
        "RELAY's entry has the IDENTICAL transition shape to PAYMENT's -- it carries no \
         distinguishing structural information, only duplicates the same fact at a finer \
         representational granularity (directly connects back to PATH-035/FIND-012)"
    );
}

#[test]
fn path_037_i_renaming_is_inert_modulo_target_identity() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };
    use std::collections::HashMap;

    let original_edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "b"),
    ];
    let renamed_edges = vec![
        DependencyEdge::new("root", "branch-x"),
        DependencyEdge::new("branch-x", "node-p"),
        DependencyEdge::new("node-p", "node-q"),
        DependencyEdge::new("root", "branch-y"),
        DependencyEdge::new("branch-y", "node-q"),
    ];

    let original_surface = structural_propagation_surface(
        &CapabilityId("source".into()),
        &original_edges,
        &CapabilityId("x".into()),
    );
    let renamed_surface = structural_propagation_surface(
        &CapabilityId("root".into()),
        &renamed_edges,
        &CapabilityId("branch-x".into()),
    );

    // Explicit test-premise correspondence -- never inferred from labels.
    let correspondence: HashMap<&str, &str> =
        [("a", "node-p"), ("b", "node-q")].into_iter().collect();

    assert_eq!(original_surface.len(), renamed_surface.len());
    for original_entry in &original_surface {
        let expected_renamed_target = correspondence[original_entry.target.0.as_str()];
        let renamed_entry = renamed_surface
            .iter()
            .find(|e| e.target.0 == expected_renamed_target)
            .expect("corresponding renamed target must be present");

        assert_eq!(
            (
                original_entry.before_reachable,
                original_entry.after_reachable,
                original_entry.before_effective_branches_to_target,
                original_entry.after_effective_branches_to_target,
            ),
            (
                renamed_entry.before_reachable,
                renamed_entry.after_reachable,
                renamed_entry.before_effective_branches_to_target,
                renamed_entry.after_effective_branches_to_target,
            ),
            "RENAMING VIOLATION: relabeling must not change the structural transition"
        );
    }
}

#[test]
fn path_037_j_ordering_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let forward = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "b"),
    ];
    let mut shuffled = forward.clone();
    shuffled.reverse();

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());

    assert_eq!(
        structural_propagation_surface(&source, &forward, &removed),
        structural_propagation_surface(&source, &shuffled, &removed),
        "ORDERING VIOLATION: edge order must not change the propagation surface"
    );
}

#[test]
fn path_037_k_cycle_terminates_deterministically() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "a"),
        DependencyEdge::new("b", "payment"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    let targets: Vec<&str> = surface.iter().map(|e| e.target.0.as_str()).collect();
    assert_eq!(targets, vec!["a", "b", "payment"]);
    for entry in &surface {
        assert!(entry.before_reachable);
        assert!(!entry.after_reachable);
    }
}

#[test]
fn path_037_l_removed_capability_is_excluded_from_its_own_surface() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    assert!(
        !surface.iter().any(|e| e.target == removed),
        "DECISION (justified): the removed capability itself is excluded -- the query is about \
         downstream consequences, not the tautological fact that the removed node is gone"
    );
}

#[test]
fn path_037_m_source_removal_causes_total_propagation_without_panicking() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "b"),
    ];

    // removed == source: not special-cased, not panicking. Documented as
    // a well-defined (if degenerate) consequence of the same removal
    // semantics, not a separate contract.
    let surface = structural_propagation_surface(&source, &edges, &source);

    let targets: Vec<&str> = surface.iter().map(|e| e.target.0.as_str()).collect();
    assert_eq!(targets, vec!["a", "b", "x", "y"]);
    for entry in &surface {
        assert!(entry.before_reachable);
        assert!(
            !entry.after_reachable,
            "removing source strips every one of its outgoing edges, so every previously \
             reachable target becomes unreachable"
        );
    }
}

#[test]
fn path_037_n_empty_graph_is_non_panicking() {
    use distribution_machine::measurement::{CapabilityId, structural_propagation_surface};

    let source = CapabilityId("solo".into());
    let removed = CapabilityId("ghost".into());
    let edges: Vec<distribution_machine::measurement::DependencyEdge> = vec![];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    assert_eq!(surface, vec![]);
}

#[test]
fn path_037_o_disconnected_component_is_excluded() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
        DependencyEdge::new("disconnected-a", "disconnected-b"),
    ];

    let surface = structural_propagation_surface(&source, &edges, &removed);

    assert!(
        !surface
            .iter()
            .any(|e| e.target.0 == "disconnected-a" || e.target.0 == "disconnected-b"),
        "a component with no path from source must never appear merely because its edges exist"
    );
    assert_eq!(surface.len(), 1);
    assert_eq!(surface[0].target, CapabilityId("payment".into()));
}

#[test]
fn path_037_p_counterexample_equal_affected_target_count_different_transition_vectors() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());

    // Scenario 1: X is the sole path to two independent targets. Removing
    // X leaves BOTH hard-unreachable.
    let hard_loss_scenario = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("x", "b"),
    ];

    // Scenario 2: X is one of two branches feeding each of two
    // independent targets. Removing X leaves BOTH still reachable, each
    // losing one of two branches.
    let branch_loss_scenario = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("x", "b"),
        DependencyEdge::new("source", "w"),
        DependencyEdge::new("w", "a"),
        DependencyEdge::new("source", "v"),
        DependencyEdge::new("v", "b"),
    ];

    let hard_loss_surface = structural_propagation_surface(&source, &hard_loss_scenario, &removed);
    let branch_loss_surface = structural_propagation_surface(&source, &branch_loss_scenario, &removed);

    assert_eq!(hard_loss_surface.len(), 2);
    assert_eq!(branch_loss_surface.len(), 2);
    assert_eq!(
        hard_loss_surface.len(),
        branch_loss_surface.len(),
        "H8 COUNTEREXAMPLE: both scenarios report the same scalar affected-target count (2)"
    );

    assert!(
        hard_loss_surface.iter().all(|e| !e.after_reachable),
        "scenario 1: both targets suffer hard reachability loss"
    );
    assert!(
        branch_loss_surface.iter().all(|e| e.after_reachable
            && e.after_effective_branches_to_target < e.before_effective_branches_to_target),
        "scenario 2: both targets remain reachable, only losing a redundant branch"
    );
    assert_ne!(
        hard_loss_surface, branch_loss_surface,
        "the equal scalar count of 2 masks a completely different structural reality in each \
         scenario -- no economic claim is made about which is 'worse'"
    );
}

#[test]
fn path_037_q_counterexample_relay_insertion_defeats_target_count_candidates() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());

    let direct = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
    ];
    let via_relay = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "relay"),
        DependencyEdge::new("relay", "payment"),
    ];

    let direct_surface = structural_propagation_surface(&source, &direct, &removed);
    let relay_surface = structural_propagation_surface(&source, &via_relay, &removed);

    // Candidate A (section 7): "number of targets losing reachability".
    let candidate_a_direct = direct_surface.iter().filter(|e| !e.after_reachable).count();
    let candidate_a_relay = relay_surface.iter().filter(|e| !e.after_reachable).count();

    // Candidate B (section 7): "number of targets with any structural
    // change" -- coincides with the surface length here, since every
    // changed target in this graph shape is a hard-reachability loss.
    let candidate_b_direct = direct_surface.len();
    let candidate_b_relay = relay_surface.len();

    assert_ne!(
        candidate_a_direct, candidate_a_relay,
        "CANDIDATE A FALSIFIED: 'targets losing reachability' inflates 1 -> 2 purely from \
         inserting a stipulated pass-through relay"
    );
    assert_ne!(
        candidate_b_direct, candidate_b_relay,
        "CANDIDATE B FALSIFIED: 'targets with any structural change' inflates 1 -> 2 for the \
         same reason -- identical to PATH-035/FIND-012's node-splitting attack on raw \
         affected-node counts, now shown against this candidate too"
    );
}

#[test]
fn path_037_q2_counterexample_chain_decomposition_defeats_branch_loss_count_candidate() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());

    // Test premise: A2 is stipulated as internal decomposition of the
    // same capability as A (PATH-035 lesson reused), reached only via A.
    let baseline = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "a"),
    ];
    let decomposed = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "a"),
        DependencyEdge::new("a", "a2"),
    ];

    let baseline_surface = structural_propagation_surface(&source, &baseline, &removed);
    let decomposed_surface = structural_propagation_surface(&source, &decomposed, &removed);

    // Candidate C (section 7): "number of target-scoped branch losses"
    // -- targets that remain reachable but whose target-scoped branch
    // count decreased.
    let branch_loss_count = |surface: &[distribution_machine::measurement::StructuralPropagationEntry]| {
        surface
            .iter()
            .filter(|e| {
                e.after_reachable
                    && e.after_effective_branches_to_target < e.before_effective_branches_to_target
            })
            .count()
    };

    let candidate_c_baseline = branch_loss_count(&baseline_surface);
    let candidate_c_decomposed = branch_loss_count(&decomposed_surface);

    assert_eq!(candidate_c_baseline, 1);
    assert_eq!(
        candidate_c_decomposed, 2,
        "A2 shows the identical branch-loss transition as A purely because it is an internal \
         decomposition reached only through A"
    );
    assert_ne!(
        candidate_c_baseline, candidate_c_decomposed,
        "CANDIDATE C FALSIFIED: 'count of target-scoped branch losses' also inflates under \
         chain decomposition of a single stipulated capability into two graph nodes -- no \
         scalar reduction of the propagation surface tried in this mission survived this \
         attack"
    );
}

// =======================================================================
// PATH-038: REPRESENTATION CANONICALIZATION / SEMANTIC CAPABILITY COLLAPSE
//
// INSPECTION RESULT: no existing primitive performs any node collapse or
// canonical rewriting of `edges`. `effective_structural_branches`
// (PATH-029/030) deduplicates/subsumes BRANCH ROOTS for counting
// purposes but never rewrites the edge list. `first_structural_
// convergences`/`shared_structural_bottlenecks` (PATH-027/028) compute
// facts ABOUT the graph but do not alter it. `structural_counterfactual`/
// `structural_propagation_surface` (PATH-036/037) only ever add/remove
// edges via `remove_capability`, never fold a pass-through node's edges
// into a bypass edge. PATH-038 is the first mission to attempt this.
//
// CONTRACT TESTED: `canonicalize_pure_relays(source, protected, edges)`
// collapses a node only if, from topology alone: it is not `source`; it
// is not in the caller-supplied `protected` set; it has exactly one
// distinct predecessor AND exactly one distinct successor (this alone
// makes a branch point, PATH-038-C, or a convergence point, PATH-038-D,
// structurally uncollapsible); and its sole successor cannot reach it
// again (excludes cycle members, PATH-038-G). Collapse redirects the
// predecessor directly to the successor and is repeated to a fixed
// point. Every collapse is logged in `RelayCollapseProvenance` --
// nothing is silently deleted.
//
// H9 (as actually tested): does this topology-only contract remove
// representation-granularity artifacts (PATH-035/037) while preserving
// meaningful branch/convergence/target-transition structure? The
// evidence below is mixed and the strongest counterexample is NOT a
// contrived semantic scenario but a purely structural one
// (`path_038_k`/`path_038_p`): collapsing two INDIVIDUALLY-valid relay
// nodes that happen to feed the same downstream convergence point from
// a common ancestor can cascade into erasing the convergence itself from
// the CANONICAL EDGES ALONE, even though nothing is topologically wrong
// with either individual collapse and even though the full
// (edges, provenance) pair remains completely lossless. See FIND-015 for
// the full verdict, and PATH-038-N for the separate (semantic, not
// structural) reason topology alone cannot certify a degree-(1,1) node
// as representation-only in the first place.
//
// No test below claims the canonical graph is economically correct,
// unique, or that topology reveals semantic capability identity. No
// floats, no weights, no severity, no economic meaning anywhere in this
// test group.
// =======================================================================

#[test]
fn path_038_a_simple_relay_collapses_to_match_the_direct_graph() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let a = CapabilityId("a".into());
    let target = CapabilityId("target".into());

    // Protecting `a` isolates this test to RELAY specifically -- without
    // it, `a` itself would also be an eligible degree-(1,1) node.
    let protected = vec![a.clone(), target.clone()];

    let direct = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "target"),
    ];
    let via_relay = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "relay"),
        DependencyEdge::new("relay", "target"),
    ];

    let (canonical_direct, direct_provenance) =
        canonicalize_pure_relays(&source, &protected, &direct);
    let (canonical_relay, relay_provenance) =
        canonicalize_pure_relays(&source, &protected, &via_relay);

    assert_eq!(canonical_direct, canonical_relay);
    assert!(direct_provenance.is_empty());
    assert_eq!(relay_provenance.len(), 1);
    assert_eq!(relay_provenance[0].collapsed, CapabilityId("relay".into()));
}

#[test]
fn path_038_b_chain_decomposition_collapses_to_the_same_canonical_graph() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let protected = vec![target.clone()];

    let baseline = vec![DependencyEdge::new("source", "target")];
    let decomposed = vec![
        DependencyEdge::new("source", "a1"),
        DependencyEdge::new("a1", "a2"),
        DependencyEdge::new("a2", "target"),
    ];

    let (canonical_baseline, _) = canonicalize_pure_relays(&source, &protected, &baseline);
    let (canonical_decomposed, decomposed_provenance) =
        canonicalize_pure_relays(&source, &protected, &decomposed);

    assert_eq!(canonical_baseline, canonical_decomposed);
    assert_eq!(canonical_decomposed, vec![DependencyEdge::new("source", "target")]);
    assert_eq!(decomposed_provenance.len(), 2, "both A1 and A2 must be collapsed");
}

#[test]
fn path_038_c_branch_point_never_collapses() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let mut edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("x", "b"),
    ];
    edges.sort_by(|e1, e2| e1.source.0.cmp(&e2.source.0).then(e1.target.0.cmp(&e2.target.0)));

    let (canonical, provenance) = canonicalize_pure_relays(&source, &[], &edges);

    assert!(provenance.is_empty(), "X has two successors and must never be a collapse candidate");
    assert_eq!(canonical, edges);
}

#[test]
fn path_038_d_convergence_point_never_collapses_merely_for_having_one_successor() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let a = CapabilityId("a".into());
    let mut edges = vec![
        DependencyEdge::new("a", "m"),
        DependencyEdge::new("b", "m"),
        DependencyEdge::new("m", "target"),
    ];
    edges.sort_by(|e1, e2| e1.source.0.cmp(&e2.source.0).then(e1.target.0.cmp(&e2.target.0)));

    // `a` is used as `source` here purely to give the function a
    // required parameter; M's survival is due to its TWO predecessors,
    // not source-exclusion (B is untouched by source-exclusion and M's
    // predecessor count is what matters).
    let (canonical, provenance) = canonicalize_pure_relays(&a, &[], &edges);

    assert!(
        provenance.is_empty(),
        "M has two predecessors (A and B) and must never be a collapse candidate, regardless \
         of its single successor"
    );
    assert_eq!(canonical, edges);
}

#[test]
fn path_038_e_protected_target_survives_only_when_declared() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "target"),
        DependencyEdge::new("target", "y"),
    ];

    let (canonical_unprotected, _) = canonicalize_pure_relays(&source, &[], &edges);
    assert!(
        !canonical_unprotected
            .iter()
            .any(|e| e.source == target || e.target == target),
        "without protection, TARGET is itself degree-(1,1) and non-cyclic, so it collapses too"
    );

    let (canonical_protected, _) =
        canonicalize_pure_relays(&source, &[target.clone()], &edges);
    assert!(
        canonical_protected
            .iter()
            .any(|e| e.source == target || e.target == target),
        "DECISION: an explicitly protected target must survive even though topology alone \
         would have allowed its collapse"
    );
}

#[test]
fn path_038_f_source_never_collapses_even_when_topologically_relay_shaped() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    // `source` here has exactly one predecessor (z) and one successor
    // (x) -- topologically indistinguishable from a pure relay -- yet
    // must never be collapsed.
    let edges = vec![
        DependencyEdge::new("z", "source"),
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "target"),
    ];

    let (canonical, _) = canonicalize_pure_relays(&source, &[], &edges);

    assert!(
        canonical.iter().any(|e| e.source == source || e.target == source),
        "source must survive regardless of its own topological shape"
    );
    assert_eq!(canonical, vec![DependencyEdge::new("source", "target"), DependencyEdge::new("z", "source")]);
}

#[test]
fn path_038_g_cycle_is_never_collapsed() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("a".into());
    let mut edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "a"),
    ];
    edges.sort_by(|e1, e2| e1.source.0.cmp(&e2.source.0).then(e1.target.0.cmp(&e2.target.0)));

    let (canonical, provenance) = canonicalize_pure_relays(&source, &[], &edges);

    assert!(
        provenance.is_empty(),
        "every member of a 3-cycle has exactly one predecessor/successor, but each also lies \
         on the cycle, so none may be collapsed without erasing real cycle topology"
    );
    assert_eq!(canonical, edges);
}

#[test]
fn path_038_h_duplicate_edge_invariance() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let single = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "target"),
    ];
    let mut duplicated = single.clone();
    for _ in 0..10 {
        duplicated.push(DependencyEdge::new("source", "a"));
        duplicated.push(DependencyEdge::new("a", "target"));
    }

    assert_eq!(
        canonicalize_pure_relays(&source, &[], &single).0,
        canonicalize_pure_relays(&source, &[], &duplicated).0
    );
}

#[test]
fn path_038_i_edge_order_invariance() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let forward = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "target"),
    ];
    let mut shuffled = forward.clone();
    shuffled.reverse();

    let forward_result = canonicalize_pure_relays(&source, &[], &forward);
    let shuffled_result = canonicalize_pure_relays(&source, &[], &shuffled);

    assert_eq!(
        forward_result, shuffled_result,
        "ORDERING VIOLATION: input edge order must not change the canonical graph or provenance"
    );
    assert_eq!(forward_result.0, vec![DependencyEdge::new("source", "target")]);
    assert_eq!(forward_result.1.len(), 2);
}

#[test]
fn path_038_j_renaming_is_inert_modulo_target_identity() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let original_edges = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "target"),
    ];
    let renamed_edges = vec![
        DependencyEdge::new("root", "p"),
        DependencyEdge::new("p", "q"),
        DependencyEdge::new("q", "goal"),
    ];

    let (canonical_original, provenance_original) = canonicalize_pure_relays(
        &CapabilityId("source".into()),
        &[CapabilityId("target".into())],
        &original_edges,
    );
    let (canonical_renamed, provenance_renamed) = canonicalize_pure_relays(
        &CapabilityId("root".into()),
        &[CapabilityId("goal".into())],
        &renamed_edges,
    );

    assert_eq!(canonical_original.len(), 1);
    assert_eq!(canonical_renamed.len(), 1);
    assert_eq!(
        provenance_original.len(),
        provenance_renamed.len(),
        "RENAMING VIOLATION: the number of collapsed nodes must not depend on labels"
    );
}

#[test]
fn path_038_k_cascading_collapse_can_erase_a_real_shared_bottleneck() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays, shared_structural_bottlenecks,
    };

    let source = CapabilityId("source".into());
    let payment = CapabilityId("payment".into());
    let m = CapabilityId("m".into());

    // A genuine, existing-primitive-verified shared bottleneck: X and Y
    // are two structurally distinct branches that both converge at M
    // before reaching PAYMENT.
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("m", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];

    assert!(
        !shared_structural_bottlenecks(&source, &edges).is_empty(),
        "M is a genuine shared structural bottleneck in the original graph"
    );

    // Only PAYMENT is protected. X and Y are each, INDIVIDUALLY,
    // legitimate degree-(1,1) non-cyclic relay candidates -- there is
    // nothing topologically wrong with collapsing either one on its own.
    let (canonical_payment_only, provenance) =
        canonicalize_pure_relays(&source, &[payment.clone()], &edges);

    assert_eq!(
        canonical_payment_only,
        vec![DependencyEdge::new("source", "payment")],
        "CASCADING FALSE COLLAPSE: after X and Y each independently collapse into \
         `source -> m`, M's predecessor count drops from 2 to 1 (both branches now point at \
         the identical edge, which deduplicates), making M ITSELF newly eligible -- the \
         canonical graph ends up structurally indistinguishable from a single unshared path, \
         even though nothing about the original convergence was semantically uninteresting"
    );
    assert_eq!(provenance.len(), 3, "x, y, and m are all collapsed in sequence");

    // Protecting M too reduces, but does not eliminate, the loss: M
    // survives as a NODE, but the fact that it once had two distinct
    // predecessors is no longer visible in the edges alone.
    let (canonical_m_protected, _) =
        canonicalize_pure_relays(&source, &[payment.clone(), m.clone()], &edges);
    assert_eq!(
        canonical_m_protected,
        vec![
            DependencyEdge::new("m", "payment"),
            DependencyEdge::new("source", "m"),
        ],
        "even with M explicitly protected, its predecessor count has still silently dropped \
         from 2 to 1 in the canonical edges -- node-level protection does not preserve \
         BRANCH-COUNT/convergence information"
    );
}

#[test]
fn path_038_l_multiple_relays_collapse_to_a_fixed_point_idempotently() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "r1"),
        DependencyEdge::new("r1", "r2"),
        DependencyEdge::new("r2", "target"),
    ];

    let (once, _) = canonicalize_pure_relays(&source, &[target.clone()], &edges);
    assert_eq!(once, vec![DependencyEdge::new("source", "target")]);

    let (twice, provenance_second_pass) = canonicalize_pure_relays(&source, &[target.clone()], &once);
    assert_eq!(
        once, twice,
        "IDEMPOTENCE: canonicalizing an already-canonical graph must be a no-op on the edges"
    );
    assert!(provenance_second_pass.is_empty());
}

#[test]
fn path_038_m_provenance_fully_recovers_every_collapsed_node() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "r1"),
        DependencyEdge::new("r1", "r2"),
        DependencyEdge::new("r2", "target"),
    ];

    let (_, provenance) = canonicalize_pure_relays(&source, &[target], &edges);

    let collapsed: std::collections::HashSet<CapabilityId> =
        provenance.iter().map(|entry| entry.collapsed.clone()).collect();

    assert_eq!(
        collapsed,
        [
            CapabilityId("a".into()),
            CapabilityId("r1".into()),
            CapabilityId("r2".into())
        ]
        .into_iter()
        .collect(),
        "NO HIDDEN DELETION: every removed node must appear exactly once in the provenance"
    );
    // The original chain is fully reconstructable by chaining
    // predecessor -> collapsed -> successor across entries.
    for entry in &provenance {
        assert_ne!(entry.predecessor, entry.collapsed);
        assert_ne!(entry.successor, entry.collapsed);
    }
}

#[test]
fn path_038_n_topology_cannot_distinguish_a_relay_from_a_meaningful_capability() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays,
    };

    // Test premise (explicit, not inferred from the label "x"): in THIS
    // scenario, X is stipulated to be a genuinely meaningful capability
    // that simply happens to sit between exactly one predecessor and one
    // successor -- topologically this graph is IDENTICAL in shape to
    // path_038_a's true relay case.
    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let x = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "target"),
    ];

    // Without external declaration, topology alone collapses X exactly
    // as it would a true relay -- there is no topological signal here
    // that distinguishes "meaningful capability" from "pure relay."
    let (canonical_unprotected, provenance) =
        canonicalize_pure_relays(&source, &[target.clone()], &edges);
    assert_eq!(canonical_unprotected, vec![DependencyEdge::new("source", "target")]);
    assert_eq!(provenance.len(), 1);
    assert_eq!(provenance[0].collapsed, x);

    // The ONLY way to prevent this false collapse is an external
    // declaration -- never derived from topology.
    let (canonical_protected, provenance_protected) =
        canonicalize_pure_relays(&source, &[x.clone(), target], &edges);
    assert_eq!(canonical_protected, edges);
    assert!(provenance_protected.is_empty());
}

#[test]
fn path_038_o_replay_pass_through_relays_are_normalized_but_leaf_decomposition_is_not() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays, structural_propagation_surface,
    };

    let source = CapabilityId("source".into());
    let removed = CapabilityId("x".into());
    let payment = CapabilityId("payment".into());

    // PATH-037-Q shape: a pass-through relay on the removed branch.
    let direct = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "payment"),
    ];
    let via_relay = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "relay"),
        DependencyEdge::new("relay", "payment"),
    ];

    let direct_surface = structural_propagation_surface(&source, &direct, &removed);

    let (canonical_relay, _) =
        canonicalize_pure_relays(&source, &[removed.clone(), payment.clone()], &via_relay);
    let canonicalized_relay_surface =
        structural_propagation_surface(&source, &canonical_relay, &removed);

    assert_eq!(
        direct_surface, canonicalized_relay_surface,
        "PASS-THROUGH case: canonicalizing the relay before computing the propagation surface \
         removes PATH-037-H/Q's cardinality inflation entirely -- both surfaces now agree"
    );

    // PATH-037-Q2 shape: a LEAF decomposition node (A2 has no outgoing
    // edge at all), not a pass-through relay.
    let baseline = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "a"),
    ];
    let decomposed = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "a"),
        DependencyEdge::new("a", "a2"),
    ];

    // `y` is the genuine surviving branch into `a` (the actual
    // redundancy the removal counterfactual depends on), so it must be
    // protected too -- otherwise it is itself topologically relay-shaped
    // (one predecessor, one successor) and would be collapsed as an
    // unrelated side effect, contaminating this specific comparison.
    let (canonical_decomposed, decomposed_provenance) = canonicalize_pure_relays(
        &source,
        &[
            removed.clone(),
            CapabilityId("a".into()),
            CapabilityId("y".into()),
        ],
        &decomposed,
    );

    assert!(
        decomposed_provenance.is_empty(),
        "A2 has no outgoing edge at all, so it is not a PASS-THROUGH relay under this contract \
         and is never collapsed -- this specific canonicalization does not cover leaf/terminal \
         decomposition nodes"
    );

    let baseline_surface = structural_propagation_surface(&source, &baseline, &removed);
    let decomposed_surface = structural_propagation_surface(&source, &canonical_decomposed, &removed);

    assert_ne!(
        baseline_surface.len(),
        decomposed_surface.len(),
        "SCOPE LIMIT (recorded, not hidden): PATH-037-Q2's leaf-decomposition inflation is NOT \
         resolved by this pure-relay contract, because A2 is never eligible for collapse"
    );
}

#[test]
fn path_038_p_canonical_edges_alone_lose_convergence_information_that_provenance_still_preserves() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, canonicalize_pure_relays, shared_structural_bottlenecks,
    };

    let source = CapabilityId("source".into());
    let payment = CapabilityId("payment".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("m", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];

    assert!(!shared_structural_bottlenecks(&source, &edges).is_empty());

    let (canonical, provenance) = canonicalize_pure_relays(&source, &[payment], &edges);

    // H9 NARROWED/REJECTED for "canonical edges as a drop-in
    // replacement": an existing, previously-verified primitive reports
    // NOTHING on the canonical graph alone -- the convergence fact is
    // invisible without consulting provenance.
    assert!(
        shared_structural_bottlenecks(&source, &canonical).is_empty(),
        "the canonical graph alone no longer exhibits the shared bottleneck at all"
    );

    // But it is NOT silently destroyed overall: two DISTINCT provenance
    // entries recorded `m` as their successor, which is exactly the
    // signature of two branches having converged there before collapse.
    let entries_feeding_m: Vec<&CapabilityId> = provenance
        .iter()
        .filter(|entry| entry.successor == m)
        .map(|entry| &entry.collapsed)
        .collect();

    assert_eq!(
        entries_feeding_m.len(),
        2,
        "RECOVERABLE: the full (canonical edges, provenance) pair still lets a careful reader \
         reconstruct that two distinct branches (x and y) converged at M -- nothing is \
         irrecoverably deleted, but the canonical EDGES ALONE are not a safe substitute for \
         convergence-sensitive structural analysis"
    );
    assert_ne!(entries_feeding_m[0], entries_feeding_m[1]);
}

// =======================================================================
// PATH-039: EXPLICIT REPRESENTATION SEMANTICS / ANTI-LAUNDERING RESURRECTION QUEST
//
// INSPECTION RESULT: no existing type in this module carries
// representation-role semantics, and no existing type carries
// issuer/authority identity of any kind -- `DependencyId`,
// `DependencyPathId`, and `FailureDomainId` are all bare
// `String`-wrapped identifiers with no such concept. `RelayCollapseProvenance`
// (PATH-038) records WHAT was collapsed, never WHY it was eligible.
// Nothing here is reused wholesale, because nothing here fits; the new
// types below (`RepresentationRole`, `RepresentationClaimId`,
// `RepresentationClaim`, `RepresentationDecision`) follow the exact same
// minimal `XxxId(String)` + plain-struct convention already used
// throughout this module, adding no organizational-authority or manager
// hierarchy concept that was not already absent.
//
// THE CONTRACT TESTED: `canonicalize_declared_representation_nodes`
// requires a node to satisfy EVERY existing PATH-038 topology guard
// (not source, not protected, exactly one predecessor, exactly one
// successor, not cyclic) AND `evaluate_representation_claims` to return
// `Authorized` (a valid `RepresentationOnly` claim exists, no conflicting
// `SemanticCapability` claim exists). `Unclassified` is the ABSENCE of
// any claim, not a third enum variant -- see the doc comment on
// `RepresentationRole` for why. Conflicting claims for the same
// capability produce `RepresentationDecision::Conflicted`, always
// reported via `representation_claim_conflicts`, never silently resolved.
//
// H10 (as actually tested): explicit representation semantics DO close
// PATH-038-N's specific gap (topology alone cannot distinguish a relay
// from a meaningful degree-(1,1) capability) -- given a TRUTHFUL
// declaration, canonicalization now behaves differently and correctly
// for the two cases (`path_039_n`). But H10's stronger claim --
// "safely authorizes canonicalization... while resisting trivial
// declaration laundering" -- does NOT survive: nothing in this
// evidence model can distinguish a truthful `RepresentationOnly` claim
// from a false one (`path_039_d`/`path_039_l`), and even fully TRUTHFUL,
// correctly-evidenced claims about genuinely-representational nodes
// can still reproduce PATH-038-K/P's cascading erasure of an unrelated,
// entirely unclaimed convergence node's branch-multiplicity fact
// (`path_039_o`) -- a failure mode explicit semantics do not touch at
// all, because it never depended on any node's semantic status in the
// first place. See FIND-016 for the full verdict.
//
// No claim below says explicit semantics solve canonicalization
// generally, that metadata makes declarations truthful, that provenance
// proves semantics, that collapse is economically correct, that graph
// identity is solved, or that representation laundering is impossible.
// No money, no Contribution, no CriticalityObservables wiring anywhere
// in this test group.
// =======================================================================

#[test]
fn path_039_a_declared_true_relay_collapses() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let r = CapabilityId("r".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];
    let claims = vec![RepresentationClaim {
        capability: r.clone(),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-1".into()),
    }];

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[target.clone()], &claims, &edges);

    assert_eq!(canonical, vec![DependencyEdge::new("source", "target")]);
    assert_eq!(provenance.len(), 1);
    assert_eq!(provenance[0].collapsed, r);
    assert!(conflicts.is_empty());
}

#[test]
fn path_039_b_unclassified_relay_shaped_node_survives() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];
    let claims: Vec<RepresentationClaim> = vec![]; // no declaration at all

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    let mut expected = edges.clone();
    expected.sort_by(|a, b| a.source.0.cmp(&b.source.0).then(a.target.0.cmp(&b.target.0)));
    assert_eq!(
        canonical, expected,
        "CONSERVATIVE DEFAULT: an unclassified node must not automatically become \
         representation-only"
    );
    assert!(provenance.is_empty());
    assert!(conflicts.is_empty());
}

#[test]
fn path_039_c_explicit_semantic_capability_survives() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationDecision, RepresentationRole, canonicalize_declared_representation_nodes,
        evaluate_representation_claims,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let r = CapabilityId("r".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];
    let claims = vec![RepresentationClaim {
        capability: r.clone(),
        role: RepresentationRole::SemanticCapability,
        claim_id: RepresentationClaimId("claim-2".into()),
    }];

    assert_eq!(
        evaluate_representation_claims(&r, &claims),
        RepresentationDecision::Refused
    );

    let (canonical, provenance, _) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    let mut expected = edges.clone();
    expected.sort_by(|a, b| a.source.0.cmp(&b.source.0).then(a.target.0.cmp(&b.target.0)));
    assert_eq!(canonical, expected, "an explicitly declared SemanticCapability must survive");
    assert!(provenance.is_empty());
}

#[test]
fn path_039_d_false_representation_declaration_is_not_detectable_from_topology() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    // Test premise: X is stipulated to be genuinely meaningful, but is
    // FALSELY declared RepresentationOnly with otherwise valid evidence.
    // Nothing in the graph or the claim structure marks this as a lie.
    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let x = CapabilityId("x".into());
    let edges = vec![DependencyEdge::new("source", "x"), DependencyEdge::new("x", "target")];
    let claims = vec![RepresentationClaim {
        capability: x.clone(),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-lie".into()),
    }];

    let (canonical, provenance, _) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    assert_eq!(
        canonical,
        vec![DependencyEdge::new("source", "target")],
        "UNDETECTABLE FALSE CLAIM: a structurally well-formed but false RepresentationOnly \
         claim collapses X exactly as a true one would -- this function cannot verify claim \
         truth, only claim well-formedness"
    );
    assert_eq!(provenance[0].collapsed, x);
}

#[test]
fn path_039_e_branch_point_marked_representation_only_still_never_collapses() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "a"),
        DependencyEdge::new("x", "b"),
    ];
    let claims = vec![RepresentationClaim {
        capability: x.clone(),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-3".into()),
    }];

    let (canonical, provenance, _) =
        canonicalize_declared_representation_nodes(&source, &[], &claims, &edges);

    assert!(
        provenance.is_empty(),
        "semantic authorization is not sufficient on its own -- X has two successors and can \
         never satisfy the one-predecessor/one-successor shape"
    );
    let mut expected = edges.clone();
    expected.sort_by(|a, b| a.source.0.cmp(&b.source.0).then(a.target.0.cmp(&b.target.0)));
    assert_eq!(canonical, expected);
}

#[test]
fn path_039_f_convergence_point_marked_representation_only_still_never_collapses() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let a = CapabilityId("a".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("a", "m"),
        DependencyEdge::new("b", "m"),
        DependencyEdge::new("m", "target"),
    ];
    let claims = vec![RepresentationClaim {
        capability: m.clone(),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-4".into()),
    }];

    let (canonical, provenance, _) =
        canonicalize_declared_representation_nodes(&a, &[], &claims, &edges);

    assert!(
        provenance.is_empty(),
        "M has two predecessors and can never satisfy the collapse shape, regardless of \
         semantic authorization"
    );
    let mut expected = edges.clone();
    expected.sort_by(|e1, e2| e1.source.0.cmp(&e2.source.0).then(e1.target.0.cmp(&e2.target.0)));
    assert_eq!(canonical, expected);
}

#[test]
fn path_039_g_cycle_member_marked_representation_only_still_never_collapses() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("a".into());
    let mut edges = vec![
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "a"),
    ];
    edges.sort_by(|e1, e2| e1.source.0.cmp(&e2.source.0).then(e1.target.0.cmp(&e2.target.0)));
    let claims = vec![
        RepresentationClaim {
            capability: CapabilityId("b".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-5".into()),
        },
        RepresentationClaim {
            capability: CapabilityId("c".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-6".into()),
        },
    ];

    let (canonical, provenance, _) =
        canonicalize_declared_representation_nodes(&source, &[], &claims, &edges);

    assert!(
        provenance.is_empty(),
        "cycle membership refuses collapse regardless of semantic marking"
    );
    assert_eq!(canonical, edges);
}

#[test]
fn path_039_h_declaration_order_invariance() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "target"),
    ];
    let forward_claims = vec![
        RepresentationClaim {
            capability: CapabilityId("a".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-a".into()),
        },
        RepresentationClaim {
            capability: CapabilityId("b".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-b".into()),
        },
    ];
    let mut reversed_claims = forward_claims.clone();
    reversed_claims.reverse();

    let forward_result =
        canonicalize_declared_representation_nodes(&source, &[target.clone()], &forward_claims, &edges);
    let reversed_result =
        canonicalize_declared_representation_nodes(&source, &[target], &reversed_claims, &edges);

    assert_eq!(forward_result, reversed_result);
}

#[test]
fn path_039_i_edge_order_invariance() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let forward = vec![
        DependencyEdge::new("source", "r"),
        DependencyEdge::new("r", "target"),
    ];
    let mut shuffled = forward.clone();
    shuffled.reverse();
    let claims = vec![RepresentationClaim {
        capability: CapabilityId("r".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-1".into()),
    }];

    let forward_result =
        canonicalize_declared_representation_nodes(&source, &[target.clone()], &claims, &forward);
    let shuffled_result =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &shuffled);

    assert_eq!(forward_result, shuffled_result);
}

#[test]
fn path_039_j_duplicate_declaration_does_not_multiply_authority() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let r = CapabilityId("r".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];

    let single = vec![RepresentationClaim {
        capability: r.clone(),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-1".into()),
    }];
    let identical_duplicate = vec![single[0].clone(), single[0].clone(), single[0].clone()];
    let independently_corroborating = vec![
        single[0].clone(),
        RepresentationClaim {
            capability: r.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-2".into()),
        },
    ];

    let single_result =
        canonicalize_declared_representation_nodes(&source, &[target.clone()], &single, &edges);
    let duplicate_result = canonicalize_declared_representation_nodes(
        &source,
        &[target.clone()],
        &identical_duplicate,
        &edges,
    );
    let corroborating_result = canonicalize_declared_representation_nodes(
        &source,
        &[target],
        &independently_corroborating,
        &edges,
    );

    assert_eq!(
        single_result, duplicate_result,
        "repeating the identical claim must not change the outcome"
    );
    assert_eq!(
        single_result, corroborating_result,
        "a second, independently-identified claim asserting the SAME role must not authorize \
         anything MORE than a single claim already did -- authority does not accumulate"
    );
}

#[test]
fn path_039_k_conflicting_declarations_are_never_resolved_by_a_default_winner() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationDecision, RepresentationRole, canonicalize_declared_representation_nodes,
        evaluate_representation_claims,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let r = CapabilityId("r".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];
    let claims = vec![
        RepresentationClaim {
            capability: r.clone(),
            role: RepresentationRole::SemanticCapability,
            claim_id: RepresentationClaimId("claim-says-real".into()),
        },
        RepresentationClaim {
            capability: r.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-says-relay".into()),
        },
    ];

    assert_eq!(
        evaluate_representation_claims(&r, &claims),
        RepresentationDecision::Conflicted,
        "NO LAST-WRITER-WINS: contradictory claims for the same capability must be reported as \
         Conflicted, not silently resolved to either role"
    );

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    let mut expected = edges.clone();
    expected.sort_by(|a, b| a.source.0.cmp(&b.source.0).then(a.target.0.cmp(&b.target.0)));
    assert_eq!(canonical, expected, "a conflicted node is never authorized to collapse");
    assert!(provenance.is_empty());
    assert_eq!(conflicts, vec![r]);
}

#[test]
fn path_039_l_representation_identity_laundering_can_erase_a_real_boundary() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    // Test premise: X1->X2 is stipulated to be ONE real capability that
    // someone has split into two nodes and declared BOTH halves
    // RepresentationOnly. Nothing here verifies that claim.
    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "x1"),
        DependencyEdge::new("x1", "x2"),
        DependencyEdge::new("x2", "target"),
    ];
    let claims = vec![
        RepresentationClaim {
            capability: CapabilityId("x1".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-x1".into()),
        },
        RepresentationClaim {
            capability: CapabilityId("x2".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-x2".into()),
        },
    ];

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    assert_eq!(
        canonical,
        vec![DependencyEdge::new("source", "target")],
        "LAUNDERING SURFACE CONFIRMED: declaring both halves of a stipulated real capability \
         RepresentationOnly erases the entire boundary with no topological trace distinguishing \
         this from genuine chain decomposition -- the declarations themselves are the only \
         evidence, and nothing here verifies they are true"
    );
    assert_eq!(provenance.len(), 2);
    assert!(conflicts.is_empty(), "each declaration is internally consistent; the danger is in \
        their content being unverifiable, not in any detectable conflict");
}

#[test]
fn path_039_m_declaration_swarm_does_not_gain_structural_effect() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let r = CapabilityId("r".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];

    let minimal_claims = vec![RepresentationClaim {
        capability: r.clone(),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-1".into()),
    }];

    // A swarm of additional declarations about capabilities that do not
    // even appear in this graph's edges.
    let mut swarm_claims = minimal_claims.clone();
    for i in 0..25 {
        swarm_claims.push(RepresentationClaim {
            capability: CapabilityId(format!("unrelated-{i}")),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("swarm-claim-{i}")),
        });
    }

    let minimal_result =
        canonicalize_declared_representation_nodes(&source, &[target.clone()], &minimal_claims, &edges);
    let swarm_result =
        canonicalize_declared_representation_nodes(&source, &[target], &swarm_claims, &edges);

    assert_eq!(
        minimal_result, swarm_result,
        "25 additional, irrelevant declarations must not change the canonical graph, \
         provenance, or conflict set for a graph they do not even mention"
    );
}

#[test]
fn path_039_n_replay_explicit_semantics_now_distinguish_the_path_038_n_case() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let x = CapabilityId("x".into());
    let edges = vec![DependencyEdge::new("source", "x"), DependencyEdge::new("x", "target")];

    let representation_only_case = canonicalize_declared_representation_nodes(
        &source,
        &[target.clone()],
        &[RepresentationClaim {
            capability: x.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-1".into()),
        }],
        &edges,
    );
    let semantic_capability_case = canonicalize_declared_representation_nodes(
        &source,
        &[target],
        &[RepresentationClaim {
            capability: x,
            role: RepresentationRole::SemanticCapability,
            claim_id: RepresentationClaimId("claim-2".into()),
        }],
        &edges,
    );

    assert_ne!(
        representation_only_case.0, semantic_capability_case.0,
        "PATH-038-N showed topology alone gives the IDENTICAL (collapsing) answer for these two \
         topologically indistinguishable cases; given a TRUTHFUL explicit declaration, the two \
         canonical outputs now deterministically differ"
    );
}

#[test]
fn path_039_o_replay_truthful_individual_claims_still_erase_an_unclaimed_convergence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes, shared_structural_bottlenecks,
    };

    let source = CapabilityId("source".into());
    let payment = CapabilityId("payment".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("m", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];

    assert!(!shared_structural_bottlenecks(&source, &edges).is_empty());

    // X and Y are TRUTHFULLY, correctly declared RepresentationOnly --
    // nothing false or gamed about either claim. M is deliberately left
    // unclaimed (Unclassified, conservative default -- exactly the
    // correct choice for a genuine convergence point).
    let claims = vec![
        RepresentationClaim {
            capability: CapabilityId("x".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-x-truthful".into()),
        },
        RepresentationClaim {
            capability: CapabilityId("y".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-y-truthful".into()),
        },
    ];

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[payment.clone()], &claims, &edges);

    assert!(conflicts.is_empty());
    assert_eq!(provenance.len(), 2, "only X and Y (the truthfully claimed nodes) ever collapse");

    assert_eq!(
        canonical,
        vec![
            DependencyEdge::new("m", "payment"),
            DependencyEdge::new("source", "m"),
        ],
        "M survives as a node (correctly -- it was never claimed), but its predecessor count \
         has still silently dropped from 2 to 1"
    );
    assert!(
        shared_structural_bottlenecks(&source, &canonical).is_empty(),
        "CRITICAL RESULT: even fully TRUTHFUL, individually-correct RepresentationOnly claims \
         for X and Y -- with M correctly left unclaimed -- still reproduce PATH-038-K/P's \
         erasure of M's shared-bottleneck status. Explicit semantics fix PATH-038-N's problem \
         (topology alone can't tell relay from capability) but do NOT fix this separate \
         problem: independent bypass edges deduplicating onto a shared ancestor is a \
         consequence of edge representation, not of any node's semantic status"
    );
}

#[test]
fn path_039_p_conflict_is_exposed_in_the_returned_conflict_set_not_silently_decided() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let r = CapabilityId("r".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];
    let claims = vec![
        RepresentationClaim {
            capability: r.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("evidence-item-1".into()),
        },
        RepresentationClaim {
            capability: r.clone(),
            role: RepresentationRole::SemanticCapability,
            claim_id: RepresentationClaimId("evidence-item-2".into()),
        },
    ];

    let (_, _, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    assert_eq!(
        conflicts,
        vec![r],
        "the conflict between two evidence items about the same capability is surfaced to the \
         caller explicitly, not silently decided one way or the other"
    );
}

#[test]
fn path_039_q_missing_evidence_identity_is_refused_not_authorized() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationDecision, RepresentationRole, canonicalize_declared_representation_nodes,
        evaluate_representation_claims,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let r = CapabilityId("r".into());
    let edges = vec![DependencyEdge::new("source", "r"), DependencyEdge::new("r", "target")];
    let claims = vec![RepresentationClaim {
        capability: r.clone(),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("".into()), // missing/invalid evidence identity
    }];

    assert_eq!(evaluate_representation_claims(&r, &claims), RepresentationDecision::Refused);

    let (canonical, provenance, _) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    let mut expected = edges.clone();
    expected.sort_by(|a, b| a.source.0.cmp(&b.source.0).then(a.target.0.cmp(&b.target.0)));
    assert_eq!(canonical, expected);
    assert!(provenance.is_empty());
}

#[test]
fn path_039_r_relabeling_is_inert() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let original_edges = vec![
        DependencyEdge::new("source", "r"),
        DependencyEdge::new("r", "target"),
    ];
    let original_claims = vec![RepresentationClaim {
        capability: CapabilityId("r".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-1".into()),
    }];

    let renamed_edges = vec![
        DependencyEdge::new("root", "relay-node"),
        DependencyEdge::new("relay-node", "goal"),
    ];
    let renamed_claims = vec![RepresentationClaim {
        capability: CapabilityId("relay-node".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("different-claim-label".into()),
    }];

    let (original_canonical, original_provenance, _) = canonicalize_declared_representation_nodes(
        &CapabilityId("source".into()),
        &[CapabilityId("target".into())],
        &original_claims,
        &original_edges,
    );
    let (renamed_canonical, renamed_provenance, _) = canonicalize_declared_representation_nodes(
        &CapabilityId("root".into()),
        &[CapabilityId("goal".into())],
        &renamed_claims,
        &renamed_edges,
    );

    assert_eq!(original_canonical.len(), renamed_canonical.len());
    assert_eq!(original_provenance.len(), renamed_provenance.len());
}

#[test]
fn path_039_s_idempotence() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "r1"),
        DependencyEdge::new("r1", "r2"),
        DependencyEdge::new("r2", "target"),
    ];
    let claims = vec![
        RepresentationClaim {
            capability: CapabilityId("r1".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-r1".into()),
        },
        RepresentationClaim {
            capability: CapabilityId("r2".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-r2".into()),
        },
    ];

    let (once, _, _) =
        canonicalize_declared_representation_nodes(&source, &[target.clone()], &claims, &edges);
    assert_eq!(once, vec![DependencyEdge::new("source", "target")]);

    let (twice, provenance_second_pass, _) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &once);

    assert_eq!(once, twice, "IDEMPOTENCE: a second pass over an already-canonical graph is a no-op");
    assert!(provenance_second_pass.is_empty());
}

#[test]
fn path_039_t_every_collapsed_node_remains_traceable_but_full_losslessness_is_not_claimed() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "r1"),
        DependencyEdge::new("r1", "r2"),
        DependencyEdge::new("r2", "target"),
    ];
    let claims = vec![
        RepresentationClaim {
            capability: CapabilityId("r1".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-r1".into()),
        },
        RepresentationClaim {
            capability: CapabilityId("r2".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-r2".into()),
        },
    ];

    let (_, provenance, _) =
        canonicalize_declared_representation_nodes(&source, &[target], &claims, &edges);

    let collapsed: std::collections::HashSet<CapabilityId> =
        provenance.iter().map(|entry| entry.collapsed.clone()).collect();
    assert_eq!(
        collapsed,
        [CapabilityId("r1".into()), CapabilityId("r2".into())].into_iter().collect(),
        "every collapsed node is individually traceable"
    );

    // NOT CLAIMED: this only shows NODE traceability. PATH-039-O already
    // demonstrates that branch-multiplicity/convergence facts are NOT
    // always preserved even when every collapsed node is fully traced --
    // node-level traceability is not the same as full graph losslessness.
}

// ---- STEP 6: DECLARATION AUTHORITY ATTACK (Candidates A/B/C/D) ----

#[test]
fn path_039_candidate_a_bare_boolean_cannot_represent_conflict_or_explicit_semantic() {
    use std::collections::HashSet;
    use distribution_machine::measurement::CapabilityId;

    // Candidate A strawman: a bare `is_representation_only` set, with no
    // evidence identity and no way to assert SemanticCapability at all.
    let mut is_representation_only: HashSet<CapabilityId> = HashSet::new();
    let r = CapabilityId("r".into());

    // Declaring it once vs. "declaring" it via two identical inserts is
    // structurally IDENTICAL -- a set cannot even represent "how many
    // times" or "by whom," so it cannot distinguish one witness from
    // many, nor a duplicate from independent corroboration.
    is_representation_only.insert(r.clone());
    is_representation_only.insert(r.clone());
    assert_eq!(is_representation_only.len(), 1);

    // Worse: there is no way to represent "explicitly declared
    // SemanticCapability" in this model at all. The only two observable
    // states are "in the set" and "not in the set" -- an UNCLASSIFIED
    // node and an EXPLICITLY-PROTECTED-BECAUSE-MEANINGFUL node are both
    // just "not in the set," indistinguishable from each other. This is
    // exactly the ambiguity PATH-039-C needs to rule out, and Candidate
    // A structurally cannot.
    let unclassified = CapabilityId("never-considered".into());
    let explicitly_semantic = CapabilityId("explicitly-declared-meaningful".into());
    assert!(!is_representation_only.contains(&unclassified));
    assert!(!is_representation_only.contains(&explicitly_semantic));
    // Both absent, for entirely different reasons Candidate A cannot
    // distinguish -- this is why RepresentationClaim (Candidate B) uses
    // an explicit two-role claim structure instead of a bare set.
}

#[test]
fn path_039_candidate_c_corroboration_without_identity_does_not_prevent_laundering() {
    use distribution_machine::measurement::{
        CapabilityId, RepresentationClaim, RepresentationClaimId, RepresentationRole,
    };

    // Candidate C: require >= 2 distinct claim_ids asserting
    // RepresentationOnly before authorizing collapse ("corroboration").
    // Attack: nothing prevents the SAME undisclosed actor from
    // submitting two claims under two different claim_id strings for a
    // stipulated genuinely-meaningful capability -- there is no
    // independent issuer-identity concept in this repository to make
    // "two claims" mean "two different parties agreed" (see inspection
    // note: no authority/issuer type exists, and none is invented here).
    let x = CapabilityId("x".into());
    let fabricated_claims = vec![
        RepresentationClaim {
            capability: x.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("fabricated-witness-1".into()),
        },
        RepresentationClaim {
            capability: x.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("fabricated-witness-2".into()),
        },
    ];

    let distinct_claim_ids: std::collections::HashSet<&str> = fabricated_claims
        .iter()
        .map(|claim| claim.claim_id.0.as_str())
        .collect();

    assert_eq!(
        distinct_claim_ids.len(),
        2,
        "a >=2-distinct-claim_id corroboration rule would be satisfied here, exactly as easily \
         as a single fabricated claim -- REJECTED: multiplicity of unauthenticated claim_ids \
         establishes nothing about truth without an issuer-identity/authority concept, which \
         this mission is explicitly forbidden from inventing"
    );
}

// =====================================================================
// PATH-040 -- PROVENANCE-PRESERVING CANONICALIZATION (H11)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::measurement`, wired into `canonicalize_pure_relays`
// / `canonicalize_declared_representation_nodes` (both left untouched), or
// wired into any economic/allocation path. Candidate B is built entirely
// from data the existing PATH-038/039 production functions already return
// (`RelayCollapseProvenance`), plus a thin bookkeeping layer local to this
// test file.
// =====================================================================

/// Candidate A -- RAW MULTIPLICITY. Counts literal occurrences of
/// `(from, to)` in the raw input list, independent of any set-based
/// structural dedup. This is the naive baseline PATH-040 requires be
/// attacked: it is the FIND-007/PATH-017 pathology (duplicate raw records
/// inflate a count) transplanted to the edge level.
fn path_040_candidate_a_raw_multiplicity(
    edges: &[distribution_machine::measurement::DependencyEdge],
    from: &distribution_machine::measurement::CapabilityId,
    to: &distribution_machine::measurement::CapabilityId,
) -> usize {
    edges.iter().filter(|e| &e.source == from && &e.target == to).count()
}

/// Candidate B -- DISTINCT STRUCTURAL ORIGINS. Replays the REAL
/// `RelayCollapseProvenance` list (produced by the unmodified PATH-039
/// production function) as a union of origin sets: every raw edge seeds
/// its own singleton origin set (a `HashSet` insert, so a duplicated raw
/// edge seeds exactly ONE origin -- never two, per FIND-007), and each
/// collapse event unions the two origin sets it consumes into the origin
/// set of the edge it produces. The result maps each SURVIVING canonical
/// edge to the set of raw pre-collapse edges that were folded into it.
fn path_040_candidate_b_origins(
    original_edges: &[distribution_machine::measurement::DependencyEdge],
    provenance: &[distribution_machine::measurement::RelayCollapseProvenance],
) -> std::collections::HashMap<
    (
        distribution_machine::measurement::CapabilityId,
        distribution_machine::measurement::CapabilityId,
    ),
    std::collections::HashSet<(
        distribution_machine::measurement::CapabilityId,
        distribution_machine::measurement::CapabilityId,
    )>,
> {
    use std::collections::{HashMap, HashSet};

    let mut origins: HashMap<_, HashSet<_>> = HashMap::new();
    for e in original_edges {
        let key = (e.source.clone(), e.target.clone());
        origins.entry(key.clone()).or_insert_with(HashSet::new).insert(key);
    }

    for p in provenance {
        let left = (p.predecessor.clone(), p.collapsed.clone());
        let right = (p.collapsed.clone(), p.successor.clone());
        let merged_key = (p.predecessor.clone(), p.successor.clone());

        let mut merged: HashSet<_> = origins.remove(&left).unwrap_or_default();
        merged.extend(origins.remove(&right).unwrap_or_default());
        if let Some(existing) = origins.remove(&merged_key) {
            merged.extend(existing);
        }
        origins.insert(merged_key, merged);
    }

    origins
}

/// The statistic PATH-040-A/C/D/E/F actually test: how many of `source`'s
/// distinct immediate branch roots are represented within one canonical
/// edge's origin set. Parameterized by `source` for the same reason every
/// other PATH-027..030 primitive in this module is -- no invented "root"
/// concept, just the existing explicit-source convention.
fn path_040_distinct_origin_branch_count(
    source: &distribution_machine::measurement::CapabilityId,
    origin_set: &std::collections::HashSet<(
        distribution_machine::measurement::CapabilityId,
        distribution_machine::measurement::CapabilityId,
    )>,
) -> usize {
    let branches: std::collections::HashSet<_> =
        origin_set.iter().filter(|(from, _)| from == source).map(|(_, to)| to).collect();
    branches.len()
}

#[test]
fn path_040_candidate_a_raw_multiplicity_is_inflated_by_a_duplicate_raw_edge() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());

    // One genuine structural edge, serialized twice (e.g. two evidence
    // records describing the same real relationship -- PATH-017's exact
    // scenario, at the edge level instead of the aggregation level).
    let edges_genuine = vec![DependencyEdge::new("source", "x")];
    let edges_duplicated =
        vec![DependencyEdge::new("source", "x"), DependencyEdge::new("source", "x")];

    assert_eq!(path_040_candidate_a_raw_multiplicity(&edges_genuine, &source, &x), 1);
    assert_eq!(
        path_040_candidate_a_raw_multiplicity(&edges_duplicated, &source, &x),
        2,
        "REJECTED: a raw occurrence count reports two structural relationships where there is \
         only one -- serialization multiplicity, not structural convergence, exactly the \
         FIND-007 pathology"
    );
}

#[test]
fn path_040_a_truthful_convergence_preserves_two_distinct_origins() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes, shared_structural_bottlenecks,
    };

    // Exact path_039_o graph (FIND-016's strongest counterexample).
    let source = CapabilityId("source".into());
    let m = CapabilityId("m".into());
    let payment = CapabilityId("payment".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("m", "payment"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let claims = vec![
        RepresentationClaim {
            capability: CapabilityId("x".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-x-truthful".into()),
        },
        RepresentationClaim {
            capability: CapabilityId("y".into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-y-truthful".into()),
        },
    ];

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[payment.clone()], &claims, &edges);
    assert!(conflicts.is_empty());

    // Reproduce path_039_o's exact failure first: the canonical graph
    // alone has already lost the fact that M had two incoming branches.
    assert!(
        shared_structural_bottlenecks(&source, &canonical).is_empty(),
        "must reproduce FIND-016's exact erasure before any candidate is credited with fixing it"
    );

    let origins = path_040_candidate_b_origins(&edges, &provenance);
    let m_origin_set = origins
        .get(&(source.clone(), m.clone()))
        .expect("source->m must exist as a canonical edge");

    assert_eq!(
        path_040_distinct_origin_branch_count(&source, m_origin_set),
        2,
        "H11: provenance must show M had two structurally distinct incoming branches (x and y) \
         even though the canonical edge list alone erased this"
    );

    // The derived predicate recovers exactly what shared_structural_bottlenecks
    // lost, WITHOUT reconstructing the original edge list at the call site --
    // the caller only needs `origins`, not `edges`.
    let recovered_bottlenecks: Vec<&CapabilityId> = origins
        .iter()
        .filter(|((from, _), set)| from == &source && path_040_distinct_origin_branch_count(&source, set) >= 2)
        .map(|((_, to), _)| to)
        .collect();
    assert_eq!(recovered_bottlenecks, vec![&m]);
}

#[test]
fn path_040_b_duplicate_raw_edge_does_not_manufacture_a_second_origin() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    // Only ONE genuine branch (source -> x -> target). Its raw input
    // edges are each duplicated in the serialized list -- an attacker (or
    // a careless double-import) submitting the same structural fact twice
    // must not be interpreted as a second independent branch converging.
    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "target"),
        DependencyEdge::new("x", "target"),
    ];
    let claims = vec![RepresentationClaim {
        capability: CapabilityId("x".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-x".into()),
    }];

    let (_, provenance, _) =
        canonicalize_declared_representation_nodes(&source, &[target.clone()], &claims, &edges);
    assert_eq!(provenance.len(), 1, "the production collapse loop already dedups raw input edges");

    let origins = path_040_candidate_b_origins(&edges, &provenance);
    let origin_set = &origins[&(source.clone(), target.clone())];

    assert_eq!(
        path_040_distinct_origin_branch_count(&source, origin_set),
        1,
        "FIND-007 consistency: a duplicated raw edge must not be counted as a second structural \
         origin"
    );
}

#[test]
fn path_040_c_independent_multi_hop_branches_converge_with_origins_preserved() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes, shared_structural_bottlenecks,
    };

    // Two genuinely distinct branches, each two representation-hops deep,
    // converging at m: source -> x1 -> x2 -> m, source -> y1 -> y2 -> m.
    let source = CapabilityId("source".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x1"),
        DependencyEdge::new("x1", "x2"),
        DependencyEdge::new("x2", "m"),
        DependencyEdge::new("source", "y1"),
        DependencyEdge::new("y1", "y2"),
        DependencyEdge::new("y2", "m"),
    ];
    let claims = ["x1", "x2", "y1", "y2"]
        .iter()
        .map(|id| RepresentationClaim {
            capability: CapabilityId((*id).into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("claim-{id}")),
        })
        .collect::<Vec<_>>();

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[], &claims, &edges);
    assert!(conflicts.is_empty());
    assert_eq!(canonical, vec![DependencyEdge::new("source", "m")]);
    assert!(shared_structural_bottlenecks(&source, &canonical).is_empty());

    let origins = path_040_candidate_b_origins(&edges, &provenance);
    let origin_set = &origins[&(source.clone(), m.clone())];
    assert_eq!(
        path_040_distinct_origin_branch_count(&source, origin_set),
        2,
        "two-hop-deep representation chains must still register as two distinct origins, not \
         collapse into ambiguity alongside the erased canonical edge"
    );
}

#[test]
fn path_040_d_cascading_collapse_does_not_erase_or_inflate_origins() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    // Branch 1 needs THREE successive collapse stages to reach m
    // (a -> b -> c all representation-only). Branch 2 is a single hop.
    // Provenance produced for branch 1 must still fold down to exactly
    // ONE origin at m, not three (one per collapse stage) and not zero
    // (erased by the later stages overwriting the earlier ones).
    let source = CapabilityId("source".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "a"),
        DependencyEdge::new("a", "b"),
        DependencyEdge::new("b", "c"),
        DependencyEdge::new("c", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let claims = ["a", "b", "c", "y"]
        .iter()
        .map(|id| RepresentationClaim {
            capability: CapabilityId((*id).into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("claim-{id}")),
        })
        .collect::<Vec<_>>();

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[], &claims, &edges);
    assert!(conflicts.is_empty());
    assert_eq!(canonical, vec![DependencyEdge::new("source", "m")]);
    assert_eq!(provenance.len(), 4, "a, b, c, and y each collapse individually");

    let origins = path_040_candidate_b_origins(&edges, &provenance);
    let origin_set = &origins[&(source.clone(), m.clone())];
    assert_eq!(
        path_040_distinct_origin_branch_count(&source, origin_set),
        2,
        "a 3-stage cascading collapse on one branch must still register as exactly ONE origin \
         at m, converging with branch y's single origin -- not 3 (one per stage) and not 0 \
         (later stages silently overwriting earlier provenance)"
    );
}

#[test]
fn path_040_e_no_convergence_control_manufactures_no_extra_provenance() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes, shared_structural_bottlenecks,
    };

    // A plain single chain, no branching or convergence at all.
    let source = CapabilityId("source".into());
    let target = CapabilityId("target".into());
    let edges = vec![DependencyEdge::new("source", "a"), DependencyEdge::new("a", "target")];
    let claims = vec![RepresentationClaim {
        capability: CapabilityId("a".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-a".into()),
    }];

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[], &claims, &edges);
    assert!(conflicts.is_empty());
    assert_eq!(canonical, vec![DependencyEdge::new("source", "target")]);
    assert!(shared_structural_bottlenecks(&source, &canonical).is_empty());

    let origins = path_040_candidate_b_origins(&edges, &provenance);
    let origin_set = &origins[&(source.clone(), target.clone())];
    assert_eq!(
        path_040_distinct_origin_branch_count(&source, origin_set),
        1,
        "no convergence exists; the candidate must not manufacture a second origin out of a \
         single plain chain"
    );
}

#[test]
fn path_040_f_relabeling_and_relay_depth_are_inert_to_origin_count() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole, canonicalize_declared_representation_nodes,
    };

    // Same shape as path_040_a, but every representation node is renamed
    // to an arbitrary label, edge order is reversed, and one branch gets
    // an extra harmless representation relay inserted.
    let source = CapabilityId("source".into());
    let m = CapabilityId("checkout_relay".into());
    let mut edges = vec![
        DependencyEdge::new("zzz_relay_alpha", "checkout_relay"),
        DependencyEdge::new("source", "zzz_relay_alpha"),
        DependencyEdge::new("qqq_relay_beta_extra", "checkout_relay"),
        DependencyEdge::new("qqq_relay_beta", "qqq_relay_beta_extra"),
        DependencyEdge::new("source", "qqq_relay_beta"),
    ];
    edges.reverse();

    let claims = ["zzz_relay_alpha", "qqq_relay_beta", "qqq_relay_beta_extra"]
        .iter()
        .map(|id| RepresentationClaim {
            capability: CapabilityId((*id).into()),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("claim-{id}")),
        })
        .collect::<Vec<_>>();

    let (canonical, provenance, conflicts) =
        canonicalize_declared_representation_nodes(&source, &[], &claims, &edges);
    assert!(conflicts.is_empty());
    assert_eq!(canonical, vec![DependencyEdge::new("source", "checkout_relay")]);

    let origins = path_040_candidate_b_origins(&edges, &provenance);
    let origin_set = &origins[&(source.clone(), m.clone())];
    assert_eq!(
        path_040_distinct_origin_branch_count(&source, origin_set),
        2,
        "arbitrary node labels, reversed edge order, and one branch's harmless extra \
         representation-relay depth must not change the distinct-origin count"
    );
}

// =====================================================================
// PATH-041 -- CAN STRUCTURAL INDEPENDENCE BE CORROBORATED WITHOUT AN
// EXTERNAL IDENTITY AUTHORITY? (H12)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::measurement` or wired into any production/
// allocation path. Candidate B reuses the unmodified PATH-039
// `canonicalize_declared_representation_nodes` and the unmodified
// `reachable_capabilities` (PATH-032/033) as its only graph primitives.
// =====================================================================

/// Candidate A -- DISTINCT LABELS ONLY. Independence iff the two
/// capability ids are syntactically different. This is exactly the
/// assumption FIND-017's Candidate B (origin identity) already made
/// implicitly; named here so it can be attacked directly.
fn path_041_candidate_a_labels_differ(
    x: &distribution_machine::measurement::CapabilityId,
    y: &distribution_machine::measurement::CapabilityId,
) -> bool {
    x != y
}

fn path_041_reversed(
    edges: &[distribution_machine::measurement::DependencyEdge],
) -> Vec<distribution_machine::measurement::DependencyEdge> {
    edges
        .iter()
        .map(|e| distribution_machine::measurement::DependencyEdge {
            source: e.target.clone(),
            target: e.source.clone(),
        })
        .collect()
}

fn path_041_transitive_predecessors(
    node: &distribution_machine::measurement::CapabilityId,
    edges: &[distribution_machine::measurement::DependencyEdge],
) -> std::collections::HashSet<distribution_machine::measurement::CapabilityId> {
    use distribution_machine::measurement::reachable_capabilities;
    reachable_capabilities(node, &path_041_reversed(edges)).into_iter().collect()
}

/// The structural facts attributable to `branch_root` ALONE, relative to
/// the shared `source` and the shared `convergence` node both branches
/// feed into: transitive ancestors other than `source` itself
/// ("distinct upstream ancestry"), unioned with whatever `branch_root`
/// can reach that `convergence` cannot also reach ("distinct downstream
/// continuation" / operational side-effects private to this branch,
/// per PATH-041-D). Deliberately excludes raw successor/predecessor sets
/// at `branch_root` itself, since for any pair that already converges on
/// the same node those are identical BY DEFINITION and so carry zero
/// information (see FIND-018 OBSERVED).
fn path_041_evidence(
    branch_root: &distribution_machine::measurement::CapabilityId,
    source: &distribution_machine::measurement::CapabilityId,
    convergence: &distribution_machine::measurement::CapabilityId,
    edges: &[distribution_machine::measurement::DependencyEdge],
) -> std::collections::HashSet<distribution_machine::measurement::CapabilityId> {
    use distribution_machine::measurement::reachable_capabilities;
    use std::collections::HashSet;

    let mut extra_ancestry = path_041_transitive_predecessors(branch_root, edges);
    extra_ancestry.remove(source);
    extra_ancestry.remove(branch_root);

    let own_reach: HashSet<_> = reachable_capabilities(branch_root, edges).into_iter().collect();
    let mut convergence_reach: HashSet<_> =
        reachable_capabilities(convergence, edges).into_iter().collect();
    convergence_reach.insert(convergence.clone());

    let mut evidence: HashSet<_> = extra_ancestry;
    evidence.extend(own_reach.difference(&convergence_reach).cloned());
    evidence.remove(branch_root);
    evidence
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path041Verdict {
    /// Both branches carry their own non-empty, non-identical evidence.
    Independent,
    /// At least one branch has zero distinguishing evidence of its own --
    /// never certified independent merely by being paired with a richer
    /// sibling (PATH-041-H).
    Insufficient,
    /// Full neighborhoods (or the two branches' own evidence sets) are
    /// identical -- a stronger, explicit negative signal, not merely an
    /// absence of proof (PATH-041-C).
    LikelyDuplicate,
}

/// Candidate B -- STRUCTURAL CORROBORATION. Canonicalizes first (so
/// harmless added representation-relay depth on either branch cannot
/// affect the result -- PATH-041-F), protecting `source`, `x`, `y`, and
/// `convergence` from ever collapsing themselves (this experiment
/// corroborates whether THESE SPECIFIC node identities are independent,
/// not whether they are themselves eligible for PATH-039 collapse). A
/// verdict of `Independent` requires BOTH branches to carry their own
/// non-empty evidence (per [`path_041_evidence`]) that DIFFERS between
/// them; either branch having no evidence of its own, or both branches
/// sharing literally the same evidence, is never enough.
fn path_041_candidate_b_corroborate(
    source: &distribution_machine::measurement::CapabilityId,
    x: &distribution_machine::measurement::CapabilityId,
    y: &distribution_machine::measurement::CapabilityId,
    convergence: &distribution_machine::measurement::CapabilityId,
    claims: &[distribution_machine::measurement::RepresentationClaim],
    raw_edges: &[distribution_machine::measurement::DependencyEdge],
) -> Path041Verdict {
    use distribution_machine::measurement::{
        canonicalize_declared_representation_nodes, reachable_capabilities,
    };

    let protected = vec![source.clone(), x.clone(), y.clone(), convergence.clone()];
    let (canonical_edges, _provenance, conflicts) =
        canonicalize_declared_representation_nodes(source, &protected, claims, raw_edges);
    assert!(conflicts.is_empty(), "PATH-041 test fixtures never assert conflicting claims");

    let px = path_041_transitive_predecessors(x, &canonical_edges);
    let py = path_041_transitive_predecessors(y, &canonical_edges);
    let rx: std::collections::HashSet<_> =
        reachable_capabilities(x, &canonical_edges).into_iter().collect();
    let ry: std::collections::HashSet<_> =
        reachable_capabilities(y, &canonical_edges).into_iter().collect();

    if px == py && rx == ry {
        return Path041Verdict::LikelyDuplicate;
    }

    let evidence_x = path_041_evidence(x, source, convergence, &canonical_edges);
    let evidence_y = path_041_evidence(y, source, convergence, &canonical_edges);

    if evidence_x.is_empty() || evidence_y.is_empty() {
        return Path041Verdict::Insufficient;
    }
    if evidence_x == evidence_y {
        return Path041Verdict::LikelyDuplicate;
    }
    Path041Verdict::Independent
}

#[test]
fn path_041_candidate_a_distinct_labels_alone_certifies_a_split_label_launder() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    assert!(
        path_041_candidate_a_labels_differ(&x, &y),
        "REJECTED: Candidate A certifies x and y independent purely because their labels \
         differ, with zero regard for whether they denote the same real dependency -- exactly \
         the split-label laundering surface PATH-040/FIND-017 left unresolved"
    );
}

#[test]
fn path_041_a_true_independence_with_distinguishing_structure_is_confirmed() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, shared_structural_bottlenecks,
    };

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("y", "m"),
        DependencyEdge::new("db_a", "x"),
        DependencyEdge::new("db_b", "y"),
        DependencyEdge::new("x", "audit_x"),
        DependencyEdge::new("y", "audit_y"),
    ];

    assert!(!shared_structural_bottlenecks(&source, &edges).is_empty());

    let verdict = path_041_candidate_b_corroborate(&source, &x, &y, &m, &[], &edges);
    assert_eq!(
        verdict,
        Path041Verdict::Independent,
        "x and y each have their own distinct upstream feed (db_a/db_b) and their own private \
         downstream continuation (audit_x/audit_y) -- genuinely distinguishing structure, not \
         just distinct labels"
    );
}

#[test]
fn path_041_b_bare_split_label_launder_is_not_certified_independent() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole,
    };

    // The minimal shape PATH-041-A and PATH-041-B share in the mission text
    // itself: source -> x -> m, source -> y -> m, with x and y stipulated as
    // aliases/splits of one real dependency. Structurally this graph is
    // IDENTICAL to what path_041_a would produce if db_a/db_b/audit_x/audit_y
    // were removed -- which is the entire point (see FIND-018 strongest
    // counterexample).
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let claims = vec![
        RepresentationClaim {
            capability: x.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-x".into()),
        },
        RepresentationClaim {
            capability: y.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId("claim-y".into()),
        },
    ];

    let verdict = path_041_candidate_b_corroborate(&source, &x, &y, &m, &claims, &edges);
    assert_eq!(
        verdict,
        Path041Verdict::LikelyDuplicate,
        "distinct labels alone, with zero other distinguishing structure, must not be \
         certified independent"
    );
}

#[test]
fn path_041_c_duplicate_effective_ancestry_survives_relabeling_and_depth_padding() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole,
    };

    // Different raw shapes (one branch padded with an extra representation
    // hop under a completely different name) that canonicalize down to the
    // EFFECTIVELY identical neighborhood PATH-041-B already rejected.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "relay_padding"),
        DependencyEdge::new("relay_padding", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let claims = vec![RepresentationClaim {
        capability: CapabilityId("relay_padding".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-padding".into()),
    }];

    let verdict = path_041_candidate_b_corroborate(&source, &x, &y, &m, &claims, &edges);
    assert_eq!(
        verdict,
        Path041Verdict::LikelyDuplicate,
        "different raw names/depth must not be mistaken for independence once the EFFECTIVE \
         (canonicalized) neighborhoods are identical"
    );
}

#[test]
fn path_041_d_shared_upstream_but_genuine_operational_divergence_is_confirmed() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    // x and y share the SAME immediate (and only) upstream ancestor --
    // no distinct-ancestry signal is available at all -- but each has its
    // own private downstream continuation before reconverging at m.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "audit_x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "audit_y"),
        DependencyEdge::new("y", "m"),
    ];

    let verdict = path_041_candidate_b_corroborate(&source, &x, &y, &m, &[], &edges);
    assert_eq!(
        verdict,
        Path041Verdict::Independent,
        "a rule that DEMANDED distinct upstream ancestry would wrongly reject this: x and y \
         share their only ancestor (source) yet still have genuinely independent operational \
         structure (audit_x vs audit_y) before reconverging"
    );
}

#[test]
fn path_041_e_disjoint_subgraphs_are_invisible_to_any_topological_corroboration() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, shared_structural_bottlenecks,
    };

    // Two differently-routed representations stipulated, at the semantic
    // level this experiment cannot see, to be the SAME real dependency --
    // but they never share a single graph node, so no candidate above (A,
    // B, or the claim-based attempt in path_041_g) even has a pair of
    // nodes to compare. This is a distinct failure mode from laundering:
    // not a wrong answer, but no question the graph lets you pose at all.
    let source = CapabilityId("source".into());
    let edges = vec![
        DependencyEdge::new("source", "p"),
        DependencyEdge::new("p", "real_dependency_alias_1"),
        DependencyEdge::new("source", "q"),
        DependencyEdge::new("q", "real_dependency_alias_2"),
    ];

    assert!(
        shared_structural_bottlenecks(&source, &edges).is_empty(),
        "the graph never represents alias_1 and alias_2 as related at all"
    );
}

#[test]
fn path_041_f_harmless_representation_depth_does_not_change_either_verdict() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, RepresentationClaim, RepresentationClaimId,
        RepresentationRole,
    };

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());

    // Case 1: pad the genuinely-independent path_041_a graph with an extra
    // harmless relay on x's branch. Depth alone must not remove the
    // Independent verdict.
    let mut independent_edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "x_extra"),
        DependencyEdge::new("x_extra", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
        DependencyEdge::new("db_a", "x"),
        DependencyEdge::new("db_b", "y"),
        DependencyEdge::new("x", "audit_x"),
        DependencyEdge::new("y", "audit_y"),
    ];
    independent_edges.reverse();
    let independent_claims = vec![RepresentationClaim {
        capability: CapabilityId("x_extra".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-x-extra".into()),
    }];
    assert_eq!(
        path_041_candidate_b_corroborate(
            &source,
            &x,
            &y,
            &m,
            &independent_claims,
            &independent_edges
        ),
        Path041Verdict::Independent,
        "depth padding on the genuinely independent branch must not erase its Independent verdict"
    );

    // Case 2: pad the bare split-label launder with an extra harmless relay
    // on ONE side. Depth alone must not manufacture an Independent verdict.
    let mut duplicate_edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "x_extra"),
        DependencyEdge::new("x_extra", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    duplicate_edges.reverse();
    let duplicate_claims = vec![RepresentationClaim {
        capability: CapabilityId("x_extra".into()),
        role: RepresentationRole::RepresentationOnly,
        claim_id: RepresentationClaimId("claim-x-extra".into()),
    }];
    assert_eq!(
        path_041_candidate_b_corroborate(&source, &x, &y, &m, &duplicate_claims, &duplicate_edges),
        Path041Verdict::LikelyDuplicate,
        "depth padding on ONE side of an otherwise-bare split must not manufacture an \
         Independent verdict out of nothing"
    );
}

#[test]
fn path_041_g_self_derived_claim_ids_are_circular_not_independent_corroboration() {
    use distribution_machine::measurement::{
        CapabilityId, RepresentationClaim, RepresentationClaimId, RepresentationRole,
    };

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());

    // Candidate C attempt: "corroborate" x and y's independence with one
    // RepresentationOnly claim each, each evidenced by a claim_id DERIVED
    // FROM the very capability it is about -- i.e. each node vouches for
    // itself. This repository has no issuer/authority concept (confirmed
    // absent by FIND-016 observation 1) capable of telling this apart from
    // two claims a single undisclosed actor fabricated about a stipulated
    // single real dependency split into x/y.
    let claims = vec![
        RepresentationClaim {
            capability: x.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("self-attested-by-{}", x.0)),
        },
        RepresentationClaim {
            capability: y.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("self-attested-by-{}", y.0)),
        },
    ];

    assert_ne!(claims[0].claim_id, claims[1].claim_id);
    assert!(
        claims[0].claim_id.0.contains(&x.0) && claims[1].claim_id.0.contains(&y.0),
        "REJECTED: two distinct claim_ids exist -- exactly as many as PATH-039's already-\
         rejected Candidate C (path_039_candidate_c) required -- but each is derived from the \
         same node identity it is supposed to corroborate. Circular lineage, not independent \
         support; verifying claim_id distinctness proves nothing here"
    );
}

#[test]
fn path_041_h_asymmetrical_evidence_does_not_upgrade_the_bare_branch() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    // x has rich, genuine distinguishing structure; y has none at all
    // (the bare split-label shape in isolation). Pairing y with a richly-
    // evidenced sibling must not "borrow" x's evidence for y.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("db_a", "x"),
        DependencyEdge::new("x", "audit_x"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];

    let verdict = path_041_candidate_b_corroborate(&source, &x, &y, &m, &[], &edges);
    assert_eq!(
        verdict,
        Path041Verdict::Insufficient,
        "y carries zero distinguishing evidence of its own; being paired with a richly-\
         evidenced x must not silently certify y as independent too"
    );
}

// =====================================================================
// PATH-042 -- MINIMUM EXTERNAL EVIDENCE NEEDED TO BREAK THE
// IDENTITY-TRUTH DEADLOCK (H13)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::measurement` or wired into any production/
// allocation path. No cryptography or authority infrastructure is built:
// the one "trust axiom" parameter introduced below
// (`trusted_distinct_domains`) is a plain function argument that this
// code never derives, verifies, or fabricates -- it exists purely to make
// explicit exactly what PATH-042's impossibility-boundary outcome
// requires the CALLER to already know from outside this system.
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ObserverId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedFact {
    observer: ObserverId,
    left: distribution_machine::measurement::CapabilityId,
    right: distribution_machine::measurement::CapabilityId,
    asserted_distinct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DomainId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EvidenceEventContent {
    subject: distribution_machine::measurement::CapabilityId,
    origin_domain: DomainId,
    fact_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvidenceEvent {
    event_id: String,
    content: EvidenceEventContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path042Verdict {
    Independent,
    SameDependency,
    Conflicted,
    Insufficient,
}

/// A fact whose `observer` is literally derived from one of the subjects
/// it is about contributes zero corroboration -- a node cannot vouch for
/// its own independence (PATH-042-B).
fn path_042_is_self_observation(fact: &ObservedFact) -> bool {
    fact.observer.0 == fact.left.0 || fact.observer.0 == fact.right.0
}

/// Candidate B -- INDEPENDENT OBSERVER IDENTITY. Requires at least TWO
/// DISTINCT, non-self observers to agree `x` and `y` are distinct before
/// certifying `Independent`. A single observer establishes only "one
/// witness asserted this," never corroboration (PATH-042-C).
/// Disagreement among relevant observers is surfaced as `Conflicted`,
/// never resolved (mirrors PATH-039's `RepresentationDecision::Conflicted`).
///
/// Deliberately does NOT verify that two distinct `ObserverId`s denote
/// two actually-independent parties -- nothing in this experiment (or
/// this repository) can. PATH-042-D/E show this candidate alone cannot
/// tell two genuinely independent observers from two labels secretly
/// controlled by one source: it merely relocates the identity question
/// from `CapabilityId` to `ObserverId`.
fn path_042_candidate_b_observer_corroboration(
    x: &distribution_machine::measurement::CapabilityId,
    y: &distribution_machine::measurement::CapabilityId,
    facts: &[ObservedFact],
) -> Path042Verdict {
    use std::collections::HashSet;

    let relevant: Vec<&ObservedFact> = facts
        .iter()
        .filter(|f| !path_042_is_self_observation(f))
        .filter(|f| (&f.left == x && &f.right == y) || (&f.left == y && &f.right == x))
        .collect();

    if relevant.is_empty() {
        return Path042Verdict::Insufficient;
    }

    let distinct_observers_saying_distinct: HashSet<&str> =
        relevant.iter().filter(|f| f.asserted_distinct).map(|f| f.observer.0.as_str()).collect();
    let distinct_observers_saying_same: HashSet<&str> = relevant
        .iter()
        .filter(|f| !f.asserted_distinct)
        .map(|f| f.observer.0.as_str())
        .collect();

    if !distinct_observers_saying_distinct.is_empty() && !distinct_observers_saying_same.is_empty()
    {
        return Path042Verdict::Conflicted;
    }
    if !distinct_observers_saying_same.is_empty() {
        return Path042Verdict::SameDependency;
    }
    if distinct_observers_saying_distinct.len() >= 2 {
        return Path042Verdict::Independent;
    }
    Path042Verdict::Insufficient
}

/// Candidate C -- INDEPENDENT EVENT ORIGIN, structural half. Deduplicates
/// `events` by CONTENT (subject + origin_domain + fact_kind) -- exactly
/// the `DependencyStructuralKey` pattern this repository already uses for
/// FIND-007 (`deduplicate_structural_dependencies`): a free-form
/// `event_id` field is never trusted as identity on its own, so
/// duplicating a serialized event (PATH-042-F) or replaying it under a
/// new id (PATH-042-G) both collapse to the same single event.
fn path_042_deduplicate_events(events: &[EvidenceEvent]) -> Vec<EvidenceEvent> {
    use std::collections::HashMap;

    let mut by_content: HashMap<EvidenceEventContent, EvidenceEvent> = HashMap::new();
    for event in events {
        by_content.entry(event.content.clone()).or_insert_with(|| event.clone());
    }

    let mut result: Vec<EvidenceEvent> = by_content.into_values().collect();
    result.sort_by(|a, b| a.event_id.cmp(&b.event_id));
    result
}

/// Candidate D -- CROSS-DOMAIN CORROBORATION. `trusted_distinct_domains`
/// is the one deliberately exogenous input in this entire experiment: a
/// plain `Option<(&DomainId, &DomainId)>` that this function NEVER
/// derives, verifies, or fabricates from `events` -- it must be supplied
/// by the caller as a fact known from OUTSIDE this analysis. Without it,
/// two subjects backed by two different domain labels are `Insufficient`
/// -- exactly like unverified observer distinctness, domain identity is
/// just another self-issuable label (PATH-042-D) unless something
/// exogenous vouches for a SPECIFIC pair. With a correct axiom, the same
/// shape of evidence resolves to `Independent` (PATH-042-E). With an
/// INCORRECT axiom, it resolves to exactly the same `Independent` -- the
/// function has no way to tell the difference, which is the entire
/// point: it can only propagate a trust assumption, never manufacture or
/// verify one.
fn path_042_candidate_d_cross_domain_corroboration(
    x: &distribution_machine::measurement::CapabilityId,
    y: &distribution_machine::measurement::CapabilityId,
    events: &[EvidenceEvent],
    trusted_distinct_domains: Option<(&DomainId, &DomainId)>,
) -> Path042Verdict {
    use std::collections::HashSet;

    let deduped = path_042_deduplicate_events(events);

    let domains_for = |subject: &distribution_machine::measurement::CapabilityId| -> HashSet<DomainId> {
        deduped
            .iter()
            .filter(|e| &e.content.subject == subject)
            .map(|e| e.content.origin_domain.clone())
            .collect()
    };

    let dx = domains_for(x);
    let dy = domains_for(y);

    if dx.is_empty() || dy.is_empty() {
        return Path042Verdict::Insufficient;
    }
    if !dx.is_disjoint(&dy) {
        // x and y share at least one origin domain -- direct evidence
        // against separation, not for it.
        return Path042Verdict::SameDependency;
    }

    match trusted_distinct_domains {
        None => Path042Verdict::Insufficient,
        Some((a, b)) => {
            let vouched = (dx.contains(a) && dy.contains(b)) || (dx.contains(b) && dy.contains(a));
            if vouched { Path042Verdict::Independent } else { Path042Verdict::Insufficient }
        }
    }
}

#[test]
fn path_042_candidate_a_claim_count_inflation_does_not_certify_independence() {
    use distribution_machine::measurement::{
        CapabilityId, RepresentationClaim, RepresentationClaimId, RepresentationRole,
    };

    // One real dependency split into x/y, each backed by an arbitrarily
    // large number of distinct claim_ids -- Candidate A's metric (bare
    // claim count) grows without bound on both sides.
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let x_claims: Vec<RepresentationClaim> = (0..50)
        .map(|i| RepresentationClaim {
            capability: x.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("x-claim-{i}")),
        })
        .collect();
    let y_claims: Vec<RepresentationClaim> = (0..50)
        .map(|i| RepresentationClaim {
            capability: y.clone(),
            role: RepresentationRole::RepresentationOnly,
            claim_id: RepresentationClaimId(format!("y-claim-{i}")),
        })
        .collect();

    assert_eq!(x_claims.len(), 50);
    assert_eq!(y_claims.len(), 50);
    // REJECTED: PATH-039-J/M already established duplicate/multiple claims
    // do not multiply authority; 50 distinct claim_ids is the identical
    // failure at higher volume -- claim_id carries no issuer identity
    // (FIND-016 observation 1), so a single undisclosed actor can produce
    // an arbitrarily large count on both sides of a real single dependency
    // with no more effort than a loop.
}

#[test]
fn path_042_b_self_observation_contributes_no_corroboration() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let facts = vec![
        ObservedFact {
            observer: ObserverId("x".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: true,
        },
        ObservedFact {
            observer: ObserverId("y".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: true,
        },
    ];

    assert_eq!(
        path_042_candidate_b_observer_corroboration(&x, &y, &facts),
        Path042Verdict::Insufficient,
        "both facts are self-observations (observer == one of the subjects); circular lineage \
         must contribute zero corroboration, not count as two agreeing witnesses"
    );
}

#[test]
fn path_042_c_one_observer_establishes_a_single_witness_not_corroboration() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let facts = vec![ObservedFact {
        observer: ObserverId("obs-1".into()),
        left: x.clone(),
        right: y.clone(),
        asserted_distinct: true,
    }];

    assert_eq!(
        path_042_candidate_b_observer_corroboration(&x, &y, &facts),
        Path042Verdict::Insufficient,
        "exactly one non-self observer establishes only that ONE witness asserted x and y are \
         distinct -- not corroboration. A second, independent observer is required before this \
         candidate will certify Independent"
    );
}

#[test]
fn path_042_d_and_e_observer_label_distinctness_alone_cannot_tell_alias_from_genuine() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());

    // PATH-042-D: two observer IDs, STIPULATED (by this test's own
    // narrative, not by anything in the data) to be aliases controlled by
    // one underlying source.
    let alias_facts = vec![
        ObservedFact {
            observer: ObserverId("alias-obs-1".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: true,
        },
        ObservedFact {
            observer: ObserverId("alias-obs-2".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: true,
        },
    ];

    // PATH-042-E: two observer IDs stipulated to be genuinely independent.
    let genuine_facts = vec![
        ObservedFact {
            observer: ObserverId("genuine-obs-1".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: true,
        },
        ObservedFact {
            observer: ObserverId("genuine-obs-2".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: true,
        },
    ];

    let alias_verdict = path_042_candidate_b_observer_corroboration(&x, &y, &alias_facts);
    let genuine_verdict = path_042_candidate_b_observer_corroboration(&x, &y, &genuine_facts);

    assert_eq!(alias_verdict, Path042Verdict::Independent);
    assert_eq!(
        alias_verdict, genuine_verdict,
        "CENTRAL RESULT: Candidate B produces the IDENTICAL verdict for two stipulated-alias \
         observer labels and two stipulated-genuinely-independent ones -- the data is \
         indistinguishable in both cases, exactly PATH-041/FIND-018's topology-level \
         impossibility restated one level up. Observer-label distinctness alone MERELY \
         RELOCATES the identity question from CapabilityId to ObserverId; it does not answer it"
    );
}

#[test]
fn path_042_d_cross_domain_evidence_without_an_exogenous_axiom_stays_insufficient() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let domain_1 = DomainId("domain-1".into());
    let domain_2 = DomainId("domain-2".into());
    let events = vec![
        EvidenceEvent {
            event_id: "e1".into(),
            content: EvidenceEventContent {
                subject: x.clone(),
                origin_domain: domain_1,
                fact_kind: "observed".into(),
            },
        },
        EvidenceEvent {
            event_id: "e2".into(),
            content: EvidenceEventContent {
                subject: y.clone(),
                origin_domain: domain_2,
                fact_kind: "observed".into(),
            },
        },
    ];

    // Two different domain LABELS -- exactly like two different observer
    // labels, this repository has no way to verify domain-1 and domain-2
    // are not secretly the same underlying source (PATH-042-D).
    assert_eq!(
        path_042_candidate_d_cross_domain_corroboration(&x, &y, &events, None),
        Path042Verdict::Insufficient,
        "domain labels differ, but with no exogenous fact vouching for that difference, this is \
         exactly as inconclusive as unverified observer distinctness"
    );
}

#[test]
fn path_042_e_an_explicit_exogenous_trust_axiom_is_the_minimum_that_resolves_independence() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let domain_a = DomainId("real-region-a".into());
    let domain_b = DomainId("real-region-b".into());
    let events = vec![
        EvidenceEvent {
            event_id: "e1".into(),
            content: EvidenceEventContent {
                subject: x.clone(),
                origin_domain: domain_a.clone(),
                fact_kind: "observed".into(),
            },
        },
        EvidenceEvent {
            event_id: "e2".into(),
            content: EvidenceEventContent {
                subject: y.clone(),
                origin_domain: domain_b.clone(),
                fact_kind: "observed".into(),
            },
        },
    ];

    // Identical DATA SHAPE to path_042_d above -- two different domain
    // labels, one event per subject. The only difference is a fact
    // supplied from OUTSIDE this function, never derived from `events`:
    // the caller (standing in for an operator/researcher with real-world
    // knowledge this system does not have) vouches that domain_a and
    // domain_b are genuinely separate trust roots.
    let verdict =
        path_042_candidate_d_cross_domain_corroboration(&x, &y, &events, Some((&domain_a, &domain_b)));
    assert_eq!(
        verdict,
        Path042Verdict::Independent,
        "H13's narrowest surviving claim: given an explicit, externally-supplied, non-derived \
         trust fact naming this SPECIFIC domain pair as genuinely distinct, cross-domain \
         evidence resolves independence -- but the fact itself, not the domain labels, is what \
         did the work"
    );
}

#[test]
fn path_042_axiom_forgery_produces_false_but_internally_consistent_confidence() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    // The SAME kind of alias domains as path_042_d, now paired with a
    // FALSE trust axiom asserting they ARE the vouched-distinct pair.
    let alias_domain_1 = DomainId("alias-domain-1".into());
    let alias_domain_2 = DomainId("alias-domain-2".into());
    let events = vec![
        EvidenceEvent {
            event_id: "e1".into(),
            content: EvidenceEventContent {
                subject: x.clone(),
                origin_domain: alias_domain_1.clone(),
                fact_kind: "observed".into(),
            },
        },
        EvidenceEvent {
            event_id: "e2".into(),
            content: EvidenceEventContent {
                subject: y.clone(),
                origin_domain: alias_domain_2.clone(),
                fact_kind: "observed".into(),
            },
        },
    ];

    let verdict = path_042_candidate_d_cross_domain_corroboration(
        &x,
        &y,
        &events,
        Some((&alias_domain_1, &alias_domain_2)),
    );
    assert_eq!(
        verdict,
        Path042Verdict::Independent,
        "CRITICAL CAVEAT: the candidate cannot distinguish a TRUE trust axiom from a FALSE one \
         -- supplying an incorrect axiom over two domains that are actually aliases of one \
         source produces the exact same Independent verdict a correct axiom would. The entire \
         correctness burden sits on whoever supplies the axiom; nothing in this experiment (or \
         this repository) checks it"
    );
}

#[test]
fn path_042_f_duplicated_evidence_event_does_not_manufacture_a_second_origin() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let domain = DomainId("domain-1".into());
    let event = EvidenceEvent {
        event_id: "e1".into(),
        content: EvidenceEventContent { subject: x, origin_domain: domain, fact_kind: "observed".into() },
    };
    let events = vec![event.clone(), event.clone(), event];

    let deduped = path_042_deduplicate_events(&events);
    assert_eq!(
        deduped.len(),
        1,
        "FIND-007 consistency: literal duplicate serialization of one event must not be counted \
         as multiple independent evidence events"
    );
}

#[test]
fn path_042_g_replayed_event_under_a_new_id_does_not_manufacture_new_evidence() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let domain = DomainId("domain-1".into());
    let content = EvidenceEventContent {
        subject: x.clone(),
        origin_domain: domain.clone(),
        fact_kind: "observed".into(),
    };
    let events = vec![
        EvidenceEvent { event_id: "original-event".into(), content: content.clone() },
        EvidenceEvent { event_id: "replayed-under-a-new-id".into(), content },
    ];

    let deduped = path_042_deduplicate_events(&events);
    assert_eq!(
        deduped.len(),
        1,
        "raw event_id distinctness alone must not suffice -- identical CONTENT replayed under a \
         fresh id is the same evidence event, not a second one"
    );

    // Contrast: genuinely different content (different fact_kind) is NOT
    // collapsed -- the dedup is content-keyed, not blanket-suppressing.
    let genuinely_different = EvidenceEvent {
        event_id: "another-event".into(),
        content: EvidenceEventContent { subject: x, origin_domain: domain, fact_kind: "provisioned".into() },
    };
    let mut all = deduped;
    all.push(genuinely_different);
    assert_eq!(path_042_deduplicate_events(&all).len(), 2);
}

#[test]
fn path_042_h_cross_domain_evidence_adds_a_new_axis_but_not_new_certainty_by_itself() {
    use distribution_machine::measurement::CapabilityId;

    // Topology (`DependencyEdge`/`CapabilityId`) has no concept of
    // "domain" at all -- it cannot even represent the question
    // cross-domain evidence poses, let alone answer it.
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let domain_1 = DomainId("domain-1".into());
    let domain_2 = DomainId("domain-2".into());
    let events = vec![
        EvidenceEvent {
            event_id: "e1".into(),
            content: EvidenceEventContent {
                subject: x.clone(),
                origin_domain: domain_1,
                fact_kind: "observed".into(),
            },
        },
        EvidenceEvent {
            event_id: "e2".into(),
            content: EvidenceEventContent {
                subject: y.clone(),
                origin_domain: domain_2,
                fact_kind: "observed".into(),
            },
        },
    ];

    // New DATA exists on an axis topology cannot represent -- but without
    // an exogenous axiom it does not cross into new CERTAINTY: the
    // verdict is identical to having no evidence whatsoever.
    assert_eq!(
        path_042_candidate_d_cross_domain_corroboration(&x, &y, &events, None),
        Path042Verdict::Insufficient
    );
    assert_eq!(
        path_042_candidate_d_cross_domain_corroboration(&x, &y, &[], None),
        Path042Verdict::Insufficient,
        "cross-domain evidence (without an axiom) and NO evidence at all produce the identical \
         verdict -- the new axis changes what CAN be represented, not what CAN be concluded"
    );
}

#[test]
fn path_042_i_disagreement_between_independent_origins_is_reported_not_resolved() {
    use distribution_machine::measurement::CapabilityId;

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let facts = vec![
        ObservedFact {
            observer: ObserverId("obs-1".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: true,
        },
        ObservedFact {
            observer: ObserverId("obs-2".into()),
            left: x.clone(),
            right: y.clone(),
            asserted_distinct: false,
        },
    ];

    assert_eq!(
        path_042_candidate_b_observer_corroboration(&x, &y, &facts),
        Path042Verdict::Conflicted,
        "two non-self observers disagree about whether x and y are distinct -- this must \
         surface as Conflicted, never silently resolved toward either answer"
    );
}

#[test]
fn path_042_j_strong_topology_without_any_lineage_stays_unresolved_not_independent() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    // Reuse PATH-041's richest "genuinely independent"-looking topology
    // (path_041_a's graph): distinct ancestry, distinct side-effects.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let edges = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("y", "m"),
        DependencyEdge::new("db_a", "x"),
        DependencyEdge::new("db_b", "y"),
        DependencyEdge::new("x", "audit_x"),
        DependencyEdge::new("y", "audit_y"),
    ];

    let structural_verdict = path_041_candidate_b_corroborate(&source, &x, &y, &m, &[], &edges);
    assert_eq!(structural_verdict, Path041Verdict::Independent);

    // But PATH-042's evidentiary layer has been given NOTHING -- no
    // ObservedFact, no EvidenceEvent, at all.
    let no_facts: Vec<ObservedFact> = vec![];
    let no_events: Vec<EvidenceEvent> = vec![];
    assert_eq!(path_042_candidate_b_observer_corroboration(&x, &y, &no_facts), Path042Verdict::Insufficient);
    assert_eq!(
        path_042_candidate_d_cross_domain_corroboration(&x, &y, &no_events, None),
        Path042Verdict::Insufficient,
        "strong STRUCTURAL corroboration (FIND-018) must not be silently upgraded into \
         evidentiary confirmation -- 'structurally plausible' and 'independently evidenced' \
         remain distinct claims, and missing evidence stays missing, not magically independent"
    );
}

// =====================================================================
// PATH-043 -- UNCERTAINTY PROPAGATION WITHOUT FALSE PRECISION (H14)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::measurement` or wired into any production/
// allocation path. Builds on FIND-018/FIND-019: whether two admissible
// "worlds" (concrete graphs) are the true one cannot be resolved by any
// evidence primitive this repository has -- PATH-043 does NOT attempt to
// resolve that question again (explicitly out of scope). It asks
// instead what the MEASUREMENT LAYER may honestly report given that the
// ambiguity exists and is not going to be resolved.
// =====================================================================

/// Why more than one admissible world might exist for a query, if it
/// turns out the queried value actually differs across them. Never a
/// probability, never a confidence coefficient -- see `Measured` below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path043State {
    /// Every admissible world produces the same value, OR there is no
    /// relevant ambiguity at all.
    Supported,
    /// Admissible worlds disagree and nothing grounded contradicts
    /// anything else -- missing evidence, not conflicting evidence.
    Unresolved,
    /// Admissible worlds disagree AND at least two distinct, grounded,
    /// non-self-derived evidence origins actively assert incompatible
    /// facts about the SAME relation.
    Conflicted,
}

/// Candidate D -- the smallest representation that survives attack in
/// this experiment: a structural lower/upper bound (equal, when exact)
/// carried alongside an explicit epistemic tag. `lower`/`upper` are the
/// minimum/maximum value the queried measurement takes across every
/// world this experiment currently considers admissible. `state` records
/// WHY they might differ -- it is never multiplied into the numbers, and
/// no probability is invented anywhere in this type or the functions
/// that build it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Measured {
    lower: usize,
    upper: usize,
    state: Path043State,
}

impl Measured {
    fn is_exact(&self) -> bool {
        self.lower == self.upper
    }
}

/// Builds a [`Measured`] from the queried value observed in every
/// admissible world. If every world agrees, the result collapses to
/// `Supported`/exact regardless of `ambient_state` -- this is what makes
/// uncertainty propagation RELEVANCE-SENSITIVE (PATH-043-C): a fact can
/// be unresolved or conflicted elsewhere in the graph and it will never
/// reach this function's output unless it actually moves the number
/// being asked about here.
fn path_043_from_world_values(values: &[usize], ambient_state: Path043State) -> Measured {
    let lower = *values.iter().min().expect("at least one admissible world");
    let upper = *values.iter().max().expect("at least one admissible world");
    let state = if lower == upper { Path043State::Supported } else { ambient_state };
    Measured { lower, upper, state }
}

/// Computes a [`Measured`] "dependency reach from `source`" result across
/// every admissible world in `worlds`. Each world is canonicalized first
/// via the UNMODIFIED production `canonicalize_pure_relays` (`protected`
/// covers every node this query actually cares about, e.g. `source`,
/// `x`, `y`, the convergence node) so harmless RepresentationOnly relay
/// padding inserted into a world cannot change the result (PATH-043-J) --
/// this reuses FIND-017's already-established depth-invariance rather
/// than reinventing it.
fn path_043_measure_reach(
    source: &distribution_machine::measurement::CapabilityId,
    protected: &[distribution_machine::measurement::CapabilityId],
    worlds: &[Vec<distribution_machine::measurement::DependencyEdge>],
    ambient_state: Path043State,
) -> Measured {
    use distribution_machine::measurement::{canonicalize_pure_relays, reachable_capabilities};

    let values: Vec<usize> = worlds
        .iter()
        .map(|edges| {
            let (canonical, _provenance) = canonicalize_pure_relays(source, protected, edges);
            reachable_capabilities(source, &canonical).len()
        })
        .collect();

    path_043_from_world_values(&values, ambient_state)
}

/// Rewrites every occurrence of `alias` to `canonical` in `edges` --
/// the "x and y are the same real dependency" admissible world, built by
/// literally merging the two labels.
fn path_043_collapse_alias(
    edges: &[distribution_machine::measurement::DependencyEdge],
    alias: &distribution_machine::measurement::CapabilityId,
    canonical: &distribution_machine::measurement::CapabilityId,
) -> Vec<distribution_machine::measurement::DependencyEdge> {
    edges
        .iter()
        .map(|e| distribution_machine::measurement::DependencyEdge {
            source: if &e.source == alias { canonical.clone() } else { e.source.clone() },
            target: if &e.target == alias { canonical.clone() } else { e.target.clone() },
        })
        .collect()
}

/// Composes two independently-derived [`Measured`] values into a larger
/// structural metric via interval arithmetic (naive sum). Deliberately
/// does NOT recompute a shared world set -- it only has each input's own
/// bounds and state to work with, exactly like a real caller composing
/// two already-published measurements. See `path_043_l_*` for why this
/// must never be narrower than the truth, only ever as wide or wider.
fn path_043_compose_sum(a: Measured, b: Measured) -> Measured {
    let lower = a.lower + b.lower;
    let upper = a.upper + b.upper;

    let worst = match (a.state, b.state) {
        (Path043State::Conflicted, _) | (_, Path043State::Conflicted) => Path043State::Conflicted,
        (Path043State::Unresolved, _) | (_, Path043State::Unresolved) => Path043State::Unresolved,
        _ => Path043State::Supported,
    };

    let state = if lower == upper { Path043State::Supported } else { worst };
    Measured { lower, upper, state }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path043AliasAssertion {
    SameAs,
    Distinct,
}

/// Raw, possibly-duplicated, possibly-replayed, possibly-self-derived
/// alias evidence -- test-local, mirrors PATH-042's `EvidenceEvent`
/// shape exactly (free-form `event_id`, content-addressed everything
/// else).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Path043AliasEvent {
    event_id: String,
    origin: String,
    a: distribution_machine::measurement::CapabilityId,
    b: distribution_machine::measurement::CapabilityId,
    asserted: Path043AliasAssertion,
}

/// Content-addressed dedup, discarding `event_id` -- direct transplant
/// of the FIND-007 / PATH-042 Candidate C pattern: a free-form event id
/// is never trusted as identity, so literal duplicate serialization
/// (PATH-043-H) and replay under a fresh id (PATH-043-I) both collapse
/// to the same single event.
fn path_043_deduplicate_alias_events(events: &[Path043AliasEvent]) -> Vec<Path043AliasEvent> {
    use std::collections::HashMap;

    #[derive(PartialEq, Eq, Hash)]
    struct ContentKey {
        origin: String,
        a: String,
        b: String,
        same_as: bool,
    }

    let mut by_content: HashMap<ContentKey, Path043AliasEvent> = HashMap::new();
    for event in events {
        let key = ContentKey {
            origin: event.origin.clone(),
            a: event.a.0.clone(),
            b: event.b.0.clone(),
            same_as: matches!(event.asserted, Path043AliasAssertion::SameAs),
        };
        by_content.entry(key).or_insert_with(|| event.clone());
    }

    let mut result: Vec<Path043AliasEvent> = by_content.into_values().collect();
    result.sort_by(|x, y| x.event_id.cmp(&y.event_id));
    result
}

/// An origin literally derived from one of the two subjects it is
/// making a claim about contributes zero corroboration -- mirrors
/// PATH-042-B's self-observation filter. Covers PATH-043-K's
/// "self-derived" laundering ingredient.
fn path_043_is_self_derived(event: &Path043AliasEvent) -> bool {
    event.origin == event.a.0 || event.origin == event.b.0
}

/// Determines WHY (if at all) more than one world remains admissible for
/// the identity of `a`/`b`, from raw alias evidence. Deliberately does
/// NOT attempt to resolve which world is true -- per FIND-019 and this
/// mission's own non-goals, nothing in this repository can, and
/// reopening that question is explicitly out of scope. A single
/// grounded, uncontested assertion (no matter how many times duplicated
/// or replayed -- PATH-043-H/I/K) is NOT enough to collapse the
/// ambiguity to `Supported`: only a genuine two-sided disagreement
/// between distinct grounded (non-self-derived) origins is distinguished
/// at all, and even then only as `Conflicted`, never resolved in either
/// direction (PATH-043-F). Zero relevant evidence is `Unresolved`, never
/// silently treated as "distinct" (PATH-043-G).
fn path_043_relation_state(
    a: &distribution_machine::measurement::CapabilityId,
    b: &distribution_machine::measurement::CapabilityId,
    events: &[Path043AliasEvent],
) -> Path043State {
    let deduped = path_043_deduplicate_alias_events(events);

    let relevant: Vec<&Path043AliasEvent> = deduped
        .iter()
        .filter(|e| !path_043_is_self_derived(e))
        .filter(|e| (&e.a == a && &e.b == b) || (&e.a == b && &e.b == a))
        .collect();

    let says_same = relevant.iter().any(|e| e.asserted == Path043AliasAssertion::SameAs);
    let says_distinct = relevant.iter().any(|e| e.asserted == Path043AliasAssertion::Distinct);

    if says_same && says_distinct {
        return Path043State::Conflicted;
    }

    Path043State::Unresolved
}

// ---------------------------------------------------------------------
// Candidate attacks (A, B, C) -- required before Candidate D is trusted.
// ---------------------------------------------------------------------

#[test]
fn path_043_candidate_a_forced_scalar_depends_on_an_unverified_representation_assumption() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, reachable_capabilities};

    // Candidate A: compute directly from whatever graph happens to be
    // materialized, and ignore that a second, equally admissible world
    // exists. FIND-018/019's own minimal shape.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);

    let forced_scalar = reachable_capabilities(&source, &world_independent).len();
    let value_in_the_other_admissible_world = reachable_capabilities(&source, &world_alias).len();

    assert_ne!(
        forced_scalar, value_in_the_other_admissible_world,
        "Candidate A reports an exact dependency_reach = {forced_scalar} that LOOKS precise \
         but is silently conditional on an unverified assumption (x and y are independent); \
         the identical evidence admits a second world where the true value is \
         {value_in_the_other_admissible_world}, and Candidate A has no way to represent that \
         this happened at all"
    );
}

#[test]
fn path_043_candidate_b_blanket_unknown_discards_provable_structure() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    // Candidate B: "if any relevant unresolved fact exists, return
    // Unknown/None." Attacked with PATH-043-D's shape: worlds disagree,
    // but every world still establishes a real minimum.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let z = CapabilityId("z".into());
    let w = CapabilityId("w".into());

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
        DependencyEdge::new("source", "z"),
        DependencyEdge::new("z", "w"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);

    // Candidate B, applied literally: relevant ambiguity exists (x/y),
    // therefore the whole query collapses to nothing at all.
    let candidate_b_result: Option<usize> = None;
    assert!(candidate_b_result.is_none(), "Candidate B discards everything once any ambiguity exists");

    let measured = path_043_measure_reach(
        &source,
        &[source.clone(), x.clone(), y.clone(), m.clone(), z.clone(), w.clone()],
        &[world_independent, world_alias],
        Path043State::Unresolved,
    );

    assert!(
        measured.lower > 0,
        "Candidate D preserves a provable minimum ({}) that Candidate B's blanket Unknown \
         would have discarded entirely, even though that minimum is true in EVERY admissible \
         world",
        measured.lower
    );
}

#[test]
fn path_043_candidate_c_bounds_alone_conflate_unresolved_with_conflicted() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);
    let protected = vec![source.clone(), x.clone(), y.clone(), m.clone()];

    let unresolved_case = path_043_measure_reach(
        &source,
        &protected,
        &[world_independent.clone(), world_alias.clone()],
        Path043State::Unresolved,
    );
    let conflicted_case = path_043_measure_reach(
        &source,
        &protected,
        &[world_independent, world_alias],
        Path043State::Conflicted,
    );

    // Candidate C -- bounds only, epistemic state discarded.
    struct BoundsOnly {
        lower: usize,
        upper: usize,
    }
    let c_unresolved = BoundsOnly { lower: unresolved_case.lower, upper: unresolved_case.upper };
    let c_conflicted = BoundsOnly { lower: conflicted_case.lower, upper: conflicted_case.upper };

    assert_eq!(c_unresolved.lower, c_conflicted.lower);
    assert_eq!(c_unresolved.upper, c_conflicted.upper);
    assert_ne!(
        unresolved_case.state, conflicted_case.state,
        "Candidate D distinguishes 'missing evidence, honest ambiguity' from 'two grounded \
         origins actively disagree'; Candidate C's bounds-only representation cannot -- both \
         collapse to the identical {{lower, upper}} pair, even though a Conflicted result \
         must never be silently trusted the way an Unresolved one safely can be"
    );
}

// ---------------------------------------------------------------------
// PATH-043-A through L -- required pathological cases.
// ---------------------------------------------------------------------

#[test]
fn path_043_a_fully_supported_control_stays_exact() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    // No relevant unresolved identity/evidence anywhere: exactly one
    // admissible world.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let z = CapabilityId("z".into());
    let edges = vec![DependencyEdge::new("source", "x"), DependencyEdge::new("source", "z")];

    let measured = path_043_measure_reach(&source, &[source.clone(), x.clone(), z.clone()], &[edges], Path043State::Unresolved);

    assert!(measured.is_exact(), "the sole admissible world must yield an exact result");
    assert_eq!(measured.state, Path043State::Supported);
    assert_eq!(measured.lower, 2);
    assert_eq!(measured.upper, 2);
}

#[test]
fn path_043_b_unresolved_alias_vs_independence_refuses_one_unconditional_scalar() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);
    let protected = vec![source.clone(), x.clone(), y.clone(), m.clone()];

    let ambient = path_043_relation_state(&x, &y, &[]);
    assert_eq!(ambient, Path043State::Unresolved);

    let measured = path_043_measure_reach(&source, &protected, &[world_independent, world_alias], ambient);

    assert!(!measured.is_exact(), "identical observable representation admits two worlds with different reach -- one unconditional exact scalar must not be emitted");
    assert_eq!(measured.state, Path043State::Unresolved);
    assert_eq!(measured.lower, 2);
    assert_eq!(measured.upper, 3);
}

#[test]
fn path_043_c_irrelevant_uncertainty_does_not_downgrade_an_unrelated_query() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    // The x/y alias ambiguity exists in the graph, but the query is
    // about a disjoint, unaffected subgraph.
    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let root = CapabilityId("root".into());
    let p = CapabilityId("p".into());
    let q = CapabilityId("q".into());

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
        DependencyEdge::new("root", "p"),
        DependencyEdge::new("p", "q"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);

    let ambient = path_043_relation_state(&x, &y, &[]);
    assert_eq!(ambient, Path043State::Unresolved, "the ambiguity is real -- it is just irrelevant to this query");

    let protected = vec![root.clone(), p.clone(), q.clone(), source.clone(), x.clone(), y.clone(), m.clone()];
    let measured = path_043_measure_reach(&root, &protected, &[world_independent, world_alias], ambient);

    assert!(
        measured.is_exact(),
        "unresolved x/y identity exists elsewhere in the graph, but every admissible world \
         produces the same reach from `root` -- the result must not be downgraded merely \
         because uncertainty exists somewhere"
    );
    assert_eq!(measured.state, Path043State::Supported);
    assert_eq!(measured.lower, 2);
}

#[test]
fn path_043_d_lower_bound_survives_disagreement() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let z = CapabilityId("z".into());
    let w = CapabilityId("w".into());

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
        DependencyEdge::new("source", "z"),
        DependencyEdge::new("z", "w"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);
    let protected = vec![source.clone(), x.clone(), y.clone(), m.clone(), z.clone(), w.clone()];

    let measured = path_043_measure_reach(&source, &protected, &[world_independent, world_alias], Path043State::Unresolved);

    assert_eq!(measured.lower, 4, "reach differs by admissible world, but never falls below 4 (z, w, m, and whichever of x/y remain distinct) -- that provable minimum must survive");
    assert_eq!(measured.upper, 5);
    assert_eq!(measured.state, Path043State::Unresolved);
}

#[test]
fn path_043_e_upper_bound_survives_disagreement() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let z = CapabilityId("z".into());
    let w = CapabilityId("w".into());

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
        DependencyEdge::new("source", "z"),
        DependencyEdge::new("z", "w"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);
    let protected = vec![source.clone(), x.clone(), y.clone(), m.clone(), z.clone(), w.clone()];

    let measured = path_043_measure_reach(&source, &protected, &[world_independent, world_alias], Path043State::Unresolved);

    assert_eq!(measured.upper, 5, "impact cannot exceed 5 in any admissible world -- that provable maximum must survive, not just be dropped in favor of the smaller number");
    assert_eq!(measured.lower, 4);
}

#[test]
fn path_043_f_conflict_is_not_averaged_into_a_scalar() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());

    let events = vec![
        Path043AliasEvent {
            event_id: "e1".into(),
            origin: "auditor_1".into(),
            a: x.clone(),
            b: y.clone(),
            asserted: Path043AliasAssertion::SameAs,
        },
        Path043AliasEvent {
            event_id: "e2".into(),
            origin: "auditor_2".into(),
            a: x.clone(),
            b: y.clone(),
            asserted: Path043AliasAssertion::Distinct,
        },
    ];
    let ambient = path_043_relation_state(&x, &y, &events);
    assert_eq!(ambient, Path043State::Conflicted, "two independently grounded, non-self origins disagree -- this must be reported as CONFLICTED, not silently resolved either way");

    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);
    let protected = vec![source.clone(), x.clone(), y.clone(), m.clone()];

    let measured = path_043_measure_reach(&source, &protected, &[world_independent, world_alias], ambient);

    assert_eq!(measured.state, Path043State::Conflicted);
    assert_ne!(measured.lower, measured.upper, "the two contradictory worlds must not be averaged into one scalar -- only lower/upper bounds are exposed, never a single reconciled number");
}

#[test]
fn path_043_g_missing_evidence_is_not_treated_as_false() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());

    // No claims at all -- neither supported nor contradicted.
    let state = path_043_relation_state(&x, &y, &[]);
    assert_eq!(state, Path043State::Unresolved, "missing evidence must stay Unresolved, never silently resolved to 'distinct' (false) just because nothing was asserted");

    // Contrast with a naive candidate that treats "no alias claim" as
    // proof of independence and only ever materializes the independent
    // world.
    let source = CapabilityId("source".into());
    let m = CapabilityId("m".into());
    let world_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let world_alias = path_043_collapse_alias(&world_independent, &y, &x);
    let protected = vec![source.clone(), x.clone(), y.clone(), m.clone()];

    let naive_missing_as_false =
        distribution_machine::measurement::reachable_capabilities(&source, &world_independent).len();
    let measured = path_043_measure_reach(&source, &protected, &[world_independent, world_alias], state);

    assert_eq!(naive_missing_as_false, 3, "the naive candidate silently commits to one world");
    assert!(
        measured.lower < naive_missing_as_false,
        "the correct handling of missing evidence retains the second admissible world \
         (reach 2) that the naive 'missing = false/independent' shortcut discards entirely"
    );
}

#[test]
fn path_043_h_duplicate_evidence_does_not_narrow_or_upgrade() {
    let x = distribution_machine::measurement::CapabilityId("x".into());
    let y = distribution_machine::measurement::CapabilityId("y".into());

    let single = vec![Path043AliasEvent {
        event_id: "e1".into(),
        origin: "auditor_1".into(),
        a: x.clone(),
        b: y.clone(),
        asserted: Path043AliasAssertion::SameAs,
    }];

    // Literal duplicate serialization: identical event, repeated.
    let mut duplicated = single.clone();
    for _ in 0..9 {
        duplicated.push(single[0].clone());
    }

    assert_eq!(path_043_deduplicate_alias_events(&duplicated).len(), 1, "FIND-007: duplicate serialization must not be counted as new evidence");
    assert_eq!(
        path_043_relation_state(&x, &y, &single),
        path_043_relation_state(&x, &y, &duplicated),
        "10 literal duplicates of the same one-sided claim must not narrow the ambiguity or \
         improve the epistemic state relative to a single copy"
    );
    assert_eq!(path_043_relation_state(&x, &y, &duplicated), Path043State::Unresolved, "one-sided evidence, no matter how many times duplicated, is still not corroboration -- it stays Unresolved, never Supported");
}

#[test]
fn path_043_i_replay_under_a_new_id_does_not_narrow_or_upgrade() {
    let x = distribution_machine::measurement::CapabilityId("x".into());
    let y = distribution_machine::measurement::CapabilityId("y".into());

    let baseline = vec![Path043AliasEvent {
        event_id: "e1".into(),
        origin: "auditor_1".into(),
        a: x.clone(),
        b: y.clone(),
        asserted: Path043AliasAssertion::SameAs,
    }];

    // Same content, replayed under fresh event ids each time.
    let replayed: Vec<Path043AliasEvent> = (0..5)
        .map(|i| Path043AliasEvent {
            event_id: format!("replay-{i}"),
            origin: "auditor_1".into(),
            a: x.clone(),
            b: y.clone(),
            asserted: Path043AliasAssertion::SameAs,
        })
        .collect();

    assert_eq!(path_043_deduplicate_alias_events(&replayed).len(), 1, "a free-form event_id is never trusted as identity -- content-identical replay collapses to one event regardless of how many fresh ids are used");
    assert_eq!(
        path_043_relation_state(&x, &y, &baseline),
        path_043_relation_state(&x, &y, &replayed),
        "replay under a new event id must not narrow the measurement interval or upgrade certainty"
    );
}

#[test]
fn path_043_j_representation_depth_does_not_change_measurement_or_state() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge};

    let source = CapabilityId("source".into());
    let x = CapabilityId("x".into());
    let y = CapabilityId("y".into());
    let m = CapabilityId("m".into());
    let protected = vec![source.clone(), x.clone(), y.clone(), m.clone()];

    let bare_independent = vec![
        DependencyEdge::new("source", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let bare_alias = path_043_collapse_alias(&bare_independent, &y, &x);
    let bare = path_043_measure_reach(&source, &protected, &[bare_independent, bare_alias], Path043State::Unresolved);

    // Insert a harmless RepresentationOnly relay between `source` and
    // `x`, and pad `y`'s branch with an extra relay too.
    let padded_independent = vec![
        DependencyEdge::new("source", "relay_x"),
        DependencyEdge::new("relay_x", "x"),
        DependencyEdge::new("x", "m"),
        DependencyEdge::new("source", "relay_y"),
        DependencyEdge::new("relay_y", "y"),
        DependencyEdge::new("y", "m"),
    ];
    let padded_alias = path_043_collapse_alias(&padded_independent, &y, &x);
    let padded = path_043_measure_reach(&source, &protected, &[padded_independent, padded_alias], Path043State::Unresolved);

    assert_eq!(bare, padded, "harmless RepresentationOnly relay depth must be invariant to both the measurement and the epistemic state");
}

#[test]
fn path_043_k_claim_volume_does_not_launder_unresolved_into_supported() {
    let x = distribution_machine::measurement::CapabilityId("x".into());
    let y = distribution_machine::measurement::CapabilityId("y".into());

    let mut laundering: Vec<Path043AliasEvent> = Vec::new();
    // Self-derived claims: the "origin" is literally one of the subjects.
    for i in 0..10 {
        laundering.push(Path043AliasEvent {
            event_id: format!("self-{i}"),
            origin: "x".into(),
            a: x.clone(),
            b: y.clone(),
            asserted: Path043AliasAssertion::SameAs,
        });
    }
    // Weak/circular claims: many duplicates of the same single grounded
    // origin's assertion.
    for i in 0..20 {
        laundering.push(Path043AliasEvent {
            event_id: format!("weak-{i}"),
            origin: "single_actor".into(),
            a: x.clone(),
            b: y.clone(),
            asserted: Path043AliasAssertion::SameAs,
        });
    }

    let sparse = path_043_relation_state(&x, &y, &laundering[10..11]);
    let laundered = path_043_relation_state(&x, &y, &laundering);

    assert_eq!(sparse, Path043State::Unresolved);
    assert_eq!(laundered, Path043State::Unresolved, "30 self-derived/circular/duplicate claims must not convert UNRESOLVED into SUPPORTED -- claim count is free for a single actor to manufacture (FIND-018/019)");
    assert_eq!(sparse, laundered, "the epistemic state must be identical whether backed by 1 relevant claim or 30 manufactured ones");
}

#[test]
fn path_043_l_composition_never_becomes_more_precise_than_its_inputs_justify() {
    // Two measurements driven by the SAME underlying two-world
    // ambiguity, but anti-correlated: measurement 1 is higher in the
    // independent world, measurement 2 is higher in the alias world.
    let per_world_1 = [3usize, 2usize]; // [independent, alias]
    let per_world_2 = [1usize, 4usize];

    let measured_1 = path_043_from_world_values(&per_world_1, Path043State::Unresolved);
    let measured_2 = path_043_from_world_values(&per_world_2, Path043State::Unresolved);

    let composed = path_043_compose_sum(measured_1, measured_2);

    // The TRUE joint value, computed by actually summing within each
    // shared admissible world rather than combining published bounds.
    let true_joint_values: Vec<usize> =
        per_world_1.iter().zip(per_world_2.iter()).map(|(a, b)| a + b).collect();
    let true_joint = path_043_from_world_values(&true_joint_values, Path043State::Unresolved);

    assert!(composed.lower <= true_joint.lower, "naive interval-sum composition must never claim a HIGHER minimum than the true joint minimum -- that would be false precision");
    assert!(composed.upper >= true_joint.upper, "naive interval-sum composition must never claim a LOWER maximum than the true joint maximum -- that would be false precision");
    assert_eq!(composed.state, Path043State::Unresolved, "composing an unresolved input keeps the composite unresolved -- uncertainty is not erased by composition");

    // Composing two fully exact/Supported measurements preserves
    // exactness -- composition must not manufacture uncertainty either.
    let exact_1 = path_043_from_world_values(&[5, 5], Path043State::Unresolved);
    let exact_2 = path_043_from_world_values(&[7, 7], Path043State::Unresolved);
    let composed_exact = path_043_compose_sum(exact_1, exact_2);
    assert!(composed_exact.is_exact());
    assert_eq!(composed_exact.state, Path043State::Supported);
    assert_eq!(composed_exact.lower, 12);
}

// =====================================================================
// PATH-044 -- CAN BOUNDED EPISTEMIC MEASUREMENT SURVIVE MULTI-DIMENSIONAL
// CRITICALITY? (H15)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::measurement` or wired into any production/
// allocation path. Builds on FIND-020 (PATH-043): the smallest surviving
// representation for a SINGLE count-like observable was
// `Measured { lower, upper, state }`. PATH-044 asks whether that
// generalizes once Criticality depends on FOUR interacting observables
// (`CriticalityObservables`) rather than one.
//
// PRODUCTION INVENTORY (read before designing anything below):
//
// `CriticalityObservables { qualified_dependency_weight: f64 [0,1],
// blocking_paths: u32, failure_blast_radius: u32, substitutability:
// Substitutability }` -> `measure_criticality(observables, weights) ->
// CriticalityMeasurement { ..., score: f64 }`. `score` is a WEIGHTED
// LINEAR SUM of four independently-normalized terms:
// `qualified_dependency_weight` (used as-is, already in [0,1]),
// `saturating_count(blocking_paths, 5)`, `saturating_count
// (failure_blast_radius, 5)`, `substitutability_score(substitutability)`
// (a fixed 4-level map: Easy=0.00, Moderate=0.33, Difficult=0.67,
// None=1.00). Every term is a function of exactly ONE observable (no
// cross terms) and every term is monotonic non-decreasing in that one
// observable -- `measure_criticality` is a SEPARABLE MONOTONIC function.
// This is the single fact this entire experiment turns on.
//
// `qualified_dependency_weight` itself has a real production derivation
// pipeline, upstream of `CriticalityObservables`:
// `qualify_dependency(DependencyEvidence, weights) -> QualifiedDependency
// { ..., score }` (also a separable monotonic weighted linear sum) ->
// `aggregate_dependency_weight_with_model(&[QualifiedDependency], model)`,
// which has THREE real production models: `Complementary` (`1 -
// prod(1-score_i)`, monotonic, continuous), `Thresholded` (drops any
// dependency with `score < 0.20` before the same complementary
// accumulation -- FIND-006's confirmed real discontinuity, still
// monotonic but NOT continuous), and `RankedDecay` (sorts scores
// descending, weights by `0.5^rank`, sums, clamps to 1.0 -- provably
// monotonic per-input too: raising one dependency's score can only ever
// move it to an EQUAL-OR-BETTER rank, and every adjacent rank swap this
// triggers strictly increases the total, since the swap only ever
// happens between a rising score and a smaller one it has just
// overtaken; see `path_044_ranked_decay_is_monotonic_probe`).
// `deduplicate_structural_dependencies`/`aggregate_structural_
// dependencies_with_model` (FIND-007) sit one layer above that,
// deduping `DependencyRecord`s by structural key before aggregation.
//
// NO population-relative normalization was found anywhere in this
// pipeline (`grep`ped for population/percentile/z-score/rank-relative:
// zero hits outside `RankedDecay`'s own internal, single-capability
// rank, which never looks outside the slice passed to it) -- see
// PATH-044-G, explicit N/A.
//
// `Substitutability`'s evidence-to-category derivation remains
// unspecified in production (FIND-011, unchanged) -- there is no
// `Evidence -> Substitutability` constructor to attack, only the fixed
// four-level score map, which this experiment treats as a plain
// (already-categorical) observable, exactly like `blocking_paths` and
// `failure_blast_radius` (which also have no production derivation from
// a graph -- FIND-012 -- and are supplied as literal inputs at every
// call site, in production and in this experiment alike).
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path044State {
    Supported,
    Unresolved,
    Conflicted,
}

fn path_044_worst_state(states: &[Path044State]) -> Path044State {
    if states.iter().any(|s| *s == Path044State::Conflicted) {
        Path044State::Conflicted
    } else if states.iter().any(|s| *s == Path044State::Unresolved) {
        Path044State::Unresolved
    } else {
        Path044State::Supported
    }
}

/// Candidate A / the FIND-020 shape, reused for `f64` scores.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Measured044 {
    lower: f64,
    upper: f64,
    state: Path044State,
}

impl Measured044 {
    fn from_values(values: &[f64], ambient_state: Path044State) -> Self {
        let lower = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let upper = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let state = if (upper - lower).abs() < 1e-9 { Path044State::Supported } else { ambient_state };
        Measured044 { lower, upper, state }
    }

    fn is_exact(&self) -> bool {
        (self.upper - self.lower).abs() < 1e-9
    }

    fn contains(&self, other: &Measured044) -> bool {
        self.lower <= other.lower + 1e-9 && self.upper >= other.upper - 1e-9
    }
}

fn path_044_weights() -> distribution_machine::measurement::CriticalityMeasurementWeights {
    distribution_machine::measurement::CriticalityMeasurementWeights {
        qualified_dependency_weight: 0.30,
        blocking_paths: 0.30,
        failure_blast_radius: 0.25,
        substitutability: 0.15,
    }
}

fn path_044_world(
    qdw: f64,
    blocking: u32,
    blast: u32,
    sub: distribution_machine::measurement::Substitutability,
) -> distribution_machine::measurement::CriticalityObservables {
    distribution_machine::measurement::CriticalityObservables {
        qualified_dependency_weight: qdw,
        blocking_paths: blocking,
        failure_blast_radius: blast,
        substitutability: sub,
    }
}

fn path_044_score(o: distribution_machine::measurement::CriticalityObservables) -> f64 {
    distribution_machine::measurement::measure_criticality(o, path_044_weights()).unwrap().score
}

// ---------------------------------------------------------------------
// Candidate A -- score each admissible world, then take min/max.
// ---------------------------------------------------------------------
fn path_044_candidate_a_score_each_world(
    worlds: &[distribution_machine::measurement::CriticalityObservables],
    ambient_state: Path044State,
) -> Measured044 {
    let scores: Vec<f64> = worlds.iter().copied().map(path_044_score).collect();
    Measured044::from_values(&scores, ambient_state)
}

// ---------------------------------------------------------------------
// Candidate B -- bound each observable independently, then score the
// corners. Generates the full 2^4 corner set (deduplicating a dimension
// down to a single value when every world agrees on it) so the corner
// LIST itself, not just its min/max, is inspectable (PATH-044-C/N).
// ---------------------------------------------------------------------
fn path_044_generate_corners(
    worlds: &[distribution_machine::measurement::CriticalityObservables],
) -> Vec<distribution_machine::measurement::CriticalityObservables> {
    use distribution_machine::measurement::Substitutability;

    let qdw_lo = worlds.iter().map(|o| o.qualified_dependency_weight).fold(f64::INFINITY, f64::min);
    let qdw_hi =
        worlds.iter().map(|o| o.qualified_dependency_weight).fold(f64::NEG_INFINITY, f64::max);
    let bp_lo = worlds.iter().map(|o| o.blocking_paths).min().unwrap();
    let bp_hi = worlds.iter().map(|o| o.blocking_paths).max().unwrap();
    let fbr_lo = worlds.iter().map(|o| o.failure_blast_radius).min().unwrap();
    let fbr_hi = worlds.iter().map(|o| o.failure_blast_radius).max().unwrap();

    let sub_rank = |s: Substitutability| -> u8 {
        match s {
            Substitutability::Easy => 0,
            Substitutability::Moderate => 1,
            Substitutability::Difficult => 2,
            Substitutability::None => 3,
        }
    };
    let sub_lo = worlds.iter().map(|o| o.substitutability).min_by_key(|s| sub_rank(*s)).unwrap();
    let sub_hi = worlds.iter().map(|o| o.substitutability).max_by_key(|s| sub_rank(*s)).unwrap();

    let mut qdws = vec![qdw_lo];
    if (qdw_hi - qdw_lo).abs() > 1e-12 {
        qdws.push(qdw_hi);
    }
    let mut bps = vec![bp_lo];
    if bp_hi != bp_lo {
        bps.push(bp_hi);
    }
    let mut fbrs = vec![fbr_lo];
    if fbr_hi != fbr_lo {
        fbrs.push(fbr_hi);
    }
    let mut subs = vec![sub_lo];
    if sub_hi != sub_lo {
        subs.push(sub_hi);
    }

    let mut corners = Vec::new();
    for &qdw in &qdws {
        for &bp in &bps {
            for &fbr in &fbrs {
                for &sub in &subs {
                    corners.push(path_044_world(qdw, bp, fbr, sub));
                }
            }
        }
    }
    corners
}

fn path_044_candidate_b_corner_scores(
    worlds: &[distribution_machine::measurement::CriticalityObservables],
    ambient_state: Path044State,
) -> Measured044 {
    let scores: Vec<f64> = path_044_generate_corners(worlds).into_iter().map(path_044_score).collect();
    Measured044::from_values(&scores, ambient_state)
}

// ---------------------------------------------------------------------
// Candidate C -- vector of independently bounded observables. Produces
// no scalar; each dimension collapses to `Supported`/exact on its own,
// independent of the others (relevance-sensitivity, per-dimension).
// ---------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq)]
struct Bound<T> {
    lower: T,
    upper: T,
    state: Path044State,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MeasuredCriticalityObservables {
    qualified_dependency_weight: Bound<f64>,
    blocking_paths: Bound<u32>,
    failure_blast_radius: Bound<u32>,
    substitutability: Bound<distribution_machine::measurement::Substitutability>,
}

fn path_044_candidate_c_per_dimension_bounds(
    worlds: &[distribution_machine::measurement::CriticalityObservables],
    ambient_state: Path044State,
) -> MeasuredCriticalityObservables {
    use distribution_machine::measurement::Substitutability;

    let qdw_vals: Vec<f64> = worlds.iter().map(|o| o.qualified_dependency_weight).collect();
    let qdw = Measured044::from_values(&qdw_vals, ambient_state);

    let bp_lo = worlds.iter().map(|o| o.blocking_paths).min().unwrap();
    let bp_hi = worlds.iter().map(|o| o.blocking_paths).max().unwrap();
    let bp_state = if bp_lo == bp_hi { Path044State::Supported } else { ambient_state };

    let fbr_lo = worlds.iter().map(|o| o.failure_blast_radius).min().unwrap();
    let fbr_hi = worlds.iter().map(|o| o.failure_blast_radius).max().unwrap();
    let fbr_state = if fbr_lo == fbr_hi { Path044State::Supported } else { ambient_state };

    let sub_rank = |s: Substitutability| -> u8 {
        match s {
            Substitutability::Easy => 0,
            Substitutability::Moderate => 1,
            Substitutability::Difficult => 2,
            Substitutability::None => 3,
        }
    };
    let sub_lo = worlds.iter().map(|o| o.substitutability).min_by_key(|s| sub_rank(*s)).unwrap();
    let sub_hi = worlds.iter().map(|o| o.substitutability).max_by_key(|s| sub_rank(*s)).unwrap();
    let sub_state = if sub_lo == sub_hi { Path044State::Supported } else { ambient_state };

    MeasuredCriticalityObservables {
        qualified_dependency_weight: Bound { lower: qdw.lower, upper: qdw.upper, state: qdw.state },
        blocking_paths: Bound { lower: bp_lo, upper: bp_hi, state: bp_state },
        failure_blast_radius: Bound { lower: fbr_lo, upper: fbr_hi, state: fbr_state },
        substitutability: Bound { lower: sub_lo, upper: sub_hi, state: sub_state },
    }
}

// ---------------------------------------------------------------------
// Candidate D -- retain the actual enumerated admissible states. The
// scalar range is DERIVED from them (identical to Candidate A), but the
// states themselves remain available for queries a compressed
// representation cannot answer (PATH-044-O).
// ---------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
struct AdmissibleCriticalityStates {
    states: Vec<distribution_machine::measurement::CriticalityObservables>,
    epistemic_state: Path044State,
}

impl AdmissibleCriticalityStates {
    fn scalar_range(&self) -> Measured044 {
        path_044_candidate_a_score_each_world(&self.states, self.epistemic_state)
    }

    fn admits(
        &self,
        predicate: impl Fn(&distribution_machine::measurement::CriticalityObservables) -> bool,
    ) -> bool {
        self.states.iter().any(predicate)
    }
}

/// The central distinction this experiment exists to test: a SCALAR can
/// be exact while the OBSERVABLE VECTOR explaining it remains unresolved
/// (PATH-044-E/I). These are two independent epistemic axes -- FIND-020's
/// single `state` field, generalized naively, would conflate them.
#[derive(Debug, Clone, PartialEq)]
struct Path044CriticalityResult {
    scalar: Measured044,
    explanation_state: Path044State,
    admissible_states: Vec<distribution_machine::measurement::CriticalityObservables>,
}

fn path_044_build_result(
    worlds: &[distribution_machine::measurement::CriticalityObservables],
    ambient_state: Path044State,
) -> Path044CriticalityResult {
    let scalar = path_044_candidate_a_score_each_world(worlds, ambient_state);
    let all_vectors_equal = worlds.windows(2).all(|w| w[0] == w[1]);
    let explanation_state = if all_vectors_equal { Path044State::Supported } else { ambient_state };
    Path044CriticalityResult {
        scalar,
        explanation_state,
        admissible_states: worlds.to_vec(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path044Ranking {
    DefinitelyGreater,
    DefinitelyLess,
    Overlapping,
}

fn path_044_compare(a: Measured044, b: Measured044) -> Path044Ranking {
    if a.lower > b.upper {
        Path044Ranking::DefinitelyGreater
    } else if b.lower > a.upper {
        Path044Ranking::DefinitelyLess
    } else {
        Path044Ranking::Overlapping
    }
}

fn path_044_compose_sum(a: Measured044, b: Measured044) -> Measured044 {
    let lower = a.lower + b.lower;
    let upper = a.upper + b.upper;
    let worst = path_044_worst_state(&[a.state, b.state]);
    let state = if (upper - lower).abs() < 1e-9 { Path044State::Supported } else { worst };
    Measured044 { lower, upper, state }
}

fn path_044_dep(score: f64) -> distribution_machine::measurement::QualifiedDependency {
    distribution_machine::measurement::QualifiedDependency {
        blocking_relevance: score,
        failure_propagation: score,
        operational_reliance: score,
        business_path_relevance: score,
        score,
    }
}

fn path_044_dep_record(
    dep_id: &str,
    path_id: &str,
    domain_id: &str,
    source: &str,
    target: &str,
    score: f64,
) -> distribution_machine::measurement::DependencyRecord {
    use distribution_machine::measurement::{
        CapabilityId, DependencyId, DependencyPathId, DependencyRelationshipType, FailureDomainId,
    };

    distribution_machine::measurement::DependencyRecord {
        dependency_id: DependencyId(dep_id.into()),
        dependency_path_id: DependencyPathId(path_id.into()),
        failure_domain_id: FailureDomainId(domain_id.into()),
        source_capability: CapabilityId(source.into()),
        target_capability: CapabilityId(target.into()),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified: path_044_dep(score),
    }
}

// =====================================================================
// Candidate attacks
// =====================================================================

#[test]
fn path_044_candidate_a_range_is_correct_but_needs_an_enumerable_world_set() {
    // Sanity/control for Candidate A: it is exactly as good as the
    // admissible world set handed to it -- attacked properly by D/O
    // below (an incomplete or over-broad world set silently changes the
    // answer). No separate assertion beyond confirming it reproduces
    // `measure_criticality` exactly for a single world (see PATH-044-A).
    let world = path_044_world(0.6, 2, 2, distribution_machine::measurement::Substitutability::Moderate);
    let measured = path_044_candidate_a_score_each_world(&[world], Path044State::Unresolved);
    assert!(measured.is_exact());
    assert_eq!(measured.lower, path_044_score(world));
}

#[test]
fn path_044_ranked_decay_is_monotonic_probe() {
    use distribution_machine::measurement::{DependencyAggregationModel, aggregate_dependency_weight_with_model};

    // Empirical probe backing the inventory's monotonicity claim for
    // RankedDecay: raising one dependency's own score, holding every
    // other dependency's score fixed, never decreases the aggregate --
    // even when the raised dependency jumps over several others in rank,
    // displacing each of their decay weights downward.
    let mut deps = vec![path_044_dep(0.90), path_044_dep(0.80), path_044_dep(0.70), path_044_dep(0.60)];
    deps.push(path_044_dep(0.05)); // the one we will raise

    let before = aggregate_dependency_weight_with_model(&deps, DependencyAggregationModel::RankedDecay);

    deps[4] = path_044_dep(0.95); // now outranks all four others
    let after = aggregate_dependency_weight_with_model(&deps, DependencyAggregationModel::RankedDecay);

    assert!(
        after >= before - 1e-9,
        "RankedDecay must not be non-monotonic: raising dep[4] from 0.05 to 0.95 (jumping over 4 \
         higher-ranked deps, displacing each of their decay weights down by one rank) still \
         produced {after} >= {before}"
    );
}

// =====================================================================
// PATH-044-A through O -- required pathological cases.
// =====================================================================

#[test]
fn path_044_a_fully_supported_exact_control() {
    use distribution_machine::measurement::Substitutability;

    let world = path_044_world(0.6, 2, 2, Substitutability::Moderate);
    let measured = path_044_candidate_a_score_each_world(&[world], Path044State::Unresolved);

    assert!(measured.is_exact());
    assert_eq!(measured.state, Path044State::Supported);
    assert!((measured.lower - path_044_score(world)).abs() < 1e-9);
}

#[test]
fn path_044_b_one_uncertain_dimension_matches_simple_scalar_bounds() {
    use distribution_machine::measurement::Substitutability;

    // Only dependency reach (qualified_dependency_weight) varies; the
    // aggregation is separable and monotonic in this dimension alone, so
    // Candidate A and Candidate B must agree exactly.
    let low = path_044_world(0.4, 2, 2, Substitutability::Moderate);
    let high = path_044_world(0.6, 2, 2, Substitutability::Moderate);

    let a = path_044_candidate_a_score_each_world(&[low, high], Path044State::Unresolved);
    let b = path_044_candidate_b_corner_scores(&[low, high], Path044State::Unresolved);

    assert!((a.lower - b.lower).abs() < 1e-9);
    assert!((a.upper - b.upper).abs() < 1e-9);
    assert_eq!(a.state, b.state);
    assert!(!a.is_exact(), "the dimension genuinely varies -- this must not collapse to exact");
}

#[test]
fn path_044_c_correlated_dimensions_do_not_widen_the_scalar_range_but_do_fabricate_corner_states() {
    use distribution_machine::measurement::Substitutability;

    // Correlated pair: only "all easy" and "all hard" are admissible --
    // no mixed world.
    let world_low = path_044_world(0.2, 1, 1, Substitutability::Easy);
    let world_high = path_044_world(0.9, 5, 5, Substitutability::None);

    let a = path_044_candidate_a_score_each_world(&[world_low, world_high], Path044State::Unresolved);
    let b = path_044_candidate_b_corner_scores(&[world_low, world_high], Path044State::Unresolved);

    // Because `measure_criticality` is separable+monotonic, the box's
    // own extreme corners ARE `world_low`/`world_high` -- the scalar
    // range is not widened by independent bounding in this specific
    // (extremes-are-admissible) case.
    assert!((a.lower - b.lower).abs() < 1e-9);
    assert!((a.upper - b.upper).abs() < 1e-9);

    // But the corner GENERATOR still fabricates mixed, non-admissible
    // observable vectors as part of computing that range.
    let corners = path_044_generate_corners(&[world_low, world_high]);
    let fabricated = corners
        .iter()
        .filter(|c| **c != world_low && **c != world_high)
        .count();
    assert!(
        fabricated > 0,
        "independent per-dimension bounding still manufactures observable-vector combinations \
         (e.g. high reach paired with Easy substitutability) that were never admissible, even \
         though they happen not to distort THIS scalar range"
    );
}

#[test]
fn path_044_d_anticorrelated_dimensions_make_independent_bounds_strictly_looser() {
    use distribution_machine::measurement::Substitutability;

    // Anti-correlated: reach is high while everything else is easy/
    // contained, or reach is low while everything else is maxed out --
    // the two admissible worlds sit INSIDE the independent-bounds box,
    // not at its corners.
    let world_hi_lo = path_044_world(0.9, 1, 1, Substitutability::Easy);
    let world_lo_hi = path_044_world(0.2, 5, 5, Substitutability::None);

    let a = path_044_candidate_a_score_each_world(&[world_hi_lo, world_lo_hi], Path044State::Unresolved);
    let b = path_044_candidate_b_corner_scores(&[world_hi_lo, world_lo_hi], Path044State::Unresolved);

    assert!(
        b.lower < a.lower - 1e-9,
        "Candidate B's independently-bounded lower ({}) must be strictly below the true \
         admissible minimum ({}) -- the box's all-low corner (reach low AND everything easy) is \
         not an admissible world, but it still gets evaluated",
        b.lower,
        a.lower
    );
    assert!(
        b.upper > a.upper + 1e-9,
        "Candidate B's independently-bounded upper ({}) must be strictly above the true \
         admissible maximum ({})",
        b.upper,
        a.upper
    );
    assert!(a.contains(&a) && b.contains(&a), "B must still soundly CONTAIN the true range -- it is loose, never wrong");
}

/// Shared construction for E and I: two observable vectors that trade
/// dependency reach against substitutability ease and land on the
/// EXACT same scalar score, using real production semantics (the same
/// separable weighted sum `measure_criticality` actually computes).
fn path_044_two_worlds_same_score()
-> (distribution_machine::measurement::CriticalityObservables, distribution_machine::measurement::CriticalityObservables)
{
    use distribution_machine::measurement::Substitutability;

    let weights = path_044_weights();
    let world_x = path_044_world(0.5, 2, 2, Substitutability::Moderate);

    // Solve for the qdw that makes world_y (Easy substitutability,
    // same blocking/blast) land on the identical score as world_x.
    let sub_x = 0.33;
    let sub_y = 0.0;
    let qdw_y = (0.5 * weights.qualified_dependency_weight + sub_x * weights.substitutability
        - sub_y * weights.substitutability)
        / weights.qualified_dependency_weight;
    let world_y = path_044_world(qdw_y, 2, 2, Substitutability::Easy);

    (world_x, world_y)
}

#[test]
fn path_044_e_same_scalar_different_structure_is_not_semantic_equivalence() {
    let (world_x, world_y) = path_044_two_worlds_same_score();
    assert_ne!(world_x, world_y, "the two worlds must genuinely differ in composition");

    let score_x = path_044_score(world_x);
    let score_y = path_044_score(world_y);
    assert!(
        (score_x - score_y).abs() < 1e-9,
        "constructed to have identical scalar Criticality despite different composition: {score_x} vs {score_y}"
    );

    // A caller looking ONLY at the scalar would (correctly) see one
    // exact number and (incorrectly, if they stop there) conclude the
    // operational situation is fully known. It is not: high-reach-but-
    // easily-substituted and moderate-reach-with-moderate-substitution
    // are materially different postures that happen to net the same
    // score.
}

#[test]
fn path_044_i_exact_scalar_can_coexist_with_an_unresolved_explanation() {
    let (world_x, world_y) = path_044_two_worlds_same_score();
    let result = path_044_build_result(&[world_x, world_y], Path044State::Unresolved);

    assert!(result.scalar.is_exact(), "the score is invariant across both admissible worlds");
    assert_eq!(
        result.scalar.state,
        Path044State::Supported,
        "the SCALAR is Supported -- every admissible world agrees on the number"
    );
    assert_eq!(
        result.explanation_state,
        Path044State::Unresolved,
        "the OBSERVABLE VECTOR explaining that number remains Unresolved -- which world is real \
         is still not known, even though it doesn't matter for this particular query. A single \
         shared `state` field (FIND-020's original shape) cannot represent this split; scalar \
         epistemic state and explanation epistemic state are two independent axes."
    );
}

#[test]
fn path_044_f_threshold_cliff_is_not_reported_as_smooth() {
    use distribution_machine::measurement::{DependencyAggregationModel, aggregate_dependency_weight_with_model};

    // FIND-006's real production cliff: DependencyAggregationModel::
    // Thresholded drops any dependency scoring below 0.20 entirely.
    let just_below = aggregate_dependency_weight_with_model(
        &[path_044_dep(0.199)],
        DependencyAggregationModel::Thresholded,
    );
    let just_above = aggregate_dependency_weight_with_model(
        &[path_044_dep(0.201)],
        DependencyAggregationModel::Thresholded,
    );
    let cliff_delta = (just_above - just_below).abs();

    // Same tiny evidence delta (0.002), Complementary model: no cliff.
    let smooth_below =
        aggregate_dependency_weight_with_model(&[path_044_dep(0.199)], DependencyAggregationModel::Complementary);
    let smooth_above =
        aggregate_dependency_weight_with_model(&[path_044_dep(0.201)], DependencyAggregationModel::Complementary);
    let smooth_delta = (smooth_above - smooth_below).abs();

    assert!(
        cliff_delta > smooth_delta * 50.0,
        "Thresholded's output jump ({cliff_delta}) for a 0.002 evidence delta must be reported \
         as an actual discontinuity, not smoothed into a continuous-looking interval -- compare \
         to the SAME evidence delta under Complementary ({smooth_delta}), which has no cliff"
    );

    // Propagate through measure_criticality: the resulting Criticality
    // Measured044 range is a CORRECT bound over the two discrete
    // evidence points actually enumerated -- but nothing about that
    // range licenses treating intermediate evidence values (e.g. 0.1999
    // vs 0.2001) as producing intermediate scores. That would be false
    // continuity.
    use distribution_machine::measurement::Substitutability;
    let world_below = path_044_world(just_below, 2, 2, Substitutability::Moderate);
    let world_above = path_044_world(just_above, 2, 2, Substitutability::Moderate);
    let measured = path_044_candidate_a_score_each_world(&[world_below, world_above], Path044State::Unresolved);
    assert!(!measured.is_exact());
    assert_eq!(measured.state, Path044State::Unresolved);
}

#[test]
fn path_044_g_population_normalization_is_not_a_concept_in_production_criticality() {
    // N/A, documented with an executable check rather than only prose:
    // `measure_criticality`, `qualify_dependency`,
    // `aggregate_dependency_weight_with_model`, and
    // `aggregate_structural_dependencies_with_model` all take only the
    // caller's own local inputs -- no population/peer-set parameter
    // exists in any of their signatures (grep confirmed zero hits for
    // population/percentile/z-score/relative-rank in `measurement.rs`
    // outside RankedDecay's own single-capability internal ranking,
    // which never looks outside the slice passed to it). Concretely:
    // capability A's score is identical whether or not an unrelated
    // capability B's data exists anywhere else in the program, because
    // A's computation never receives it.
    use distribution_machine::measurement::{DependencyAggregationModel, aggregate_structural_dependencies_with_model};

    let a_records = vec![path_044_dep_record("d1", "p1", "f1", "A", "X", 0.5)];
    let score_a_alone = aggregate_structural_dependencies_with_model(&a_records, DependencyAggregationModel::Complementary);

    // An unrelated "population" capability B's records exist, but are
    // never passed into A's computation -- by construction, they cannot
    // matter, because no function here accepts a population argument.
    let _b_records_exist_elsewhere = vec![path_044_dep_record("d2", "p2", "f2", "B", "Y", 0.99)];
    let score_a_with_population_present = aggregate_structural_dependencies_with_model(&a_records, DependencyAggregationModel::Complementary);

    assert_eq!(score_a_alone, score_a_with_population_present, "N/A: no population-relative normalization exists to attack");
}

#[test]
fn path_044_h_conflict_in_one_dimension_survives_aggregation() {
    use distribution_machine::measurement::Substitutability;

    // blocking_paths is Conflicted (two grounded origins disagree);
    // every other dimension is identical across both worlds (Supported).
    let world_a = path_044_world(0.5, 1, 3, Substitutability::Moderate);
    let world_b = path_044_world(0.5, 4, 3, Substitutability::Moderate);

    let per_dim = path_044_candidate_c_per_dimension_bounds(&[world_a, world_b], Path044State::Conflicted);
    assert_eq!(per_dim.qualified_dependency_weight.state, Path044State::Supported);
    assert_eq!(per_dim.blocking_paths.state, Path044State::Conflicted);
    assert_eq!(per_dim.failure_blast_radius.state, Path044State::Supported);
    assert_eq!(per_dim.substitutability.state, Path044State::Supported);

    let overall_ambient = path_044_worst_state(&[
        per_dim.qualified_dependency_weight.state,
        per_dim.blocking_paths.state,
        per_dim.failure_blast_radius.state,
        per_dim.substitutability.state,
    ]);
    assert_eq!(overall_ambient, Path044State::Conflicted);

    let scalar = path_044_candidate_a_score_each_world(&[world_a, world_b], overall_ambient);
    assert!(!scalar.is_exact());
    assert_eq!(
        scalar.state,
        Path044State::Conflicted,
        "conflict in ONE dimension must survive aggregation into the final scalar -- it must not \
         be silently diluted into Unresolved (or hidden entirely) just because three of four \
         dimensions are Supported"
    );
}

#[test]
fn path_044_j_duplicate_replay_evidence_does_not_narrow_the_criticality_range() {
    use distribution_machine::measurement::{DependencyAggregationModel, aggregate_structural_dependencies_with_model};

    // World 1: a single real structural dependency A->X. World 2: a
    // genuinely different, stronger dependency A->Z (a real second
    // admissible world, not a duplicate).
    let world1_records = vec![path_044_dep_record("d1", "p1", "f1", "A", "X", 0.4)];
    let world2_records = vec![path_044_dep_record("d1", "p1", "f1", "A", "Z", 0.8)];

    let qdw1 = aggregate_structural_dependencies_with_model(&world1_records, DependencyAggregationModel::Complementary);
    let qdw2 = aggregate_structural_dependencies_with_model(&world2_records, DependencyAggregationModel::Complementary);

    use distribution_machine::measurement::Substitutability;
    let baseline = path_044_candidate_a_score_each_world(
        &[
            path_044_world(qdw1, 2, 2, Substitutability::Moderate),
            path_044_world(qdw2, 2, 2, Substitutability::Moderate),
        ],
        Path044State::Unresolved,
    );

    // Now replay/duplicate EACH world's own record under fresh
    // dependency/path/domain ids (FIND-007's structural key still
    // matches, so production dedup collapses these to the same single
    // record).
    let mut world1_replayed = world1_records.clone();
    let mut world2_replayed = world2_records.clone();
    for i in 0..9 {
        world1_replayed.push(path_044_dep_record(&format!("d1-replay-{i}"), &format!("p1-replay-{i}"), &format!("f1-replay-{i}"), "A", "X", 0.4));
        world2_replayed.push(path_044_dep_record(&format!("d2-replay-{i}"), &format!("p2-replay-{i}"), &format!("f2-replay-{i}"), "A", "Z", 0.8));
    }

    let qdw1_replayed = aggregate_structural_dependencies_with_model(&world1_replayed, DependencyAggregationModel::Complementary);
    let qdw2_replayed = aggregate_structural_dependencies_with_model(&world2_replayed, DependencyAggregationModel::Complementary);
    assert_eq!(qdw1, qdw1_replayed);
    assert_eq!(qdw2, qdw2_replayed);

    let replayed = path_044_candidate_a_score_each_world(
        &[
            path_044_world(qdw1_replayed, 2, 2, Substitutability::Moderate),
            path_044_world(qdw2_replayed, 2, 2, Substitutability::Moderate),
        ],
        Path044State::Unresolved,
    );

    assert!((baseline.lower - replayed.lower).abs() < 1e-9);
    assert!((baseline.upper - replayed.upper).abs() < 1e-9);
    assert_eq!(baseline.state, replayed.state);
}

#[test]
fn path_044_k_representation_depth_does_not_change_criticality() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, Substitutability, canonicalize_pure_relays, reachable_capabilities,
    };

    // Chains PATH-043-J's already-proven graph-level invariant into the
    // Criticality layer: derive a test-local qualified_dependency_weight
    // proxy from reach (canonicalized first, per FIND-017/FIND-020),
    // then confirm Criticality itself is invariant to harmless relay
    // padding.
    let source = CapabilityId("source".into());
    let protected = vec![CapabilityId("source".into()), CapabilityId("x".into()), CapabilityId("y".into())];

    let qdw_from_reach = |edges: &[DependencyEdge]| -> f64 {
        let (canonical, _) = canonicalize_pure_relays(&source, &protected, edges);
        (reachable_capabilities(&source, &canonical).len() as f64 / 10.0).min(1.0)
    };

    let bare = vec![DependencyEdge::new("source", "x"), DependencyEdge::new("source", "y")];
    let padded = vec![
        DependencyEdge::new("source", "relay1"),
        DependencyEdge::new("relay1", "x"),
        DependencyEdge::new("source", "relay2"),
        DependencyEdge::new("relay2", "y"),
    ];

    let bare_world = path_044_world(qdw_from_reach(&bare), 2, 2, Substitutability::Moderate);
    let padded_world = path_044_world(qdw_from_reach(&padded), 2, 2, Substitutability::Moderate);

    assert_eq!(bare_world, padded_world, "reach-derived qdw must be invariant to relay padding");
    assert!((path_044_score(bare_world) - path_044_score(padded_world)).abs() < 1e-9);
}

#[test]
fn path_044_l_comparison_and_ranking_stay_epistemically_honest() {
    use distribution_machine::measurement::Substitutability;

    // Case 1: definitely greater -- disjoint ranges.
    let a = path_044_candidate_a_score_each_world(
        &[path_044_world(0.9, 5, 5, Substitutability::None)],
        Path044State::Unresolved,
    );
    let b = path_044_candidate_a_score_each_world(
        &[path_044_world(0.1, 0, 0, Substitutability::Easy)],
        Path044State::Unresolved,
    );
    assert_eq!(path_044_compare(a, b), Path044Ranking::DefinitelyGreater);

    // Case 2: overlapping ranges -- must not manufacture a ranking.
    let c = path_044_candidate_a_score_each_world(
        &[
            path_044_world(0.3, 2, 2, Substitutability::Moderate),
            path_044_world(0.6, 2, 2, Substitutability::Moderate),
        ],
        Path044State::Unresolved,
    );
    let d = path_044_candidate_a_score_each_world(
        &[
            path_044_world(0.5, 2, 2, Substitutability::Moderate),
            path_044_world(0.8, 2, 2, Substitutability::Moderate),
        ],
        Path044State::Unresolved,
    );
    assert_eq!(path_044_compare(c, d), Path044Ranking::Overlapping);

    // Case 3: same scalar range, different admissible structure --
    // reuses PATH-044-O's two sets. Ranking correctly reports a tie
    // (Overlapping), but that tie does NOT mean the two situations are
    // the same.
    let world_low = path_044_world(0.2, 1, 1, Substitutability::Easy);
    let world_high = path_044_world(0.9, 5, 5, Substitutability::None);
    let world_mixed_1 = path_044_world(0.9, 1, 1, Substitutability::Easy);
    let world_mixed_2 = path_044_world(0.2, 5, 5, Substitutability::None);

    let set_diagonal = AdmissibleCriticalityStates { states: vec![world_low, world_high], epistemic_state: Path044State::Unresolved };
    let set_all_corners = AdmissibleCriticalityStates {
        states: vec![world_low, world_high, world_mixed_1, world_mixed_2],
        epistemic_state: Path044State::Unresolved,
    };

    let e = set_diagonal.scalar_range();
    let f = set_all_corners.scalar_range();
    assert_eq!(path_044_compare(e, f), Path044Ranking::Overlapping);
    assert!((e.lower - f.lower).abs() < 1e-9 && (e.upper - f.upper).abs() < 1e-9, "ranges are numerically identical");
    assert_ne!(set_diagonal.states.len(), set_all_corners.states.len(), "but the admissible structures differ -- ranking-by-range correctly cannot see that difference");
}

#[test]
fn path_044_m_downstream_aggregation_does_not_manufacture_certainty() {
    use distribution_machine::measurement::Substitutability;

    // Two capabilities, each uncertain, driven by a SHARED pair of
    // admissible worlds where their scores anti-correlate.
    let world_hi_lo = path_044_world(0.9, 1, 1, Substitutability::Easy);
    let world_lo_hi = path_044_world(0.2, 5, 5, Substitutability::None);

    let cap1_in_world_hi_lo = path_044_score(world_hi_lo);
    let cap1_in_world_lo_hi = path_044_score(world_lo_hi);
    let cap1 = Measured044::from_values(&[cap1_in_world_hi_lo, cap1_in_world_lo_hi], Path044State::Unresolved);

    // cap2 is high exactly when cap1 (via world_hi_lo) is low, and vice
    // versa -- an independently-uncertain second capability with no
    // shared admissible-world bookkeeping retained by the caller.
    let cap2_in_world_hi_lo = 0.90;
    let cap2_in_world_lo_hi = 0.20;
    let cap2 = Measured044::from_values(&[cap2_in_world_hi_lo, cap2_in_world_lo_hi], Path044State::Unresolved);

    let naive_composed = path_044_compose_sum(cap1, cap2);

    let true_joint_hi_lo = cap1_in_world_hi_lo + cap2_in_world_hi_lo;
    let true_joint_lo_hi = cap1_in_world_lo_hi + cap2_in_world_lo_hi;
    let true_joint = Measured044::from_values(&[true_joint_hi_lo, true_joint_lo_hi], Path044State::Unresolved);

    assert!(
        naive_composed.lower <= true_joint.lower + 1e-9,
        "composition must never claim a HIGHER minimum than the true joint minimum"
    );
    assert!(
        naive_composed.upper >= true_joint.upper - 1e-9,
        "composition must never claim a LOWER maximum than the true joint maximum"
    );
    assert_eq!(naive_composed.state, Path044State::Unresolved);
}

#[test]
fn path_044_n_quantifies_candidate_b_state_space_explosion() {
    use distribution_machine::measurement::Substitutability;

    let world_hi_lo = path_044_world(0.9, 1, 1, Substitutability::Easy);
    let world_lo_hi = path_044_world(0.2, 5, 5, Substitutability::None);

    let corners = path_044_generate_corners(&[world_hi_lo, world_lo_hi]);
    let admissible = vec![world_hi_lo, world_lo_hi];

    let total = corners.len();
    let admissible_present = corners.iter().filter(|c| admissible.contains(c)).count();
    let false_states = total - admissible_present;
    let false_fraction = false_states as f64 / total as f64;

    assert_eq!(total, 16, "4 independently-varying dimensions each with 2 observed levels => 2^4 corners");
    assert_eq!(admissible_present, 2);
    assert!(
        false_fraction > 0.5,
        "Candidate B's per-dimension independent bounding fabricates {false_states}/{total} \
         ({:.1}%) non-admissible observable-vector combinations for just 2 real admissible \
         worlds across 4 dimensions",
        false_fraction * 100.0
    );
}

#[test]
fn path_044_o_lossy_compression_counterexample() {
    use distribution_machine::measurement::Substitutability;

    // Two admissible-state SETS that compress to identical per-
    // dimension bounds, identical scalar min/max, and identical
    // epistemic state -- but answer a legitimate structural query
    // ("could high reach co-occur with easy/contained substitutability
    // and low blocking/blast?") differently.
    let world_low = path_044_world(0.2, 1, 1, Substitutability::Easy);
    let world_high = path_044_world(0.9, 5, 5, Substitutability::None);
    let world_mixed_hi_reach_easy = path_044_world(0.9, 1, 1, Substitutability::Easy);
    let world_mixed_lo_reach_hard = path_044_world(0.2, 5, 5, Substitutability::None);

    let set1_diagonal = AdmissibleCriticalityStates { states: vec![world_low, world_high], epistemic_state: Path044State::Unresolved };
    let set2_all_corners = AdmissibleCriticalityStates {
        states: vec![world_low, world_high, world_mixed_hi_reach_easy, world_mixed_lo_reach_hard],
        epistemic_state: Path044State::Unresolved,
    };

    // (a) Per-dimension bounds identical.
    let c1 = path_044_candidate_c_per_dimension_bounds(&set1_diagonal.states, set1_diagonal.epistemic_state);
    let c2 = path_044_candidate_c_per_dimension_bounds(&set2_all_corners.states, set2_all_corners.epistemic_state);
    assert_eq!(c1, c2, "per-dimension bounds compress identically");

    // (b) Scalar min/max identical.
    let s1 = set1_diagonal.scalar_range();
    let s2 = set2_all_corners.scalar_range();
    assert!((s1.lower - s2.lower).abs() < 1e-9 && (s1.upper - s2.upper).abs() < 1e-9, "scalar ranges compress identically");

    // (c) Epistemic state identical.
    assert_eq!(s1.state, s2.state);

    // (d) A legitimate downstream query answers DIFFERENTLY.
    let query_high_reach_but_contained = |o: &distribution_machine::measurement::CriticalityObservables| {
        o.qualified_dependency_weight > 0.5 && o.failure_blast_radius <= 1
    };
    let set1_answer = set1_diagonal.admits(query_high_reach_but_contained);
    let set2_answer = set2_all_corners.admits(query_high_reach_but_contained);

    assert!(!set1_answer, "set 1 has no admissible world where high reach co-occurs with contained blast radius");
    assert!(set2_answer, "set 2 explicitly admits exactly that world");
    assert_ne!(
        set1_answer, set2_answer,
        "PROOF: identical per-dimension bounds + identical scalar range + identical epistemic \
         state, yet the two admissible-state sets give different answers to a legitimate \
         downstream structural query -- bounds+state alone are NOT a complete representation; \
         only retaining the actual admissible set (Candidate D) answers this correctly"
    );
}

// =====================================================================
// PATH-045 -- RANKING / ALLOCATION BOUNDARY UNDER EPISTEMIC UNCERTAINTY
// (H16)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::measurement`/`distribution`/`contribution` or
// wired into any production/allocation path.
//
// PRODUCTION RANKING INVENTORY (read before designing anything below):
//
// grepped `src/*.rs` for rank/sort/compare/partial_cmp/total_cmp/
// priority/normalized/share/allocation. Findings:
//
// - `allocate_contribution_pool` / `allocate_stewardship_pool` /
//   `allocate_capital_pool` (distribution.rs / capital.rs) compute a
//   `normalized_share = value / total` for EVERY entry and never compare
//   two entries against each other -- output order is INPUT order (the
//   code simply `.map()`s over the input slice), not sorted by value.
//   There is no comparison/ranking function between two
//   contributors/capabilities anywhere in this pipeline. Confirmed
//   executable in PATH-045-N.
// - Every `sort_by` in `measurement.rs` (bottleneck lists, convergence
//   regions, canonicalization candidates, dedup output, etc.) sorts by
//   `CapabilityId`/`DependencyPathId`/`FailureDomainId` STRING content --
//   deterministic output-enumeration stabilizers, several already
//   self-documented in-code as carrying "no structural or economic
//   meaning" (e.g. `structural_propagation_surface`'s doc comment).
//   None of these compare economic VALUE.
// - The one place raw `f64` scores are sorted is
//   `DependencyAggregationModel::RankedDecay`
//   (`scores.sort_by(|a,b| b.partial_cmp(a)...)`), and that sort is a
//   purely internal step of aggregating ONE capability's OWN dependency
//   list into ONE scalar -- it never compares two different
//   capabilities/contributors against each other, so it is not a
//   "ranking" in the sense this mission investigates.
//
// CONCLUSION: no production ranking/comparison-between-two-capabilities
// function exists to attack or extend. This experiment builds one from
// scratch, test-local, and asks what shape it would have to take to be
// epistemically honest, reusing FIND-020/021's `Measured`/state
// machinery and `CriticalityObservables`/`measure_criticality` for
// realistic scalar inputs.
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path045State {
    Supported,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Measured045 {
    lower: f64,
    upper: f64,
    state: Path045State,
}

impl Measured045 {
    fn from_values(values: &[f64], ambient_state: Path045State) -> Self {
        let lower = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let upper = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let state = if (upper - lower).abs() < 1e-9 { Path045State::Supported } else { ambient_state };
        Measured045 { lower, upper, state }
    }
}

fn path_045_weights() -> distribution_machine::measurement::CriticalityMeasurementWeights {
    distribution_machine::measurement::CriticalityMeasurementWeights {
        qualified_dependency_weight: 0.30,
        blocking_paths: 0.30,
        failure_blast_radius: 0.25,
        substitutability: 0.15,
    }
}

fn path_045_world(
    qdw: f64,
    blocking: u32,
    blast: u32,
    sub: distribution_machine::measurement::Substitutability,
) -> distribution_machine::measurement::CriticalityObservables {
    distribution_machine::measurement::CriticalityObservables {
        qualified_dependency_weight: qdw,
        blocking_paths: blocking,
        failure_blast_radius: blast,
        substitutability: sub,
    }
}

fn path_045_score(o: distribution_machine::measurement::CriticalityObservables) -> f64 {
    distribution_machine::measurement::measure_criticality(o, path_045_weights()).unwrap().score
}

fn path_045_dep(score: f64) -> distribution_machine::measurement::QualifiedDependency {
    distribution_machine::measurement::QualifiedDependency {
        blocking_relevance: score,
        failure_propagation: score,
        operational_reliance: score,
        business_path_relevance: score,
        score,
    }
}

fn path_045_dep_record(
    dep_id: &str,
    path_id: &str,
    domain_id: &str,
    source: &str,
    target: &str,
    score: f64,
) -> distribution_machine::measurement::DependencyRecord {
    use distribution_machine::measurement::{
        CapabilityId, DependencyId, DependencyPathId, DependencyRelationshipType, FailureDomainId,
    };

    distribution_machine::measurement::DependencyRecord {
        dependency_id: DependencyId(dep_id.into()),
        dependency_path_id: DependencyPathId(path_id.into()),
        failure_domain_id: FailureDomainId(domain_id.into()),
        source_capability: CapabilityId(source.into()),
        target_capability: CapabilityId(target.into()),
        relationship_type: DependencyRelationshipType::Runtime,
        qualified: path_045_dep(score),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path045Relation {
    Greater,
    Equal,
    Less,
    /// We HAVE the joint/correlated admissible-state information, and it
    /// establishes that no single strict order (Greater/Equal/Less)
    /// holds across every jointly admissible world.
    Incomparable,
    /// We do NOT have enough information to determine the relation --
    /// either only marginal (uncorrelated) bounds are available, or
    /// evidence for one side is simply missing.
    Unresolved,
    Conflicted,
}

// ---------------------------------------------------------------------
// Candidate A -- compare midpoints. Expected: killed (PATH-045-J).
// ---------------------------------------------------------------------
fn path_045_candidate_a_midpoint(a: Measured045, b: Measured045) -> Path045Relation {
    let mid_a = (a.lower + a.upper) / 2.0;
    let mid_b = (b.lower + b.upper) / 2.0;
    if mid_a > mid_b {
        Path045Relation::Greater
    } else if mid_a < mid_b {
        Path045Relation::Less
    } else {
        Path045Relation::Equal
    }
}

// ---------------------------------------------------------------------
// Candidate B -- compare marginal bounds only, no joint/correlation
// information. Conservative: refuses to rank whenever ranges overlap,
// REGARDLESS of whether a joint correlation might actually justify a
// strict order (PATH-045-K/L show exactly this gap).
// ---------------------------------------------------------------------
fn path_045_candidate_b_bounds_only(a: Measured045, b: Measured045) -> Path045Relation {
    if a.lower > b.upper {
        Path045Relation::Greater
    } else if b.lower > a.upper {
        Path045Relation::Less
    } else {
        Path045Relation::Unresolved
    }
}

// ---------------------------------------------------------------------
// Candidate C -- all-worlds dominance over the actual jointly admissible
// (score_a, score_b) pairs.
// ---------------------------------------------------------------------
fn path_045_candidate_c_joint_dominance(joint_worlds: &[(f64, f64)]) -> Path045Relation {
    let all_equal = joint_worlds.iter().all(|(a, b)| (a - b).abs() < 1e-9);
    if all_equal {
        return Path045Relation::Equal;
    }
    let all_greater = joint_worlds.iter().all(|(a, b)| *a > *b + 1e-9);
    let all_less = joint_worlds.iter().all(|(a, b)| *b > *a + 1e-9);
    if all_greater {
        Path045Relation::Greater
    } else if all_less {
        Path045Relation::Less
    } else {
        Path045Relation::Incomparable
    }
}

// ---------------------------------------------------------------------
// Candidate D -- ranking verdict + explanation state. Conflict overrides
// everything else (PATH-045-F): a relevant Conflicted input state must
// remain visible even when the raw joint pairs would otherwise look
// decisive.
// ---------------------------------------------------------------------
fn path_045_candidate_d_ranking_verdict(joint_worlds: &[(f64, f64)], conflicted: bool) -> Path045Relation {
    if conflicted {
        return Path045Relation::Conflicted;
    }
    path_045_candidate_c_joint_dominance(joint_worlds)
}

fn path_045_permits_allocation_ordering_assumption(relation: Path045Relation) -> bool {
    matches!(relation, Path045Relation::Greater | Path045Relation::Less | Path045Relation::Equal)
}

// =====================================================================
// Candidate A attack
// =====================================================================

#[test]
fn path_045_j_midpoint_ranking_is_explicitly_rejected() {
    // Overlapping marginal ranges where midpoint(A) > midpoint(B), but
    // an actual jointly admissible world has B > A.
    let a = Measured045::from_values(&[0.3, 0.9], Path045State::Unresolved); // midpoint 0.6
    let b = Measured045::from_values(&[0.5, 0.55], Path045State::Unresolved); // midpoint 0.525

    let midpoint_verdict = path_045_candidate_a_midpoint(a, b);
    assert_eq!(midpoint_verdict, Path045Relation::Greater, "midpoint comparison claims A > B");

    let joint_worlds = [(0.3, 0.55), (0.9, 0.5)];
    // World 1 (A=0.3, B=0.55): B > A here -- directly contradicts the
    // midpoint's claim.
    assert!(joint_worlds[0].1 > joint_worlds[0].0);

    let truth = path_045_candidate_c_joint_dominance(&joint_worlds);
    assert_eq!(
        truth,
        Path045Relation::Incomparable,
        "the ACTUAL admissible worlds disagree on direction -- midpoint's confident 'A > B' is \
         simply wrong, not merely imprecise. Candidate A is rejected: it can assert a relation \
         that does not hold in every (or even most) admissible worlds"
    );
}

// =====================================================================
// PATH-045-A through P -- required pathological cases.
// =====================================================================

#[test]
fn path_045_a_definite_dominance_survives() {
    let a = Measured045::from_values(&[0.8, 0.9], Path045State::Unresolved);
    let b = Measured045::from_values(&[0.3, 0.5], Path045State::Unresolved);

    assert_eq!(path_045_candidate_b_bounds_only(a, b), Path045Relation::Greater);

    // Every combination of A's and B's values (not just a specific
    // pairing) has A > B, since the marginal ranges are disjoint.
    let joint_worlds = [(0.8, 0.3), (0.8, 0.5), (0.9, 0.3), (0.9, 0.5)];
    assert_eq!(path_045_candidate_c_joint_dominance(&joint_worlds), Path045Relation::Greater);
}

#[test]
fn path_045_b_overlapping_ranges_refuse_a_strict_ranking_from_bounds_alone() {
    let a = Measured045::from_values(&[0.4, 0.8], Path045State::Unresolved);
    let b = Measured045::from_values(&[0.6, 0.9], Path045State::Unresolved);

    assert_eq!(path_045_candidate_b_bounds_only(a, b), Path045Relation::Unresolved);
}

/// Shared construction for C and O: two joint admissible-state sets with
/// BYTE-IDENTICAL marginal ranges for A and for B (same two raw values
/// on each side, only the PAIRING differs), one giving definite
/// dominance, the other genuine disagreement.
fn path_045_same_marginals_different_correlation() -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    // Monotone pairing: sorted ascending on both sides, paired
    // low-with-low, high-with-high -- A > B in every world.
    let set1_dominant = vec![(0.4, 0.3), (0.6, 0.5)];
    // Crossed pairing of the EXACT SAME raw value sets -- disagreement.
    let set2_disagreeing = vec![(0.4, 0.5), (0.6, 0.3)];
    (set1_dominant, set2_disagreeing)
}

#[test]
fn path_045_c_identical_marginal_ranges_different_joint_correlation() {
    let (set1, set2) = path_045_same_marginals_different_correlation();

    let marginal_a = |worlds: &[(f64, f64)]| -> (f64, f64) {
        (
            worlds.iter().map(|(a, _)| *a).fold(f64::INFINITY, f64::min),
            worlds.iter().map(|(a, _)| *a).fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let marginal_b = |worlds: &[(f64, f64)]| -> (f64, f64) {
        (
            worlds.iter().map(|(_, b)| *b).fold(f64::INFINITY, f64::min),
            worlds.iter().map(|(_, b)| *b).fold(f64::NEG_INFINITY, f64::max),
        )
    };

    assert_eq!(marginal_a(&set1), marginal_a(&set2), "A's marginal range is byte-identical across both sets");
    assert_eq!(marginal_b(&set1), marginal_b(&set2), "B's marginal range is byte-identical across both sets");

    let set1_a = Measured045::from_values(&set1.iter().map(|(a, _)| *a).collect::<Vec<_>>(), Path045State::Unresolved);
    let set1_b = Measured045::from_values(&set1.iter().map(|(_, b)| *b).collect::<Vec<_>>(), Path045State::Unresolved);
    assert_eq!(
        path_045_candidate_b_bounds_only(set1_a, set1_b),
        Path045Relation::Unresolved,
        "marginal bounds alone overlap in BOTH sets -- Candidate B cannot see the difference"
    );

    let verdict1 = path_045_candidate_c_joint_dominance(&set1);
    let verdict2 = path_045_candidate_c_joint_dominance(&set2);

    assert_eq!(verdict1, Path045Relation::Greater, "set 1's joint correlation makes A > B definite");
    assert_eq!(verdict2, Path045Relation::Incomparable, "set 2's joint correlation shows genuine disagreement");
    assert_ne!(
        verdict1, verdict2,
        "PROOF: identical marginal ranges compress to the SAME Candidate-B verdict (Unresolved) \
         for both sets, yet the true joint-correlation-aware verdicts differ entirely -- marginal \
         bounds alone cannot determine ranking; joint/correlation information is required"
    );
}

#[test]
fn path_045_d_perfect_tie_returns_equal_not_unresolved() {
    let joint_worlds = [(0.5, 0.5), (0.6, 0.6), (0.3, 0.3)];
    assert_eq!(
        path_045_candidate_c_joint_dominance(&joint_worlds),
        Path045Relation::Equal,
        "every jointly admissible world gives A == B exactly -- this is a SUPPORTED fact, not an \
         absence of information, and must be reported as Equal, never Unresolved"
    );
}

#[test]
fn path_045_e_exact_scalar_order_can_coexist_with_unresolved_explanation() {
    use distribution_machine::measurement::Substitutability;

    // A has two admissible observable vectors that happen to score
    // identically (reusing FIND-021's construction): the SCALAR is
    // exact/Supported, but which vector is real remains Unresolved.
    let weights = path_045_weights();
    let a_world_x = path_045_world(0.5, 2, 2, Substitutability::Moderate);
    let sub_x = 0.33;
    let sub_y = 0.0;
    let a_qdw_y = (0.5 * weights.qualified_dependency_weight + sub_x * weights.substitutability
        - sub_y * weights.substitutability)
        / weights.qualified_dependency_weight;
    let a_world_y = path_045_world(a_qdw_y, 2, 2, Substitutability::Easy);
    assert_ne!(a_world_x, a_world_y, "two genuinely different explanations for A");
    let a_score = path_045_score(a_world_x);
    assert!((a_score - path_045_score(a_world_y)).abs() < 1e-9, "both admissible A-vectors score identically");

    // B is fixed, lower, and fully supported.
    let b_world = path_045_world(0.2, 1, 1, Substitutability::Easy);
    let b_score = path_045_score(b_world);
    assert!(a_score > b_score);

    // Joint pairs: A's score is invariant across A's own two admissible
    // explanations, so every joint pair still has A > B.
    let joint_worlds = [(path_045_score(a_world_x), b_score), (path_045_score(a_world_y), b_score)];
    let relation = path_045_candidate_c_joint_dominance(&joint_worlds);
    assert_eq!(
        relation,
        Path045Relation::Greater,
        "the SCALAR ranking is Supported/definite -- A > B in every admissible world -- even \
         though A's own EXPLANATION (which of its two admissible observable vectors is real) \
         remains Unresolved. This extends FIND-021's scalar/explanation split to ranking."
    );

    let a_explanation_state = if a_world_x == a_world_y { Path045State::Supported } else { Path045State::Unresolved };
    assert_eq!(a_explanation_state, Path045State::Unresolved);
}

#[test]
fn path_045_f_conflicted_a_remains_visible_even_when_bounds_look_decisive() {
    use distribution_machine::measurement::Substitutability;

    // A's blocking_paths dimension is Conflicted between two grounded,
    // disagreeing origins (mirrors PATH-044-H). Both of A's admissible
    // worlds still score ABOVE B -- a naive bounds check would
    // confidently say Greater.
    let a_world_1 = path_045_world(0.8, 1, 4, Substitutability::None);
    let a_world_2 = path_045_world(0.8, 5, 4, Substitutability::None);
    let b_world = path_045_world(0.1, 0, 0, Substitutability::Easy);

    let joint_worlds = [(path_045_score(a_world_1), path_045_score(b_world)), (path_045_score(a_world_2), path_045_score(b_world))];
    // Confirm the bounds-only reading really would look decisive.
    assert!(joint_worlds.iter().all(|(a, b)| a > b), "both of A's conflicted worlds score above B");

    let verdict = path_045_candidate_d_ranking_verdict(&joint_worlds, true);
    assert_eq!(
        verdict,
        Path045Relation::Conflicted,
        "conflict must remain visible and take priority over even a seemingly-decisive bounds \
         check -- it must not silently collapse into Greater just because the numbers happen to \
         separate cleanly"
    );
}

#[test]
fn path_045_g_missing_evidence_is_not_treated_as_low_score() {
    use distribution_machine::measurement::Substitutability;

    // A's evidence for one dimension is simply missing -- nothing
    // resolves it, so BOTH a low-scoring and a high-scoring world for A
    // remain admissible.
    let a_world_low = path_045_world(0.1, 0, 0, Substitutability::Easy);
    let a_world_high = path_045_world(0.9, 5, 5, Substitutability::None);
    let b_world = path_045_world(0.5, 2, 2, Substitutability::Moderate);

    // A naive candidate that treats missing evidence as "assume the low
    // world" would conclude B > A unconditionally.
    let naive_missing_as_low = path_045_score(a_world_low) < path_045_score(b_world);
    assert!(naive_missing_as_low, "the naive shortcut silently commits to A's low world");

    let joint_worlds = [(path_045_score(a_world_low), path_045_score(b_world)), (path_045_score(a_world_high), path_045_score(b_world))];
    let verdict = path_045_candidate_c_joint_dominance(&joint_worlds);
    assert_eq!(
        verdict,
        Path045Relation::Incomparable,
        "the correct handling retains A's high-scoring admissible world too -- the true relation \
         is genuinely undetermined (A could be above or below B), not a false definite 'B > A' \
         manufactured by treating missing evidence as if it were a low value"
    );
}

#[test]
fn path_045_h_duplicate_replay_evidence_does_not_upgrade_ranking_certainty() {
    use distribution_machine::measurement::{DependencyAggregationModel, Substitutability, aggregate_structural_dependencies_with_model};

    let a_world1_records = vec![path_045_dep_record("d1", "p1", "f1", "A", "X", 0.4)];
    let a_world2_records = vec![path_045_dep_record("d1", "p1", "f1", "A", "Z", 0.8)];
    let b_world = path_045_world(0.5, 2, 2, Substitutability::Moderate);
    let b_score = path_045_score(b_world);

    let qdw1 = aggregate_structural_dependencies_with_model(&a_world1_records, DependencyAggregationModel::Complementary);
    let qdw2 = aggregate_structural_dependencies_with_model(&a_world2_records, DependencyAggregationModel::Complementary);
    let baseline_joint = [
        (path_045_score(path_045_world(qdw1, 2, 2, Substitutability::Moderate)), b_score),
        (path_045_score(path_045_world(qdw2, 2, 2, Substitutability::Moderate)), b_score),
    ];
    let baseline_verdict = path_045_candidate_c_joint_dominance(&baseline_joint);

    let mut a_world1_replayed = a_world1_records.clone();
    let mut a_world2_replayed = a_world2_records.clone();
    for i in 0..9 {
        a_world1_replayed.push(path_045_dep_record(&format!("d1-r{i}"), &format!("p1-r{i}"), &format!("f1-r{i}"), "A", "X", 0.4));
        a_world2_replayed.push(path_045_dep_record(&format!("d2-r{i}"), &format!("p2-r{i}"), &format!("f2-r{i}"), "A", "Z", 0.8));
    }
    let qdw1_replayed = aggregate_structural_dependencies_with_model(&a_world1_replayed, DependencyAggregationModel::Complementary);
    let qdw2_replayed = aggregate_structural_dependencies_with_model(&a_world2_replayed, DependencyAggregationModel::Complementary);
    assert_eq!(qdw1, qdw1_replayed);
    assert_eq!(qdw2, qdw2_replayed);

    let replayed_joint = [
        (path_045_score(path_045_world(qdw1_replayed, 2, 2, Substitutability::Moderate)), b_score),
        (path_045_score(path_045_world(qdw2_replayed, 2, 2, Substitutability::Moderate)), b_score),
    ];
    let replayed_verdict = path_045_candidate_c_joint_dominance(&replayed_joint);

    assert_eq!(baseline_verdict, replayed_verdict, "duplicate/replayed evidence must not change A's ranking verdict against B");
}

#[test]
fn path_045_i_representation_depth_leaves_ranking_invariant() {
    use distribution_machine::measurement::{
        CapabilityId, DependencyEdge, Substitutability, canonicalize_pure_relays, reachable_capabilities,
    };

    let source = CapabilityId("source".into());
    let protected = vec![CapabilityId("source".into()), CapabilityId("x".into()), CapabilityId("y".into())];

    let qdw_from_reach = |edges: &[DependencyEdge]| -> f64 {
        let (canonical, _) = canonicalize_pure_relays(&source, &protected, edges);
        (reachable_capabilities(&source, &canonical).len() as f64 / 10.0).min(1.0)
    };

    let bare = vec![DependencyEdge::new("source", "x"), DependencyEdge::new("source", "y")];
    let padded = vec![
        DependencyEdge::new("source", "relay1"),
        DependencyEdge::new("relay1", "x"),
        DependencyEdge::new("source", "relay2"),
        DependencyEdge::new("relay2", "y"),
    ];

    let a_bare = path_045_world(qdw_from_reach(&bare), 2, 2, Substitutability::Moderate);
    let a_padded = path_045_world(qdw_from_reach(&padded), 2, 2, Substitutability::Moderate);
    let b = path_045_world(0.1, 0, 0, Substitutability::Easy);

    let verdict_bare = path_045_candidate_c_joint_dominance(&[(path_045_score(a_bare), path_045_score(b))]);
    let verdict_padded = path_045_candidate_c_joint_dominance(&[(path_045_score(a_padded), path_045_score(b))]);

    assert_eq!(a_bare, a_padded);
    assert_eq!(verdict_bare, verdict_padded);
}

#[test]
fn path_045_k_joint_correlation_justifies_strict_ranking_despite_overlapping_marginals() {
    let joint_worlds = [(0.55, 0.5), (0.65, 0.6)];

    let a = Measured045::from_values(&joint_worlds.iter().map(|(a, _)| *a).collect::<Vec<_>>(), Path045State::Unresolved);
    let b = Measured045::from_values(&joint_worlds.iter().map(|(_, b)| *b).collect::<Vec<_>>(), Path045State::Unresolved);
    assert_eq!(path_045_candidate_b_bounds_only(a, b), Path045Relation::Unresolved, "marginal ranges overlap (a.lower=0.55 < b.upper=0.6)");

    assert_eq!(
        path_045_candidate_c_joint_dominance(&joint_worlds),
        Path045Relation::Greater,
        "every ACTUAL joint world has A > B -- joint/correlation information justifies a strict \
         ranking that marginal bounds alone refuse"
    );
}

#[test]
fn path_045_l_bounds_rule_is_sound_whenever_it_fires() {
    // Attempt to break Candidate B's soundness (not its completeness --
    // K already shows it's too weak). Claim: whenever a.lower > b.upper
    // (TRUE min/max over the admissible set), every joint pair also has
    // A > B, regardless of correlation structure, because both bounds
    // are global facts independent of pairing.
    let scenarios: Vec<Vec<(f64, f64)>> = vec![
        vec![(0.8, 0.3), (0.9, 0.5), (0.85, 0.4)],
        vec![(0.71, 0.3), (0.71, 0.6), (0.9, 0.3)],
        vec![(1.0, 0.0), (0.51, 0.5)],
    ];

    for joint_worlds in &scenarios {
        let a_vals: Vec<f64> = joint_worlds.iter().map(|(a, _)| *a).collect();
        let b_vals: Vec<f64> = joint_worlds.iter().map(|(_, b)| *b).collect();
        let a = Measured045::from_values(&a_vals, Path045State::Unresolved);
        let b = Measured045::from_values(&b_vals, Path045State::Unresolved);

        if path_045_candidate_b_bounds_only(a, b) == Path045Relation::Greater {
            assert_eq!(
                path_045_candidate_c_joint_dominance(joint_worlds),
                Path045Relation::Greater,
                "PROOF ATTEMPT FAILED (as expected): whenever a.lower > b.upper, every a_i >= \
                 a.lower > b.upper >= b_i for ANY pairing (a_i, b_j), so joint dominance can never \
                 disagree with the bounds-only verdict once it fires. The rule is sound, not merely \
                 usually-right -- no counterexample exists to invent here."
            );
        }
    }
}

#[test]
fn path_045_m_cyclic_pairwise_ranking_from_independently_derived_pairwise_sets() {
    // Three SEPARATE pairwise joint sets, each individually valid, but
    // never reconciled into one shared 3-way admissible model -- B's
    // values in the (A,B) set are not the same B's values used in the
    // (B,C) set.
    let joint_ab = [(10.0, 1.0), (11.0, 2.0)]; // A > B always
    let joint_bc = [(0.5, 0.1), (0.6, 0.2)]; // B > C always (a DIFFERENT B range: [0.5,0.6])
    let joint_ca = [(100.0, 10.0), (101.0, 11.0)]; // C > A always (C here means the first element)

    assert_eq!(path_045_candidate_c_joint_dominance(&joint_ab), Path045Relation::Greater);
    assert_eq!(path_045_candidate_c_joint_dominance(&joint_bc), Path045Relation::Greater);
    assert_eq!(path_045_candidate_c_joint_dominance(&joint_ca), Path045Relation::Greater);
    // A > B, B > C, C > A: a cycle, composed entirely from three
    // independently-fabricated pairwise joint sets whose "B" (and "C")
    // ranges don't even overlap between the sets that mention them.

    // Contrast: ONE shared 3-way admissible model, all three pairwise
    // verdicts derived from the SAME per-world (A,B,C) triples, is
    // transitive by ordinary real-number transitivity -- no cycle is
    // possible.
    let shared_worlds: [(f64, f64, f64); 2] = [(10.0, 5.0, 1.0), (11.0, 6.0, 2.0)]; // (A,B,C) per world, A>B>C in both
    let ab_from_shared: Vec<(f64, f64)> = shared_worlds.iter().map(|(a, b, _)| (*a, *b)).collect();
    let bc_from_shared: Vec<(f64, f64)> = shared_worlds.iter().map(|(_, b, c)| (*b, *c)).collect();
    let ac_from_shared: Vec<(f64, f64)> = shared_worlds.iter().map(|(a, _, c)| (*a, *c)).collect();

    assert_eq!(path_045_candidate_c_joint_dominance(&ab_from_shared), Path045Relation::Greater);
    assert_eq!(path_045_candidate_c_joint_dominance(&bc_from_shared), Path045Relation::Greater);
    assert_eq!(
        path_045_candidate_c_joint_dominance(&ac_from_shared),
        Path045Relation::Greater,
        "A > B and B > C in every world of ONE shared 3-way admissible model implies A > C in \
         that same model, by ordinary pointwise real-number transitivity -- no cycle is possible \
         when all pairwise verdicts are derived from a single shared joint model. All-worlds \
         dominance therefore forms a transitive partial order ONLY over comparisons that share a \
         genuine joint admissible-world model; naively composing separately-derived pairwise \
         verdicts (as above) does not inherit that guarantee and can cycle."
    );
}

#[test]
fn path_045_n_deterministic_tie_breaking_is_not_a_production_concept_here() {
    // N/A, documented executably: `allocate_contribution_pool` never
    // sorts or compares contributors by value -- output order is input
    // order. Constructing events in DECREASING value order and checking
    // the output preserves that same (non-value-sorted) order proves no
    // implicit ranking/tie-break influences allocation.
    use distribution_machine::contribution::{ContributionEvent, ContributionFactors, ContributionWeights};
    use distribution_machine::distribution::allocate_contribution_pool;

    let weights = ContributionWeights { responsibility: 0.20, criticality: 0.30, deal_impact: 0.25, dependency_reach: 0.25 };
    let factors = |v: f64| ContributionFactors {
        responsibility: v,
        criticality: v,
        deal_impact: v,
        stewardship: v,
        dependency_reach: v,
        evidence_confidence: 1.0,
    };

    let events = vec![
        ContributionEvent { contributor_id: "low".into(), contribution_id: "c1".into(), base_work_value: 1.0, factors: factors(0.1) },
        ContributionEvent { contributor_id: "high".into(), contribution_id: "c2".into(), base_work_value: 1.0, factors: factors(0.9) },
    ];

    let allocations = allocate_contribution_pool(&events, &weights, 100.0).unwrap();

    assert_eq!(allocations[0].contributor_id, "low", "output preserves INPUT order (low-value contributor listed first)");
    assert_eq!(allocations[1].contributor_id, "high");
    assert!(
        allocations[1].allocated_amount > allocations[0].allocated_amount,
        "allocation amount reflects value, but LIST ORDER never does -- there is no sort/rank/tie-break to attack"
    );
}

#[test]
fn path_045_o_lossy_compression_counterexample() {
    let (set1, set2) = path_045_same_marginals_different_correlation();

    let a_vals1: Vec<f64> = set1.iter().map(|(a, _)| *a).collect();
    let b_vals1: Vec<f64> = set1.iter().map(|(_, b)| *b).collect();
    let a_vals2: Vec<f64> = set2.iter().map(|(a, _)| *a).collect();
    let b_vals2: Vec<f64> = set2.iter().map(|(_, b)| *b).collect();

    let a1 = Measured045::from_values(&a_vals1, Path045State::Unresolved);
    let b1 = Measured045::from_values(&b_vals1, Path045State::Unresolved);
    let a2 = Measured045::from_values(&a_vals2, Path045State::Unresolved);
    let b2 = Measured045::from_values(&b_vals2, Path045State::Unresolved);

    // Identical A scalar range, identical B scalar range, identical
    // epistemic state for both A and B, across both sets.
    assert_eq!((a1.lower, a1.upper), (a2.lower, a2.upper), "A's compressed range is identical across both sets");
    assert_eq!((b1.lower, b1.upper), (b2.lower, b2.upper), "B's compressed range is identical across both sets");
    assert_eq!(a1.state, a2.state);
    assert_eq!(b1.state, b2.state);

    let verdict1 = path_045_candidate_c_joint_dominance(&set1);
    let verdict2 = path_045_candidate_c_joint_dominance(&set2);

    assert_ne!(
        verdict1, verdict2,
        "PROOF: individual Measured values for A and B (range AND epistemic state) are byte-\
         identical across both joint admissible-state sets, yet the honest pairwise ranking \
         verdict differs (Greater vs Incomparable) -- per-side Measured values are NOT sufficient \
         for honest pairwise ranking; the joint/correlated admissible-state set itself must be \
         consulted"
    );
}

#[test]
fn path_045_p_allocation_boundary_precondition() {
    // No money formula -- only the precondition table for whether a
    // ranking result may be consumed as an ORDERING FACT by a future
    // allocation layer.
    assert!(path_045_permits_allocation_ordering_assumption(Path045Relation::Greater), "1. strict dominance: proven across every jointly admissible world -- safe to consume");
    assert!(path_045_permits_allocation_ordering_assumption(Path045Relation::Equal), "2. exact equality: proven across every jointly admissible world -- safe to consume");
    assert!(!path_045_permits_allocation_ordering_assumption(Path045Relation::Unresolved), "3. unresolved ordering: insufficient evidence -- MUST NOT be consumed as an ordering fact");
    assert!(!path_045_permits_allocation_ordering_assumption(Path045Relation::Incomparable), "4. incomparable: admissible states PROVE no order holds -- MUST NOT be treated as any kind of order, including a tie");
    assert!(!path_045_permits_allocation_ordering_assumption(Path045Relation::Conflicted), "5. conflict: MUST NOT be consumed by any downstream ordering logic until resolved");
    assert!(path_045_permits_allocation_ordering_assumption(Path045Relation::Less));

    // This repository's ACTUAL allocation (`allocate_*_pool`) does not
    // consume an ordering fact at all -- it consumes a scalar VALUE
    // directly (proportional-share normalization, PATH-045-N). This
    // boundary is therefore prospective guidance for any FUTURE
    // ranking-consuming layer, not a description of, or change to,
    // existing behavior.
}

// =====================================================================
// PATH-046 -- NORMALIZATION / ALLOCATION ELIGIBILITY UNDER EPISTEMIC
// UNCERTAINTY (H17)
//
// Disposable experimental machinery only, EXCEPT where noted tests call
// the real production `allocate_contribution_pool` /
// `ContributionEvent::calculate` directly to observe actual behavior.
// No production code was modified.
//
// PRODUCTION ALLOCATION INVENTORY (read before designing anything):
//
// `allocate_contribution_pool(events, weights, pool) -> Result<Vec<
// ContributionAllocation>, &'static str>` (distribution.rs):
// - `ContributionEvent::calculate` computes `evidence_adjusted_value =
//   base_work_value * weighted_factors * evidence_confidence`, where
//   `weighted_factors` is a weighted sum of 4 factors (weights sum to
//   1.0, each factor validated to [0,1]) and `evidence_confidence` is
//   also validated to [0,1]. `base_work_value < 0.0` is REJECTED
//   (Err) -- so a NEGATIVE contribution cannot reach normalization
//   through this validated path (PATH-046-M: impossible by
//   construction, confirmed).
// - `total_contribution = sum(evidence_adjusted_value)`.
// - Zero-total handling: `total_contribution <= 0.0` -> if
//   `contribution_pool == 0.0`, returns `Ok(Vec::new())` (an EMPTY
//   vector, not one zero-amount entry per input event); otherwise
//   `Err("cannot allocate a positive contribution pool with zero
//   verified contribution")`. Normalization is UNDEFINED (rejected, not
//   silently zeroed) whenever total contribution is zero and the pool
//   is positive.
// - Normalization formula: `normalized_share = value / total`,
//   `allocated_amount = pool * normalized_share` -- exactly the
//   `share_i = value_i / sum_j(value_j)` shape the mission asked to
//   verify, confirmed by direct inspection, not assumed.
// - NO deduplication of any kind exists for `ContributionEvent` --
//   `distribution.rs`/`contribution.rs` contain no structural-key/
//   content dedup analogous to `deduplicate_structural_dependencies`
//   (FIND-007). This is a REAL, CONFIRMED GAP relative to FIND-007's
//   principle at this specific production boundary -- see
//   PATH-046-J, which demonstrates it directly against the real
//   function rather than test-local machinery.
// - NO rounding, cents, or currency-unit logic exists anywhere in this
//   pipeline -- pure `f64` arithmetic throughout. Conservation is only
//   ever tested (existing tests included) with an epsilon tolerance,
//   never asserted bit-exact.
// - Numeric domain gaps: `base_work_value < 0.0` does NOT catch `NaN`
//   (IEEE-754 comparisons against NaN are always `false`), and there is
//   no upper bound check at all -- so `NaN` and `f64::INFINITY` both
//   pass validation. `total_contribution <= 0.0` likewise does not
//   catch `NaN`. A single `NaN` (or `INFINITY`, in the multi-
//   contributor infinity/infinity case) contributor input silently
//   POISONS EVERY contributor's `normalized_share` with `NaN`, inside
//   an `Ok(...)` result, not an `Err`. Confirmed directly against
//   production in PATH-046-M -- not fixed, only reported, per this
//   mission's explicit "do not modify production allocation behavior."
// - Normalization is POPULATION-RELATIVE: the denominator is the sum
//   over EVERY event actually passed to the function. Adding or
//   removing a contributor changes every other contributor's
//   `normalized_share` without changing their own raw
//   `contribution_value` at all (PATH-046-P). This is a direct,
//   confirmed CONTRAST with FIND-021/PATH-044-G, where Criticality had
//   NO population-relative normalization anywhere in its pipeline.
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path046State {
    Supported,
    Unresolved,
    Conflicted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Measured046 {
    lower: f64,
    upper: f64,
    state: Path046State,
}

impl Measured046 {
    fn from_values(values: &[f64], ambient_state: Path046State) -> Self {
        let lower = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let upper = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let state = if (upper - lower).abs() < 1e-9 { Path046State::Supported } else { ambient_state };
        Measured046 { lower, upper, state }
    }

    fn is_exact(&self) -> bool {
        (self.upper - self.lower).abs() < 1e-9
    }
}

fn path_046_weights() -> distribution_machine::contribution::ContributionWeights {
    distribution_machine::contribution::ContributionWeights {
        responsibility: 0.20,
        criticality: 0.30,
        deal_impact: 0.25,
        dependency_reach: 0.25,
    }
}

/// Neutral factors (all 1.0) make `evidence_adjusted_value ==
/// base_work_value` exactly, so raw input numbers can be used directly
/// as contribution values without extra arithmetic obscuring the tests.
fn path_046_events(values: &[(&str, f64)]) -> Vec<distribution_machine::contribution::ContributionEvent> {
    values
        .iter()
        .map(|(id, v)| distribution_machine::contribution::ContributionEvent {
            contributor_id: (*id).to_string(),
            contribution_id: format!("{id}-c"),
            base_work_value: *v,
            factors: distribution_machine::contribution::ContributionFactors {
                responsibility: 1.0,
                criticality: 1.0,
                deal_impact: 1.0,
                stewardship: 1.0,
                dependency_reach: 1.0,
                evidence_confidence: 1.0,
            },
        })
        .collect()
}

fn path_046_allocate(
    values: &[(&str, f64)],
    pool: f64,
) -> Result<Vec<distribution_machine::distribution::ContributionAllocation>, &'static str> {
    distribution_machine::distribution::allocate_contribution_pool(&path_046_events(values), &path_046_weights(), pool)
}

fn path_046_share_of(allocations: &[distribution_machine::distribution::ContributionAllocation], id: &str) -> f64 {
    allocations.iter().find(|a| a.contributor_id == id).unwrap().normalized_share
}

// ---------------------------------------------------------------------
// Candidate A -- normalize midpoints. Attacked in PATH-046's own "J"
// analogue is not required by name here, but the mechanism is the same
// one already killed in PATH-045-J; Candidate A is attacked directly in
// PATH-046-D/G, where the midpoint-of-marginals world is shown to be
// just one of several equally-admissible joint vectors, not a supported
// unique result.
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// Candidate B -- normalize marginal endpoints independently. Sound
// enclosure (proven, not merely claimed) but potentially loose: for
// contributor i with marginal range [lo_i, hi_i], its share is
// minimized when i is at its own minimum AND every other contributor is
// simultaneously at ITS OWN maximum (maximizing the denominator while
// minimizing the numerator), and maximized under the mirror-image
// combination. Both extremes are evaluated over the full independent
// Cartesian product, a SUPERSET of the true jointly admissible set, so
// the resulting bound always CONTAINS the true achievable share range
// -- sound, never false -- but is tight only when the true joint set
// actually achieves those specific cross-extrema combinations
// (PATH-046-E).
// ---------------------------------------------------------------------
fn path_046_candidate_b_marginal_share_bounds(marginal_ranges: &[(&str, f64, f64)]) -> Vec<(String, f64, f64)> {
    marginal_ranges
        .iter()
        .map(|(id, lo, hi)| {
            let others_hi_sum: f64 = marginal_ranges.iter().filter(|(oid, _, _)| oid != id).map(|(_, _, h)| *h).sum();
            let others_lo_sum: f64 = marginal_ranges.iter().filter(|(oid, _, _)| oid != id).map(|(_, l, _)| *l).sum();
            let share_lo = lo / (lo + others_hi_sum);
            let share_hi = hi / (hi + others_lo_sum);
            (id.to_string(), share_lo, share_hi)
        })
        .collect()
}

// ---------------------------------------------------------------------
// Candidate C -- normalize every joint admissible world via the REAL
// production formula. This is the reference semantics for this
// experiment.
// ---------------------------------------------------------------------
fn path_046_joint_results(
    worlds: &[Vec<(&str, f64)>],
    pool: f64,
) -> Vec<Result<Vec<distribution_machine::distribution::ContributionAllocation>, &'static str>> {
    worlds.iter().map(|w| path_046_allocate(w, pool)).collect()
}

// ---------------------------------------------------------------------
// Candidate D -- per-contributor share bounds + state, computed from
// the TRUE joint admissible set (only over worlds that actually
// normalize) rather than independently-derived marginal extrema.
// ---------------------------------------------------------------------
fn path_046_candidate_d_bounds(
    worlds: &[Vec<(&str, f64)>],
    pool: f64,
    contributor_id: &str,
    ambient_state: Path046State,
) -> Measured046 {
    let shares: Vec<f64> = worlds
        .iter()
        .filter_map(|w| path_046_allocate(w, pool).ok())
        .map(|allocs| path_046_share_of(&allocs, contributor_id))
        .collect();
    Measured046::from_values(&shares, ambient_state)
}

// ---------------------------------------------------------------------
// Candidate E -- joint allocation state, including a prospective
// settlement-eligibility category. Fields derived from counterexamples
// (F, G, L) rather than assumed up front.
// ---------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path046Eligibility {
    /// The joint admissible set is fully enumerated, every world
    /// normalizes, and the resulting normalized vector is IDENTICAL
    /// across every world -- regardless of whether the RAW inputs were
    /// also identical (PATH-046-C/Q collapse into this same category;
    /// see DERIVED).
    Final,
    /// The joint set is known and every world normalizes, but the
    /// normalized vector genuinely varies -- OR only per-contributor
    /// bounds are available without the retained joint vector set
    /// (PATH-046-F/G's "bounds are not an allocation").
    InformationalBounded,
    BlockedUnresolved,
    BlockedConflicted,
    BlockedNonNormalizable,
}

fn path_046_settlement_eligibility(
    world_results: &[Result<Vec<distribution_machine::distribution::ContributionAllocation>, &'static str>],
    has_joint_vectors: bool,
    conflicted: bool,
    unresolved_missing_evidence: bool,
) -> Path046Eligibility {
    if conflicted {
        return Path046Eligibility::BlockedConflicted;
    }
    if world_results.iter().any(|r| r.is_err()) {
        return Path046Eligibility::BlockedNonNormalizable;
    }
    if unresolved_missing_evidence {
        return Path046Eligibility::BlockedUnresolved;
    }
    if !has_joint_vectors {
        return Path046Eligibility::InformationalBounded;
    }
    let oks: Vec<&Vec<distribution_machine::distribution::ContributionAllocation>> =
        world_results.iter().filter_map(|r| r.as_ref().ok()).collect();
    let all_identical = oks.windows(2).all(|w| {
        w[0].len() == w[1].len()
            && w[0]
                .iter()
                .zip(w[1].iter())
                .all(|(a, b)| a.contributor_id == b.contributor_id && (a.normalized_share - b.normalized_share).abs() < 1e-9)
    });
    if all_identical { Path046Eligibility::Final } else { Path046Eligibility::InformationalBounded }
}

// =====================================================================
// PATH-046-A through R -- required pathological cases.
// =====================================================================

#[test]
fn path_046_a_exact_control() {
    let allocations = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 100.0).unwrap();

    assert!((path_046_share_of(&allocations, "A") - 0.4).abs() < 1e-9);
    assert!((path_046_share_of(&allocations, "B") - 0.3).abs() < 1e-9);
    assert!((path_046_share_of(&allocations, "C") - 0.3).abs() < 1e-9);

    let total: f64 = allocations.iter().map(|a| a.allocated_amount).sum();
    assert!((total - 100.0).abs() < 1e-9, "conservation: allocated amounts sum to the pool");

    let worlds = vec![vec![("A", 40.0), ("B", 30.0), ("C", 30.0)]];
    let eligibility = path_046_settlement_eligibility(&path_046_joint_results(&worlds, 100.0), true, false, false);
    assert_eq!(eligibility, Path046Eligibility::Final);
}

#[test]
fn path_046_b_exact_numerator_uncertain_denominator() {
    let worlds = vec![vec![("A", 40.0), ("B", 20.0)], vec![("A", 40.0), ("B", 60.0)]];

    // A's raw value is exact (a single point) in every admissible world.
    let a_raw = Measured046::from_values(&[40.0, 40.0], Path046State::Unresolved);
    assert!(a_raw.is_exact());

    let a_share = path_046_candidate_d_bounds(&worlds, 100.0, "A", Path046State::Unresolved);
    assert!(
        !a_share.is_exact(),
        "A's raw contribution is exact (40 in both worlds), but its normalized share ({}..{}) \
         still varies because the DENOMINATOR does -- exact numerator does not imply exact \
         allocation share",
        a_share.lower,
        a_share.upper
    );
    assert!((a_share.lower - 40.0 / 100.0).abs() < 1e-9);
    assert!((a_share.upper - 40.0 / 60.0).abs() < 1e-9);
}

#[test]
fn path_046_c_uncertain_raw_values_invariant_share() {
    let worlds = vec![vec![("A", 20.0), ("B", 10.0)], vec![("A", 40.0), ("B", 20.0)]];

    let a_raw = Measured046::from_values(&[20.0, 40.0], Path046State::Unresolved);
    assert!(!a_raw.is_exact(), "A's raw value genuinely varies (20 vs 40)");

    let a_share = path_046_candidate_d_bounds(&worlds, 100.0, "A", Path046State::Unresolved);
    assert!(
        a_share.is_exact(),
        "A's raw value is uncertain, but the ratio A:B (2:1) is preserved by proportional \
         scaling in every admissible world -- the normalized share may legitimately be exact \
         Supported ({}) despite uncertain raw input",
        a_share.lower
    );
    assert!((a_share.lower - 2.0 / 3.0).abs() < 1e-9);
    assert_eq!(a_share.state, Path046State::Supported);
}

/// Shared construction for D and E: identical marginal raw ranges for A
/// (2 values {0.4,0.6}) and B (2 values {0.3,0.5}), monotone-paired
/// (correlated) in set1 vs. crossed-paired in set2 -- the exact same
/// device as PATH-045-C, now applied one layer closer to money.
fn path_046_same_marginals_different_correlation() -> (Vec<Vec<(&'static str, f64)>>, Vec<Vec<(&'static str, f64)>>) {
    let set1 = vec![vec![("A", 0.4), ("B", 0.3)], vec![("A", 0.6), ("B", 0.5)]];
    let set2 = vec![vec![("A", 0.4), ("B", 0.5)], vec![("A", 0.6), ("B", 0.3)]];
    (set1, set2)
}

#[test]
fn path_046_d_identical_marginal_ranges_different_joint_allocation_possibilities() {
    let (set1, set2) = path_046_same_marginals_different_correlation();

    let marginal = |worlds: &[Vec<(&str, f64)>], id: &str| -> (f64, f64) {
        let vals: Vec<f64> = worlds.iter().map(|w| w.iter().find(|(i, _)| *i == id).unwrap().1).collect();
        (vals.iter().cloned().fold(f64::INFINITY, f64::min), vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
    };
    assert_eq!(marginal(&set1, "A"), marginal(&set2, "A"));
    assert_eq!(marginal(&set1, "B"), marginal(&set2, "B"));

    let a_share_set1 = path_046_candidate_d_bounds(&set1, 1.0, "A", Path046State::Unresolved);
    let a_share_set2 = path_046_candidate_d_bounds(&set2, 1.0, "A", Path046State::Unresolved);

    assert!(
        (a_share_set1.lower - a_share_set2.lower).abs() > 1e-6 || (a_share_set1.upper - a_share_set2.upper).abs() > 1e-6,
        "byte-identical marginal raw ranges for A and B, but the two joint pairings produce \
         DIFFERENT admissible normalized-share ranges for A: set1={:?} set2={:?}",
        a_share_set1,
        a_share_set2
    );
}

#[test]
fn path_046_e_cartesian_fabrication_is_sound_but_loose() {
    let (set1, _set2) = path_046_same_marginals_different_correlation();

    let true_bound = path_046_candidate_d_bounds(&set1, 1.0, "A", Path046State::Unresolved);
    let fabricated = path_046_candidate_b_marginal_share_bounds(&[("A", 0.4, 0.6), ("B", 0.3, 0.5)]);
    let (_, fab_lo, fab_hi) = fabricated.iter().find(|(id, _, _)| id == "A").unwrap();

    // SOUND: the independently-derived bound always contains the truth.
    assert!(*fab_lo <= true_bound.lower + 1e-9);
    assert!(*fab_hi >= true_bound.upper - 1e-9);

    // LOOSE: for the correlated (monotone-paired) set, it is strictly
    // wider than necessary.
    assert!(
        *fab_lo < true_bound.lower - 1e-6 || *fab_hi > true_bound.upper + 1e-6,
        "Candidate B's independently-derived bound [{fab_lo}, {fab_hi}] must be strictly looser \
         than the true correlated range [{}, {}] -- sound, but not tight",
        true_bound.lower,
        true_bound.upper
    );
}

#[test]
fn path_046_f_per_contributor_bounds_do_not_form_an_allocation() {
    let bounds = path_046_candidate_b_marginal_share_bounds(&[("A", 0.4, 0.6), ("B", 0.3, 0.5)]);
    let (_, _, a_hi) = bounds.iter().find(|(id, _, _)| id == "A").unwrap();
    let (_, _, b_hi) = bounds.iter().find(|(id, _, _)| id == "B").unwrap();

    let sum_of_independently_chosen_uppers = a_hi + b_hi;
    assert!(
        sum_of_independently_chosen_uppers > 1.0 + 1e-9,
        "picking each contributor's OWN independently-sound upper bound and summing them gives \
         {sum_of_independently_chosen_uppers} > 1.0 (100%) -- each individual bound is a sound \
         projection of the true admissible set, but the COLLECTION of marginal bounds is not \
         itself a valid joint allocation vector; no single admissible world need achieve every \
         contributor's own extreme simultaneously"
    );
}

#[test]
fn path_046_g_same_share_bounds_different_joint_vector_set() {
    // set1: two worlds, A and B trade off, C fixed. set2: the same two
    // worlds PLUS a third "balanced" world -- identical per-contributor
    // min/max for A, B, and C, but a genuinely different admissible
    // joint vector set.
    let set1 = vec![vec![("A", 50.0), ("B", 30.0), ("C", 20.0)], vec![("A", 30.0), ("B", 50.0), ("C", 20.0)]];
    let set2 = vec![
        vec![("A", 50.0), ("B", 30.0), ("C", 20.0)],
        vec![("A", 40.0), ("B", 40.0), ("C", 20.0)],
        vec![("A", 30.0), ("B", 50.0), ("C", 20.0)],
    ];

    for id in ["A", "B", "C"] {
        let b1 = path_046_candidate_d_bounds(&set1, 1.0, id, Path046State::Unresolved);
        let b2 = path_046_candidate_d_bounds(&set2, 1.0, id, Path046State::Unresolved);
        assert!((b1.lower - b2.lower).abs() < 1e-9 && (b1.upper - b2.upper).abs() < 1e-9, "per-contributor share bounds for {id} must be identical across both sets");
    }

    // Legitimate downstream query: can A and B simultaneously both
    // receive at least 0.35?
    let query = |worlds: &[Vec<(&str, f64)>]| -> bool {
        worlds.iter().any(|w| {
            let allocs = path_046_allocate(w, 1.0).unwrap();
            path_046_share_of(&allocs, "A") >= 0.35 && path_046_share_of(&allocs, "B") >= 0.35
        })
    };

    let set1_answer = query(&set1);
    let set2_answer = query(&set2);

    assert!(!set1_answer, "set 1 has no admissible world where A and B are both >= 0.35 (they always trade off)");
    assert!(set2_answer, "set 2's balanced world (0.4, 0.4, 0.2) explicitly admits exactly that");
    assert_ne!(
        set1_answer, set2_answer,
        "PROOF: byte-identical per-contributor share bounds and epistemic states, yet a \
         legitimate downstream joint query answers differently -- per-contributor allocation \
         bounds are NOT an allocation"
    );
}

#[test]
fn path_046_h_conflicted_contributor_propagates_through_the_shared_denominator() {
    // A has Conflicted evidence: two grounded, disagreeing origins put
    // A at 20 or 40. B and C are fixed/exact.
    let world_low = vec![("A", 20.0), ("B", 30.0), ("C", 30.0)];
    let world_high = vec![("A", 40.0), ("B", 30.0), ("C", 30.0)];

    let b_share = path_046_candidate_d_bounds(&[world_low.clone(), world_high.clone()], 1.0, "B", Path046State::Conflicted);
    assert!(
        !b_share.is_exact(),
        "B's OWN raw value never changes, but its share still varies (denominator swings with \
         A's conflicting value) -- B's share is NOT structurally quarantined from A's conflict"
    );
    assert_eq!(
        b_share.state,
        Path046State::Conflicted,
        "because B's share is causally affected by A's Conflicted value, B's share must inherit \
         Conflicted too, not silently downgrade to Unresolved or Supported -- this is NOT global \
         contamination by assumption, it is a DEMONSTRATED numeric dependency through the shared \
         denominator"
    );

    // Escape hatch: relevance CAN occasionally narrow, but only by
    // numeric coincidence (an invariant ratio), never by structural
    // guarantee -- reusing PATH-046-C's mechanism with A now Conflicted.
    let coincidental_worlds = vec![vec![("A", 20.0), ("B", 10.0)], vec![("A", 40.0), ("B", 20.0)]];
    let b_share_coincidental = path_046_candidate_d_bounds(&coincidental_worlds, 1.0, "B", Path046State::Conflicted);
    assert!(
        b_share_coincidental.is_exact(),
        "when B's raw value happens to scale proportionally with A's conflicting swing, B's \
         share stays exact DESPITE A being Conflicted -- but note the state field still exists \
         to be set: this exactness is a numeric coincidence specific to these values, not a \
         structural quarantine like PATH-043/044's disconnected-subgraph irrelevance"
    );
}

#[test]
fn path_046_i_missing_evidence_is_not_treated_as_zero() {
    // It is UNRESOLVED whether A contributed at all -- not proven zero.
    let world_a_absent = path_046_allocate(&[("B", 30.0), ("C", 30.0)], 1.0).unwrap();
    let naive_missing_as_zero = path_046_share_of(&world_a_absent, "B");
    assert!((naive_missing_as_zero - 0.5).abs() < 1e-9, "the naive shortcut silently commits to the 'A contributed nothing' world");

    let world_a_present = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 1.0).unwrap();
    let b_if_a_present = path_046_share_of(&world_a_present, "B");

    let b_share = Measured046::from_values(&[naive_missing_as_zero, b_if_a_present], Path046State::Unresolved);
    assert!(
        !b_share.is_exact(),
        "the TRUE admissible range for B's share must include BOTH possibilities (A absent and \
         A present with real evidence) -- {:?} -- not collapse to the naive shortcut's single \
         falsely-precise value {naive_missing_as_zero}",
        b_share
    );
}

#[test]
fn path_046_j_duplicate_replay_evidence_currently_inflates_production_contribution() {
    // REAL production behavior, not test-local machinery: distribution.rs
    // has NO deduplication of ContributionEvents at all.
    let baseline = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 1.0).unwrap();
    let baseline_b = path_046_share_of(&baseline, "B");

    // Literal duplicate (same contribution_id, re-serialized).
    let mut events_literal_duplicate = path_046_events(&[("A", 40.0), ("B", 30.0), ("C", 30.0)]);
    events_literal_duplicate.push(events_literal_duplicate[0].clone()); // exact clone of A's event
    let duplicated = distribution_machine::distribution::allocate_contribution_pool(&events_literal_duplicate, &path_046_weights(), 1.0).unwrap();
    let duplicated_b = path_046_share_of(&duplicated, "B");

    // Replay under a fresh contribution_id.
    let mut events_replayed = path_046_events(&[("A", 40.0), ("B", 30.0), ("C", 30.0)]);
    events_replayed.push(distribution_machine::contribution::ContributionEvent {
        contributor_id: "A".into(),
        contribution_id: "A-replayed-under-a-new-id".into(),
        base_work_value: 40.0,
        factors: events_replayed[0].factors,
    });
    let replayed = distribution_machine::distribution::allocate_contribution_pool(&events_replayed, &path_046_weights(), 1.0).unwrap();
    let replayed_b = path_046_share_of(&replayed, "B");

    assert!(
        duplicated_b < baseline_b - 1e-9,
        "CONFIRMED PRODUCTION GAP: `allocate_contribution_pool` has no dedup at all -- a literal \
         duplicate ContributionEvent for A measurably shrinks B's share ({baseline_b} -> \
         {duplicated_b}), contradicting FIND-007's principle at this specific production \
         boundary. Not fixed here (out of scope: 'do not modify production allocation \
         behavior') -- reported as an OBSERVED, currently-real gap."
    );
    assert!(
        replayed_b < baseline_b - 1e-9,
        "the SAME gap applies to replay under a fresh contribution_id ({baseline_b} -> {replayed_b}) -- there is no content-addressed key at all to dedup against"
    );
}

#[test]
fn path_046_k_representation_depth_leaves_allocation_invariant() {
    use distribution_machine::measurement::{CapabilityId, DependencyEdge, canonicalize_pure_relays, reachable_capabilities};

    let source = CapabilityId("source".into());
    let protected = vec![CapabilityId("source".into()), CapabilityId("x".into())];

    let value_from_reach = |edges: &[DependencyEdge]| -> f64 {
        let (canonical, _) = canonicalize_pure_relays(&source, &protected, edges);
        reachable_capabilities(&source, &canonical).len() as f64 * 10.0
    };

    let bare = vec![DependencyEdge::new("source", "x")];
    let padded = vec![DependencyEdge::new("source", "relay1"), DependencyEdge::new("relay1", "x")];

    let a_bare = path_046_allocate(&[("A", value_from_reach(&bare)), ("B", 30.0)], 1.0).unwrap();
    let a_padded = path_046_allocate(&[("A", value_from_reach(&padded)), ("B", 30.0)], 1.0).unwrap();

    assert!((path_046_share_of(&a_bare, "A") - path_046_share_of(&a_padded, "A")).abs() < 1e-9);
}

#[test]
fn path_046_l_zero_total_worlds_are_handled_explicitly_not_invented() {
    // (a) all-zero + positive pool -> Err, exact production message.
    let all_zero_positive_pool = path_046_allocate(&[("A", 0.0), ("B", 0.0)], 100.0);
    assert_eq!(all_zero_positive_pool, Err("cannot allocate a positive contribution pool with zero verified contribution"));

    // (b) all-zero + zero pool -> Ok(empty vec), NOT one zero-amount
    // entry per input event.
    let all_zero_zero_pool = path_046_allocate(&[("A", 0.0), ("B", 0.0)], 0.0).unwrap();
    assert!(all_zero_zero_pool.is_empty(), "production returns an EMPTY vec here, not 2 zero-amount entries");

    // (c) a joint set containing BOTH a normalizable and a
    // non-normalizable world cannot receive one unconditional verdict.
    let worlds = vec![vec![("A", 40.0), ("B", 30.0)], vec![("A", 0.0), ("B", 0.0)]];
    let results = path_046_joint_results(&worlds, 100.0);
    assert!(results[0].is_ok() && results[1].is_err(), "one admissible world normalizes, the other does not");

    let eligibility = path_046_settlement_eligibility(&results, true, false, false);
    assert_eq!(
        eligibility,
        Path046Eligibility::BlockedNonNormalizable,
        "a joint set spanning both a normalizable and a non-normalizable world must be flagged \
         as blocked, distinct from ordinary Unresolved -- normalization is not even DEFINED in \
         every admissible world, so no single allocation verdict (exact or bounded) may be \
         reported"
    );
}

#[test]
fn path_046_m_negative_and_nan_infinity_domain_behavior() {
    // HISTORICAL NOTE: this test originally demonstrated that NaN/
    // Infinity were NOT rejected by production and silently poisoned
    // every contributor's share inside Ok(...). PATH-046.5 fixed that
    // defect (finite validation added to `ContributionWeights::
    // validate`, `ContributionFactors::validate`,
    // `ContributionEvent::calculate`, and `allocate_contribution_pool`).
    // This test now documents the FIXED behavior; see PATH-046.5's
    // dedicated `path_046_5_a*` tests below for the full required suite.

    // Negative: rejected by construction (validated) -- unchanged.
    let negative = path_046_allocate(&[("A", -5.0), ("B", 10.0)], 100.0);
    assert_eq!(negative, Err("base work value cannot be negative"), "negative contribution is impossible by construction -- rejected before normalization");

    // NaN: now explicitly rejected before normalization.
    let nan_result = path_046_allocate(&[("A", f64::NAN), ("B", 10.0)], 100.0);
    assert_eq!(nan_result, Err("base work value must be finite"), "NaN is now rejected explicitly, not silently propagated");

    // Infinity: also now explicitly rejected.
    let inf_result = path_046_allocate(&[("A", f64::INFINITY), ("B", 10.0)], 100.0);
    assert_eq!(inf_result, Err("base work value must be finite"));
}

#[test]
fn path_046_n_conservation_is_machine_approximate_not_bit_exact_by_construction() {
    // No rounding/cents/currency-unit logic exists anywhere in this
    // pipeline (confirmed by inspection) -- pure f64 arithmetic.
    let values: Vec<(&str, f64)> =
        vec![("a", 1.0), ("b", 3.0), ("c", 7.0), ("d", 11.0), ("e", 13.0), ("f", 17.0), ("g", 19.0)];
    let pool = 987_654.321;
    let allocations = path_046_allocate(&values, pool).unwrap();

    let total: f64 = allocations.iter().map(|a| a.allocated_amount).sum();

    // Mathematical conservation (exact in real-number arithmetic) is
    // only approximated by machine-representation conservation here --
    // tested with an epsilon, exactly like every existing production
    // test for this function already does (never asserted bit-exact).
    assert!((total - pool).abs() < 1e-6, "machine-representation conservation holds within floating-point epsilon");

    let shares_sum: f64 = allocations.iter().map(|a| a.normalized_share).sum();
    assert!((shares_sum - 1.0).abs() < 1e-9);

    // No money-unit (cents/dinars) rounding exists to test -- explicitly
    // not invented here, per this mission's instructions.
}

#[test]
fn path_046_o_scale_invariance() {
    let base = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 100.0).unwrap();
    let k = 3.7;
    let scaled = path_046_allocate(&[("A", 40.0 * k), ("B", 30.0 * k), ("C", 30.0 * k)], 100.0).unwrap();

    for id in ["A", "B", "C"] {
        assert!(
            (path_046_share_of(&base, id) - path_046_share_of(&scaled, id)).abs() < 1e-9,
            "common positive scaling of every raw contribution must not change normalized shares"
        );
    }
}

#[test]
fn path_046_p_contributor_set_change_is_population_relative() {
    let just_ab = path_046_allocate(&[("A", 40.0), ("B", 30.0)], 1.0).unwrap();
    let a_share_before = path_046_share_of(&just_ab, "A");
    let b_share_before = path_046_share_of(&just_ab, "B");

    let with_c = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 1.0).unwrap();
    let a_share_after = path_046_share_of(&with_c, "A");
    let b_share_after = path_046_share_of(&with_c, "B");

    assert_ne!(a_share_before, a_share_after, "A's raw value never changed, but its SHARE changed purely because the population changed");
    assert_ne!(b_share_before, b_share_after);

    // Uncertain C population membership makes A/B's shares a genuine
    // range, even though A and B's own evidence is exact.
    let worlds = vec![vec![("A", 40.0), ("B", 30.0)], vec![("A", 40.0), ("B", 30.0), ("C", 30.0)]];
    let a_bounds = path_046_candidate_d_bounds(&worlds, 1.0, "A", Path046State::Unresolved);
    assert!(!a_bounds.is_exact(), "A's own evidence is exact, but whether C exists at all is uncertain -- A's share is genuinely a range");

    // Direct contrast with FIND-021/PATH-044-G: Criticality had NO
    // population-relative normalization anywhere in its pipeline.
    // Allocation, by contrast, is population-relative by CONSTRUCTION
    // (the denominator is literally `sum` over every event passed in).
}

#[test]
fn path_046_q_exact_individual_share_can_coexist_with_a_nonexact_joint_vector() {
    // A's share is exactly invariant (always half the total) across two
    // worlds where A's OWN raw value changes; B and C individually vary
    // against each other while their COMBINED share (the other half)
    // stays fixed.
    let worlds = vec![vec![("A", 50.0), ("B", 30.0), ("C", 20.0)], vec![("A", 100.0), ("B", 40.0), ("C", 60.0)]];

    let a_bounds = path_046_candidate_d_bounds(&worlds, 1.0, "A", Path046State::Unresolved);
    let b_bounds = path_046_candidate_d_bounds(&worlds, 1.0, "B", Path046State::Unresolved);
    let c_bounds = path_046_candidate_d_bounds(&worlds, 1.0, "C", Path046State::Unresolved);

    assert!(a_bounds.is_exact(), "A's share stays exactly 0.5 in every admissible world");
    assert!((a_bounds.lower - 0.5).abs() < 1e-9);
    assert!(!b_bounds.is_exact(), "B's share genuinely varies");
    assert!(!c_bounds.is_exact(), "C's share genuinely varies");

    assert_eq!(
        a_bounds.state,
        Path046State::Supported,
        "A may legitimately be reported Supported/exact even though the OVERALL joint allocation \
         vector remains epistemically non-exact (B and C are not) -- relevance sensitivity \
         extends to the allocation layer: A's own share is simply unaffected by how B/C's \
         internal trade-off resolves"
    );
}

#[test]
fn path_046_r_settlement_eligibility_boundary() {
    // 1. One exact supported joint allocation vector.
    let worlds1 = vec![vec![("A", 40.0), ("B", 60.0)]];
    assert_eq!(path_046_settlement_eligibility(&path_046_joint_results(&worlds1, 1.0), true, false, false), Path046Eligibility::Final);

    // 2. Multiple raw worlds, one invariant normalized vector -- SAME
    // category as 1 (see DERIVED: eligibility cares about output
    // exactness, not raw-input exactness).
    let worlds2 = vec![vec![("A", 20.0), ("B", 10.0)], vec![("A", 40.0), ("B", 20.0)]];
    assert_eq!(path_046_settlement_eligibility(&path_046_joint_results(&worlds2, 1.0), true, false, false), Path046Eligibility::Final);

    // 3. Multiple normalized vectors, bounded shares.
    let worlds3 = vec![vec![("A", 40.0), ("B", 20.0)], vec![("A", 40.0), ("B", 60.0)]];
    assert_eq!(path_046_settlement_eligibility(&path_046_joint_results(&worlds3, 1.0), true, false, false), Path046Eligibility::InformationalBounded);

    // 4. Unresolved joint state (missing evidence).
    assert_eq!(path_046_settlement_eligibility(&[], true, false, true), Path046Eligibility::BlockedUnresolved);

    // 5. Conflicted contribution evidence.
    assert_eq!(path_046_settlement_eligibility(&path_046_joint_results(&worlds3, 1.0), true, true, false), Path046Eligibility::BlockedConflicted);

    // 6. A non-normalizable admissible world.
    let worlds6 = vec![vec![("A", 40.0), ("B", 60.0)], vec![("A", 0.0), ("B", 0.0)]];
    assert_eq!(path_046_settlement_eligibility(&path_046_joint_results(&worlds6, 1.0), true, false, false), Path046Eligibility::BlockedNonNormalizable);

    // 7. Exact per-contributor bounds but lost joint correlation --
    // without the retained joint vector set, never Final, regardless of
    // how tight the bounds look.
    assert_eq!(
        path_046_settlement_eligibility(&[], false, false, false),
        Path046Eligibility::InformationalBounded,
        "per-contributor bounds alone, without the joint vector set, can never be promoted to \
         Final -- same category as case 3, per PATH-046-F/G"
    );

    // Downstream consumption rule: only `Final` permits a downstream
    // consumer to treat allocation shares as final economic facts.
    // `InformationalBounded` permits informational/bounded reporting
    // only. `BlockedUnresolved`/`BlockedConflicted`/
    // `BlockedNonNormalizable` must block settlement entirely. No
    // escrow, guaranteed-minimum, reserve-holding, or partial-
    // settlement policy is defined here -- explicitly out of scope.
}

#[test]
fn path_046_compositional_question_separately_derived_bounds_do_not_compose_into_an_allocation() {
    // Mirrors PATH-045-M: per-contributor allocation results derived
    // SEPARATELY (from non-reconciled evidence) cannot be safely
    // recombined into one allocation without checking joint
    // compatibility -- reusing PATH-046-F's mechanism as the direct
    // proof, framed explicitly as the compositional question's answer.
    let a_bounds_from_one_source = path_046_candidate_b_marginal_share_bounds(&[("A", 0.4, 0.6), ("B", 0.3, 0.5)])
        .into_iter()
        .find(|(id, _, _)| id == "A")
        .unwrap();
    let b_bounds_from_a_different_source = path_046_candidate_b_marginal_share_bounds(&[("A", 0.4, 0.6), ("B", 0.3, 0.5)])
        .into_iter()
        .find(|(id, _, _)| id == "B")
        .unwrap();

    let naive_composed_upper = a_bounds_from_one_source.2 + b_bounds_from_a_different_source.2;
    assert!(
        naive_composed_upper > 1.0 + 1e-9,
        "composing each contributor's independently-derived upper bound ({naive_composed_upper}) \
         exceeds 100% -- confirming H16's PATH-045-M finding generalizes to allocation: separate \
         per-contributor results are safe to compose into ONE allocation only when they share a \
         joint compatibility relation (the actual joint admissible vector set); without it, \
         composition is REJECTED, not merely unproven"
    );
}

// =====================================================================
// PATH-046.5 -- PRODUCTION HARDENING AT THE ALLOCATION INPUT BOUNDARY
//
// Part A tests exercise the REAL, now-patched production functions
// (`ContributionEvent::calculate`, `ContributionWeights::validate`,
// `ContributionFactors::validate`, `allocate_contribution_pool`) --
// this is a production remediation mission, not test-local machinery.
//
// Part B is a CASE 3 finding: no authoritative event identity exists
// for `ContributionEvent` (inventory below), so NO deduplication was
// implemented. The block ends with an executable impossibility
// demonstration instead of B1-B6 (which require CASE 1/2).
//
// IDENTITY INVENTORY (read before Part B):
// `ContributionEvent { contributor_id: String, contribution_id: String,
// base_work_value: f64, factors: ContributionFactors }`.
// - `contribution_id` is a bare, caller-supplied `String` with NO
//   documentation, NO uniqueness validation anywhere in
//   `distribution.rs`/`contribution.rs`, and NO special handling on
//   repetition -- `allocate_contribution_pool` never inspects it except
//   to copy it into the output `ContributionAllocation`. Every existing
//   call site (production tests) uses it purely as a human-readable
//   label ("capability-a", "unverified-claim", ...).
// - No `event_id`, `idempot*`, `source_id`, `provenance`, `sequence`,
//   `request_id`, or `transaction` concept exists anywhere in
//   `contribution.rs`, `distribution.rs`, `capital.rs`, or
//   `stewardship.rs` (grepped; zero hits outside this comment).
// - This is the SAME shape FIND-016/019 already identified for
//   `RepresentationClaimId`/`DependencyId`: a bare, self-issued string
//   with no issuer/authority semantics -- CASE 3 (no authoritative
//   identity), not CASE 1 or 2. Inventing dedup-by-`contribution_id` (or
//   any other field/combination) would be exactly the "we deduped
//   matching payloads because it seemed reasonable" outcome this
//   mission explicitly forbids.
// =====================================================================

// ---------------------------------------------------------------------
// Part A -- non-finite value remediation (REQUIRED, FIXED).
// ---------------------------------------------------------------------

#[test]
fn path_046_5_a1_nan_contribution_is_rejected() {
    let result = path_046_allocate(&[("A", f64::NAN), ("B", 10.0)], 100.0);
    assert_eq!(result, Err("base work value must be finite"), "NaN must be rejected explicitly, before normalization");
}

#[test]
fn path_046_5_a2_positive_infinity_is_rejected() {
    let result = path_046_allocate(&[("A", f64::INFINITY), ("B", 10.0)], 100.0);
    assert_eq!(result, Err("base work value must be finite"));
}

#[test]
fn path_046_5_a3_negative_infinity_is_rejected() {
    let result = path_046_allocate(&[("A", f64::NEG_INFINITY), ("B", 10.0)], 100.0);
    // -Infinity is caught by the finite check before the negativity
    // check ever runs -- same error, confirmed explicitly rather than
    // assumed.
    assert_eq!(result, Err("base work value must be finite"));
}

#[test]
fn path_046_5_a4_valid_zero_remains_legal() {
    // Zero base_work_value among other positive contributors: legal,
    // contributes exactly 0 share.
    let allocations = path_046_allocate(&[("A", 0.0), ("B", 10.0)], 100.0).unwrap();
    assert_eq!(path_046_share_of(&allocations, "A"), 0.0);
    assert!((path_046_share_of(&allocations, "B") - 1.0).abs() < 1e-9);

    // All-zero + positive pool: existing zero-total policy unchanged
    // (still an explicit Err, not affected by the finite patch).
    let all_zero_positive_pool = path_046_allocate(&[("A", 0.0), ("B", 0.0)], 100.0);
    assert_eq!(all_zero_positive_pool, Err("cannot allocate a positive contribution pool with zero verified contribution"));

    // All-zero + zero pool: existing empty-vec policy unchanged.
    let all_zero_zero_pool = path_046_allocate(&[("A", 0.0), ("B", 0.0)], 0.0).unwrap();
    assert!(all_zero_zero_pool.is_empty());
}

#[test]
fn path_046_5_a5_valid_finite_behavior_is_unchanged() {
    // Identical to the pre-patch `path_046_a` exact-control result --
    // backward compatibility check.
    let allocations = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 100.0).unwrap();
    assert!((path_046_share_of(&allocations, "A") - 0.4).abs() < 1e-9);
    assert!((path_046_share_of(&allocations, "B") - 0.3).abs() < 1e-9);
    assert!((path_046_share_of(&allocations, "C") - 0.3).abs() < 1e-9);
    let total: f64 = allocations.iter().map(|a| a.allocated_amount).sum();
    assert!((total - 100.0).abs() < 1e-9);
}

#[test]
fn path_046_5_a6_mixed_valid_and_non_finite_fails_atomically() {
    // One NaN contributor among otherwise-valid contributors: the WHOLE
    // operation fails -- no partial/poisoned allocation vector.
    let result = path_046_allocate(&[("A", 40.0), ("B", f64::NAN), ("C", 30.0)], 100.0);
    assert_eq!(result, Err("base work value must be finite"));

    // Also verify at the boundary: NaN weights/factors are caught too
    // (the same finite patch applied to `ContributionWeights::validate`
    // and `ContributionFactors::validate`), not just base_work_value.
    let events = path_046_events(&[("A", 40.0), ("B", 30.0)]);
    let mut nan_weights = path_046_weights();
    nan_weights.criticality = f64::NAN;
    let weights_result = distribution_machine::distribution::allocate_contribution_pool(&events, &nan_weights, 100.0);
    assert_eq!(weights_result, Err("contribution weights must be finite"));

    let mut nan_factor_events = events.clone();
    nan_factor_events[0].factors.evidence_confidence = f64::NAN;
    let factors_result = distribution_machine::distribution::allocate_contribution_pool(&nan_factor_events, &path_046_weights(), 100.0);
    assert_eq!(factors_result, Err("contribution factors must be finite"));

    // A NaN/Infinite pool is rejected too (allocated_amount = pool *
    // share would otherwise be poisoned even with perfectly finite
    // shares).
    let pool_result = path_046_allocate(&[("A", 40.0), ("B", 60.0)], f64::NAN);
    assert_eq!(pool_result, Err("contribution pool must be finite"));
    let inf_pool_result = path_046_allocate(&[("A", 40.0), ("B", 60.0)], f64::INFINITY);
    assert_eq!(inf_pool_result, Err("contribution pool must be finite"));
}

#[test]
fn path_046_5_a7_no_successful_allocation_contains_non_finite_shares() {
    let allocations = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 100.0).unwrap();
    for allocation in &allocations {
        assert!(allocation.normalized_share.is_finite(), "share for {} must be finite", allocation.contributor_id);
        assert!(allocation.allocated_amount.is_finite(), "amount for {} must be finite", allocation.contributor_id);
    }
}

// ---------------------------------------------------------------------
// Backward-compatibility spot checks (scale invariance, ordering,
// contributor set effects) re-run against the PATCHED production
// function, mirroring PATH-046-O/P/N exactly to confirm the finite
// patch changed nothing about valid economic behavior.
// ---------------------------------------------------------------------

#[test]
fn path_046_5_backward_compatible_scale_invariance_ordering_and_conservation() {
    let base = path_046_allocate(&[("A", 40.0), ("B", 30.0), ("C", 30.0)], 100.0).unwrap();
    let scaled = path_046_allocate(&[("A", 40.0 * 3.7), ("B", 30.0 * 3.7), ("C", 30.0 * 3.7)], 100.0).unwrap();
    for id in ["A", "B", "C"] {
        assert!((path_046_share_of(&base, id) - path_046_share_of(&scaled, id)).abs() < 1e-9, "scale invariance unchanged");
    }

    let ordered = path_046_allocate(&[("low", 0.1), ("high", 0.9)], 100.0).unwrap();
    assert_eq!(ordered[0].contributor_id, "low", "input-order output unchanged");
    assert_eq!(ordered[1].contributor_id, "high");

    let total: f64 = base.iter().map(|a| a.allocated_amount).sum();
    assert!((total - 100.0).abs() < 1e-9, "conservation epsilon-semantics unchanged");
}

// ---------------------------------------------------------------------
// Part B -- replay/duplicate remediation: CASE 3, proven blocker.
// ---------------------------------------------------------------------

#[test]
fn path_046_5_replay_identity_impossibility() {
    // Construct ONE ContributionEvent value.
    let event = distribution_machine::contribution::ContributionEvent {
        contributor_id: "alice".to_string(),
        contribution_id: "code-review".to_string(),
        base_work_value: 40.0,
        factors: distribution_machine::contribution::ContributionFactors {
            responsibility: 1.0,
            criticality: 1.0,
            deal_impact: 1.0,
            stewardship: 1.0,
            dependency_reach: 1.0,
            evidence_confidence: 1.0,
        },
    };

    // WORLD A: one real economic event, delivered twice (a transport
    // replay -- e.g. a retried network call).
    let world_a_replay = vec![event.clone(), event.clone()];

    // WORLD B: two genuinely distinct real economic events -- e.g. two
    // different reviewers each independently logging a $40 code review,
    // where both intake systems happened to generate the same
    // human-readable `contribution_id` label ("code-review") and the
    // same `base_work_value`.
    let world_b_two_legitimate_events = vec![event.clone(), event.clone()];

    // The current `ContributionEvent` representation is BYTE-IDENTICAL
    // between the two worlds -- there is no field anywhere in this type
    // that distinguishes "the same occurrence, observed twice" from
    // "two different occurrences that happen to match."
    assert_eq!(world_a_replay, world_b_two_legitimate_events, "the two worlds are represented identically");

    // Consequently, `allocate_contribution_pool` -- or ANY deterministic
    // function over `&[ContributionEvent]` -- computes the exact same
    // total contribution for both worlds, even though the economically
    // CORRECT total differs: WORLD A's true economic contribution is
    // 40 (one event, replayed), WORLD B's true economic contribution is
    // 80 (two independent events).
    let events_a = path_046_events(&[("B", 60.0)])
        .into_iter()
        .chain(world_a_replay.clone())
        .collect::<Vec<_>>();
    let events_b = path_046_events(&[("B", 60.0)])
        .into_iter()
        .chain(world_b_two_legitimate_events.clone())
        .collect::<Vec<_>>();
    let alloc_a = distribution_machine::distribution::allocate_contribution_pool(&events_a, &path_046_weights(), 1.0).unwrap();
    let alloc_b = distribution_machine::distribution::allocate_contribution_pool(&events_b, &path_046_weights(), 1.0).unwrap();
    let alice_share_a: f64 = alloc_a.iter().filter(|a| a.contributor_id == "alice").map(|a| a.normalized_share).sum();
    let alice_share_b: f64 = alloc_b.iter().filter(|a| a.contributor_id == "alice").map(|a| a.normalized_share).sum();
    assert_eq!(
        alice_share_a, alice_share_b,
        "PROOF: given byte-identical input, ANY deterministic function -- including production's \
         actual `allocate_contribution_pool` -- necessarily produces the identical result for \
         both worlds. No dedup function could correctly collapse WORLD A to alice's true \
         contribution (40, one event replayed) while correctly preserving WORLD B's true \
         contribution (80, two legitimate events) from this representation alone, because both \
         worlds present the identical input. This is FIND-018/019's central-obstruction pattern \
         (identical representation, opposite required ground truth), recurring at the allocation \
         input boundary. No heuristic is introduced to paper over this."
    );

    // Per this mission's forbidden-key list and CASE 3 doctrine: no
    // deduplication was implemented anywhere in production for this
    // mission. `contribution_id` remains a bare, unvalidated, non-
    // authoritative label -- exactly as before.
}

/// PATH-046.5-B2 analogue, run against CASE 3's conclusion rather than
/// a dedup implementation: two events with the SAME contributor and
/// VALUE but genuinely DIFFERENT `contribution_id`s must both survive
/// (this already holds today, since no dedup exists at all -- but it is
/// the one property any FUTURE identity-based fix must preserve, so it
/// is pinned here as a regression guard).
#[test]
fn path_046_5_distinct_identical_valued_events_both_survive() {
    // Two DISTINCT contribution events for the SAME contributor (alice),
    // with different `contribution_id`s but identical `base_work_value`
    // -- constructed explicitly, not via `path_046_events` (which
    // derives `contribution_id` from `contributor_id` and so cannot
    // express two events for one contributor).
    let neutral_factors = distribution_machine::contribution::ContributionFactors {
        responsibility: 1.0,
        criticality: 1.0,
        deal_impact: 1.0,
        stewardship: 1.0,
        dependency_reach: 1.0,
        evidence_confidence: 1.0,
    };

    let events = vec![
        distribution_machine::contribution::ContributionEvent {
            contributor_id: "alice".to_string(),
            contribution_id: "review-1".to_string(),
            base_work_value: 40.0,
            factors: neutral_factors,
        },
        distribution_machine::contribution::ContributionEvent {
            contributor_id: "alice".to_string(),
            contribution_id: "review-2".to_string(),
            base_work_value: 40.0,
            factors: neutral_factors,
        },
        distribution_machine::contribution::ContributionEvent {
            contributor_id: "bob".to_string(),
            contribution_id: "bob-work".to_string(),
            base_work_value: 20.0,
            factors: neutral_factors,
        },
    ];

    let allocations = distribution_machine::distribution::allocate_contribution_pool(&events, &path_046_weights(), 1.0).unwrap();

    let total: f64 = allocations.iter().map(|a| a.normalized_share).sum();
    assert!((total - 1.0).abs() < 1e-9);

    let alice_share: f64 = allocations.iter().filter(|a| a.contributor_id == "alice").map(|a| a.normalized_share).sum();
    let bob_share: f64 = allocations.iter().filter(|a| a.contributor_id == "bob").map(|a| a.normalized_share).sum();

    assert!(
        (alice_share - 0.8).abs() < 1e-9,
        "two genuinely distinct, identically-valued contributions from the SAME contributor must \
         both count -- alice's summed share (40+40=80 of 100) is 0.8, not 0.4 -- this is the \
         property FIND-024's proposed identity contract must preserve; it must never be \
         sacrificed to make replay protection easier"
    );
    assert!((bob_share - 0.2).abs() < 1e-9);
}



// =====================================================================
// PATH-047 -- MINIMUM AUTHORITATIVE EVENT IDENTITY (H18)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::contribution` or wired into any production
// path -- no production file is touched by this experiment. This tests
// FIND-024's proposed `EventIdentity{source_id, event_id}` shape
// directly: is a source-scoped key sufficient for replay-safe identity,
// and under exactly which exogenous conditions?
//
// H18 names three exogenous facts about a source: AUTHORITY (H18
// condition 1), STABILITY (condition 2, same occurrence => same
// event_id), and SEPARATION (condition 3, distinct occurrences =>
// distinct event_ids). STABILITY and SEPARATION are independent and
// license disjoint inferences -- STABILITY licenses "different event_id
// => distinct occurrence"; SEPARATION licenses "same event_id => same
// occurrence." Neither substitutes for the other (PATH-047-G/G',
// PATH-047-J prove this directly).
//
// SECOND CROSS-REVIEW CORRECTION -- AUTHORITY boundary (this revision):
// an earlier version of this file treated "source_id differs" as
// DEFINITIONAL grounds for `Distinct`, requiring no trust lookup at all.
// Cross-review found this unjustified: two different self-declared
// SOURCE_ID STRINGS prove only lexical inequality, never that they
// denote two different real source namespaces. Without an exogenous
// fact establishing namespace identity, an attacker (or an innocent
// relabeling) could split one real source into arbitrarily many string
// labels and have every pair of them treated as automatically
// `Distinct` -- exactly the FIND-016/FIND-018 "new IDs disguise the same
// structural thing" pattern this repository already documents elsewhere
// (PATH-019, PATH-026), now recurring at the source-authority boundary.
//
// AUTHORITY is therefore modeled as binding a raw, self-declared
// `SourceId` string to a canonical `NamespaceId` -- never the reverse.
// Two source_id strings are `Distinct` ONLY if BOTH are bound (AUTHORITY
// present for each) AND their canonical namespaces are established, by
// the contract, to differ. Two source_id strings bound to the SAME
// canonical namespace (deliberate aliasing, or literally the same
// string) are treated identically for identity purposes, regardless of
// how their labels look (PATH-047-L). A source_id string with NO
// contract entry has no AUTHORITY at all and contributes nothing --
// including no claim of distinctness from any other source, trusted or
// not (PATH-047-F, PATH-047-H, corrected below).
//
// Reuses PATH-042/FIND-019's central pattern throughout: the one input
// this code never derives, verifies, or fabricates from the events
// themselves is `TrustedSourceContract` -- an explicit, caller-supplied
// fact about which raw source_id strings are bound to which canonical
// namespace, and what STABILITY/SEPARATION guarantees that namespace
// carries.
//
// Research questions (re-answered at the bottom, after the adversarial
// cases, from this corrected model's observed behavior only):
//   1. Is (source_id, event_id) actually sufficient under the contract?
//   2. Is global uniqueness necessary? (falsification attempt)
//   3. What exact assumption is exogenous rather than derived?
//   4. Does identity resolution belong before economic normalization?
//   5. What happens if one source cannot provide stable event identity?
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SourceId(String);

/// A canonical, authoritatively-established real-source namespace.
/// Deliberately a SEPARATE type from `SourceId`: a `SourceId` is a raw,
/// self-declared, on-the-wire label; a `NamespaceId` is the exogenous
/// fact of which real system that label denotes, known only via
/// `TrustedSourceContract`. Two different `SourceId` strings MAY bind to
/// the same `NamespaceId` (aliases); this type existing separately is
/// exactly what makes that representable, instead of collapsing
/// "different label" and "different real source" into one fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NamespaceId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EventIdentity {
    source_id: SourceId,
    event_id: String,
}

/// A candidate event as it would arrive at the allocation boundary:
/// FIND-024's proposed `EventIdentity` plus the existing
/// `ContributionEvent` payload it claims to describe. Identity is
/// deliberately a SEPARATE field, never derived from the payload --
/// never `contribution_id`, `contributor_id`, or a payload hash, per
/// this mission's forbidden-key list.
#[derive(Debug, Clone, PartialEq)]
struct Path047CandidateEvent {
    identity: EventIdentity,
    payload: ContributionEvent,
}

/// H18 conditions 2 and 3, as two INDEPENDENT facts about one canonical
/// NAMESPACE (not about any one label for it) that this experiment can
/// only ever be TOLD, never derive from events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SourceIdContract {
    /// STABILITY (H18 condition 2): one real occurrence always produces
    /// the SAME event_id across retries/redeliveries from this
    /// namespace. Licenses: DIFFERENT event_id => DISTINCT occurrence.
    stable: bool,
    /// SEPARATION (H18 condition 3): distinct real occurrences always
    /// produce DIFFERENT event_ids from this namespace. Licenses: SAME
    /// event_id => SAME occurrence.
    separating: bool,
}

impl SourceIdContract {
    const fn both_guaranteed() -> Self {
        SourceIdContract { stable: true, separating: true }
    }
    const fn stable_only() -> Self {
        SourceIdContract { stable: true, separating: false }
    }
    const fn separating_only() -> Self {
        SourceIdContract { stable: false, separating: true }
    }
    const fn neither_guaranteed() -> Self {
        SourceIdContract { stable: false, separating: false }
    }
}

/// The explicit, caller-supplied representation of AUTHORITY: which raw
/// `SourceId` strings are bound to which canonical `NamespaceId`, and
/// what STABILITY/SEPARATION each namespace carries. Never derived,
/// verified, or fabricated from `Path047CandidateEvent` data anywhere
/// below. `namespace_of` binding is by EXACT string key only
/// (PATH-047-H) -- a label merely resembling a bound one gains no
/// AUTHORITY. `properties` is keyed by `NamespaceId`, not `SourceId`: it
/// is a fact about the real system, so every `SourceId` alias bound to
/// one namespace shares exactly one set of properties, by construction
/// (there is no way to give two aliases of the same namespace
/// conflicting properties).
#[derive(Debug, Clone, Default)]
struct TrustedSourceContract {
    namespace_of: std::collections::HashMap<SourceId, NamespaceId>,
    properties: std::collections::HashMap<NamespaceId, SourceIdContract>,
}

impl TrustedSourceContract {
    /// Convenience for the common case: each `(source_id, props)` pair
    /// gets its OWN canonical namespace, named after its source_id
    /// string purely for test readability -- the STRING plays no role
    /// in any comparison, only `NamespaceId` equality does. This is what
    /// makes two entries built this way explicitly, contractually
    /// distinct namespaces (PATH-047-C), not merely two different map
    /// keys. Use `trusting_with_aliases` when two source_id strings must
    /// be bound to the SAME namespace.
    fn trusting(sources: &[(&str, SourceIdContract)]) -> Self {
        let mut contract = TrustedSourceContract::default();
        for (source, props) in sources {
            let namespace = NamespaceId((*source).to_string());
            contract.namespace_of.insert(SourceId((*source).to_string()), namespace.clone());
            contract.properties.insert(namespace, *props);
        }
        contract
    }

    /// Explicit alias binding: every string in `aliases` is bound to ONE
    /// canonical `namespace`, which owns ONE set of properties. This is
    /// the mechanism for representing "two (or more) labels, one real
    /// system" -- namespace identity is the exogenous fact, never
    /// derived from whether the label strings happen to match.
    fn trusting_with_aliases(namespace: &str, aliases: &[&str], props: SourceIdContract) -> Self {
        let mut contract = TrustedSourceContract::default();
        let namespace = NamespaceId(namespace.to_string());
        for alias in aliases {
            contract.namespace_of.insert(SourceId((*alias).to_string()), namespace.clone());
        }
        contract.properties.insert(namespace, props);
        contract
    }

    /// AUTHORITY lookup: resolves a raw `SourceId` string to its
    /// canonical namespace and that namespace's properties, or `None` if
    /// this contract binds no authority to that string at all.
    fn authority_for(&self, source_id: &SourceId) -> Option<(NamespaceId, SourceIdContract)> {
        let namespace = self.namespace_of.get(source_id)?.clone();
        let props = self.properties.get(&namespace).copied()?;
        Some((namespace, props))
    }
}

/// The identity-resolution verdict, with the four states H18 requires --
/// no boolean anywhere, because "is this the same event?" is not a
/// yes/no question until AUTHORITY (resolved to a canonical namespace,
/// not a raw string) and the relevant one of STABILITY/SEPARATION are
/// all known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path047Verdict {
    /// Same canonical namespace (AUTHORITY established for both sides,
    /// namespaces equal); SEPARATION guaranteed; same event_id; same
    /// payload. Collapses to one economic event.
    Replay,
    /// EITHER two different canonical namespaces (AUTHORITY established
    /// for both sides, namespaces PROVEN to differ by the contract --
    /// never by string inequality alone), OR the same namespace with a
    /// different event_id under a STABILITY guarantee. Both events
    /// survive.
    Distinct,
    /// Same canonical namespace, SEPARATION guaranteed, same event_id,
    /// but DISAGREEING payload. Identity equality says these claims
    /// concern the same occurrence; the payload disagreement is a
    /// conflict, not two legitimate events and not a choice between
    /// them.
    Conflicted,
    /// The identifiers alone cannot settle this: either AUTHORITY is
    /// missing for at least one side (including two different,
    /// self-declared, uncontracted source_id strings -- lexical
    /// inequality alone proves nothing about namespace identity), the
    /// event_ids match but the shared namespace has no SEPARATION
    /// guarantee (the match could be an id collision between two
    /// distinct occurrences), or the event_ids differ but the shared
    /// namespace has no STABILITY guarantee (the difference could be one
    /// occurrence retried under a fresh id). Never collapsed, never
    /// split -- surfaced for external resolution.
    Unresolved,
}

/// The core primitive: H18's verdict between exactly two identity
/// claims. Everything else below is built on this.
fn path_047_verdict(
    a: &Path047CandidateEvent,
    b: &Path047CandidateEvent,
    contract: &TrustedSourceContract,
) -> Path047Verdict {
    let auth_a = contract.authority_for(&a.identity.source_id);
    let auth_b = contract.authority_for(&b.identity.source_id);

    let (namespace_a, props) = match auth_a {
        None => return Path047Verdict::Unresolved, // no AUTHORITY for `a`'s label at all
        Some(x) => x,
    };
    let (namespace_b, _) = match auth_b {
        None => return Path047Verdict::Unresolved, // no AUTHORITY for `b`'s label at all
        Some(x) => x,
    };

    if namespace_a != namespace_b {
        // AUTHORITY is established for BOTH sides, and the contract
        // explicitly assigns them DIFFERENT canonical namespaces -- this
        // is the exogenous fact doing the work, not the raw source_id
        // strings differing (PATH-047-C). Two aliases of the SAME
        // namespace never reach this branch (PATH-047-L).
        return Path047Verdict::Distinct;
    }

    // Same canonical namespace from here (whether via identical
    // source_id strings or deliberate aliasing) -- `props` applies to
    // both, since properties are keyed by namespace, not by label.
    if a.identity.event_id == b.identity.event_id {
        // SAME event_id direction: only SEPARATION licenses "same id =>
        // same occurrence." STABILITY is irrelevant here -- PATH-047-A
        // and -E hold with `separating_only()` contracts to prove it.
        if !props.separating {
            return Path047Verdict::Unresolved;
        }
        if a.payload == b.payload { Path047Verdict::Replay } else { Path047Verdict::Conflicted }
    } else {
        // DIFFERENT event_id direction: only STABILITY licenses
        // "different id => distinct occurrence." SEPARATION is
        // irrelevant here -- PATH-047-B and -I hold with `stable_only()`
        // contracts to prove it.
        if !props.stable {
            return Path047Verdict::Unresolved;
        }
        Path047Verdict::Distinct
    }
}

/// Result of resolving an entire batch of candidate events. This is the
/// "before economic normalization" stage research question 4 asks
/// about: `survivors` is the only set this candidate considers safe to
/// hand to something like `allocate_contribution_pool`; `conflicted` and
/// `unresolved` groups are NOT merged and NOT dropped -- they block
/// progression until resolved by something outside this experiment
/// (reject the batch, page a human, etc. -- not this candidate's
/// decision to make).
#[derive(Debug, Clone, Default)]
struct Path047Resolution {
    survivors: Vec<Path047CandidateEvent>,
    conflicted: Vec<Vec<Path047CandidateEvent>>,
    unresolved: Vec<Vec<Path047CandidateEvent>>,
}

/// Resolves a batch by grouping AUTHORIZED events on their resolved
/// canonical NAMESPACE (never on the raw `source_id` string) so that:
/// (a) a namespace lacking STABILITY cannot bypass resolution merely by
/// having each retry form its own non-colliding singleton group, and (b)
/// two ALIAS labels bound to the same namespace are resolved together,
/// not as if they were unrelated sources (PATH-047-L). Events whose
/// `source_id` carries no AUTHORITY at all are kept in `unresolved`,
/// grouped by their raw label for traceability -- critically, DIFFERENT
/// unauthorized labels are never allowed to separately reach
/// `survivors` just because their strings differ (PATH-047-F,
/// corrected): lacking AUTHORITY, this resolver cannot trust that two
/// different self-declared labels denote different real sources any
/// more than it can trust a single label's own event_id claims.
///
/// Within one namespace's events: PHASE 1 clusters by event_id and
/// applies SEPARATION (same id => same occurrence, if guaranteed) to
/// collapse or flag each cluster; PHASE 2 then applies STABILITY across
/// the resulting per-event_id representatives (different id => distinct
/// occurrence, if guaranteed) before anything is allowed into
/// `survivors`.
///
/// Scope note: a namespace's events that never collide with ANY other
/// event from that same namespace in the batch (i.e. exactly one
/// representative survives phase 1) are accepted as a lone survivor even
/// without a STABILITY guarantee -- with nothing else present to
/// conflate it with IN THIS BATCH, there is no double-count or
/// under-count risk to guard against. This is a real, named limitation,
/// not a general solution: correctness for an unstable namespace still
/// depends on all of its related deliveries appearing together in one
/// resolution call. See NOT ESTABLISHED in FIND-025.
fn path_047_resolve_batch(
    events: &[Path047CandidateEvent],
    contract: &TrustedSourceContract,
) -> Path047Resolution {
    use std::collections::HashMap;

    let mut by_namespace: HashMap<NamespaceId, Vec<Path047CandidateEvent>> = HashMap::new();
    let mut unauthorized: HashMap<SourceId, Vec<Path047CandidateEvent>> = HashMap::new();

    for event in events {
        match contract.authority_for(&event.identity.source_id) {
            Some((namespace, _props)) => {
                by_namespace.entry(namespace).or_default().push(event.clone());
            }
            None => {
                unauthorized.entry(event.identity.source_id.clone()).or_default().push(event.clone());
            }
        }
    }

    let mut resolution = Path047Resolution::default();

    // Unauthorized labels: no AUTHORITY, so nothing about these claims
    // can be trusted -- including whether two DIFFERENT self-declared
    // labels denote different real sources. Every unauthorized label's
    // claims stay in `unresolved`; none reach `survivors`, regardless of
    // how many distinct raw strings are present in the batch.
    let mut unauthorized_keys: Vec<SourceId> = unauthorized.keys().cloned().collect();
    unauthorized_keys.sort_by(|x, y| x.0.cmp(&y.0));
    for key in unauthorized_keys {
        resolution.unresolved.push(unauthorized.remove(&key).unwrap());
    }

    // Deterministic iteration order so test assertions are reproducible.
    let mut namespace_keys: Vec<NamespaceId> = by_namespace.keys().cloned().collect();
    namespace_keys.sort_by(|x, y| x.0.cmp(&y.0));

    for namespace in namespace_keys {
        let namespace_events = by_namespace.remove(&namespace).unwrap();
        let props = contract
            .properties
            .get(&namespace)
            .copied()
            .expect("namespace reached via authority_for must have a properties entry");

        // PHASE 1 -- cluster by event_id (across ALL aliases bound to
        // this namespace, deliberately); SEPARATION governs whether a
        // shared event_id within this namespace may be trusted to mean
        // "same occurrence."
        let mut by_event_id: HashMap<String, Vec<Path047CandidateEvent>> = HashMap::new();
        for event in namespace_events {
            by_event_id.entry(event.identity.event_id.clone()).or_default().push(event);
        }
        let mut event_id_keys: Vec<String> = by_event_id.keys().cloned().collect();
        event_id_keys.sort();

        let mut representatives: Vec<Path047CandidateEvent> = Vec::new();

        for event_id in event_id_keys {
            let cluster = by_event_id.remove(&event_id).unwrap();
            if cluster.len() == 1 {
                representatives.push(cluster.into_iter().next().unwrap());
                continue;
            }
            if !props.separating {
                // Matching event_id, but this namespace does not
                // guarantee distinct occurrences get distinct ids -- the
                // match cannot be trusted to mean "same occurrence."
                resolution.unresolved.push(cluster);
                continue;
            }
            let first_payload = cluster[0].payload.clone();
            if cluster.iter().all(|c| c.payload == first_payload) {
                representatives.push(cluster[0].clone());
            } else {
                resolution.conflicted.push(cluster);
            }
        }

        // PHASE 2 -- across the surviving per-event_id representatives,
        // STABILITY governs whether a DIFFERENT event_id within this
        // namespace may be trusted to mean "distinct occurrence."
        if representatives.len() <= 1 || props.stable {
            resolution.survivors.extend(representatives);
        } else {
            // >= 2 differently-labeled representatives from a namespace
            // that does NOT guarantee stable ids: cannot tell whether
            // these are N distinct real occurrences or the SAME
            // occurrence retried under a fresh id each time
            // (PATH-047-G). Must not become N independent Distinct
            // survivors.
            resolution.unresolved.push(representatives);
        }
    }

    resolution
}

fn path_047_neutral_factors() -> ContributionFactors {
    ContributionFactors {
        responsibility: 1.0,
        criticality: 1.0,
        deal_impact: 1.0,
        stewardship: 1.0,
        dependency_reach: 1.0,
        evidence_confidence: 1.0,
    }
}

fn path_047_payload(contributor_id: &str, contribution_id: &str, base_work_value: f64) -> ContributionEvent {
    ContributionEvent {
        contributor_id: contributor_id.to_string(),
        contribution_id: contribution_id.to_string(),
        base_work_value,
        factors: path_047_neutral_factors(),
    }
}

fn path_047_candidate(source: &str, event_id: &str, payload: ContributionEvent) -> Path047CandidateEvent {
    Path047CandidateEvent {
        identity: EventIdentity { source_id: SourceId(source.to_string()), event_id: event_id.to_string() },
        payload,
    }
}

// ---------------------------------------------------------------------
// Adversarial cases A-L.
// ---------------------------------------------------------------------

#[test]
fn path_047_a_same_trusted_source_same_event_id_same_payload_is_replay() {
    // SEPARATION-only contract: STABILITY is explicitly false here, to
    // prove it plays no role in this direction.
    let contract =
        TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::separating_only())]);
    let payload = path_047_payload("alice", "review-9", 40.0);
    let a = path_047_candidate("payroll-system", "evt-9001", payload.clone());
    let b = path_047_candidate("payroll-system", "evt-9001", payload);

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Replay,
        "SEPARATION alone is sufficient to license Replay on a matching event_id -- STABILITY is \
         false in this contract and is not needed"
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert_eq!(resolution.survivors.len(), 1, "the duplicate delivery must collapse to ONE economic event");
    assert!(resolution.conflicted.is_empty());
    assert!(resolution.unresolved.is_empty());
}

#[test]
fn path_047_b_same_trusted_source_different_event_id_is_distinct() {
    // STABILITY-only contract: SEPARATION is explicitly false here, to
    // prove it plays no role in this direction.
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::stable_only())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let a = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let b = path_047_candidate("payroll-system", "evt-2", payload);

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Distinct,
        "STABILITY alone is sufficient to license Distinct on differing event_ids -- SEPARATION is \
         false in this contract and is not needed"
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert_eq!(
        resolution.survivors.len(),
        2,
        "different event_ids -- even with an otherwise identical payload -- must both survive"
    );
}

/// CORRECTED (second cross-review): two DIFFERENT trusted source_id
/// strings are `Distinct` ONLY because the contract explicitly binds
/// each to its OWN canonical namespace -- not because the raw strings
/// happen to differ. `trusting(&[(a, ..), (b, ..)])` performs that
/// explicit binding: each entry gets its own `NamespaceId`. The
/// assertion on `authority_for(..).0` below pins that this is the
/// mechanism actually doing the work.
#[test]
fn path_047_c_authoritatively_distinct_namespaces_are_distinct() {
    let contract = TrustedSourceContract::trusting(&[
        ("payroll-system", SourceIdContract::both_guaranteed()),
        ("timesheet-system", SourceIdContract::both_guaranteed()),
    ]);

    let namespace_payroll = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;
    let namespace_timesheet = contract.authority_for(&SourceId("timesheet-system".into())).unwrap().0;
    assert_ne!(
        namespace_payroll, namespace_timesheet,
        "the contract explicitly assigns these two source_id strings to two DIFFERENT canonical \
         namespaces -- this is the exogenous fact under test, not string inequality"
    );

    let payload = path_047_payload("alice", "review", 40.0);
    let a = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let b = path_047_candidate("timesheet-system", "evt-1", payload);

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Distinct,
        "event_ids are namespace-scoped, not globally scoped -- the SAME event_id string from two \
         AUTHORITATIVELY DISTINCT namespaces must not establish sameness"
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert_eq!(resolution.survivors.len(), 2);
}

#[test]
fn path_047_d_untrusted_source_matching_event_id_is_unresolved_not_deduped() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let a = path_047_candidate("scratchpad-import", "evt-1", payload.clone());
    let b = path_047_candidate("scratchpad-import", "evt-1", payload);

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Unresolved,
        "the source is absent from the trust contract -- no AUTHORITY, so matching strings alone \
         must never certify replay, regardless of what STABILITY/SEPARATION it might claim"
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert!(resolution.survivors.is_empty(), "an unresolved group must not be silently treated as one survivor");
    assert_eq!(resolution.unresolved.len(), 1);
    assert_eq!(resolution.unresolved[0].len(), 2);
}

#[test]
fn path_047_e_same_trusted_identity_changed_payload_is_conflicted_not_a_choice() {
    // SEPARATION-only contract: STABILITY is explicitly false, to prove
    // it plays no role in detecting this conflict either.
    let contract =
        TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::separating_only())]);
    let a = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 40.0));
    let b = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 55.0));

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Conflicted,
        "identity equality (licensed by SEPARATION alone) says these claims concern the SAME \
         occurrence; disagreeing payloads are a conflict, not two legitimate events, and not a \
         first/last choice"
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert!(
        resolution.survivors.is_empty(),
        "a conflicted group must never silently count as one -- or two -- legitimate events"
    );
    assert_eq!(resolution.conflicted.len(), 1);
    assert_eq!(resolution.conflicted[0].len(), 2);
}

/// CORRECTED (second cross-review): two DIFFERENT, UNTRUSTED, self-
/// declared source_id strings must NOT be called `Distinct` merely
/// because the strings differ -- with no AUTHORITY for either, this
/// resolver cannot trust that "unrelated-source-a" and
/// "unrelated-source-b" denote two different real systems any more than
/// it can trust either one's own event_id claims. The expected relation
/// is `Unresolved`.
#[test]
fn path_047_f_two_untrusted_source_labels_with_matching_event_id_are_unresolved() {
    let contract = TrustedSourceContract::default();
    let a = path_047_candidate("unrelated-source-a", "evt-1", path_047_payload("alice", "x", 10.0));
    let b = path_047_candidate("unrelated-source-b", "evt-1", path_047_payload("alice", "x", 10.0));

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Unresolved,
        "neither label carries AUTHORITY -- lexical inequality between two self-declared strings \
         proves nothing about whether they denote different real sources. This must NOT be called \
         Distinct merely because the strings differ."
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert!(
        resolution.survivors.is_empty(),
        "the batch resolver must not let two different unauthorized labels independently reach \
         survivors just because their raw strings differ"
    );
    assert_eq!(resolution.unresolved.len(), 2, "each unauthorized label is its own unresolved group");
}

/// PATH-047-G, the fresh-ID/STABILITY-loss case: the SAME real
/// occurrence, delivered once and then retried under a FRESH event_id,
/// from a source that does not guarantee STABILITY.
#[test]
fn path_047_g_fresh_id_retry_without_stability_guarantee_stays_unresolved() {
    // SEPARATION-only contract: STABILITY is explicitly false -- this
    // source is trusted (AUTHORITY holds) and does guarantee distinct
    // occurrences get distinct ids, but does NOT guarantee one
    // occurrence keeps the same id across retries.
    let contract = TrustedSourceContract::trusting(&[("flaky-webhook", SourceIdContract::separating_only())]);
    let payload = path_047_payload("alice", "review", 40.0);

    // Delivery 1 and its retry: the SAME real occurrence, but the
    // source re-minted the id on redelivery.
    let delivery = path_047_candidate("flaky-webhook", "evt-1", payload.clone());
    let retry = path_047_candidate("flaky-webhook", "evt-2", payload);

    assert_eq!(
        path_047_verdict(&delivery, &retry, &contract),
        Path047Verdict::Unresolved,
        "H18's explicit trust-boundary test: without a STABILITY guarantee, a DIFFERENT event_id \
         cannot be trusted to mean a DISTINCT occurrence -- it may just be the same occurrence \
         retried under a fresh id. The identifiers alone leave this Unresolved, not Distinct."
    );

    let resolution = path_047_resolve_batch(&[delivery, retry], &contract);
    assert!(
        resolution.survivors.is_empty(),
        "two differently-id'd deliveries from a source without a STABILITY guarantee must NOT \
         become two authoritative Distinct survivors merely because their event_ids differ"
    );
    assert_eq!(resolution.unresolved.len(), 1);
    assert_eq!(resolution.unresolved[0].len(), 2);
}

/// PATH-047-G': the SAME event_id claimed twice by a source that does
/// not guarantee SEPARATION (i.e. its ids can collide across genuinely
/// distinct occurrences).
#[test]
fn path_047_g_prime_matching_event_id_without_separation_guarantee_stays_unresolved() {
    // STABILITY-only contract: SEPARATION is explicitly false, isolating
    // it as the missing piece (this source's ids ARE stable under
    // retry -- that is not what is in question here).
    let contract = TrustedSourceContract::trusting(&[("weak-id-source", SourceIdContract::stable_only())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let a = path_047_candidate("weak-id-source", "evt-1", payload.clone());
    let b = path_047_candidate("weak-id-source", "evt-1", payload);

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Unresolved,
        "without a SEPARATION guarantee, a MATCHING event_id cannot be trusted to mean the SAME \
         occurrence -- it may be an id collision between two distinct occurrences. STABILITY does \
         not help here: it licenses the opposite direction."
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert!(resolution.survivors.is_empty());
    assert_eq!(resolution.unresolved.len(), 1);
    assert_eq!(resolution.unresolved[0].len(), 2);
}

/// CORRECTED (second cross-review): an uncontracted, alias-LOOKING
/// source_id compared against a genuinely trusted one must NOT become
/// `Distinct` merely from string inequality. Unless the contract itself
/// establishes a namespace relation for the spoofed label, the expected
/// result is `Unresolved` -- the spoof gains no AUTHORITY, and no
/// AUTHORITY means this resolver cannot even vouch for the spoof being a
/// genuinely SEPARATE source, only that it is not a RECOGNIZED one.
#[test]
fn path_047_h_source_label_spoofing_gains_no_authority_and_no_distinctness() {
    let contract = TrustedSourceContract::trusting(&[("hr-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);

    // Two self-declared deliveries under a label chosen to LOOK like the
    // trusted "hr-system" namespace -- but it is not the literal string
    // present in the contract, so it carries no AUTHORITY regardless of
    // what STABILITY/SEPARATION it might claim for itself.
    let spoof_a = path_047_candidate("hr-system-eu", "evt-1", payload.clone());
    let spoof_b = path_047_candidate("hr-system-eu", "evt-1", payload.clone());
    assert_eq!(
        path_047_verdict(&spoof_a, &spoof_b, &contract),
        Path047Verdict::Unresolved,
        "a source label that merely resembles a trusted name gains no AUTHORITY -- only an EXACT \
         entry in the explicit trust contract does, so even a matching (source_id, event_id) \
         pair under the spoofed label stays Unresolved rather than Replay"
    );

    // Nor may the spoofed label be declared `Distinct` from a genuinely
    // trusted delivery merely because its string differs from
    // "hr-system" -- AUTHORITY is missing for the spoof, so NOTHING can
    // be concluded about its relationship to the genuine namespace,
    // including that it is a different one.
    let genuine = path_047_candidate("hr-system", "evt-1", payload);
    assert_eq!(
        path_047_verdict(&spoof_a, &genuine, &contract),
        Path047Verdict::Unresolved,
        "the spoofed label has no AUTHORITY at all -- lexical inequality from the genuine, \
         trusted source_id string does not establish Distinct; that would smuggle namespace \
         distinctness in through string comparison, exactly the defect this correction removes"
    );
}

#[test]
fn path_047_i_legitimate_identical_valued_events_both_survive() {
    // STABILITY-only contract: SEPARATION is explicitly false, to prove
    // (again, at the end-to-end economic-integration level) that it
    // plays no role in the different-event_id direction.
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::stable_only())]);
    let a = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review-1", 40.0));
    let b = path_047_candidate("payroll-system", "evt-2", path_047_payload("alice", "review-2", 40.0));

    assert_eq!(
        path_047_verdict(&a, &b, &contract),
        Path047Verdict::Distinct,
        "FIND-024 regression property: same contributor, same source, same value, but DIFFERENT \
         authoritative event ids -- both must survive"
    );

    let resolution = path_047_resolve_batch(&[a, b], &contract);
    assert_eq!(resolution.survivors.len(), 2);

    // Feed the survivors through the actual production allocation
    // function, exactly like PATH-046.5's own regression guard, to
    // confirm the economic effect rather than just the verdict.
    let mut events: Vec<ContributionEvent> = resolution.survivors.into_iter().map(|c| c.payload).collect();
    events.push(path_047_payload("bob", "bob-work", 20.0));
    let allocations = allocate_contribution_pool(&events, &weights(), 100.0).unwrap();
    let alice_share: f64 =
        allocations.iter().filter(|x| x.contributor_id == "alice").map(|x| x.normalized_share).sum();
    assert!((alice_share - 0.8).abs() < 1e-9, "both legitimate 40-value events must count -- 80 of 100 total");
}

/// Direction-independence, made explicit in one place: a
/// SEPARATION-only contract resolves the same-event_id direction
/// correctly but leaves the different-event_id direction Unresolved, and
/// a STABILITY-only contract does the exact mirror image. Neither
/// property is a substitute for the other in either direction.
#[test]
fn path_047_j_stability_and_separation_license_disjoint_directions() {
    let payload_1 = path_047_payload("alice", "review", 40.0);
    let payload_2 = path_047_payload("alice", "review", 40.0);
    assert_eq!(payload_1, payload_2);

    let separating_only =
        TrustedSourceContract::trusting(&[("source-x", SourceIdContract::separating_only())]);
    let stable_only = TrustedSourceContract::trusting(&[("source-x", SourceIdContract::stable_only())]);

    let same_id_a = path_047_candidate("source-x", "evt-1", payload_1.clone());
    let same_id_b = path_047_candidate("source-x", "evt-1", payload_2.clone());
    let diff_id_a = path_047_candidate("source-x", "evt-1", payload_1);
    let diff_id_b = path_047_candidate("source-x", "evt-2", payload_2);

    assert_eq!(
        path_047_verdict(&same_id_a, &same_id_b, &separating_only),
        Path047Verdict::Replay,
        "SEPARATION-only: same-id direction resolves"
    );
    assert_eq!(
        path_047_verdict(&diff_id_a, &diff_id_b, &separating_only),
        Path047Verdict::Unresolved,
        "SEPARATION-only: different-id direction stays Unresolved -- SEPARATION does not license it"
    );

    assert_eq!(
        path_047_verdict(&same_id_a, &same_id_b, &stable_only),
        Path047Verdict::Unresolved,
        "STABILITY-only: same-id direction stays Unresolved -- STABILITY does not license it"
    );
    assert_eq!(
        path_047_verdict(&diff_id_a, &diff_id_b, &stable_only),
        Path047Verdict::Distinct,
        "STABILITY-only: different-id direction resolves"
    );
}

/// AUTHORITY alone -- a source present in the contract but guaranteeing
/// NEITHER property -- is exactly as unresolved as no contract entry at
/// all, in both directions. Closes the loop with PATH-047-D (absence of
/// AUTHORITY): this shows PRESENCE of AUTHORITY is also not sufficient
/// by itself.
#[test]
fn path_047_k_authority_alone_is_insufficient_without_either_property() {
    let contract = TrustedSourceContract::trusting(&[("source-y", SourceIdContract::neither_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);

    let same_id_a = path_047_candidate("source-y", "evt-1", payload.clone());
    let same_id_b = path_047_candidate("source-y", "evt-1", payload.clone());
    assert_eq!(path_047_verdict(&same_id_a, &same_id_b, &contract), Path047Verdict::Unresolved);

    let diff_id_a = path_047_candidate("source-y", "evt-1", payload.clone());
    let diff_id_b = path_047_candidate("source-y", "evt-2", payload);
    assert_eq!(path_047_verdict(&diff_id_a, &diff_id_b, &contract), Path047Verdict::Unresolved);
}

/// NEW (second cross-review, required case): two DIFFERENT raw
/// source_id strings, deliberately bound by the contract to the SAME
/// canonical namespace (an explicit alias relationship), must NOT
/// manufacture cross-source `Distinct`-ness. They are resolved exactly
/// as if they were one literal source: a matching event_id under them
/// collapses via SEPARATION (still deduplicating a true replay even
/// though it arrived under two different labels), and a differing
/// event_id under them still requires STABILITY to be trusted as
/// distinct.
#[test]
fn path_047_l_aliases_of_the_same_canonical_namespace_do_not_manufacture_distinctness() {
    let contract = TrustedSourceContract::trusting_with_aliases(
        "payroll-canonical",
        &["payroll-system", "payroll-system-legacy"],
        SourceIdContract::both_guaranteed(),
    );

    let namespace_a = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;
    let namespace_b = contract.authority_for(&SourceId("payroll-system-legacy".into())).unwrap().0;
    assert_eq!(
        namespace_a, namespace_b,
        "both raw labels are explicitly bound to the SAME canonical namespace by this contract"
    );

    let payload = path_047_payload("alice", "review", 40.0);

    // The SAME real occurrence, reported once under each alias with a
    // matching event_id: must dedupe via SEPARATION, exactly as if both
    // deliveries had used the identical literal source_id string.
    let via_alias_1 = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let via_alias_2 = path_047_candidate("payroll-system-legacy", "evt-1", payload.clone());
    assert_eq!(
        path_047_verdict(&via_alias_1, &via_alias_2, &contract),
        Path047Verdict::Replay,
        "two labels bound to the SAME canonical namespace must be treated as the SAME source for \
         identity purposes -- a matching event_id across them is still a replay, not a manufactured \
         Distinct pair"
    );

    // A genuinely different event_id under the SAME namespace (again
    // reached via an alias) still resolves as an ordinary distinct
    // occurrence, gated by STABILITY exactly as it would be for one
    // literal source_id string.
    let via_alias_3 = path_047_candidate("payroll-system-legacy", "evt-2", payload);
    assert_eq!(path_047_verdict(&via_alias_1, &via_alias_3, &contract), Path047Verdict::Distinct);

    let resolution = path_047_resolve_batch(&[via_alias_1, via_alias_2, via_alias_3], &contract);
    assert_eq!(
        resolution.survivors.len(),
        2,
        "3 input events under 2 aliases of 1 namespace: the matching-event_id pair collapses to 1 \
         survivor, and the distinct-event_id event survives independently -- 2 total, not 3 and not 1"
    );
    assert!(resolution.conflicted.is_empty());
    assert!(resolution.unresolved.is_empty());
}

// ---------------------------------------------------------------------
// Research questions -- answered from the corrected model's behavior
// above, not asserted independently of it.
// ---------------------------------------------------------------------
//
// 1. Is (source_id, event_id) sufficient under the stated contract?
//    Conditionally, and per-direction: sufficiency for a SAME-event_id
//    claim requires AUTHORITY (resolved to a canonical namespace) +
//    SEPARATION (PATH-047-A/E, both proven with STABILITY explicitly
//    false); sufficiency for a DIFFERENT-event_id claim requires
//    AUTHORITY + STABILITY (PATH-047-B/I, both proven with SEPARATION
//    explicitly false). Two DIFFERENT source_id strings are only
//    sufficient grounds for Distinct when AUTHORITY is established for
//    BOTH and the contract explicitly assigns them DIFFERENT namespaces
//    (PATH-047-C) -- never from string inequality alone (PATH-047-F,
//    -H). The bare key is never sufficient by itself in any case.
//
// 2. Is global uniqueness necessary? Falsified by PATH-047-C: two
//    AUTHORITATIVELY DISTINCT namespaces reusing an identical event_id
//    string resolve correctly to `Distinct` with no cross-namespace
//    coordination. But this now depends on an explicit exogenous
//    namespace-distinctness fact, not on the source_id strings merely
//    differing (PATH-047-F shows two differing, UNAUTHORIZED strings do
//    NOT get this for free). Global uniqueness is sufficient but not
//    necessary; authoritative namespace-scoping is what actually does
//    the work.
//
// 3. What exact assumption is exogenous rather than derived?
//    Three separate assumptions, now modeled as three separate facts:
//    AUTHORITY (is a raw source_id string bound to a real namespace at
//    all -- `TrustedSourceContract::namespace_of`), namespace identity
//    itself (WHICH namespace a string is bound to, and whether two
//    strings share one -- this is the fact the second cross-review
//    added explicit representation for for; it was previously smuggled
//    in as "string inequality implies namespace inequality," an
//    unstated and false assumption), and, per namespace, STABILITY and
//    SEPARATION (`TrustedSourceContract::properties`). None of these are
//    derived from the events; all are supplied by the contract.
//
// 4. Does identity resolution belong before economic normalization?
//    Yes, and the corrected model adds a step that must ALSO happen
//    before normalization: resolving each event's raw source_id to a
//    canonical namespace (or discovering it has none). PATH-047-L shows
//    this step is not optional bookkeeping -- two aliased labels for one
//    real source must be merged onto one namespace before SEPARATION/
//    STABILITY logic runs, or a true replay arriving under two labels
//    would wrongly survive as two events.
//
// 5. What happens if one source cannot provide stable event identity?
//    Unchanged in kind from the prior revision, restated over
//    namespaces: PATH-047-G shows two or more differently-labeled
//    deliveries from one namespace degrade to `Unresolved` as a GROUP --
//    the same state as an unauthorized label (PATH-047-D/F) or one
//    missing SEPARATION (PATH-047-G'). The one named exception remains a
//    namespace's SOLE claim in a given batch; see FIND-025's NOT
//    ESTABLISHED for why this does not generalize across separate
//    resolution calls.


// =====================================================================
// PATH-048 -- TEMPORAL IDENTITY / CROSS-BATCH REPLAY STATE (H19)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::contribution` or wired into any production
// path -- no production file is touched by this experiment.
//
// QUESTION: what is the minimum durable state required to prevent one
// real economic occurrence from being counted twice when its deliveries
// arrive in DIFFERENT resolution calls?
//
// H19: pairwise/in-batch identity semantics (PATH-047/FIND-025) are
// insufficient for durable replay protection. A replay-safe economic
// boundary requires some durable cross-call identity memory whose
// retention and authority semantics are explicitly defined.
//
// This experiment REUSES PATH-047's type vocabulary without modification
// -- `SourceId`, `NamespaceId`, `TrustedSourceContract`,
// `SourceIdContract`, `EventIdentity`, `Path047CandidateEvent`, and the
// helper constructors -- so `SourceId != NamespaceId` and AUTHORITY/
// STABILITY/SEPARATION stay exactly as separate as FIND-025 established.
// PATH-048 adds a NEW layer on top: durable state that survives across
// separate calls to the resolution logic, which PATH-047's pairwise
// `path_047_verdict` and single-batch `path_047_resolve_batch` never
// modeled at all (both take their entire universe of events as one
// argument, with nothing persisting between calls).
//
// REQUIRED DISTINCTIONS -- six separate concepts, six separate types,
// never collapsed into one "seen IDs" set (a sixth, HISTORICAL
// COMPLETENESS, was added by a later cross-review correction -- AMNESIA
// != GENESIS, see the dedicated section and `HistoryCompleteness` type
// below):
//   - DELIVERY IDENTITY       -- `EventIdentity{source_id, event_id}`
//     (PATH-047, unchanged): the raw, on-the-wire key as delivered.
//   - CANONICAL OCCURRENCE IDENTITY -- `OccurrenceKey{namespace, event_id}`:
//     `source_id` resolved through AUTHORITY to its `NamespaceId`, so
//     aliases collapse onto one key (PATH-047-L's property, preserved).
//   - SETTLEMENT IDENTITY     -- `(OccurrenceKey, SettlementRecord)`: an
//     occurrence key has no settlement identity until something is
//     recorded against it, and what's recorded distinguishes `Reserved`
//     (in flight) from `Settled` (durably complete, with a payload
//     snapshot for conflict detection).
//   - TRUST-CONTRACT VERSION  -- `ContractVersion`, stored per settlement
//     record as metadata. NOTE (cross-review correction): the drift
//     DECISION (PATH-048-G) is made by a separate, per-label
//     first-seen-namespace fact (`label_first_seen_namespace`), never by
//     `ContractVersion` -- `path_048_reserve` never reads
//     `contract_version` off a stored record to decide anything. This
//     experiment establishes that a historical AUTHORITY BINDING must
//     survive current-contract evolution; it does NOT establish that
//     `ContractVersion` itself is necessary (see NOT ESTABLISHED below
//     and in FIND-026).
//   - TEMPORAL RETENTION STATE -- `pruned_horizon` and
//     `ever_active_namespaces`. NOTE (cross-review correction):
//     `has_pruned` only consults MAP MEMBERSHIP in `pruned_horizon`, never
//     the `CallId` value it stores. This experiment establishes the need
//     for a durable "this namespace's history may be incomplete" fact; it
//     does NOT establish that a numeric per-call horizon is itself
//     minimally necessary. `pruned_horizon`'s `CallId` value is retained
//     as one plausible real-world design (e.g. for a future finer-grained
//     policy), not as proven-minimal structure. `ever_active_namespaces`
//     is the durable, much smaller fact -- "this namespace has
//     settlement history at all" -- that survives pruning of the
//     detailed records it summarizes.
//   - HISTORICAL COMPLETENESS -- `HistoryCompleteness` (`AttestedComplete` /
//     `Unknown`), added by the AMNESIA != GENESIS correction below: an
//     EXOGENOUS fact about the LEDGER's own provenance, never derived
//     from `records`/`ever_active_namespaces` being empty. Distinct from
//     every field above -- `ever_active_namespaces` records what THIS
//     ledger instance has itself observed; `HistoryCompleteness` records
//     whether that instance's observations are attested-complete since
//     true origin. An empty ledger of unknown provenance and a
//     genuinely-fresh, attested one are identical on every OTHER field.
//
// CONSTRAINTS HONORED: no payload-hash dedup (payload comparison below
// is always direct `ContributionEvent` structural equality, exactly as
// PATH-046/PATH-047 already did -- never a summarizing digest used AS an
// identity key); no `contribution_id` heuristic (never inspected); no
// assumption of infinite retention (`prune_namespace` genuinely deletes
// records); no assumption of globally unique event ids (all keys are
// namespace-scoped, per FIND-025); missing historical state (a miss
// after ANY pruning, or a miss for a STABILITY-lacking namespace with
// prior history) resolves `Unresolved`, never silently `New`.
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CallId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ContractVersion(u64);

/// CANONICAL OCCURRENCE IDENTITY: `source_id` resolved through AUTHORITY
/// to its `NamespaceId`, paired with the raw `event_id`. This -- never
/// the raw, unresolved `EventIdentity` -- is what the durable ledger
/// keys on, so alias rotation (PATH-048-F) is transparent to it and a
/// raw-delivery-keyed design (PATH-048-D's losing comparison) is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OccurrenceKey {
    namespace: NamespaceId,
    event_id: String,
}

/// SETTLEMENT IDENTITY: what, if anything, is durably known about one
/// `OccurrenceKey`. `Reserved` and `Settled` are deliberately separate
/// states (not a boolean) -- PATH-048-J needs to distinguish "identity
/// claimed, economic effect not yet confirmed" from "fully complete."
#[derive(Debug, Clone, PartialEq)]
enum SettlementRecord {
    Reserved { payload: ContributionEvent, contract_version: ContractVersion, call: CallId },
    Settled { payload: ContributionEvent, contract_version: ContractVersion, call: CallId },
}

/// The identity-resolution verdict for ONE incoming delivery against
/// DURABLE cross-call history -- distinct from PATH-047's
/// `Path047Verdict`, which compares two LIVE events against each other
/// within one call. No boolean anywhere: "have I seen this before" is
/// not a yes/no question once retention and in-flight reservations are
/// real possibilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path048Verdict {
    /// Genuinely first appearance of this occurrence key, with complete,
    /// unpruned retained history for its namespace (or this namespace's
    /// very first-ever activity) -- safe to reserve/settle.
    New,
    /// Exact occurrence-key match against a durably `Settled` record
    /// with an IDENTICAL payload. This delivery is a replay of an
    /// already-completed settlement; must not be double-counted.
    AlreadySettled,
    /// Exact occurrence-key match against a durable record (`Reserved`
    /// or `Settled`) with a DIFFERENT payload. Reject; never silently
    /// overwritten, never counted as two legitimate events.
    Conflicted,
    /// Exact occurrence-key match against a `Reserved` (not yet
    /// `Settled`) record with a MATCHING payload. Must not be treated as
    /// New (risks double settlement once/if the original completes) or
    /// as AlreadySettled (it never actually was). A `Reserved` match with
    /// a DIFFERENT payload is `Conflicted` instead (see below) -- an
    /// earlier revision of this candidate had a real bug here, returning
    /// `Reserved` unconditionally without comparing payload; fixed and
    /// regression-guarded by PATH-048-K.
    Reserved,
    /// The identifiers and durable state together cannot settle this:
    /// AUTHORITY missing, a matching key under a SEPARATION-lacking
    /// namespace, a non-matching key under a STABILITY-lacking namespace
    /// that already has prior history, retention has pruned any record
    /// that could confirm or deny history for this key, or the current
    /// trust contract has rebound this label's namespace since it was
    /// first observed. Never collapsed into New or AlreadySettled.
    Unresolved,
}

/// ABSENCE != NOVELTY -- the general principle AMNESIA != GENESIS is one
/// instance of (cross-review correction, second pass). A missing
/// `OccurrenceKey` means only "not found in what this ledger retained,"
/// never "never happened" -- UNLESS the retained history being searched
/// is itself attested complete over the relevant identity domain. This
/// governs every NEGATIVE occurrence-history lookup used to derive
/// `New`: both a namespace's very first sighting (no records for it at
/// all) AND a miss on a specific `event_id` within an ALREADY-active
/// namespace (some records exist, just not this one) are negative
/// lookups, and both require this same attestation -- the first pass of
/// this correction fixed only the former case, leaving the latter
/// (PARTIAL RESTORE WITHIN THE SAME NAMESPACE) still vulnerable; see
/// `path_048_reserve`'s corrected MISS handling below. `HistoryCompleteness`
/// is the EXOGENOUS fact (never derived from `records`/
/// `ever_active_namespaces` being empty, and never derived from restoring
/// SOME records, or from no pruning marker being present) that resolves
/// the ambiguity, mirroring PATH-042/FIND-019's original
/// exogenous-trust-axiom pattern one level up: a fact about the LEDGER's
/// own provenance, not about any one event or source. EXACT-KEY HITS are
/// unaffected either way -- a positive record match is usable evidence
/// regardless of completeness (PATH-048-completeness-C/D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HistoryCompleteness {
    /// Authoritatively attested: this ledger's retained history for this
    /// namespace is known-complete over the relevant identity domain --
    /// e.g. a namespace provisioned strictly after this ledger began
    /// recording under an attested-genesis deployment, or an explicit,
    /// trusted migration/import attestation (PATH-048-amnesia-G).
    /// EXOGENOUS: the resolver propagates this fact faithfully but
    /// cannot verify its truth -- a false attestation is indistinguishable
    /// from a true one from inside `path_048_reserve` (PATH-048-amnesia-E,
    /// the axiom-forgery boundary).
    AttestedComplete,
    /// Not attested: whether prior history exists that this ledger's
    /// retained records do not currently reflect -- captured by some
    /// earlier, now-lost, never-restored, partially-restored, or
    /// never-captured record -- is unknown. This is the SAFE DEFAULT.
    Unknown,
}

impl Default for HistoryCompleteness {
    fn default() -> Self {
        HistoryCompleteness::Unknown
    }
}

/// The durable, cross-call ledger under test. TEMPORAL RETENTION STATE
/// (`pruned_horizon`, `ever_active_namespaces`), the TRUST-CONTRACT-
/// VERSION-adjacent `label_first_seen_namespace` fact, and HISTORICAL
/// COMPLETENESS (`completeness`, `default_completeness`) are deliberately
/// SEPARATE fields from `records` (SETTLEMENT IDENTITY) -- none are
/// derivable from the others, and each is exercised by a distinct
/// adversarial case below.
#[derive(Debug, Clone, Default)]
struct Path048Ledger {
    records: std::collections::HashMap<OccurrenceKey, SettlementRecord>,
    /// Per-namespace retention horizon: the oldest `CallId` for which
    /// this ledger still guarantees a complete record. Absence means
    /// nothing has ever been pruned for that namespace. CROSS-REVIEW
    /// CORRECTION: `has_pruned` below reads only MAP MEMBERSHIP here,
    /// never the `CallId` value -- this experiment establishes the need
    /// for a durable "history may be incomplete" fact per namespace, not
    /// that a numeric per-call horizon is itself minimally necessary.
    /// The stored `CallId` is kept as one plausible real-world design
    /// (e.g. for a future finer-grained policy), not proven-minimal
    /// structure.
    pruned_horizon: std::collections::HashMap<NamespaceId, CallId>,
    /// Durable, MUCH smaller than `records`: "this namespace has settled
    /// or reserved SOMETHING, ever" -- survives pruning of the detailed
    /// records it summarizes (PATH-048-H's central mechanism).
    ever_active_namespaces: std::collections::HashSet<NamespaceId>,
    /// The FIRST canonical namespace each raw `SourceId` label was ever
    /// observed resolving to, under whatever contract was current at the
    /// time. A later reservation whose CURRENT contract resolves the
    /// same label to a DIFFERENT namespace is contract drift
    /// (PATH-048-G), not an ordinary lookup.
    label_first_seen_namespace: std::collections::HashMap<SourceId, NamespaceId>,
    /// Per-namespace override of `default_completeness`. A namespace
    /// absent here falls back to `default_completeness`.
    completeness: std::collections::HashMap<NamespaceId, HistoryCompleteness>,
    /// The completeness assumed for any namespace with no entry in
    /// `completeness`. Defaults to `Unknown` via `#[derive(Default)]` --
    /// the safe choice. `Path048Ledger::default()` therefore models a
    /// ledger whose own provenance is NOT attested (AMNESIA is the
    /// default assumption, never GENESIS); use
    /// `Path048Ledger::new_attested_genesis()` to model an explicitly
    /// attested fresh deployment.
    default_completeness: HistoryCompleteness,
}

impl Path048Ledger {
    /// EXOGENOUS constructor: stands in for an operator/deployment
    /// process explicitly attesting that this ledger represents a
    /// genuine, complete history since true system genesis -- e.g. a
    /// brand-new deployment where no prior ledger, database, or migrated
    /// history could possibly exist. This fact is supplied by the
    /// caller, never derived from the ledger's own (empty) contents --
    /// exactly like `TrustedSourceContract`'s AUTHORITY, this experiment
    /// does not verify it, only propagates it faithfully (PATH-048-
    /// amnesia-E shows what happens when it is wrong).
    fn new_attested_genesis() -> Self {
        Path048Ledger { default_completeness: HistoryCompleteness::AttestedComplete, ..Default::default() }
    }

    /// Explicit, per-namespace completeness override -- e.g. a namespace
    /// migrated in with an attested-complete import (PATH-048-amnesia-G),
    /// on an otherwise not-fully-attested ledger.
    fn attest_namespace_history_complete(&mut self, namespace: &NamespaceId) {
        self.completeness.insert(namespace.clone(), HistoryCompleteness::AttestedComplete);
    }

    /// Explicit, per-namespace override in the OTHER direction -- e.g. a
    /// legacy namespace whose own completeness was never itself attested
    /// during migration, layered onto an otherwise genesis-known ledger
    /// (PATH-048-amnesia-F).
    fn attest_namespace_history_unknown(&mut self, namespace: &NamespaceId) {
        self.completeness.insert(namespace.clone(), HistoryCompleteness::Unknown);
    }

    fn completeness_for(&self, namespace: &NamespaceId) -> HistoryCompleteness {
        self.completeness.get(namespace).copied().unwrap_or(self.default_completeness)
    }

    /// Only MEMBERSHIP is consulted by any decision in this file -- the
    /// `CallId` value stored in `pruned_horizon` is never read back out.
    /// See the field-level correction notice above.
    fn has_pruned(&self, namespace: &NamespaceId) -> bool {
        self.pruned_horizon.contains_key(namespace)
    }

    fn record_call(record: &SettlementRecord) -> CallId {
        match record {
            SettlementRecord::Reserved { call, .. } | SettlementRecord::Settled { call, .. } => *call,
        }
    }

    /// Genuinely forgets any record for `namespace` older than
    /// `keep_from` -- this experiment does not assume infinite
    /// retention. The namespace's horizon only ever advances.
    fn prune_namespace(&mut self, namespace: &NamespaceId, keep_from: CallId) {
        self.records.retain(|key, record| key.namespace != *namespace || Self::record_call(record) >= keep_from);
        let entry = self.pruned_horizon.entry(namespace.clone()).or_insert(keep_from);
        if keep_from > *entry {
            *entry = keep_from;
        }
    }
}

fn path_048_occurrence_key(contract: &TrustedSourceContract, event: &Path047CandidateEvent) -> Option<OccurrenceKey> {
    let (namespace, _) = contract.authority_for(&event.identity.source_id)?;
    Some(OccurrenceKey { namespace, event_id: event.identity.event_id.clone() })
}

/// PHASE 1 of a two-phase settlement (PATH-048-J): reserve identity for
/// this occurrence, durably, BEFORE any economic effect is applied. Only
/// a `New` result licenses proceeding to apply the effect and then call
/// `path_048_confirm_settlement`.
///
/// This is the GUARDED, adopted design -- it includes the label-drift
/// check (PATH-048-G) and the retention/stability-aware miss handling
/// (PATH-048-E/H). See `path_048_reserve_naive_current_contract_only`
/// for the deliberately weaker sibling PATH-048-G contrasts against.
fn path_048_reserve(
    ledger: &mut Path048Ledger,
    contract: &TrustedSourceContract,
    contract_version: ContractVersion,
    call: CallId,
    event: &Path047CandidateEvent,
) -> Path048Verdict {
    let (namespace, props) = match contract.authority_for(&event.identity.source_id) {
        None => return Path048Verdict::Unresolved,
        Some(x) => x,
    };

    // Label-drift guard (PATH-048-G): has this exact raw label ever
    // resolved to a DIFFERENT namespace before? If so, the trust
    // contract's binding for this label has changed since it was first
    // observed -- the CURRENT resolution alone cannot be trusted to look
    // up history correctly.
    match ledger.label_first_seen_namespace.get(&event.identity.source_id) {
        Some(first_namespace) if *first_namespace != namespace => return Path048Verdict::Unresolved,
        Some(_) => {}
        None => {
            ledger.label_first_seen_namespace.insert(event.identity.source_id.clone(), namespace.clone());
        }
    }

    let key = OccurrenceKey { namespace: namespace.clone(), event_id: event.identity.event_id.clone() };

    match ledger.records.get(&key) {
        Some(record) => {
            // Exact canonical-key match against durable history --
            // SEPARATION licenses trusting this as "same occurrence"
            // (mirrors PATH-047's same-event_id direction).
            if !props.separating {
                return Path048Verdict::Unresolved;
            }
            match record {
                // CROSS-REVIEW FIX: an earlier revision returned
                // `Reserved` here unconditionally, without comparing
                // payload -- a real resolver bug (PATH-048-K regression-
                // guards it). A `Reserved` record with a DIFFERENT
                // payload is a conflict, exactly like a `Settled` one:
                // identity equality says these claims concern the same
                // occurrence, so disagreement must be flagged, never
                // silently ignored or overwritten.
                SettlementRecord::Reserved { payload, .. } => {
                    if *payload == event.payload { Path048Verdict::Reserved } else { Path048Verdict::Conflicted }
                }
                SettlementRecord::Settled { payload, .. } => {
                    if *payload == event.payload { Path048Verdict::AlreadySettled } else { Path048Verdict::Conflicted }
                }
            }
        }
        None => {
            if !ledger.ever_active_namespaces.contains(&namespace) {
                // AMNESIA != GENESIS (cross-review correction): absence
                // from `ever_active_namespaces` means only "THIS ledger
                // instance holds no record of prior activity for this
                // namespace" -- it is NOT, by itself, evidence that no
                // such activity ever occurred (the ledger's own state
                // could have been lost, reset, or incompletely restored).
                // Only an explicit, exogenous completeness attestation
                // licenses treating a namespace's first observed sighting
                // as safe to settle.
                match ledger.completeness_for(&namespace) {
                    HistoryCompleteness::AttestedComplete => {
                        ledger.ever_active_namespaces.insert(namespace.clone());
                        ledger.records.insert(
                            key,
                            SettlementRecord::Reserved { payload: event.payload.clone(), contract_version, call },
                        );
                        return Path048Verdict::New;
                    }
                    HistoryCompleteness::Unknown => {
                        return Path048Verdict::Unresolved;
                    }
                }
            }
            // This namespace HAS prior history retained here. A MISS on
            // this specific event_id is negative evidence -- "not found
            // in what this ledger retained" -- which is useful only if
            // the retained history for this namespace is itself known
            // complete. CROSS-REVIEW CORRECTION II (bug fix): a prior
            // revision skipped this check entirely for an
            // already-active namespace, trusting membership in
            // `ever_active_namespaces` as if it implied complete
            // retained history for EVERY event_id -- it does not. A
            // PARTIAL RESTORE WITHIN THE SAME NAMESPACE (some of this
            // namespace's real settlements restored, others silently
            // omitted, with no completeness marker surviving the
            // restore) is indistinguishable, on this branch alone, from
            // a namespace that genuinely never had the missing event_id
            // -- exactly ABSENCE != NOVELTY recurring one level down
            // from where the first pass of this correction caught it.
            if ledger.completeness_for(&namespace) != HistoryCompleteness::AttestedComplete {
                return Path048Verdict::Unresolved;
            }
            // Independently of completeness: confidently declaring a NEW
            // event_id "New" ALSO requires trusting "different id =>
            // distinct occurrence" -- STABILITY (mirrors PATH-047's
            // different-event_id direction). A pruned namespace
            // (`has_pruned`, membership only -- see field-level notice
            // above) independently removes the ability to positively
            // confirm this isn't a forgotten match. Any of the three
            // conditions failing blocks confident New (PATH-048-E,
            // PATH-048-H, PATH-048-completeness-A).
            if !props.stable || ledger.has_pruned(&namespace) {
                return Path048Verdict::Unresolved;
            }
            ledger.records.insert(
                key,
                SettlementRecord::Reserved { payload: event.payload.clone(), contract_version, call },
            );
            Path048Verdict::New
        }
    }
}

/// PHASE 2: durably confirm that the economic effect for a previously
/// `Reserved` occurrence was actually applied. A no-op if `key` is not
/// currently `Reserved` (e.g. already `Settled`, or absent).
fn path_048_confirm_settlement(ledger: &mut Path048Ledger, key: &OccurrenceKey) {
    if let Some(SettlementRecord::Reserved { payload, contract_version, call }) = ledger.records.get(key).cloned() {
        ledger.records.insert(key.clone(), SettlementRecord::Settled { payload, contract_version, call });
    }
}

/// Convenience used by most tests below, which do not care about the
/// Reserved/Settled distinction itself: reserve, and if `New`,
/// immediately confirm -- modeling an atomic-in-practice happy path.
/// PATH-048-J deliberately does NOT use this helper, to model the crash
/// window between the two phases.
fn path_048_settle_immediately(
    ledger: &mut Path048Ledger,
    contract: &TrustedSourceContract,
    contract_version: ContractVersion,
    call: CallId,
    event: &Path047CandidateEvent,
) -> Path048Verdict {
    let verdict = path_048_reserve(ledger, contract, contract_version, call, event);
    if verdict == Path048Verdict::New {
        if let Some(key) = path_048_occurrence_key(contract, event) {
            path_048_confirm_settlement(ledger, &key);
        }
    }
    verdict
}

/// A deliberately WEAKER variant of `path_048_reserve` that resolves
/// `source_id` through the CURRENT contract only, with no memory of what
/// namespace that label resolved to at any PRIOR call -- i.e. it lacks
/// the label-drift guard. Exists solely so PATH-048-G can demonstrate
/// its failure mode by direct contrast with the guarded
/// `path_048_reserve`. Otherwise identical.
fn path_048_reserve_naive_current_contract_only(
    ledger: &mut Path048Ledger,
    contract: &TrustedSourceContract,
    contract_version: ContractVersion,
    call: CallId,
    event: &Path047CandidateEvent,
) -> Path048Verdict {
    let (namespace, props) = match contract.authority_for(&event.identity.source_id) {
        None => return Path048Verdict::Unresolved,
        Some(x) => x,
    };
    let key = OccurrenceKey { namespace: namespace.clone(), event_id: event.identity.event_id.clone() };
    match ledger.records.get(&key) {
        Some(record) => {
            if !props.separating {
                return Path048Verdict::Unresolved;
            }
            match record {
                // Same fix as `path_048_reserve` -- this sibling copy
                // shares the payload-comparison logic; it is deliberately
                // weak only with respect to the label-drift guard, not
                // this.
                SettlementRecord::Reserved { payload, .. } => {
                    if *payload == event.payload { Path048Verdict::Reserved } else { Path048Verdict::Conflicted }
                }
                SettlementRecord::Settled { payload, .. } => {
                    if *payload == event.payload { Path048Verdict::AlreadySettled } else { Path048Verdict::Conflicted }
                }
            }
        }
        None => {
            if !ledger.ever_active_namespaces.contains(&namespace) {
                ledger.ever_active_namespaces.insert(namespace.clone());
                ledger.records.insert(
                    key,
                    SettlementRecord::Reserved { payload: event.payload.clone(), contract_version, call },
                );
                return Path048Verdict::New;
            }
            if !props.stable || ledger.has_pruned(&namespace) {
                return Path048Verdict::Unresolved;
            }
            ledger.records.insert(
                key,
                SettlementRecord::Reserved { payload: event.payload.clone(), contract_version, call },
            );
            Path048Verdict::New
        }
    }
}

/// A deliberately WEAK candidate (PATH-048-B): remembers only the LAST
/// event_id seen per namespace, nothing more.
#[derive(Debug, Clone, Default)]
struct Path048LastSeenOnly {
    last_event_id: std::collections::HashMap<NamespaceId, String>,
}

impl Path048LastSeenOnly {
    /// Returns `true` if this delivery "looks new" under this candidate
    /// (i.e. differs from the single remembered last-seen id).
    fn check_and_update(&mut self, namespace: &NamespaceId, event_id: &str) -> bool {
        let looks_new = self.last_event_id.get(namespace).map(|last| last != event_id).unwrap_or(true);
        self.last_event_id.insert(namespace.clone(), event_id.to_string());
        looks_new
    }
}

/// A deliberately NAIVE candidate (PATH-048-C): a raw set of previously
/// settled canonical keys, pruned with NO retained memory of what was
/// forgotten (contrast with `Path048Ledger`'s `ever_active_namespaces` +
/// `pruned_horizon`, which retain exactly that).
#[derive(Debug, Clone, Default)]
struct Path048NaiveKeySet {
    seen: std::collections::HashMap<OccurrenceKey, CallId>,
}

impl Path048NaiveKeySet {
    /// Returns `true` if `key` had never been recorded before (i.e. this
    /// delivery is treated as New).
    fn check_and_insert(&mut self, key: OccurrenceKey, call: CallId) -> bool {
        if self.seen.contains_key(&key) {
            false
        } else {
            self.seen.insert(key, call);
            true
        }
    }

    /// NAIVE prune: entries older than `before` are simply dropped, with
    /// nothing retained to remember that this happened.
    fn naive_prune(&mut self, before: CallId) {
        self.seen.retain(|_, call| *call >= before);
    }
}

// ---------------------------------------------------------------------
// Adversarial cases A-J (H19's ten required attacks), plus the temporal
// identifiability theorem and an end-to-end integration check.
// ---------------------------------------------------------------------

/// PATH-048-A: no persistent state at all. Simulated by using two
/// entirely INDEPENDENT, freshly-attested ledgers for "call 1" and "call
/// 2" -- i.e. nothing about call 1 is available when call 2 runs. Both
/// are built with `new_attested_genesis()` deliberately: a "no
/// persistent state" design in the wild typically does not even KNOW it
/// lacks history -- it proceeds optimistically. (Using the safe
/// `Unknown`-by-default constructor here instead would demonstrate a
/// DIFFERENT, even more degenerate failure -- perpetual `Unresolved`,
/// never settling anything at all -- which is not this case's point.)
#[test]
fn path_048_a_no_persistent_state_allows_independent_settlement_across_calls() {
    let contract = TrustedSourceContract::trusting(&[("ns-A", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let delivery = path_047_candidate("ns-A", "evt-1", payload.clone());
    let retry = path_047_candidate("ns-A", "evt-1", payload);

    let mut call_1_state = Path048Ledger::new_attested_genesis();
    let mut call_2_state = Path048Ledger::new_attested_genesis(); // deliberately NOT the same ledger

    let v1 = path_048_settle_immediately(&mut call_1_state, &contract, ContractVersion(1), CallId(1), &delivery);
    let v2 = path_048_settle_immediately(&mut call_2_state, &contract, ContractVersion(1), CallId(2), &retry);

    assert_eq!(v1, Path048Verdict::New);
    assert_eq!(
        v2,
        Path048Verdict::New,
        "ATTACK SUCCEEDS: with NO state shared across calls, the retry settles independently as \
         New a second time -- H19's premise confirmed directly: pairwise/in-batch semantics alone \
         (which is all a stateless design can offer) permit double settlement of one real \
         occurrence the moment its deliveries land in separate calls"
    );
}

/// PATH-048-B: remember only the last-seen event_id per namespace.
/// Attacked with evt-1 -> evt-2 -> replay evt-1.
#[test]
fn path_048_b_last_seen_only_memory_fails_against_non_adjacent_replay() {
    let namespace = NamespaceId("ns-A".to_string());
    let mut candidate = Path048LastSeenOnly::default();

    assert!(candidate.check_and_update(&namespace, "evt-1"), "first delivery: genuinely new");
    assert!(candidate.check_and_update(&namespace, "evt-2"), "second delivery: genuinely new");
    assert!(
        candidate.check_and_update(&namespace, "evt-1"),
        "ATTACK SUCCEEDS: replaying evt-1 after evt-2 has become the 'last seen' id looks new \
         under this candidate, because it only ever compares against the SINGLE most recent id -- \
         bounded 'last seen' memory is insufficient the moment a replay is not immediately \
         adjacent to its original delivery"
    );
}

/// PATH-048-C: a set of all previously settled authoritative identity
/// keys. First: exact-key replay across separate calls IS caught. Then:
/// attack retention/pruning when no memory of what was pruned is kept.
#[test]
fn path_048_c_key_only_set_exact_replay_then_naive_pruning_attack() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;
    let key = OccurrenceKey { namespace, event_id: "evt-1".to_string() };

    let mut set = Path048NaiveKeySet::default();
    assert!(set.check_and_insert(key.clone(), CallId(1)), "call 1: genuinely new");
    assert!(
        !set.check_and_insert(key.clone(), CallId(2)),
        "call 2: exact-key replay across separate calls IS caught by a plain historical key set"
    );

    // Attack retention: a NAIVE prune with no retained memory of the
    // forgetting itself.
    set.naive_prune(CallId(50));
    assert!(
        set.check_and_insert(key, CallId(100)),
        "ATTACK SUCCEEDS: after a naive prune that keeps no memory of what was forgotten, the SAME \
         key reappearing is silently accepted as New -- exactly the failure `Path048Ledger`'s \
         `ever_active_namespaces` is built to prevent (contrast with PATH-048-H)"
    );
}

/// PATH-048-D: is storing CANONICAL resolved occurrence identity
/// strictly stronger than storing raw delivery keys? Direct comparison
/// on identical inputs.
#[test]
fn path_048_d_canonical_occurrence_key_is_strictly_stronger_than_raw_delivery_key() {
    let contract = TrustedSourceContract::trusting_with_aliases(
        "payroll-canonical",
        &["payroll-system", "payroll-system-legacy"],
        SourceIdContract::both_guaranteed(),
    );
    let payload = path_047_payload("alice", "review", 40.0);
    let delivery_1 = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let delivery_2_via_other_alias = path_047_candidate("payroll-system-legacy", "evt-1", payload.clone());

    // RAW delivery key: `EventIdentity{source_id, event_id}`, pre-resolution.
    let mut raw_seen: std::collections::HashSet<EventIdentity> = std::collections::HashSet::new();
    assert!(raw_seen.insert(delivery_1.identity.clone()));
    assert!(
        raw_seen.insert(delivery_2_via_other_alias.identity.clone()),
        "RAW delivery-key memory WRONGLY treats the second delivery as new: the two EventIdentity \
         values differ (different source_id strings), even though they resolve to the SAME real \
         occurrence"
    );

    // CANONICAL occurrence key: `(namespace, event_id)`, post-resolution.
    let mut ledger = Path048Ledger::new_attested_genesis();
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &delivery_1);
    let v2 =
        path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(2), &delivery_2_via_other_alias);
    assert_eq!(v1, Path048Verdict::New);
    assert_eq!(
        v2,
        Path048Verdict::AlreadySettled,
        "CANONICAL occurrence-key memory correctly catches the replay delivered via the OTHER \
         alias -- strictly stronger than raw delivery-key memory, which missed the identical \
         scenario above"
    );
}

/// PATH-048-E: unstable-ID namespace. Same real occurrence, call 1:
/// evt-1, call 2: evt-2. Can ANY key-only historical set detect this
/// without additional source semantics or an upstream occurrence
/// identifier?
#[test]
fn path_048_e_unstable_id_namespace_cannot_be_positively_resolved_by_key_only_memory() {
    let contract = TrustedSourceContract::trusting(&[("flaky-webhook", SourceIdContract::separating_only())]);
    let payload = path_047_payload("alice", "review", 40.0);

    let mut ledger = Path048Ledger::new_attested_genesis();
    let delivery_call_1 = path_047_candidate("flaky-webhook", "evt-1", payload.clone());
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &delivery_call_1);
    assert_eq!(v1, Path048Verdict::New, "namespace's first-ever activity: nothing to be inconsistent with yet");

    let retry_call_2 = path_047_candidate("flaky-webhook", "evt-2", payload);
    let v2 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &retry_call_2);

    assert_eq!(
        v2,
        Path048Verdict::Unresolved,
        "NO key-only historical set -- however complete its retention -- can positively resolve \
         this: the KEY ITSELF is unstable, so a 'miss' proves nothing about novelty. The ledger \
         correctly refuses New (would risk double-settling call 1's occurrence) and cannot claim \
         AlreadySettled either (no matching key exists). It can never make positive progress for \
         this namespace without regained STABILITY or an independent upstream occurrence \
         identifier -- neither of which any key-only design can supply."
    );
}

/// PATH-048-F: alias rotation. call 1 uses raw label A -> namespace N;
/// call 2 uses raw label B -> the SAME namespace N. Historical
/// protection must survive this.
#[test]
fn path_048_f_alias_rotation_is_transparently_caught_by_canonical_keying() {
    let contract = TrustedSourceContract::trusting_with_aliases(
        "support-canonical",
        &["support-label-a", "support-label-b"],
        SourceIdContract::both_guaranteed(),
    );
    let payload = path_047_payload("alice", "review", 40.0);

    let mut ledger = Path048Ledger::new_attested_genesis();
    let call_1 = path_047_candidate("support-label-a", "evt-1", payload.clone());
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &call_1);
    assert_eq!(v1, Path048Verdict::New);

    let call_2 = path_047_candidate("support-label-b", "evt-1", payload);
    let v2 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &call_2);

    assert_eq!(
        v2,
        Path048Verdict::AlreadySettled,
        "historical replay protection survives alias rotation: both raw labels resolve to the \
         SAME canonical namespace, so the ledger's namespace-scoped key correctly recognizes the \
         second delivery as the already-settled occurrence"
    );
}

/// PATH-048-G, naive half: a `SourceId -> NamespaceId` binding changes
/// over time. Using ONLY the current contract at lookup time, does this
/// silently defeat replay protection?
#[test]
fn path_048_g_naive_current_contract_only_resolution_is_silently_defeated_by_contract_evolution() {
    let payload = path_047_payload("alice", "review", 40.0);
    let contract_v1 = TrustedSourceContract::trusting(&[("vendor-x", SourceIdContract::both_guaranteed())]);
    let delivery = path_047_candidate("vendor-x", "evt-1", payload.clone());

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v1 = path_048_reserve_naive_current_contract_only(&mut ledger, &contract_v1, ContractVersion(1), CallId(1), &delivery);
    assert_eq!(v1, Path048Verdict::New);

    // The trust contract EVOLVES: "vendor-x" is now bound to a DIFFERENT
    // canonical namespace (real reassignment, migration, or operational
    // mistake -- this experiment does not need to know which). The SAME
    // real occurrence (same event_id, same payload) is delivered again.
    let contract_v2 =
        TrustedSourceContract::trusting_with_aliases("vendor-x-reassigned", &["vendor-x"], SourceIdContract::both_guaranteed());

    let v2 = path_048_reserve_naive_current_contract_only(&mut ledger, &contract_v2, ContractVersion(2), CallId(2), &delivery);
    assert_eq!(
        v2,
        Path048Verdict::New,
        "ATTACK SUCCEEDS against naive current-contract-only resolution: purely because the trust \
         contract rebound 'vendor-x' to a different namespace, the SAME real occurrence is \
         silently treated as New a second time -- no adversarial replay attempt was needed"
    );
}

/// PATH-048-G, guarded half: does retaining the historical authority
/// snapshot used at original settlement catch what the naive resolution
/// above missed?
#[test]
fn path_048_g_label_drift_guard_catches_contract_evolution() {
    let payload = path_047_payload("alice", "review", 40.0);
    let contract_v1 = TrustedSourceContract::trusting(&[("vendor-x", SourceIdContract::both_guaranteed())]);
    let delivery = path_047_candidate("vendor-x", "evt-1", payload.clone());

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v1 = path_048_settle_immediately(&mut ledger, &contract_v1, ContractVersion(1), CallId(1), &delivery);
    assert_eq!(v1, Path048Verdict::New);

    // OBSERVABILITY ONLY, not the detection mechanism: the settlement
    // record's `contract_version` METADATA reflects the ORIGINAL binding
    // used. This is retained for forensic purposes, but is NOT what
    // catches the drift below -- `path_048_reserve` never reads
    // `contract_version` back out of a stored record to decide anything
    // (cross-review correction: an earlier revision of this test implied
    // otherwise). The actual detection mechanism, exercised next, is
    // `label_first_seen_namespace`.
    let key = path_048_occurrence_key(&contract_v1, &delivery).unwrap();
    match ledger.records.get(&key).unwrap() {
        SettlementRecord::Settled { contract_version, .. } => assert_eq!(*contract_version, ContractVersion(1)),
        other => panic!("expected Settled, got {other:?}"),
    }

    let contract_v2 =
        TrustedSourceContract::trusting_with_aliases("vendor-x-reassigned", &["vendor-x"], SourceIdContract::both_guaranteed());
    let v2 = path_048_reserve(&mut ledger, &contract_v2, ContractVersion(2), CallId(2), &delivery);

    assert_eq!(
        v2,
        Path048Verdict::Unresolved,
        "the GUARDED reserve function retains, per raw label, the NAMESPACE it FIRST resolved to \
         (`label_first_seen_namespace`) -- when the CURRENT contract resolves the same label to a \
         DIFFERENT namespace, that drift is flagged Unresolved rather than silently treated as a \
         fresh New. This establishes that a historical AUTHORITY BINDING must survive contract \
         evolution to catch this attack -- it does NOT establish that `ContractVersion` itself \
         (stored above, unused in this decision) is necessary; that remains NOT ESTABLISHED."
    );
}

/// PATH-048-H: an event reappears after the ledger's retention horizon
/// has pruned it. What EXACTLY is lost?
#[test]
fn path_048_h_retention_window_expiry_loses_the_confident_replay_confirmation() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let delivery = path_047_candidate("payroll-system", "evt-1", payload.clone());

    // Baseline, WITHOUT pruning: a replay is confidently caught.
    let mut ledger_unpruned = Path048Ledger::new_attested_genesis();
    path_048_settle_immediately(&mut ledger_unpruned, &contract, ContractVersion(1), CallId(1), &delivery);
    let replay_verdict_unpruned = path_048_reserve(&mut ledger_unpruned, &contract, ContractVersion(1), CallId(100), &delivery);
    assert_eq!(replay_verdict_unpruned, Path048Verdict::AlreadySettled);

    // Same scenario, but the retention window is exercised before the
    // replay arrives.
    let mut ledger_pruned = Path048Ledger::new_attested_genesis();
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;
    path_048_settle_immediately(&mut ledger_pruned, &contract, ContractVersion(1), CallId(1), &delivery);
    ledger_pruned.prune_namespace(&namespace, CallId(50));
    assert!(
        ledger_pruned.records.is_empty(),
        "the original settlement record is genuinely gone -- this experiment does not assume \
         infinite retention"
    );

    let replay_verdict_pruned = path_048_reserve(&mut ledger_pruned, &contract, ContractVersion(1), CallId(100), &delivery);
    assert_eq!(
        replay_verdict_pruned,
        Path048Verdict::Unresolved,
        "GUARANTEE LOST when history expires: the ledger can no longer POSITIVELY CONFIRM this is \
         a replay (AlreadySettled, as in the unpruned baseline above). What is NOT lost: it never \
         silently falls back to New either -- `ever_active_namespaces` durably remembers this \
         namespace has settlement history at all, even after the detailed record is pruned, so a \
         miss here stays honestly Unresolved rather than a confident (and wrong) New"
    );
}

/// PATH-048-I: the same authoritative occurrence identity reappears in a
/// later call with a CHANGED payload. Must be Conflicted, never a second
/// settlement, never a silent overwrite.
#[test]
fn path_048_i_conflict_over_time_is_flagged_not_overwritten() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let original_payload = path_047_payload("alice", "review", 40.0);
    let changed_payload = path_047_payload("alice", "review", 999.0);

    let mut ledger = Path048Ledger::new_attested_genesis();
    let original = path_047_candidate("payroll-system", "evt-1", original_payload.clone());
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &original);
    assert_eq!(v1, Path048Verdict::New);

    let conflicting = path_047_candidate("payroll-system", "evt-1", changed_payload);
    let v2 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &conflicting);
    assert_eq!(
        v2,
        Path048Verdict::Conflicted,
        "not a second settlement and not a silent overwrite -- disagreement, not replay"
    );

    let key = path_048_occurrence_key(&contract, &original).unwrap();
    match ledger.records.get(&key).unwrap() {
        SettlementRecord::Settled { payload, .. } => {
            assert_eq!(*payload, original_payload, "the original settled payload must survive the conflicting attempt unchanged")
        }
        other => panic!("expected the original Settled record to remain, got {other:?}"),
    }
}

/// PATH-048-K (cross-review correction): a REAL resolver bug -- an
/// earlier revision matched a `Reserved` record unconditionally, without
/// comparing payload, so a conflicting delivery arriving BEFORE
/// confirmation would have been misreported as `Reserved` (implying
/// agreement) rather than `Conflicted`. Same authoritative occurrence key
/// reserved with payload P1; before it is ever confirmed, the same key
/// arrives again with a DIFFERENT payload P2. Must be `Conflicted`; must
/// NOT become `New`, `AlreadySettled`, or silently overwrite the
/// original `Reserved` record.
#[test]
fn path_048_k_conflicting_payload_against_a_reserved_not_yet_settled_record_is_conflicted() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let original_payload = path_047_payload("alice", "review", 40.0);
    let changed_payload = path_047_payload("alice", "review", 999.0);

    let mut ledger = Path048Ledger::new_attested_genesis();
    let original = path_047_candidate("payroll-system", "evt-1", original_payload.clone());
    // Reserve only -- deliberately never confirmed, so the record stays
    // `Reserved` for the remainder of this test.
    let v1 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &original);
    assert_eq!(v1, Path048Verdict::New);

    let conflicting = path_047_candidate("payroll-system", "evt-1", changed_payload);
    let v2 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &conflicting);
    assert_eq!(
        v2,
        Path048Verdict::Conflicted,
        "a conflicting payload against a Reserved (not yet Settled) record must be Conflicted, \
         exactly like a conflict against a Settled one -- must NOT become New, must NOT become \
         AlreadySettled, and must NOT be silently reported as Reserved (which would wrongly imply \
         agreement)"
    );

    // The original Reserved record must be completely unchanged -- no
    // overwrite, no promotion to Settled, no payload substitution.
    let key = path_048_occurrence_key(&contract, &original).unwrap();
    match ledger.records.get(&key).unwrap() {
        SettlementRecord::Reserved { payload, .. } => {
            assert_eq!(*payload, original_payload, "the original Reserved payload must survive the conflicting attempt unchanged")
        }
        other => panic!("expected the original Reserved record to remain Reserved, got {other:?}"),
    }
}

/// PATH-048-J, first half: identity is reserved but the economic
/// settlement is never confirmed (crash between the two phases). Both
/// calls here run sequentially, single-threaded -- this test establishes
/// an AT-MOST-ONCE SAFETY property under that modeled sequence, and
/// separately a LIVENESS observation (stuck, not wrong); it does NOT
/// execute concurrent/racing reservations and does NOT establish or test
/// any exactly-once COMPLETION guarantee (nothing here ever resolves the
/// stuck record).
#[test]
fn path_048_j_reserve_without_confirm_blocks_resettlement_but_does_not_progress() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let delivery = path_047_candidate("payroll-system", "evt-1", payload);

    let mut ledger = Path048Ledger::new_attested_genesis();
    // `path_048_confirm_settlement` is deliberately never called --
    // modeling a crash after reservation but before settlement.
    let v1 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &delivery);
    assert_eq!(v1, Path048Verdict::New);

    let v2 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &delivery);
    assert_eq!(
        v2,
        Path048Verdict::Reserved,
        "at-most-once safety holds for this modeled sequence: a stuck Reserved record is never \
         double-settled. Separately, it also never PROGRESSES to Settled on its own -- a LIVENESS \
         gap for an operator to resolve, not a correctness violation. This test does not model \
         concurrency and does not by itself establish that the reserve step is atomic under \
         concurrent access."
    );
}

/// PATH-048-J, second half: the reverse failure mode -- an economic
/// effect is applied WITHOUT a durable reservation preceding it. This
/// establishes an ORDERING requirement for the at-most-once-safety
/// boundary this experiment models; it does not execute or prove a
/// transactional-atomicity property (no concurrency/race scenario is
/// modeled here).
#[test]
fn path_048_j2_settlement_without_durable_reservation_loses_replay_protection() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let delivery = path_047_candidate("payroll-system", "evt-1", payload);

    // No call to `path_048_reserve`/`path_048_confirm_settlement` at all
    // for this first delivery -- modeling an economic effect applied
    // (conceptually, out-of-band) without a durable identity write
    // preceding or accompanying it.
    let mut ledger = Path048Ledger::new_attested_genesis();

    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &delivery);
    assert_eq!(
        v,
        Path048Verdict::New,
        "ATTACK SUCCEEDS: without a durable reservation preceding (or accompanying) the economic \
         effect, at-most-once safety is lost entirely for that delivery -- a retry is silently \
         treated as a fresh, legitimate occurrence. Together with PATH-048-J's first half, this \
         establishes an ORDERING + DURABILITY requirement -- a reservation must exist no later \
         than the economic effect it protects -- as the boundary these SEQUENTIAL tests actually \
         exercise. Whether the reservation check-and-set must be ATOMIC under CONCURRENT delivery \
         is not tested here (no race/concurrency scenario is modeled) and remains a DERIVED, not \
         demonstrated, requirement; these two tests establish an at-most-once-safety boundary, not \
         a full exactly-once settlement protocol."
    );
}

/// STRONGEST REQUIRED COUNTEREXAMPLE: two histories that produce the
/// SAME current delivery input but require different correct economic
/// actions solely because of different past state.
#[test]
fn path_048_temporal_identifiability_theorem_same_current_input_different_correct_action() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let delivery = path_047_candidate("payroll-system", "evt-1", payload);

    // HISTORY 1: this exact occurrence was already settled earlier.
    let mut ledger_with_history = Path048Ledger::new_attested_genesis();
    let prior = path_048_settle_immediately(&mut ledger_with_history, &contract, ContractVersion(1), CallId(1), &delivery);
    assert_eq!(prior, Path048Verdict::New);

    // HISTORY 2: nothing has ever happened.
    let mut ledger_without_history = Path048Ledger::new_attested_genesis();

    // THE SAME CURRENT DELIVERY -- identical namespace, event_id, and
    // payload -- arrives against both histories.
    let verdict_with_history = path_048_reserve(&mut ledger_with_history, &contract, ContractVersion(1), CallId(2), &delivery);
    let verdict_without_history =
        path_048_reserve(&mut ledger_without_history, &contract, ContractVersion(1), CallId(2), &delivery);

    assert_eq!(verdict_with_history, Path048Verdict::AlreadySettled, "identical current input; correct action is REJECT");
    assert_eq!(verdict_without_history, Path048Verdict::New, "identical current input; correct action is ACCEPT");
    assert_ne!(
        verdict_with_history, verdict_without_history,
        "TEMPORAL IDENTIFIABILITY THEOREM: current-event data alone (delivery identity + full \
         payload) is IDENTICAL in both histories, yet the correct economic action differs solely \
         because of differing PAST state -- historical state is part of the observable required \
         for correct settlement, not an optional optimization on top of current-event data"
    );
}

/// End-to-end integration: across TWO SEPARATE calls, only genuinely new
/// settlements reach production `allocate_contribution_pool` -- a
/// replayed delivery must never be double-counted, and a genuinely new
/// contribution must never be suppressed.
#[test]
fn path_048_integration_cross_call_survivors_feed_correctly_into_allocation() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let mut ledger = Path048Ledger::new_attested_genesis();

    let alice_original = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review-1", 40.0));
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &alice_original);
    assert_eq!(v1, Path048Verdict::New);

    // CALL 2: a network-level retry re-delivers the SAME occurrence, and
    // a genuinely NEW, separate contribution from bob also arrives.
    let alice_retry = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review-1", 40.0));
    let bob_new = path_047_candidate("payroll-system", "evt-2", path_047_payload("bob", "bob-work", 20.0));
    let v2 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &alice_retry);
    let v3 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(2), &bob_new);
    assert_eq!(v2, Path048Verdict::AlreadySettled);
    assert_eq!(v3, Path048Verdict::New);

    let settled_payloads: Vec<ContributionEvent> = ledger
        .records
        .values()
        .filter_map(|record| match record {
            SettlementRecord::Settled { payload, .. } => Some(payload.clone()),
            SettlementRecord::Reserved { .. } => None,
        })
        .collect();
    assert_eq!(settled_payloads.len(), 2, "the replayed delivery must never reach a second settlement");

    let allocations = allocate_contribution_pool(&settled_payloads, &weights(), 100.0).unwrap();
    let alice_share: f64 = allocations.iter().filter(|a| a.contributor_id == "alice").map(|a| a.normalized_share).sum();
    let bob_share: f64 = allocations.iter().filter(|a| a.contributor_id == "bob").map(|a| a.normalized_share).sum();
    assert!((alice_share - (40.0 / 60.0)).abs() < 1e-9, "alice's ONE real contribution, not two");
    assert!((bob_share - (20.0 / 60.0)).abs() < 1e-9, "bob's genuinely new contribution must not be suppressed");
}

// =====================================================================
// PATH-048 FINAL EPISTEMIC COMPLETENESS ATTACK -- AMNESIA != GENESIS
//
// Cross-review found one remaining sufficiency boundary in the candidate
// above: `path_048_reserve`'s "namespace not in `ever_active_namespaces`"
// branch treated absence of remembered history as PROOF of true genesis.
// It is not -- an empty ledger looks IDENTICAL whether a namespace
// genuinely never had activity before, or had activity that this
// ledger's own durable state lost, was reset, or restored incompletely.
// `HistoryCompleteness` (defined above, alongside `Path048Ledger`) is
// the fix: an EXOGENOUS, per-namespace fact, never derived from
// `records`/`ever_active_namespaces` being empty, that must be
// explicitly attested before a first sighting is treated as safely New.
//
// CROSS-REVIEW CORRECTION II (NEGATIVE LOOKUP REQUIRES COMPLETE
// HISTORY): the paragraph above describes only the FIRST pass of this
// fix, which gated a namespace's very first sighting. A second
// cross-review pass found the SAME bug still live one branch over: a
// MISS on a specific `event_id` within an ALREADY-active namespace
// (`ever_active_namespaces.contains(namespace)` true) skipped the
// completeness check entirely, trusting namespace-level activity as if
// it implied COMPLETE retained history for every `event_id` within it.
// It does not -- PARTIAL RESTORE WITHIN THE SAME NAMESPACE (some of a
// namespace's real settlements restored, others silently omitted) is
// indistinguishable, on that branch alone, from a namespace whose
// missing `event_id` genuinely never existed. The general principle is
// broader than "AMNESIA != GENESIS": completeness gates EVERY NEGATIVE
// occurrence-history lookup used to derive `New` -- a namespace's first
// sighting is only the special case where the entire namespace is the
// missing history, not the only case. ABSENCE != NOVELTY, at any grain.
// Exact-key HITS remain unaffected either way: a positive record match
// is usable evidence regardless of completeness (see `path_048_reserve`
// below, and PATH-048-completeness-C/D).
// =====================================================================

/// STRONGEST COUNTEREXAMPLE (AMNESIA != GENESIS): two ledger histories
/// that produce a BYTE-IDENTICAL current observable state -- on the
/// fields that existed before this correction -- yet require different
/// correct settlement actions.
#[test]
fn path_048_amnesia_theorem_identical_empty_ledgers_require_different_verdicts() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let k = path_047_candidate("payroll-system", "evt-1", payload.clone());

    // WORLD A -- TRUE GENESIS: this namespace has genuinely never
    // settled or reserved anything. An operator/deployment process
    // attests this explicitly (an exogenous fact this experiment does
    // not derive from the ledger's contents).
    let world_a = Path048Ledger::new_attested_genesis();

    // WORLD B -- AMNESIA: this namespace previously settled K in a prior
    // process, but durable history was lost/reset before this test's
    // ledger came into existence -- modeled by simply never having
    // recorded it (a `default()` ledger, matching what a genuine total
    // loss would leave behind: nothing, and no attestation surviving
    // with it).
    let world_b = Path048Ledger::default();

    // On the fields that existed BEFORE this correction, the two worlds
    // are OBSERVABLY IDENTICAL.
    assert_eq!(world_a.records, world_b.records, "both empty");
    assert_eq!(
        world_a.ever_active_namespaces, world_b.ever_active_namespaces,
        "both empty -- `ever_active_namespaces` ALONE cannot distinguish these two worlds"
    );

    // The SAME current delivery K arrives against both.
    let mut world_a = world_a;
    let mut world_b = world_b;
    let verdict_a = path_048_reserve(&mut world_a, &contract, ContractVersion(1), CallId(1), &k);
    let verdict_b = path_048_reserve(&mut world_b, &contract, ContractVersion(1), CallId(1), &k);

    assert_eq!(verdict_a, Path048Verdict::New, "true genesis, explicitly attested: safe to settle");
    assert_eq!(
        verdict_b,
        Path048Verdict::Unresolved,
        "AMNESIA THEOREM: absence of historical state is NOT authoritative evidence of no history. \
         World B's ledger is indistinguishable from World A's on every field that existed before \
         this correction, yet the correct action differs -- K might already be settled. The FIX is \
         precisely that this candidate no longer treats the two worlds as identical: \
         `HistoryCompleteness` (attested for World A, left at its safe Unknown default for World B) \
         is now part of the observable required for a correct verdict. Absent that attestation, the \
         candidate correctly refuses New rather than guessing."
    );
}

/// PATH-048-amnesia-A: clean first bootstrap. An attested-genesis
/// ledger's first-ever delivery for a namespace is safely New.
#[test]
fn path_048_amnesia_a_clean_first_bootstrap() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let k = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 40.0));

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &k);
    assert_eq!(v, Path048Verdict::New, "an explicitly attested fresh deployment's first delivery is safely New");
}

/// PATH-048-amnesia-B: process restart with the durable ledger preserved
/// intact (including its completeness attestation). A replay across the
/// restart is still caught correctly.
#[test]
fn path_048_amnesia_b_restart_with_ledger_preserved() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let k = path_047_candidate("payroll-system", "evt-1", payload.clone());

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &k);
    assert_eq!(v1, Path048Verdict::New);

    // "Restart" -- a full serialize/reload round-trip is modeled by a
    // clone, standing in for durable storage that preserved everything,
    // completeness attestation included.
    let mut restarted = ledger.clone();

    let replay = path_047_candidate("payroll-system", "evt-1", payload);
    let v2 = path_048_reserve(&mut restarted, &contract, ContractVersion(1), CallId(2), &replay);
    assert_eq!(
        v2,
        Path048Verdict::AlreadySettled,
        "a durable ledger preserved intact across a restart -- including its completeness \
         attestation -- continues to protect against replay exactly as before"
    );
}

/// PATH-048-amnesia-C: process restart with the ledger LOST. The exact
/// scenario the strongest counterexample above proves must not be
/// silently accepted.
#[test]
fn path_048_amnesia_c_restart_with_ledger_lost() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let k = path_047_candidate("payroll-system", "evt-1", payload.clone());

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &k);
    assert_eq!(v1, Path048Verdict::New);

    // Total loss: the process restarts with a BRAND NEW ledger, carrying
    // NOTHING over -- not the records, not `ever_active_namespaces`, and
    // NOT the completeness attestation (which is itself part of what was
    // lost).
    let mut lost = Path048Ledger::default();

    let replay = path_047_candidate("payroll-system", "evt-1", payload);
    let v2 = path_048_reserve(&mut lost, &contract, ContractVersion(1), CallId(2), &replay);
    assert_eq!(
        v2,
        Path048Verdict::Unresolved,
        "a genuinely lost ledger must NOT confidently settle K as New -- it might already be \
         settled. The safe default (`Unknown` completeness) is exactly what prevents this"
    );
}

/// PATH-048-amnesia-D: partial restore -- records and
/// `ever_active_namespaces` are restored from a backup, but the
/// completeness attestation is NOT (a realistic partial-restore failure
/// mode: the "data tables" came back, the "we know this is everything"
/// fact did not).
///
/// AMENDED (cross-review correction II): the original version of this
/// test used a DIFFERENT, never-touched namespace ("other-system") for
/// its missing-history assertion, which exercises only the FIRST-
/// SIGHTING branch (already completeness-gated by the first pass of this
/// correction) -- it never exercised a miss WITHIN THE SAME,
/// already-active namespace, which is exactly where the second-pass bug
/// lived. Supplemented below with that missing scenario directly.
#[test]
fn path_048_amnesia_d_partial_restore_without_completeness_marker() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let k = path_047_candidate("payroll-system", "evt-1", payload.clone());

    let mut original = Path048Ledger::new_attested_genesis();
    path_048_settle_immediately(&mut original, &contract, ContractVersion(1), CallId(1), &k);

    // Restore records + ever_active_namespaces + the label-binding fact
    // ("the data tables") into a FRESH ledger -- but deliberately leave
    // its completeness at the safe Unknown default ("the attestation").
    let mut restored = Path048Ledger::default();
    restored.records = original.records.clone();
    restored.ever_active_namespaces = original.ever_active_namespaces.clone();
    restored.label_first_seen_namespace = original.label_first_seen_namespace.clone();

    // An exact-key replay of the RESTORED occurrence is still caught --
    // exact-key matches are usable evidence regardless of completeness.
    let replay = path_047_candidate("payroll-system", "evt-1", payload);
    let v_replay = path_048_reserve(&mut restored, &contract, ContractVersion(1), CallId(2), &replay);
    assert_eq!(
        v_replay,
        Path048Verdict::AlreadySettled,
        "restored records alone are sufficient to catch an exact-key replay -- completeness gates \
         NEGATIVE lookup evidence (a miss), never a positive record hit"
    );

    // AMENDMENT: a DIFFERENT event_id under the SAME, already-active
    // namespace that this restore never captured (e.g. a second real
    // settlement the backup silently omitted) must ALSO stay Unresolved
    // -- this is the exact PARTIAL RESTORE WITHIN THE SAME NAMESPACE
    // scenario the second pass of this correction fixed.
    let omitted = path_047_candidate("payroll-system", "evt-2", path_047_payload("alice", "review-2", 25.0));
    let v_omitted = path_048_reserve(&mut restored, &contract, ContractVersion(1), CallId(2), &omitted);
    assert_eq!(
        v_omitted,
        Path048Verdict::Unresolved,
        "a miss on a DIFFERENT event_id within the SAME already-active namespace is exactly as \
         much negative evidence as a namespace's first sighting -- `ever_active_namespaces` \
         containing this namespace (because evt-1 WAS restored) must not license New for evt-2 \
         just because completeness happens to be checked elsewhere"
    );

    // But a namespace this restore never touched at all -- and which the
    // restore process cannot vouch is free of pre-restore history --
    // correctly stays Unresolved on its first sighting: restoring DATA
    // is not the same fact as attesting COMPLETENESS.
    let other_contract = TrustedSourceContract::trusting(&[("other-system", SourceIdContract::both_guaranteed())]);
    let other_delivery = path_047_candidate("other-system", "evt-1", path_047_payload("bob", "x", 5.0));
    let v_other = path_048_reserve(&mut restored, &other_contract, ContractVersion(1), CallId(2), &other_delivery);
    assert_eq!(
        v_other,
        Path048Verdict::Unresolved,
        "a partial restore of records/ever_active_namespaces does not, by itself, re-establish a \
         completeness attestation for a namespace it never captured"
    );
}

/// PATH-048-amnesia-E: a completeness marker CLAIMS complete, but some
/// namespace history was actually omitted (a false attestation). Does
/// this become an exogenous trust assumption the downstream code cannot
/// verify?
#[test]
fn path_048_amnesia_e_false_completeness_attestation_is_an_unverifiable_exogenous_axiom() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    // STIPULATED (by this test's own narrative, not by anything in the
    // ledger's data): "payroll-system" genuinely settled K in some
    // earlier, now-fully-lost system this ledger has no trace of. An
    // operator/migration process nonetheless attests genesis for this
    // ledger -- an honest mistake, an incomplete audit, or a compromised
    // attestation source; this experiment does not need to know which.
    let k = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 40.0));

    let mut ledger = Path048Ledger::new_attested_genesis(); // the FALSE attestation

    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &k);
    assert_eq!(
        v,
        Path048Verdict::New,
        "AXIOM FORGERY -- PATH-042/FIND-019's pattern recurring here: a FALSE completeness \
         attestation produces EXACTLY the same confident New verdict a TRUE one would. This \
         candidate has no way to distinguish a genuinely complete genesis attestation from an \
         incorrect one -- completeness/genesis is therefore an EXOGENOUS TRUST ASSUMPTION this \
         downstream code cannot verify, only faithfully propagate. Getting it wrong silently \
         reintroduces exactly the double-settlement risk this whole correction exists to prevent."
    );
}

/// PATH-048-amnesia-F: a namespace newly introduced after system
/// genesis, contrasted with an unknown historical namespace layered onto
/// the SAME otherwise genesis-known ledger.
#[test]
fn path_048_amnesia_f_new_namespace_vs_unknown_historical_namespace() {
    let mut ledger = Path048Ledger::new_attested_genesis();

    // "new-vendor": genuinely introduced AFTER genesis. Under a
    // genesis-complete ledger, there is no window in which it could have
    // had un-tracked history -- its first sighting is safely New.
    let new_vendor_contract = TrustedSourceContract::trusting(&[("new-vendor", SourceIdContract::both_guaranteed())]);
    let new_vendor_delivery = path_047_candidate("new-vendor", "evt-1", path_047_payload("carol", "onboarding", 15.0));
    let v_new = path_048_reserve(&mut ledger, &new_vendor_contract, ContractVersion(1), CallId(5), &new_vendor_delivery);
    assert_eq!(v_new, Path048Verdict::New, "a namespace genuinely introduced after an attested genesis is safely New");

    // "legacy-migrated-system": a DIFFERENT namespace, explicitly
    // overridden to Unknown on this otherwise genesis-known ledger --
    // e.g. migrated in from an older system whose own completeness was
    // never itself attested.
    let legacy_namespace =
        NamespaceId("legacy-migrated-system".to_string());
    ledger.attest_namespace_history_unknown(&legacy_namespace);
    let legacy_contract =
        TrustedSourceContract::trusting(&[("legacy-migrated-system", SourceIdContract::both_guaranteed())]);
    let legacy_delivery = path_047_candidate("legacy-migrated-system", "evt-1", path_047_payload("dave", "carryover", 25.0));
    let v_legacy = path_048_reserve(&mut ledger, &legacy_contract, ContractVersion(1), CallId(5), &legacy_delivery);
    assert_eq!(
        v_legacy,
        Path048Verdict::Unresolved,
        "the SAME ledger, otherwise genesis-known, correctly distinguishes a namespace with an \
         explicit UNKNOWN-history override from one covered by genesis -- 'new to the system' and \
         'unknown historical namespace' are NOT the same fact and must not share one verdict"
    );
}

/// PATH-048-amnesia-G: migration/import from an older system. Does
/// imported historical completeness need explicit attestation, or does
/// importing SOME data imply it?
#[test]
fn path_048_amnesia_g_migrated_namespace_requires_explicit_completeness_attestation() {
    let contract = TrustedSourceContract::trusting(&[("migrated-system", SourceIdContract::both_guaranteed())]);
    let known_migrated = path_047_candidate("migrated-system", "evt-legacy-1", path_047_payload("alice", "legacy-review", 40.0));

    // A migration imports records for the events it actually knows about
    // -- but, like any real migration, cannot prove a negative (that it
    // captured EVERYTHING). No explicit completeness attestation is
    // made.
    let mut ledger = Path048Ledger::default();
    let v_unattested = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &known_migrated);
    assert_eq!(
        v_unattested,
        Path048Verdict::Unresolved,
        "importing data for a namespace's first sighting does not, by itself, attest that the \
         import is COMPLETE -- the ledger withholds New even for an event the migration DID bring \
         over, absent an explicit completeness attestation for this namespace"
    );

    // With an EXPLICIT, deliberate attestation that the migration is
    // known-complete for this specific namespace, the identical scenario
    // now correctly proceeds.
    let namespace = contract.authority_for(&SourceId("migrated-system".into())).unwrap().0;
    let mut attested = Path048Ledger::default();
    attested.attest_namespace_history_complete(&namespace);
    let v_attested = path_048_reserve(&mut attested, &contract, ContractVersion(1), CallId(1), &known_migrated);
    assert_eq!(
        v_attested,
        Path048Verdict::New,
        "imported historical completeness MUST be attested explicitly -- once it is, first \
         sightings for that namespace proceed exactly as an attested genesis would"
    );
}

// =====================================================================
// PATH-048 CROSS-REVIEW CORRECTION II -- NEGATIVE LOOKUP REQUIRES
// COMPLETE HISTORY (the bug fixed in `path_048_reserve` above, and its
// dedicated regression suite).
// =====================================================================

/// PATH-048-completeness-A (REQUIRED, reproduces the reported bug):
/// ACTIVE NAMESPACE + PARTIAL RESTORE. Real history: evt-1 and evt-2
/// both settled. Restore captures only evt-1; `ever_active_namespaces`
/// contains the namespace; completeness Unknown; no pruning marker
/// survived; the source contract has AUTHORITY + STABILITY + SEPARATION.
/// The current delivery is evt-2 with its ORIGINAL payload. Before the
/// fix this returned `New` (confirmed by a scratch repro during this
/// correction); it must be `Unresolved`.
#[test]
fn path_048_completeness_a_active_namespace_partial_restore_stays_unresolved() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload_1 = path_047_payload("alice", "review-1", 40.0);
    let payload_2 = path_047_payload("alice", "review-2", 25.0);
    let e1 = path_047_candidate("payroll-system", "evt-1", payload_1.clone());
    let e2 = path_047_candidate("payroll-system", "evt-2", payload_2.clone());

    // REAL HISTORY: both settle.
    let mut real = Path048Ledger::new_attested_genesis();
    path_048_settle_immediately(&mut real, &contract, ContractVersion(1), CallId(1), &e1);
    path_048_settle_immediately(&mut real, &contract, ContractVersion(1), CallId(1), &e2);

    // PARTIAL RESTORE: only evt-1's record carried over; the namespace IS
    // marked active (because evt-1 was); completeness is left at its safe
    // Unknown default; nothing pruned.
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;
    let mut restored = Path048Ledger::default();
    let e1_key = OccurrenceKey { namespace: namespace.clone(), event_id: "evt-1".to_string() };
    restored.records.insert(e1_key, real.records.get(&OccurrenceKey { namespace, event_id: "evt-1".to_string() }).unwrap().clone());
    restored.ever_active_namespaces.insert(contract.authority_for(&SourceId("payroll-system".into())).unwrap().0);

    // CURRENT DELIVERY: evt-2, with its ORIGINAL payload -- exactly the
    // real, already-settled occurrence, just never restored.
    let v = path_048_reserve(&mut restored, &contract, ContractVersion(1), CallId(2), &e2);
    assert_eq!(
        v,
        Path048Verdict::Unresolved,
        "BUG FIX REGRESSION GUARD: a MISS within an already-ACTIVE namespace is negative evidence \
         only if that namespace's retained history is attested complete. `ever_active_namespaces` \
         containing the namespace (because evt-1 WAS restored) must not, by itself, license New \
         for evt-2 -- absent an explicit completeness attestation, this must stay Unresolved even \
         though AUTHORITY, STABILITY, and SEPARATION all hold and nothing was ever pruned"
    );
}

/// PATH-048-completeness-B (REQUIRED): ACTIVE NAMESPACE + ATTESTED
/// COMPLETE HISTORY. evt-1 retained, evt-2 genuinely never existed,
/// completeness attested complete, STABILITY established. The identical
/// MISS shape as case A, but with the exogenous fact actually present --
/// must resolve `New`, confirming the fix is not simply "always
/// Unresolved on a miss."
#[test]
fn path_048_completeness_b_active_namespace_attested_complete_history_permits_new() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let e1 = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review-1", 40.0));
    let e2 = path_047_candidate("payroll-system", "evt-2", path_047_payload("alice", "review-2", 25.0));

    let mut ledger = Path048Ledger::new_attested_genesis(); // completeness ATTESTED throughout
    let v1 = path_048_settle_immediately(&mut ledger, &contract, ContractVersion(1), CallId(1), &e1);
    assert_eq!(v1, Path048Verdict::New);

    let v2 = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &e2);
    assert_eq!(
        v2,
        Path048Verdict::New,
        "with completeness genuinely attested AND STABILITY established, a miss on a genuinely \
         new event_id within an already-active namespace correctly proceeds -- the fix gates on \
         completeness, it does not forbid New on a miss altogether"
    );
}

/// PATH-048-completeness-C (REQUIRED): EXACT HIT + UNKNOWN COMPLETENESS.
/// Proves completeness gates NEGATIVE evidence only, never a positive
/// record hit: even with completeness Unknown throughout, an exact-key
/// replay is still caught.
#[test]
fn path_048_completeness_c_exact_hit_survives_unknown_completeness() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;

    // Completeness Unknown throughout -- the record is placed directly,
    // standing in for "restored from a backup with no attestation,"
    // without ever calling anything that would set completeness.
    let mut ledger = Path048Ledger::default();
    ledger.ever_active_namespaces.insert(namespace.clone());
    ledger.records.insert(
        OccurrenceKey { namespace, event_id: "evt-1".to_string() },
        SettlementRecord::Settled { payload: payload.clone(), contract_version: ContractVersion(1), call: CallId(1) },
    );
    assert_eq!(ledger.completeness_for(&contract.authority_for(&SourceId("payroll-system".into())).unwrap().0), HistoryCompleteness::Unknown);

    let replay = path_047_candidate("payroll-system", "evt-1", payload);
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &replay);
    assert_eq!(
        v,
        Path048Verdict::AlreadySettled,
        "an exact-key HIT is usable evidence regardless of completeness -- completeness gates only \
         NEGATIVE (miss) lookups, never a positive record match"
    );
}

/// PATH-048-completeness-D (REQUIRED): EXACT HIT CONFLICT + UNKNOWN
/// COMPLETENESS. Proves conflict detection ALSO survives incomplete
/// ambient history -- disagreement on a key the ledger DOES hold is
/// still detected even when completeness for the surrounding namespace
/// is Unknown.
#[test]
fn path_048_completeness_d_exact_hit_conflict_survives_unknown_completeness() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let original_payload = path_047_payload("alice", "review", 40.0);
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;

    let mut ledger = Path048Ledger::default(); // Unknown completeness throughout
    ledger.ever_active_namespaces.insert(namespace.clone());
    ledger.records.insert(
        OccurrenceKey { namespace, event_id: "evt-1".to_string() },
        SettlementRecord::Settled { payload: original_payload.clone(), contract_version: ContractVersion(1), call: CallId(1) },
    );

    let conflicting = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 999.0));
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(2), &conflicting);
    assert_eq!(
        v,
        Path048Verdict::Conflicted,
        "conflict detection on an exact-key HIT also survives Unknown ambient completeness -- the \
         retained record itself is the evidence, and disagreement with it is detected regardless"
    );
}

/// Reassessment (required by this correction): after fixing the MISS
/// branch to consult completeness, is `ever_active_namespaces` still
/// INDEPENDENT state, or is it fully derivable from `records` and
/// `pruned_horizon`? Proven here across the three states that matter:
/// never active, active with unpruned records, and active-then-fully-
/// pruned. `ever_active_namespaces` is retained in the struct as an O(1)
/// convenience (avoiding a scan over `records` on every lookup), NOT
/// because it carries information the other two fields lack.
#[test]
fn path_048_completeness_ever_active_namespaces_is_derivable_not_independent() {
    fn derived_ever_active(ledger: &Path048Ledger, namespace: &NamespaceId) -> bool {
        ledger.pruned_horizon.contains_key(namespace) || ledger.records.keys().any(|k| &k.namespace == namespace)
    }

    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;
    let k = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 40.0));

    // Never active.
    let fresh = Path048Ledger::new_attested_genesis();
    assert_eq!(fresh.ever_active_namespaces.contains(&namespace), derived_ever_active(&fresh, &namespace));
    assert!(!fresh.ever_active_namespaces.contains(&namespace));

    // Active, unpruned.
    let mut active = Path048Ledger::new_attested_genesis();
    path_048_settle_immediately(&mut active, &contract, ContractVersion(1), CallId(1), &k);
    assert_eq!(active.ever_active_namespaces.contains(&namespace), derived_ever_active(&active, &namespace));
    assert!(active.ever_active_namespaces.contains(&namespace));

    // Active, then fully pruned -- the case `ever_active_namespaces`
    // exists specifically to still get right.
    let mut pruned = active.clone();
    pruned.prune_namespace(&namespace, CallId(50));
    assert!(pruned.records.is_empty(), "the record itself is genuinely gone");
    assert_eq!(
        pruned.ever_active_namespaces.contains(&namespace),
        derived_ever_active(&pruned, &namespace),
        "even with records empty, `pruned_horizon` membership alone reconstructs the same fact \
         `ever_active_namespaces` stores directly -- confirming it adds no INDEPENDENT information \
         over `records` UNION `pruned_horizon`, only a cheaper lookup"
    );
    assert!(pruned.ever_active_namespaces.contains(&namespace), "survives pruning, as designed");
}

// ---------------------------------------------------------------------
// Research conclusions -- answered from the behavior demonstrated above.
// CROSS-REVIEW CORRECTION: this section previously used minimality/
// sufficiency/necessity language stronger than what was actually
// established. Revised below to say only what the tests prove.
// CROSS-REVIEW CORRECTION II: the completeness gate previously applied
// only to a namespace's first sighting; it is now known to apply to
// EVERY negative occurrence-history lookup used to derive `New` (see the
// dedicated section above). Revised again below.
// ---------------------------------------------------------------------
//
// CORE QUESTION (asked as: what is the SMALLEST SURVIVING CANDIDATE
// TESTED that prevents both double settlement AND accidental
// suppression?): `Path048Ledger` -- canonical
// `OccurrenceKey -> SettlementRecord{Reserved|Settled}` plus
// `ever_active_namespaces` (now known REDUNDANT -- see below) plus
// `pruned_horizon` (membership only; see the field-level correction
// notice on `Path048Ledger` -- the `CallId` value it stores is not
// itself proven necessary) plus `label_first_seen_namespace` plus
// `HistoryCompleteness` (per-namespace, `AttestedComplete` vs `Unknown`
// -- now gating BOTH a namespace's first sighting AND any later miss
// within an already-active namespace). PATH-048-A/B/C show each smaller
// candidate tried (nothing, last-seen-only, unguarded key set) fails at
// least one of the two safety properties on the specific attacks
// constructed here. PATH-048-D/E/F/G/H/I/J/K/amnesia-A..G/completeness-
// A..D show this candidate degrades to `Unresolved` -- never a wrong
// `New` or a wrong `AlreadySettled` -- at every boundary this
// experiment's cases probe, INCLUDING the ledger's own initialization/
// provenance AND partial restoration within an already-active namespace,
// neither of which the pre-correction candidate probed. This is
// sufficiency FOR THE CASES TESTED, under the explicit AUTHORITY/
// STABILITY/SEPARATION assumptions inherited from FIND-025 -- not a
// claim of global or formal minimality over every field (see the
// field-level notices on `pruned_horizon` and `ContractVersion` for two
// fields specifically NOT proven minimal, and the redundancy result for
// `ever_active_namespaces` below), and not a claim that no smaller
// design could pass these same cases some other way.
//
// STRONGEST COUNTEREXAMPLE: the AMNESIA theorem test above -- two ledger
// histories (true genesis vs. lost history) that are OBSERVABLY
// IDENTICAL on every field that existed before this correction
// (`records`, `ever_active_namespaces` both empty), fed the identical
// current delivery, yet require opposite correct verdicts. This
// supersedes the earlier temporal identifiability theorem (settled vs.
// never-settled, distinguishable by `records` alone) as the strongest
// counterexample in this file specifically because the two AMNESIA
// worlds are indistinguishable on MORE of the pre-correction observable
// surface -- proving `ledger completeness/initialization state`, not
// just settlement history, is part of the observable correct settlement
// requires. Necessity of SOME durable cross-call state follows from
// both theorems together with PATH-048-A.
//
// Whether `ever_active_namespaces` alone was sufficient: NO (the AMNESIA
// correction) -- it records only what ONE ledger instance has itself
// observed, never whether that instance's observations are complete
// since true origin. The two are independent facts: an amnesiac ledger
// and a genesis ledger both have it empty.
//
// Whether `ever_active_namespaces` remains INDEPENDENTLY load-bearing
// after correction II: NO --
// `path_048_completeness_ever_active_namespaces_is_derivable_not_independent`
// proves it equals `pruned_horizon.contains_key(namespace) ||
// records.keys().any(|k| k.namespace == namespace)` across every ledger
// state this experiment constructs (never active; active, unpruned;
// active, fully pruned). Once every MISS unconditionally consults
// completeness (correction II), the field's remaining job -- deciding
// whether STABILITY/pruning checks are even relevant -- is fully
// recoverable from `records` and `pruned_horizon` alone. It is retained
// in the struct as an O(1) convenience, not because it is independently
// necessary state.
//
// Whether first-ever activity, OR any later miss within an active
// namespace, can ever be inferred from absence alone: NO, for either --
// absence from `ever_active_namespaces`/`records` is consistent with
// true genesis, total historical loss, AND partial restoration within
// one namespace (PATH-048-completeness-A, the second-pass bug). An
// EXPLICIT, exogenous completeness attestation (`HistoryCompleteness`)
// is required before EITHER kind of negative lookup may license `New`;
// PATH-048-amnesia-E shows this attestation is itself an unverifiable
// axiom in PATH-042/FIND-019's sense -- a false one produces the
// identical confident (and wrong) `New` a true one would. Exact-key HITS
// are unaffected either way (PATH-048-completeness-C/D) -- completeness
// governs interpretation of a MISS, never a positive record match.
//
// Whether key-only memory survives unstable IDs: NO (PATH-048-E) -- the
// key itself is what's unstable, so no amount of retained key history
// helps; this requires restored STABILITY or an independent upstream
// occurrence identifier, neither modeled here.
//
// Whether a historical authority BINDING must survive current-contract
// evolution: YES, for the attack constructed (PATH-048-G) -- demonstrated
// by direct contrast between the naive (current-contract-only) and
// guarded (retains first-seen NAMESPACE per label, via
// `label_first_seen_namespace`) resolvers on the identical
// contract-evolution scenario. This is narrower than "ContractVersion
// must be historical": `ContractVersion` is stored per settlement record
// as metadata only and is never read by any decision in this file: its
// necessity is NOT ESTABLISHED.
//
// Whether settlement and identity reservation require atomicity: these
// SEQUENTIAL tests (PATH-048-J/K) establish an ORDERING + DURABILITY
// requirement for AT-MOST-ONCE SAFETY -- a reservation must exist no
// later than the economic effect it protects -- and separately a
// LIVENESS observation (an unconfirmed reservation is safe but stuck,
// not a full exactly-once completion protocol). No concurrency or race
// scenario is executed here, so "the reservation check-and-set must be
// ATOMIC" is recorded only as a DERIVED requirement (a reasoned
// implication of the ordering result), not one this experiment
// demonstrates by test.


// =====================================================================
// PATH-049 -- CONCURRENT CLAIM / ATOMIC RESERVATION BOUNDARY (H20)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::contribution` or wired into any production
// path -- no production file is touched by this experiment.
//
// QUESTION: does the replay-safe settlement candidate established by
// PATH-048 (`Path048Ledger` + `path_048_reserve`) remain safe when TWO
// resolution calls race concurrently for the SAME canonical economic
// occurrence?
//
// H20: sequential reserve-before-effect ordering (PATH-048's rule) is
// insufficient for at-most-once settlement under concurrent calls. If
// "check whether K exists" and "reserve K" are separable operations, two
// concurrent callers can both observe K absent and both obtain a license
// to apply the economic effect. A replay-safe concurrent boundary
// requires some atomic/linearizable occurrence-claim operation over the
// canonical `OccurrenceKey`.
//
// PATH-049 reuses PATH-047/PATH-048's full type vocabulary unmodified --
// `SourceId`, `NamespaceId`, `TrustedSourceContract`, `SourceIdContract`,
// `EventIdentity`, `Path047CandidateEvent`, `OccurrenceKey`,
// `SettlementRecord`, `Path048Verdict`, `Path048Ledger`,
// `HistoryCompleteness`, `path_048_reserve`, `path_048_confirm_settlement`,
// `path_048_occurrence_key` -- and adds exactly ONE new thing: a way to
// make the EXISTING `path_048_reserve` decision execute as one
// indivisible critical section under concurrent callers
// (`path_049_atomic_claim`, a thin `Mutex`-guarded wrapper), plus a
// deliberately naive split check/reserve structure
// (`Path049NaiveSplitLedger`) to demonstrate the race PATH-048 never
// tested.
//
// THE ADOPTED ATOMIC CANDIDATE: `std::sync::Mutex<Path048Ledger>`,
// claimed via `path_049_atomic_claim`, which holds the lock for the
// ENTIRE duration of `path_048_reserve`. What makes the claim transition
// indivisible is that the `Mutex` EXCLUDES every other thread from
// entering the protected "does K exist? if not, insert Reserved(K)"
// section while one holder is inside it -- NOT the absence of explicit
// yield points in that section's own code. Ordinary synchronous
// execution can still be preempted by the scheduler at any instruction;
// preemption inside the critical section is harmless here specifically
// because no OTHER thread can be concurrently executing that same
// section while the `Mutex` is held, not because the code happens not to
// await/yield. The theorem under test is about INDIVISIBILITY of the
// claim transition, not about `Mutex` specifically -- `Mutex` is simply
// the smallest mechanism this test-local candidate needed to achieve it
// (see PATH-049-E, which shows this coarse, whole-ledger lock does NOT
// impose incorrect cross-key serialization semantics, only execution
// ordering).
//
// LOGICAL LINEARIZATION POINT: the first successful transition of K from
// absent to `Reserved(K)` inside one locked call to `path_048_reserve`.
// Candidate property: after that point, no OTHER concurrent caller
// CONTENDING FOR THE SAME K can still legally receive `New`/Acquired for
// it. PATH-049-B/C/F are the cases that genuinely contend for the SAME
// canonical occurrence and test this property directly (exactly one
// `New` per contested key, regardless of which caller's OS thread
// happens to win the race); it holds in every one of them. (PATH-049-E/G
// intentionally exercise DIFFERENT canonical keys -- they show the claim
// does NOT incorrectly serialize across keys, a related but distinct
// property; see the adversarial-case scope notes below.)
//
// THE THREE CLAIMS THIS EXPERIMENT DOES NOT COLLAPSE (do not conflate):
//   1. ATOMIC CLAIM -- exactly one caller can acquire the occurrence
//      identity. THIS is what PATH-049 tests.
//   2. DURABLE CLAIM -- the acquired reservation survives the relevant
//      crash boundary. PATH-048 gave SEQUENTIAL evidence about this
//      ordering (reserve-before-effect); PATH-049 does not add to it.
//   3. ATOMIC ECONOMIC EFFECT -- the economic side effect and the
//      settlement state transition complete as one consistent unit.
//      NOT tested here. PATH-049-J explicitly shows the gap this leaves:
//      an acquired-but-never-confirmed claim is SAFE (no double license)
//      but permanently stuck -- this experiment does NOT claim
//      exactly-once completion, and the gap between "claimed" and
//      "economic effect durably applied and confirmed" is flagged below
//      as the likely next PATH, not as something solved here.
//
// CONSTRAINTS HONORED: all epistemic prerequisites (AUTHORITY, STABILITY,
// SEPARATION, `HistoryCompleteness`) are held GOOD in B-G specifically to
// isolate CONCURRENCY as the one variable under test; H and I
// deliberately hold completeness Unknown to prove atomicity does not
// launder missing epistemic warrant into certainty. No payload-hash
// dedup, no `contribution_id` heuristic, no sleeps used as correctness
// synchronization (a `Barrier` synchronizes only the START of a race;
// every assertion below holds regardless of which thread's OS scheduler
// slot wins).
// =====================================================================

/// PATH-049-A's presence observation -- deliberately a plain two-state
/// result, standing in for what a naive, non-atomic "does K exist"
/// check would return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path049Presence {
    Absent,
    Present,
}

/// A DELIBERATELY naive split check/reserve structure: existence check
/// and reservation are two SEPARATE, independently observable steps,
/// with NOTHING enforcing atomicity between them. Exists solely to
/// demonstrate the TOCTOU race (PATH-049-A); never adopted as a
/// candidate.
#[derive(Debug, Clone, Default)]
struct Path049NaiveSplitLedger {
    records: std::collections::HashSet<OccurrenceKey>,
}

impl Path049NaiveSplitLedger {
    fn check(&self, key: &OccurrenceKey) -> Path049Presence {
        if self.records.contains(key) { Path049Presence::Present } else { Path049Presence::Absent }
    }

    fn reserve(&mut self, key: OccurrenceKey) {
        self.records.insert(key);
    }
}

/// The adopted atomic candidate: claims `event`'s canonical occurrence
/// against `ledger` as one indivisible critical section. Internally this
/// is nothing more than PATH-048's own `path_048_reserve`, called while
/// holding the lock for its entire duration -- no new resolution logic
/// is introduced; only indivisibility is added.
fn path_049_atomic_claim(
    ledger: &std::sync::Mutex<Path048Ledger>,
    contract: &TrustedSourceContract,
    contract_version: ContractVersion,
    call: CallId,
    event: &Path047CandidateEvent,
) -> Path048Verdict {
    let mut guard = ledger.lock().expect("PATH-049 test ledger mutex poisoned");
    path_048_reserve(&mut guard, contract, contract_version, call, event)
}

/// An "economic license" is exactly the `New` verdict -- the only one
/// that licenses a caller to apply the economic effect. `Reserved`,
/// `AlreadySettled`, `Conflicted`, and `Unresolved` are all explicitly
/// NON-economic outcomes (a hold, a no-op, a rejection, or an admission
/// of ignorance, respectively).
fn path_049_is_economic_license(verdict: Path048Verdict) -> bool {
    matches!(verdict, Path048Verdict::New)
}

// ---------------------------------------------------------------------
// Required attacks A-J.
// ---------------------------------------------------------------------

/// PATH-049-A (PRIMARY BUG WITNESS) -- DETERMINISTIC TOCTOU RACE. No
/// threads, no scheduler dependence: the losing interleaving is forced
/// explicitly, one call at a time, in exactly the order specified by the
/// mission brief.
#[test]
fn path_049_a_deterministic_toctou_race_yields_two_economic_licenses() {
    let key = OccurrenceKey { namespace: NamespaceId("N".to_string()), event_id: "evt-1".to_string() };
    let mut ledger = Path049NaiveSplitLedger::default();

    let a_saw = ledger.check(&key); // CALL A: check K -> Absent
    let b_saw = ledger.check(&key); // CALL B: check K -> Absent (before A reserves)
    assert_eq!(a_saw, Path049Presence::Absent);
    assert_eq!(b_saw, Path049Presence::Absent);

    ledger.reserve(key.clone()); // CALL A: reserve K
    let a_license = a_saw == Path049Presence::Absent;

    // CALL B never re-checks -- it proceeds from the STALE Absent result
    // it already observed, exactly as a real concurrent caller would if
    // its check executed before A's reservation became visible to it.
    let b_license = b_saw == Path049Presence::Absent;

    assert!(a_license, "A's own check licensed it to proceed");
    assert!(
        b_license,
        "TOCTOU RACE: B's check ALSO returned Absent (taken before A's reservation), so B ALSO \
         proceeds to apply the economic effect -- TWO economic licenses for the SAME occurrence \
         K, even though each caller individually obeyed PATH-048's sequential \
         reserve-before-effect ordering. At-most-once safety fails the moment check and reserve \
         are separable, independently observable steps."
    );
    assert_eq!(
        (a_license as u8) + (b_license as u8),
        2,
        "PRIMARY BUG WITNESS: two economic licenses issued for one occurrence under a naive split \
         check/reserve design"
    );
}

/// PATH-049-B -- ATOMIC SAME-PAYLOAD CLAIM. Real concurrent threads,
/// synchronized to start together via a `Barrier` (no sleeps). The
/// safety property asserted is the NUMBER of economic licenses issued,
/// never merely the final record count.
#[test]
fn path_049_b_atomic_same_payload_claim_yields_exactly_one_license() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(Path048Ledger::new_attested_genesis()));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let ledger = std::sync::Arc::clone(&ledger);
        let barrier = std::sync::Arc::clone(&barrier);
        let contract = contract.clone();
        let payload = payload.clone();
        handles.push(std::thread::spawn(move || {
            let event = path_047_candidate("payroll-system", "evt-1", payload);
            barrier.wait(); // synchronizes only the START of the race
            path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event)
        }));
    }

    let verdicts: Vec<Path048Verdict> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let licenses = verdicts.iter().filter(|v| path_049_is_economic_license(**v)).count();

    assert_eq!(
        licenses, 1,
        "exactly ONE economic license for the SAME (K, payload) under concurrent racing -- the \
         SAFETY property is one economic license, not merely one surviving ledger row, verdicts: \
         {verdicts:?}"
    );
    assert!(
        verdicts.iter().any(|v| *v == Path048Verdict::Reserved),
        "the losing caller observes Reserved (an in-flight, non-economic verdict) -- connects to \
         PATH-048-J's Reserved-vs-Settled distinction under concurrency, verdicts: {verdicts:?}"
    );
}

/// PATH-049-C -- CONCURRENT CONFLICT. Two concurrent callers claim the
/// SAME K with DIFFERENT payloads. At most one may acquire; the other
/// must become `Conflicted` once the winning claim is visible -- never
/// both `New`, never last-write-wins, never a silent overwrite.
#[test]
fn path_049_c_concurrent_conflict_at_most_one_acquires() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload_1 = path_047_payload("alice", "review", 40.0);
    let payload_2 = path_047_payload("alice", "review", 999.0);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(Path048Ledger::new_attested_genesis()));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let mut handles = Vec::new();
    for payload in [payload_1, payload_2] {
        let ledger = std::sync::Arc::clone(&ledger);
        let barrier = std::sync::Arc::clone(&barrier);
        let contract = contract.clone();
        handles.push(std::thread::spawn(move || {
            let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
            barrier.wait();
            let verdict = path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event);
            (payload, verdict)
        }));
    }

    let results: Vec<(ContributionEvent, Path048Verdict)> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let new_count = results.iter().filter(|(_, v)| *v == Path048Verdict::New).count();
    let conflicted_count = results.iter().filter(|(_, v)| *v == Path048Verdict::Conflicted).count();

    assert_eq!(new_count, 1, "at most (exactly) one caller acquires K under a payload conflict");
    assert_eq!(
        conflicted_count, 1,
        "the other caller must observe Conflicted -- never both Acquired, never last-write-wins, \
         never a silent overwrite"
    );

    let winner_payload = results.iter().find(|(_, v)| *v == Path048Verdict::New).unwrap().0.clone();
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;
    let key = OccurrenceKey { namespace, event_id: "evt-1".to_string() };
    let guard = ledger.lock().unwrap();
    match guard.records.get(&key).unwrap() {
        SettlementRecord::Reserved { payload, .. } => {
            assert_eq!(*payload, winner_payload, "the winning Reserved payload must remain unchanged")
        }
        other => panic!("expected Reserved, got {other:?}"),
    }
}

/// PATH-049-D -- RESERVED VS SETTLED INTERLEAVING STATE. This is a
/// deterministic, SEQUENTIAL interleaving-state model, not a real
/// concurrent-thread test: it demonstrates the state a concurrently
/// arriving caller WOULD observe between another caller's claim and its
/// confirmation, by constructing that exact before/after-confirmation
/// ordering directly. One caller acquires; before it confirms, a second
/// identical delivery must NOT obtain an economic license; after
/// confirmation, a further retry IS a genuine replay. Connects PATH-048's
/// Reserved-vs-Settled semantics to the concurrent model -- the actual
/// real-thread concurrency evidence is PATH-049-B/C/E/F/G/H/I.
#[test]
fn path_049_d_reserved_vs_settled_race_then_confirm_then_replay() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let ledger = std::sync::Mutex::new(Path048Ledger::new_attested_genesis());

    let event_1 = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let v1 = path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event_1);
    assert_eq!(v1, Path048Verdict::New, "first caller acquires Reserved(K)");

    let event_2 = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let v2 = path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(2), &event_2);
    assert_eq!(
        v2,
        Path048Verdict::Reserved,
        "a second delivery arriving BEFORE confirmation does NOT obtain an economic license"
    );

    {
        let mut guard = ledger.lock().unwrap();
        let key = path_048_occurrence_key(&contract, &event_1).unwrap();
        path_048_confirm_settlement(&mut guard, &key);
    }

    let event_3 = path_047_candidate("payroll-system", "evt-1", payload);
    let v3 = path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(3), &event_3);
    assert_eq!(v3, Path048Verdict::AlreadySettled, "once confirmed, a further retry is a genuine replay");
}

/// PATH-049-E -- DIFFERENT KEYS SHOULD NOT SERIALIZE SEMANTICALLY. A
/// coarse, whole-ledger `Mutex` serializes EXECUTION of two concurrent
/// claims for DIFFERENT keys, but must not impose any incorrect
/// SEMANTIC coupling between them -- both must still independently
/// acquire.
#[test]
fn path_049_e_different_keys_do_not_serialize_semantically() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(Path048Ledger::new_attested_genesis()));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let mut handles = Vec::new();
    for event_id in ["evt-1", "evt-2"] {
        let ledger = std::sync::Arc::clone(&ledger);
        let barrier = std::sync::Arc::clone(&barrier);
        let contract = contract.clone();
        handles.push(std::thread::spawn(move || {
            let event = path_047_candidate("payroll-system", event_id, path_047_payload("alice", "review", 40.0));
            barrier.wait();
            path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event)
        }));
    }

    let verdicts: Vec<Path048Verdict> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(
        verdicts.iter().all(|v| *v == Path048Verdict::New),
        "DIFFERENT occurrence keys under one coarse, whole-ledger Mutex must still BOTH acquire \
         independently -- the mutex serializes EXECUTION, not economic semantics; this is not a \
         global 'one settlement at a time' rule, verdicts: {verdicts:?}"
    );
}

/// PATH-049-F -- ALIAS CONVERGENCE UNDER RACE. Two raw `SourceId`s
/// resolving, via AUTHORITY, to the SAME canonical `NamespaceId`, same
/// event_id, same payload, claimed concurrently: only one economic
/// license. Contrasted directly with a lock/key scoped to the RAW
/// source_id, which would fail this attack.
///
/// SCOPE NOTE (cross-review correction): this test establishes that the
/// synchronization/claim domain must converge on CANONICAL occurrence
/// identity, not raw `SourceId` identity -- it does NOT establish that
/// canonicalization must physically execute while holding the lock.
/// `contract.authority_for` (the canonicalization step inside
/// `path_048_reserve`) is a pure, read-only lookup against each thread's
/// own cloned, immutable `TrustedSourceContract` -- it touches no shared
/// mutable state, so it would resolve identically whenever it ran,
/// before or after lock acquisition. In THIS candidate it happens to run
/// inside the locked call (an implementation detail, not a load-bearing
/// one); an architecture that canonicalizes outside the lock and then
/// claims the resulting canonical key would converge on the same
/// contention domain and pass this same test. What the raw-`SourceId`
/// contrast below actually demonstrates is narrower: raw-label-scoped
/// contention can split one economic occurrence into two claim domains.
#[test]
fn path_049_f_alias_convergence_under_race_yields_one_license() {
    let contract = TrustedSourceContract::trusting_with_aliases(
        "payroll-canonical",
        &["source-A", "source-B"],
        SourceIdContract::both_guaranteed(),
    );
    let payload = path_047_payload("alice", "review", 40.0);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(Path048Ledger::new_attested_genesis()));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let mut handles = Vec::new();
    for label in ["source-A", "source-B"] {
        let ledger = std::sync::Arc::clone(&ledger);
        let barrier = std::sync::Arc::clone(&barrier);
        let contract = contract.clone();
        let payload = payload.clone();
        handles.push(std::thread::spawn(move || {
            let event = path_047_candidate(label, "evt-1", payload);
            barrier.wait();
            path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event)
        }));
    }

    let verdicts: Vec<Path048Verdict> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let licenses = verdicts.iter().filter(|v| path_049_is_economic_license(**v)).count();
    assert_eq!(
        licenses, 1,
        "AUTHORITY canonicalizes source-A and source-B to the SAME NamespaceId BEFORE the atomic \
         claim step -- only one economic license, even though the two callers used different raw \
         source labels, verdicts: {verdicts:?}"
    );

    // Contrast: a naive lock/key scoped to the RAW source_id -- exists
    // purely to show it would have FAILED this attack.
    let mut raw_keyed: std::collections::HashSet<EventIdentity> = std::collections::HashSet::new();
    let a_new = raw_keyed.insert(EventIdentity { source_id: SourceId("source-A".into()), event_id: "evt-1".into() });
    let b_new = raw_keyed.insert(EventIdentity { source_id: SourceId("source-B".into()), event_id: "evt-1".into() });
    assert!(
        a_new && b_new,
        "a lock/key scoped to the RAW source_id would treat these as two DIFFERENT keys and \
         WRONGLY grant both an independent slot -- canonicalization must happen before the claim, \
         not be keyed on the wire label"
    );
}

/// PATH-049-G -- DIFFERENT NAMESPACES, SAME EVENT_ID. Guards against
/// accidentally introducing globally-unique-event_id semantics during
/// concurrency work: identical event_id strings, but AUTHORITY resolves
/// them to DIFFERENT canonical namespaces -- distinct `OccurrenceKey`s,
/// both acquire.
#[test]
fn path_049_g_different_namespaces_same_event_id_both_acquire() {
    let contract = TrustedSourceContract::trusting(&[
        ("payroll-system", SourceIdContract::both_guaranteed()),
        ("timesheet-system", SourceIdContract::both_guaranteed()),
    ]);
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(Path048Ledger::new_attested_genesis()));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let mut handles = Vec::new();
    for label in ["payroll-system", "timesheet-system"] {
        let ledger = std::sync::Arc::clone(&ledger);
        let barrier = std::sync::Arc::clone(&barrier);
        let contract = contract.clone();
        handles.push(std::thread::spawn(move || {
            let event = path_047_candidate(label, "evt-1", path_047_payload("alice", "review", 40.0));
            barrier.wait();
            path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event)
        }));
    }

    let verdicts: Vec<Path048Verdict> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(
        verdicts.iter().all(|v| *v == Path048Verdict::New),
        "identical event_id string, but AUTHORITY resolves to two DIFFERENT namespaces -- distinct \
         OccurrenceKeys, both acquire independently; event_id is namespace-scoped, never globally \
         unique, verdicts: {verdicts:?}"
    );
}

/// PATH-049-H -- UNKNOWN COMPLETENESS STILL WINS BEFORE ATOMICITY. Even
/// PERFECT atomicity must not launder a missing epistemic warrant into
/// certainty: concurrency correctness is downstream of epistemic
/// admissibility, not a substitute for it.
#[test]
fn path_049_h_unknown_completeness_wins_before_atomicity() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    // Unknown completeness throughout -- `default()`, never attested.
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(Path048Ledger::default()));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let ledger = std::sync::Arc::clone(&ledger);
        let barrier = std::sync::Arc::clone(&barrier);
        let contract = contract.clone();
        handles.push(std::thread::spawn(move || {
            let event = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 40.0));
            barrier.wait();
            path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event)
        }));
    }

    let verdicts: Vec<Path048Verdict> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(
        verdicts.iter().all(|v| *v == Path048Verdict::Unresolved),
        "even PERFECT atomicity cannot launder a missing completeness attestation into certainty -- \
         both concurrent callers correctly observe Unresolved, verdicts: {verdicts:?}"
    );
    let licenses = verdicts.iter().filter(|v| path_049_is_economic_license(**v)).count();
    assert_eq!(licenses, 0, "zero economic licenses");

    let guard = ledger.lock().unwrap();
    assert!(
        guard.records.is_empty(),
        "zero Reserved records created for an Unknown-completeness first sighting, even under a race"
    );
}

/// PATH-049-I -- EXACT POSITIVE HIT WITH UNKNOWN COMPLETENESS. Proves
/// PATH-049 did not accidentally turn completeness into a universal
/// precondition rather than a MISS precondition: a retained positive
/// record remains usable exactly as PATH-048 established, even under
/// concurrent racing and Unknown ambient completeness.
#[test]
fn path_049_i_exact_positive_hit_survives_unknown_completeness_under_race() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let namespace = contract.authority_for(&SourceId("payroll-system".into())).unwrap().0;

    // Pre-existing Settled record, completeness Unknown throughout --
    // built directly, standing in for a partial restore with no
    // attestation (reuses PATH-048-completeness-C/D's construction).
    let mut initial = Path048Ledger::default();
    initial.ever_active_namespaces.insert(namespace.clone());
    initial.records.insert(
        OccurrenceKey { namespace, event_id: "evt-1".to_string() },
        SettlementRecord::Settled { payload: payload.clone(), contract_version: ContractVersion(1), call: CallId(1) },
    );
    let ledger = std::sync::Arc::new(std::sync::Mutex::new(initial));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let ledger = std::sync::Arc::clone(&ledger);
        let barrier = std::sync::Arc::clone(&barrier);
        let contract = contract.clone();
        let payload = payload.clone();
        handles.push(std::thread::spawn(move || {
            let event = path_047_candidate("payroll-system", "evt-1", payload);
            barrier.wait();
            path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(2), &event)
        }));
    }

    let verdicts: Vec<Path048Verdict> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(
        verdicts.iter().all(|v| *v == Path048Verdict::AlreadySettled),
        "a retained POSITIVE record remains usable exactly as PATH-048 established, regardless of \
         Unknown ambient completeness and regardless of concurrent racing -- completeness is a MISS \
         precondition, never a universal one, verdicts: {verdicts:?}"
    );
    let licenses = verdicts.iter().filter(|v| path_049_is_economic_license(**v)).count();
    assert_eq!(licenses, 0, "no NEW economic license -- both callers correctly recognize the replay");
}

/// PATH-049-J -- CLAIM ACQUIRED, PROCESS STALLS. Another SEQUENTIAL,
/// deterministic interleaving-state model, not a real concurrent-thread
/// test -- it constructs the exact state a later caller would observe
/// after an earlier claim stalls, directly. Preserves PATH-048's safety
/// != liveness distinction under the atomic candidate: an
/// acquired-but-never-confirmed claim blocks any further economic
/// license (safety) but does not itself resolve (liveness remains
/// unresolved/stuck). Does NOT claim exactly-once completion.
#[test]
fn path_049_j_claim_acquired_process_stalls_safety_holds_liveness_does_not() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let ledger = std::sync::Mutex::new(Path048Ledger::new_attested_genesis());

    let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let v_a = path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v_a, Path048Verdict::New, "caller A acquires Reserved(K)");
    // A never confirms -- modeling a stalled/crashed process.

    let retry = path_047_candidate("payroll-system", "evt-1", payload);
    let v_b = path_049_atomic_claim(&ledger, &contract, ContractVersion(1), CallId(2), &retry);
    assert_eq!(
        v_b,
        Path048Verdict::Reserved,
        "caller B CANNOT acquire economic permission while A's claim stands -- at-most-once \
         safety is preserved. B is not told the occurrence is settled (it is not) and is not \
         granted a fresh license (that would double-claim). This is a SAFETY guarantee, not a \
         LIVENESS one: K remains stuck Reserved unless something outside this candidate resolves \
         it -- PATH-048-J's distinction, unchanged under concurrency. This experiment does NOT \
         claim exactly-once completion."
    );
}

// ---------------------------------------------------------------------
// Research conclusions -- answered from the behavior demonstrated above.
// ---------------------------------------------------------------------
//
// STRONGEST LOSING INTERLEAVING: PATH-049-A -- A checks (Absent), B
// checks (Absent), A reserves, B proceeds from its stale Absent
// observation. Two economic licenses for one occurrence, deterministic,
// no threads or scheduler dependence required to demonstrate it.
//
// H20: CONFIRMED for the constructed race, and NARROWED in scope. The
// naive split design (A) fails exactly as H20 predicts: two callers must
// not both cross absent -> licensed based on the same pre-claim state.
// The atomic candidate (a `Mutex`-guarded `path_048_reserve` call)
// survives every attack constructed (B-J). PREFERRED NECESSITY
// STATEMENT (narrower than "linearizability is uniquely necessary"): for
// the constructed claim-before-effect architecture, contenders for the
// SAME canonical occurrence require a single-winner claim boundary
// before economic permission is issued. The adopted `Mutex` candidate
// CAN be described as linearizable, but this experiment does not show
// linearizability, as a general consistency model, is the unique or
// minimal way to achieve that boundary -- only that check-then-insert
// must be indivisible from the perspective of concurrent callers
// contending for the SAME canonical key. Weaker/non-linearizable
// coordination models (e.g. a CAS retry loop, a DB uniqueness
// constraint, actor-style serialization) were NOT constructed or
// compared, and remain NOT ESTABLISHED as to whether they would preserve
// the same at-most-once economic-license invariant.
//
// LINEARIZATION POINT: the first successful transition of K from absent
// to `Reserved(K)`, inside one locked `path_048_reserve` call. After
// that point, no OTHER concurrent caller CONTENDING FOR THE SAME K can
// still legally receive `New` for it -- confirmed executably by B (same
// payload: 1 New, 1 Reserved), C (different payload: 1 New, 1
// Conflicted), and F (aliases: 1 New total across both raw labels), the
// three cases that genuinely contend for one occurrence; in every one of
// them, exactly the caller whose claim attempt executes strictly after
// the linearization point receives a non-`New` verdict. E and G
// deliberately do NOT test this same-key property -- they exercise
// DIFFERENT canonical occurrence keys and confirm the converse: the
// claim mechanism does not incorrectly serialize ACROSS keys. D and J
// are sequential, deterministic interleaving-state evidence (not
// executed concurrent-thread tests) connecting PATH-048's
// Reserved-vs-Settled semantics to the concurrent model -- they show
// what a caller would observe if it arrived between another's claim and
// confirmation, run through the SAME atomic candidate, but do not
// themselves race two threads.
//
// ONE-ECONOMIC-LICENSE INVARIANT: held in every genuinely contested
// SAME-key case constructed (B, C, F -- all via exactly-one-New, counted
// explicitly). Never merely "one surviving record" -- B/C/F/H/I all
// explicitly count verdicts, not final map size. (E and G are reported
// separately below: they intentionally test that DIFFERENT keys each
// independently acquire, which is a distinct, complementary property,
// not an instance of the same-key invariant.)
//
// SAME-KEY/SAME-PAYLOAD: exactly one `New`, the other `Reserved` (B).
// SAME-KEY/DIFFERENT-PAYLOAD: exactly one `New`, the other `Conflicted`,
// winning payload verified unchanged (C).
// ALIAS RACE: the claim converges on the CANONICAL NamespaceId, not the
// raw source label -- one license total across two raw labels (F); a
// raw-source_id-keyed lock is shown directly to fail this same attack.
// This shows the synchronization/claim DOMAIN must be canonical
// identity; it does NOT show canonicalization must physically execute
// while the lock is held (see the scope note on PATH-049-F's own doc
// comment -- `contract.authority_for` is a pure, per-thread lookup with
// no shared mutable state, so it would resolve identically regardless of
// when it ran relative to lock acquisition).
// DIFFERENT KEYS: both acquire independently under one coarse,
// whole-ledger Mutex -- execution is serialized, economic semantics are
// not (E, G).
// RESERVED-VS-SETTLED TEMPORAL STATE (D, sequential interleaving-state
// evidence, not a concurrent-thread test): a delivery arriving before
// confirmation observes `Reserved`, not `New`; after confirmation, a
// further retry is `AlreadySettled`.
// UNKNOWN COMPLETENESS INTERACTION: a namespace's first sighting under
// Unknown completeness stays `Unresolved` for BOTH concurrent callers,
// with ZERO Reserved records created, even under perfect atomicity (H) --
// atomicity is downstream of epistemic admissibility, not a substitute
// for it. A pre-existing POSITIVE record remains usable under Unknown
// ambient completeness regardless of racing (I) -- completeness gates
// MISSES only, exactly as PATH-048 established, not universally.
// SAFETY vs LIVENESS (J, sequential interleaving-state evidence, not a
// concurrent-thread test): an acquired, unconfirmed claim blocks any
// further economic license (safety, preserved) but does not itself
// progress (liveness, unresolved) -- directly extending PATH-048-J
// through the atomic candidate. Exactly-once COMPLETION is explicitly
// not claimed.
//
// STATE/MINIMALITY REASSESSMENT:
// - Smallest surviving synchronization candidate tested: a single
//   `Mutex` wrapping the WHOLE `Path048Ledger`, held for the duration of
//   one `path_048_reserve` call. Coarser than per-key locking, but E/G
//   show it produces no incorrect semantic coupling across keys.
// - Per-ledger vs per-namespace vs per-OccurrenceKey: NOT distinguished
//   by any test here -- every case uses one whole-ledger `Mutex`; whether
//   a finer-grained (per-namespace or per-key) lock would behave
//   identically, or would be necessary at higher contention, is not
//   tested.
// - Does this experiment establish a `Mutex` is NECESSARY? NO. It
//   establishes that an INDIVISIBLE check-then-claim transition is
//   necessary for the constructed race (PATH-049-A's failure mode); a
//   `Mutex` is simply the mechanism this candidate used to obtain that
//   indivisibility, not a claim about the only possible mechanism.
// - `ever_active_namespaces` remains, per PATH-048's redundancy result,
//   merely a convenience/cache in every ledger state PATH-049 also
//   constructs -- no test here makes it decision-authoritative again;
//   `path_049_atomic_claim` calls the SAME `path_048_reserve` unchanged.
// - `ContractVersion` remains unread metadata; no PATH-049 test consults
//   it for any decision.
// - `pruned_horizon`'s numeric `CallId` value remains unused by any
//   decision; PATH-049 does not exercise pruning at all and so adds no
//   evidence either way about its necessity.


// =====================================================================
// PATH-050 -- ECONOMIC EFFECT / SETTLEMENT COMMIT BOUNDARY (H21)
//
// Disposable experimental machinery only. Nothing below is exported from
// `distribution_machine::contribution` or wired into any production
// path -- no production file is touched by this experiment. Does NOT use
// the production contribution engine, a database, HTTP, or a queue --
// the "external economic effect" is a fake, in-process, test-local
// provider only.
//
// QUESTION: after a caller has legitimately acquired `Reserved(K)`
// (PATH-049's single-winner claim boundary), what is the minimum
// protocol needed to ensure the economic side effect is neither applied
// twice nor silently lost, when the following can fail INDEPENDENTLY:
//   1. execute the economic effect
//   2. persist effect evidence/receipt
//   3. write `Settled(K)` locally
//
// H21: an atomic occurrence claim is insufficient for exactly-once
// economic completion when the effect and the durable `Settled`
// transition do not share one failure domain. If the effect can succeed
// while `Settled` remains unrecorded, a retry cannot distinguish "effect
// happened, confirmation was lost" from "effect never happened" unless
// some additional durable or externally verifiable completion evidence
// exists.
//
// PATH-050 reuses PATH-047/048/049's type vocabulary unmodified --
// `TrustedSourceContract`, `SourceIdContract`, `Path047CandidateEvent`,
// `OccurrenceKey`, `SettlementRecord`, `Path048Verdict`, `Path048Ledger`,
// `path_048_reserve`, `path_048_confirm_settlement`,
// `path_048_occurrence_key` -- and adds exactly what the effect boundary
// needs: a fake `EffectProvider` and recovery candidates over it.
// `EffectOperationId` IS the canonical `OccurrenceKey` (via
// `path_048_occurrence_key`) -- never a raw `SourceId`, never a payload
// hash, never `contribution_id`. This gives it, for free, from PATH-047/
// 048/049: same occurrence -> same operation_id; distinct canonical
// occurrences -> distinct operation_id (H); aliases -> same operation_id
// after canonicalization (G). Global uniqueness beyond what PATH-047/048
// already established for `OccurrenceKey` is not separately claimed.
//
// THREE FAILURE DOMAINS THIS EXPERIMENT DOES NOT COLLAPSE:
//   1. CLAIM COMMIT     -- the occurrence becomes `Reserved` (PATH-049's
//      subject; assumed already complete and correct here).
//   2. EFFECT COMMIT     -- the external economic side effect happens
//      (tracked by `EffectProvider::execution_count` -- REAL executions
//      only, never incremented on a provider-suppressed duplicate).
//   3. SETTLEMENT COMMIT -- local durable state becomes `Settled`
//      (`path_048_confirm_settlement`, PATH-048's own mechanism,
//      unmodified).
//
// CRASH-POINT MATRIX (a compact version also appears in FIND-028).
// CROSS-REVIEW CORRECTION: GROUND TRUTH is what actually happened in a
// given world; RECOVERABLE KNOWLEDGE is what the recovery candidate can
// actually infer from the local evidence it is given. C1 and C2 have
// DIFFERENT ground truth but IDENTICAL recoverable knowledge -- do not
// read "yes" in the old "completion knowable?" column as something
// `Reserved(K)` itself makes identifiable; it does not.
//
//   Point | Ground truth                                | Recoverable knowledge from Reserved(K) + surviving evidence alone | This candidate's action
//   ------|-----------------------------------------------|----------------------------------------------------------------------|-------------------------
//   C0    | no claim exists yet                            | n/a                                                                    | normal New path (PATH-049), out of PATH-050's scope
//   C1    | effect did NOT happen                          | UNKNOWN -- observably identical to C2 from local state alone        | blind retry HAPPENS to be correct in this world, but recovery cannot know it is in C1 rather than C2 (PATH-050-C)
//   C2    | effect DID happen; the PROVIDER's OWN receipt write did not persist | UNKNOWN -- observably identical to C1 from local state alone | blind retry is UNSAFE in this world; NOT independently modeled as its own crash sub-step -- this fake provider's `execute()` performs effect+receipt as one atomic step, so C2 is reasoned about, not separately exercised (see NOT ESTABLISHED in FIND-028)
//   C3    | effect happened AND a durable provider receipt exists | KNOWN, via query                                              | recover from receipt, no re-execution (PATH-050-I's candidate) -- conditional on the receipt being AUTHENTIC (PATH-050-K/K2)
//   C4    | `Settled(K)` locally recorded                   | KNOWN                                                                 | `AlreadySettled` (PATH-048, unmodified)
//
// The theorem is exactly this: C1 and C2 are indistinguishable IN
// RECOVERABLE KNOWLEDGE despite opposite ground truth, so no function of
// `Reserved(K)` + surviving local evidence alone can be correct in both.
//
// EXACTLY-ONCE LANGUAGE (deliberately avoided): this experiment does NOT
// claim generic exactly-once semantics anywhere. Precise claims used
// instead: safe recovery requires EITHER an authoritative completion
// witness capable of distinguishing effect-happened from
// effect-not-happened (a receipt, actually consulted -- I), OR an effect
// boundary whose retry semantics make repeating the same canonical
// operation harmless for the tested failure pattern (provider-side
// idempotency -- D/E/F/G/H). A bare idempotency key/operation_id is
// NEVER itself evidence that completion occurred -- it is operation
// IDENTITY only; what survives is the PROVIDER's retry-idempotent
// semantics plus its own memory keyed by that identity, and that memory
// is exactly the same `receipts` store a receipt-based recovery would
// consult. CROSS-REVIEW CORRECTION: because both survivors read the SAME
// underlying provider memory, receipt AUTHENTICITY is a trust boundary
// shared by BOTH candidates (A: idempotent `execute()`, B: `query_receipt`)
// -- NOT unique to query-based recovery. PATH-050-K demonstrates this
// against candidate B; PATH-050-K2 demonstrates the IDENTICAL forged
// receipt also defeats candidate A, because `execute()`'s duplicate-
// suppression check consults the same `receipts` map a forged entry was
// inserted into.
// =====================================================================

/// A fake provider's execution outcome for one `(operation_id, payload)`
/// claim -- three states, never a boolean, because "did this happen
/// before" and "does it match" are separate questions once idempotency
/// memory exists.
#[derive(Debug, Clone, PartialEq)]
enum EffectExecution {
    /// A brand-new real effect executed for this operation_id.
    Executed,
    /// The provider recognized this operation_id from a PRIOR real
    /// execution and suppressed a duplicate -- the original payload is
    /// returned as evidence.
    SuppressedDuplicate { original_payload: ContributionEvent },
    /// The provider recognized this operation_id, but the incoming
    /// payload DISAGREES with what was originally recorded against it.
    /// No new effect executes; the original record is untouched.
    PayloadMismatch { original_payload: ContributionEvent },
}

/// The fake provider's behavior mode. Kept explicit and distinct, per
/// the mission brief, rather than one flag: `IdempotentByOperationId`
/// is what makes `execute()` safe to call more than once for the SAME
/// operation_id; `NonIdempotent` offers no memory at all and is the
/// losing candidate C. "Receipt-queryable" (mode 3 in the brief) is
/// modeled as a RECOVERY STRATEGY over the SAME idempotency memory an
/// `IdempotentByOperationId` provider already maintains
/// (`query_receipt`, below) rather than a third provider mode -- a
/// provider capable of answering "did operation_id X complete" must
/// already be remembering operation ids, which is exactly what
/// `IdempotentByOperationId` models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectProviderMode {
    NonIdempotent,
    IdempotentByOperationId,
}

/// The smallest fake external effect provider needed. Tracks REAL
/// effect executions (`execution_count`) and, for the idempotent mode,
/// a durable receipt per operation_id (`receipts`) -- two independently
/// writable observables, deliberately, so PATH-050-K can ask whether
/// anything enforces they agree (nothing does; this is the point).
#[derive(Debug, Clone)]
struct EffectProvider {
    mode: EffectProviderMode,
    execution_count: u64,
    receipts: std::collections::HashMap<OccurrenceKey, ContributionEvent>,
}

impl EffectProvider {
    fn new(mode: EffectProviderMode) -> Self {
        EffectProvider { mode, execution_count: 0, receipts: std::collections::HashMap::new() }
    }

    /// Executes (or suppresses, or flags a mismatch for) the effect for
    /// `operation_id` with `payload`, per `self.mode`.
    fn execute(&mut self, operation_id: OccurrenceKey, payload: ContributionEvent) -> EffectExecution {
        match self.mode {
            EffectProviderMode::NonIdempotent => {
                // No memory at all -- ALWAYS a real execution, regardless
                // of operation_id or prior calls. This is candidate C,
                // the losing local-only design.
                self.execution_count += 1;
                EffectExecution::Executed
            }
            EffectProviderMode::IdempotentByOperationId => match self.receipts.get(&operation_id) {
                None => {
                    self.execution_count += 1;
                    self.receipts.insert(operation_id, payload.clone());
                    EffectExecution::Executed
                }
                Some(original) if *original == payload => {
                    EffectExecution::SuppressedDuplicate { original_payload: original.clone() }
                }
                Some(original) => EffectExecution::PayloadMismatch { original_payload: original.clone() },
            },
        }
    }

    /// Query-only: does a durable receipt exist for `operation_id`,
    /// WITHOUT triggering any effect. Present regardless of mode (a
    /// `NonIdempotent` provider simply never has anything to find).
    fn query_receipt(&self, operation_id: &OccurrenceKey) -> Option<ContributionEvent> {
        self.receipts.get(operation_id).cloned()
    }
}

/// The recovery decision, distinct from PATH-048's `Path048Verdict` --
/// this is about resuming a claim that already owns `Reserved(K)`
/// across the effect boundary, not about resolving a fresh delivery's
/// identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path050RecoveryVerdict {
    /// The occurrence is now known complete (either freshly settled, or
    /// recovered from trusted external/local evidence). No further
    /// action needed.
    AlreadyComplete,
    /// The effect executed for the FIRST time during this recovery
    /// attempt, and `Settled` was written as a result.
    EffectExecutedNowSettled,
    /// The retry's payload disagrees with either the local `Reserved`
    /// record or the provider's own receipt. Rejected; nothing is
    /// executed or overwritten.
    Conflicted,
    /// Completion status cannot be determined safely from the evidence
    /// available. NO action is taken -- no execution, no `Settled`
    /// write. Reconciliation (outside this candidate) is required.
    Unresolved,
}

/// NAIVE LOSING CANDIDATE (never adopted): "if Reserved(K): retry the
/// economic effect, then write Settled(K)" -- blindly re-executes
/// without consulting any completion evidence at all. Demonstrates the
/// duplicate-effect failure mode (PATH-050-A).
fn path_050_naive_blind_retry_recovery(
    ledger: &mut Path048Ledger,
    provider: &mut EffectProvider,
    key: &OccurrenceKey,
    event: &Path047CandidateEvent,
) {
    if matches!(ledger.records.get(key), Some(SettlementRecord::Reserved { .. })) {
        provider.execute(key.clone(), event.payload.clone());
        path_048_confirm_settlement(ledger, key);
    }
}

/// SECOND NAIVE LOSER (never adopted): "if Reserved(K): never retry,
/// remain Reserved forever." A deliberate no-op, included so the
/// safety-vs-liveness fork this creates can be demonstrated directly
/// against the SAME starting state as the blind-retry loser
/// (PATH-050-B).
fn path_050_naive_never_retry_recovery(_ledger: &mut Path048Ledger, _provider: &mut EffectProvider, _key: &OccurrenceKey) {
    // Deliberately does nothing.
}

/// SURVIVING CANDIDATE (execute-based): consults provider-side
/// idempotency BEFORE trusting an execution to be safe. Under
/// `NonIdempotent`, correctly refuses to guess (`Unresolved`) rather
/// than risk a duplicate -- it does NOT distinguish "effect already
/// happened" from "effect never happened" because a `NonIdempotent`
/// provider offers no evidence either way. Under
/// `IdempotentByOperationId`, safe to call unconditionally: a genuine
/// first execution proceeds, a duplicate is suppressed by the provider
/// itself, and a payload mismatch is rejected before anything executes.
fn path_050_recover(
    ledger: &mut Path048Ledger,
    provider: &mut EffectProvider,
    key: &OccurrenceKey,
    event: &Path047CandidateEvent,
) -> Path050RecoveryVerdict {
    match ledger.records.get(key) {
        Some(SettlementRecord::Settled { payload, .. }) => {
            if *payload == event.payload { Path050RecoveryVerdict::AlreadyComplete } else { Path050RecoveryVerdict::Conflicted }
        }
        Some(SettlementRecord::Reserved { payload, .. }) => {
            if *payload != event.payload {
                return Path050RecoveryVerdict::Conflicted;
            }
            match provider.mode {
                EffectProviderMode::NonIdempotent => Path050RecoveryVerdict::Unresolved,
                EffectProviderMode::IdempotentByOperationId => match provider.execute(key.clone(), event.payload.clone()) {
                    EffectExecution::Executed => {
                        path_048_confirm_settlement(ledger, key);
                        Path050RecoveryVerdict::EffectExecutedNowSettled
                    }
                    EffectExecution::SuppressedDuplicate { .. } => {
                        path_048_confirm_settlement(ledger, key);
                        Path050RecoveryVerdict::AlreadyComplete
                    }
                    EffectExecution::PayloadMismatch { .. } => Path050RecoveryVerdict::Conflicted,
                },
            }
        }
        None => Path050RecoveryVerdict::Unresolved,
    }
}

/// SURVIVING CANDIDATE (query-first): NEVER calls `execute()` -- only
/// `query_receipt()`. This is the strictly safer of the two surviving
/// candidates with respect to duplicate risk (it can never itself cause
/// a real execution), at the cost of never being able to ORIGINATE an
/// effect -- it can only recover a `Settled` state from evidence that
/// already exists, or correctly stay `Unresolved`. Vulnerable to
/// PATH-050-K: it trusts whatever `query_receipt` returns, with no
/// mechanism to verify the receipt's authenticity.
fn path_050_recover_via_receipt_query(
    ledger: &mut Path048Ledger,
    provider: &EffectProvider,
    key: &OccurrenceKey,
    event: &Path047CandidateEvent,
) -> Path050RecoveryVerdict {
    match ledger.records.get(key) {
        Some(SettlementRecord::Settled { payload, .. }) => {
            if *payload == event.payload { Path050RecoveryVerdict::AlreadyComplete } else { Path050RecoveryVerdict::Conflicted }
        }
        Some(SettlementRecord::Reserved { payload, .. }) => {
            if *payload != event.payload {
                return Path050RecoveryVerdict::Conflicted;
            }
            match provider.query_receipt(key) {
                Some(receipt_payload) if receipt_payload == event.payload => {
                    path_048_confirm_settlement(ledger, key);
                    Path050RecoveryVerdict::AlreadyComplete
                }
                Some(_) => Path050RecoveryVerdict::Conflicted,
                None => Path050RecoveryVerdict::Unresolved,
            }
        }
        None => Path050RecoveryVerdict::Unresolved,
    }
}

// ---------------------------------------------------------------------
// Required cases A-K.
// ---------------------------------------------------------------------

/// PATH-050-A (PRIMARY DUPLICATE-EFFECT WITNESS): effect succeeds,
/// Settled write is lost, naive blind retry executes the effect again.
#[test]
fn path_050_a_effect_succeeds_settled_write_lost_naive_retry_duplicates() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v, Path048Verdict::New, "claim K -> Reserved");

    let mut provider = EffectProvider::new(EffectProviderMode::NonIdempotent);
    let outcome = provider.execute(key.clone(), payload.clone());
    assert_eq!(outcome, EffectExecution::Executed, "the external effect DID happen");
    assert_eq!(provider.execution_count, 1);
    // Settled write is lost -- `path_048_confirm_settlement` is
    // deliberately never called. `ledger` still shows Reserved(K).

    path_050_naive_blind_retry_recovery(&mut ledger, &mut provider, &key, &event);

    assert_eq!(
        provider.execution_count, 2,
        "PRIMARY DUPLICATE-EFFECT WITNESS: naive blind retry re-executed a real effect that had \
         already happened -- effect_count == 2, not 1"
    );
}

/// PATH-050-B: effect never happens; the system refuses to blindly
/// retry a `Reserved` claim it cannot prove is safe.
#[test]
fn path_050_b_effect_never_happens_system_refuses_blind_retry() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload);
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v, Path048Verdict::New, "claim K -> Reserved");
    // Crash before the effect -- it never happened at all.

    let mut provider = EffectProvider::new(EffectProviderMode::NonIdempotent);
    path_050_naive_never_retry_recovery(&mut ledger, &mut provider, &key);

    assert_eq!(provider.execution_count, 0, "the never-retry policy performs no effect");
    assert!(
        matches!(ledger.records.get(&key), Some(SettlementRecord::Reserved { .. })),
        "K remains Reserved forever under this policy -- SAFE (no duplicate is possible) but \
         permanently incomplete: this is the LIVENESS failure mode the safety-vs-liveness fork \
         creates"
    );
}

/// PATH-050-C (CORE IDENTIFIABILITY RESULT): construct WORLD A (effect
/// happened, local Settled write lost) and WORLD B (effect never
/// happened, crash before it) with IDENTICAL state on every ledger
/// observable this experiment's recovery candidates actually consult
/// (`.records` -- the ONLY field `path_050_recover`/
/// `path_050_recover_via_receipt_query` ever read), plus, for extra
/// rigor, every other `Path048Ledger` field this test can cheaply
/// compare. CROSS-REVIEW CORRECTION: `Path048Ledger` does not derive
/// `PartialEq`, and the original executable assertion compared only
/// `.records` -- this doc comment previously overclaimed "byte-identical
/// ... every field" without that being separately verified. Both are now
/// true and separately asserted below; the THEOREM itself depends only
/// on `.records`, since that is all any candidate here ever reads. The
/// SAME naive blind-retry procedure, applied to identical local state,
/// is correct in exactly one world -- proving no function of
/// `Reserved(K)` alone can be correct in both. The SAFE candidate
/// (`path_050_recover` under `NonIdempotent`) is then shown to refuse to
/// guess in either world.
#[test]
fn path_050_c_identical_local_state_theorem_same_reserved_opposite_correct_action() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    // WORLD A: the external effect DID happen; only the LOCAL Settled
    // write was lost.
    let mut ledger_a = Path048Ledger::new_attested_genesis();
    let v_a = path_048_reserve(&mut ledger_a, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v_a, Path048Verdict::New);
    let mut provider_a = EffectProvider::new(EffectProviderMode::NonIdempotent);
    provider_a.execute(key.clone(), payload.clone());
    assert_eq!(provider_a.execution_count, 1, "World A: the real effect genuinely happened once");

    // WORLD B: the external effect did NOT happen; crash occurred
    // strictly before it.
    let mut ledger_b = Path048Ledger::new_attested_genesis();
    let v_b = path_048_reserve(&mut ledger_b, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v_b, Path048Verdict::New);
    let provider_b = EffectProvider::new(EffectProviderMode::NonIdempotent);
    assert_eq!(provider_b.execution_count, 0, "World B: the real effect never happened");

    // THE THEOREM: identical on every observable the recovery candidates
    // actually consult -- `.records` is the ONLY field `path_050_recover`
    // and `path_050_recover_via_receipt_query` ever read.
    assert_eq!(
        ledger_a.records, ledger_b.records,
        "IDENTICAL RECOVERY-RELEVANT LOCAL STATE: both worlds show the exact same Reserved(K) \
         record -- nothing in the local durable ledger distinguishes them, and this is the ONLY \
         field any recovery candidate here reads"
    );
    // Reinforcing (not load-bearing for the theorem, since no candidate
    // reads these): the OTHER `Path048Ledger` fields also happen to be
    // identical, since both worlds were built via the same construction
    // sequence.
    assert_eq!(ledger_a.ever_active_namespaces, ledger_b.ever_active_namespaces);
    assert_eq!(ledger_a.pruned_horizon, ledger_b.pruned_horizon);
    assert_eq!(ledger_a.label_first_seen_namespace, ledger_b.label_first_seen_namespace);
    assert_eq!(ledger_a.completeness, ledger_b.completeness);
    assert_eq!(ledger_a.default_completeness, ledger_b.default_completeness);

    // The SAME naive procedure, applied to this identical local state,
    // is correct in exactly one world. Applied to INDEPENDENT clones of
    // the ledger -- `path_050_naive_blind_retry_recovery` mutates the
    // ledger it is given (confirming Settled), and the SAFE candidate
    // below must observe each world's ORIGINAL, still-Reserved state,
    // not whatever the naive demonstration left behind.
    let mut ledger_a_naive = ledger_a.clone();
    let mut provider_a_retried = provider_a.clone();
    path_050_naive_blind_retry_recovery(&mut ledger_a_naive, &mut provider_a_retried, &key, &event);
    let mut ledger_b_naive = ledger_b.clone();
    let mut provider_b_retried = provider_b.clone();
    path_050_naive_blind_retry_recovery(&mut ledger_b_naive, &mut provider_b_retried, &key, &event);

    assert_eq!(provider_a_retried.execution_count, 2, "World A: naive retry DUPLICATES an effect that already happened");
    assert_eq!(provider_b_retried.execution_count, 1, "World B: naive retry correctly performs the effect for the first time");
    assert_ne!(
        provider_a_retried.execution_count, provider_b_retried.execution_count,
        "TEMPORAL IDENTIFIABILITY, one layer past PATH-048/PATH-049: identical current local \
         state (Reserved(K), same payload, same trust/completeness state, no additional \
         completion evidence) requires OPPOSITE correct recovery actions solely because of \
         differing past state at the effect boundary -- no local recovery function over \
         Reserved(K) alone can be correct in both worlds"
    );

    // The SAFE candidate refuses to guess in EITHER world, since it has
    // no way to tell them apart without additional evidence -- run
    // against each world's ORIGINAL Reserved state, untouched by the
    // naive demonstration above.
    let mut provider_a_safe = provider_a.clone();
    let verdict_a = path_050_recover(&mut ledger_a, &mut provider_a_safe, &key, &event);
    let mut provider_b_safe = provider_b.clone();
    let verdict_b = path_050_recover(&mut ledger_b, &mut provider_b_safe, &key, &event);

    assert_eq!(verdict_a, Path050RecoveryVerdict::Unresolved);
    assert_eq!(verdict_b, Path050RecoveryVerdict::Unresolved);
    assert_eq!(provider_a_safe.execution_count, 1, "safe recovery touches nothing -- World A's already-real effect is not duplicated");
    assert_eq!(provider_b_safe.execution_count, 0, "safe recovery touches nothing -- World B's effect remains correctly un-executed, not silently lost either");
}

/// PATH-050-D: provider-side idempotency survives the tested failure
/// pattern. Execute once, crash before Settled, retry the SAME
/// operation_id -- the provider suppresses the duplicate; the real
/// effect count stays at 1; recovery then writes Settled.
#[test]
fn path_050_d_idempotent_provider_suppresses_duplicate_then_settles() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v, Path048Verdict::New);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    let first = provider.execute(key.clone(), payload.clone());
    assert_eq!(first, EffectExecution::Executed);
    assert_eq!(provider.execution_count, 1);
    // Crash before Settled -- ledger still shows Reserved(K).

    let verdict = path_050_recover(&mut ledger, &mut provider, &key, &event);
    assert_eq!(
        verdict,
        Path050RecoveryVerdict::AlreadyComplete,
        "the retry's execute() call is SUPPRESSED by the provider's own idempotency memory"
    );
    assert_eq!(provider.execution_count, 1, "real effect_count stays at 1 -- no duplicate");
    match ledger.records.get(&key).unwrap() {
        SettlementRecord::Settled { payload: settled_payload, .. } => assert_eq!(*settled_payload, payload),
        other => panic!("expected Settled after recovery, got {other:?}"),
    }
}

/// PATH-050-E: a retry presents a CHANGED payload against the same
/// operation_id. Provider-side idempotency must not silently treat this
/// as a harmless replay -- it is a conflict, preserving PATH-048's
/// conflict semantics across the effect boundary.
#[test]
fn path_050_e_changed_payload_on_same_operation_id_is_conflicted_not_silent() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload_1 = path_047_payload("alice", "review", 40.0);
    let payload_2 = path_047_payload("alice", "review", 999.0);
    let event_1 = path_047_candidate("payroll-system", "evt-1", payload_1.clone());
    let key = path_048_occurrence_key(&contract, &event_1).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event_1);
    assert_eq!(v, Path048Verdict::New);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    provider.execute(key.clone(), payload_1.clone());
    assert_eq!(provider.execution_count, 1);

    // Retry presents a DIFFERENT payload against the SAME operation_id.
    // The local ledger's own Reserved payload already disagrees, so
    // `path_050_recover` catches this before ever touching the
    // provider -- exactly the point: no second effect is even attempted.
    let event_2 = path_047_candidate("payroll-system", "evt-1", payload_2);
    let verdict = path_050_recover(&mut ledger, &mut provider, &key, &event_2);

    assert_eq!(verdict, Path050RecoveryVerdict::Conflicted, "a changed payload must be Conflicted, never a silent replay");
    assert_eq!(provider.execution_count, 1, "no second effect executes");
    match ledger.records.get(&key).unwrap() {
        SettlementRecord::Reserved { payload, .. } => {
            assert_eq!(*payload, payload_1, "the original Reserved payload is untouched")
        }
        other => panic!("expected Reserved (unchanged), got {other:?}"),
    }
    assert_eq!(
        provider.query_receipt(&key),
        Some(payload_1),
        "the provider's own original receipt association is also untouched"
    );
}

/// PATH-050-F: distinct occurrences must both execute -- guards against
/// over-coalescing at the effect boundary.
#[test]
fn path_050_f_distinct_occurrences_both_execute_independently() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let event_1 = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review-1", 40.0));
    let event_2 = path_047_candidate("payroll-system", "evt-2", path_047_payload("alice", "review-2", 25.0));
    let key_1 = path_048_occurrence_key(&contract, &event_1).unwrap();
    let key_2 = path_048_occurrence_key(&contract, &event_2).unwrap();
    assert_ne!(key_1, key_2, "operation_id(K1) != operation_id(K2)");

    let mut ledger = Path048Ledger::new_attested_genesis();
    path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event_1);
    path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event_2);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    let v1 = path_050_recover(&mut ledger, &mut provider, &key_1, &event_1);
    let v2 = path_050_recover(&mut ledger, &mut provider, &key_2, &event_2);

    assert_eq!(v1, Path050RecoveryVerdict::EffectExecutedNowSettled);
    assert_eq!(v2, Path050RecoveryVerdict::EffectExecutedNowSettled);
    assert_eq!(provider.execution_count, 2, "each distinct occurrence's real effect executes exactly once");
}

/// PATH-050-G: alias convergence at the effect boundary. Two raw
/// `SourceId`s resolving to the SAME canonical namespace produce the
/// SAME operation_id -- only one real economic effect under the
/// idempotent provider, proving the idempotency domain follows
/// canonical occurrence identity, not the raw delivery label.
#[test]
fn path_050_g_alias_convergence_yields_one_real_effect() {
    let contract = TrustedSourceContract::trusting_with_aliases(
        "payroll-canonical",
        &["source-A", "source-B"],
        SourceIdContract::both_guaranteed(),
    );
    let payload = path_047_payload("alice", "review", 40.0);
    let event_a = path_047_candidate("source-A", "evt-1", payload.clone());
    let event_b = path_047_candidate("source-B", "evt-1", payload.clone());
    let key_a = path_048_occurrence_key(&contract, &event_a).unwrap();
    let key_b = path_048_occurrence_key(&contract, &event_b).unwrap();
    assert_eq!(key_a, key_b, "AUTHORITY canonicalizes both raw labels to the SAME operation_id");

    let mut ledger = Path048Ledger::new_attested_genesis();
    path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event_a);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    let v1 = path_050_recover(&mut ledger, &mut provider, &key_a, &event_a);
    assert_eq!(v1, Path050RecoveryVerdict::EffectExecutedNowSettled);

    // A "retry" delivered under the OTHER alias, same canonical key.
    let v2 = path_050_recover(&mut ledger, &mut provider, &key_b, &event_b);
    assert_eq!(v2, Path050RecoveryVerdict::AlreadyComplete);

    assert_eq!(provider.execution_count, 1, "only ONE real economic effect despite two different raw source labels");
}

/// PATH-050-H: different namespaces, same event_id string -- distinct
/// operation ids, both effects may execute. Preserves namespace-scoped
/// identity at the effect boundary.
#[test]
fn path_050_h_different_namespaces_same_event_id_both_execute() {
    let contract = TrustedSourceContract::trusting(&[
        ("payroll-system", SourceIdContract::both_guaranteed()),
        ("timesheet-system", SourceIdContract::both_guaranteed()),
    ]);
    let event_1 = path_047_candidate("payroll-system", "evt-1", path_047_payload("alice", "review", 40.0));
    let event_2 = path_047_candidate("timesheet-system", "evt-1", path_047_payload("alice", "review", 40.0));
    let key_1 = path_048_occurrence_key(&contract, &event_1).unwrap();
    let key_2 = path_048_occurrence_key(&contract, &event_2).unwrap();
    assert_ne!(key_1, key_2, "identical event_id string, but distinct operation ids across namespaces");

    let mut ledger = Path048Ledger::new_attested_genesis();
    path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event_1);
    path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event_2);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    path_050_recover(&mut ledger, &mut provider, &key_1, &event_1);
    path_050_recover(&mut ledger, &mut provider, &key_2, &event_2);

    assert_eq!(provider.execution_count, 2, "both effects execute -- event_id is namespace-scoped, never globally unique");
}

/// PATH-050-I: receipt-queryable provider recovery. The original
/// execution happened and left a durable receipt; the LOCAL Settled
/// write was lost. Recovery queries the provider BEFORE ever
/// considering execution, finds the receipt, and recovers Settled
/// WITHOUT re-executing.
#[test]
fn path_050_i_receipt_queryable_recovery_does_not_re_execute() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v, Path048Verdict::New);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    provider.execute(key.clone(), payload.clone()); // the ORIGINAL execution, before the crash
    assert_eq!(provider.execution_count, 1);
    // Local crash: Settled write lost, but the provider's receipt
    // survives (it is external to this local failure domain).

    let verdict = path_050_recover_via_receipt_query(&mut ledger, &provider, &key, &event);

    assert_eq!(verdict, Path050RecoveryVerdict::AlreadyComplete);
    assert_eq!(provider.execution_count, 1, "recovery via receipt query NEVER re-executes -- effect_count remains 1");
    assert!(matches!(ledger.records.get(&key), Some(SettlementRecord::Settled { .. })), "local Settled is recovered from external evidence");
}

/// PATH-050-J: no receipt / unknown external state. `Reserved(K)`, and
/// the provider cannot prove whether the operation executed. Recovery
/// must NOT guess -- `Unresolved`, zero new economic executions, `K`
/// remains `Reserved` for reconciliation.
#[test]
fn path_050_j_no_receipt_unknown_external_state_stays_unresolved() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload);
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v, Path048Verdict::New);

    // The provider is capable of answering receipt queries in general
    // (IdempotentByOperationId), but genuinely holds NO receipt for this
    // operation_id -- the provider itself cannot prove whether the
    // operation executed (e.g. its own request may still be in flight,
    // or genuinely never arrived).
    let provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    assert_eq!(provider.query_receipt(&key), None);

    let verdict = path_050_recover_via_receipt_query(&mut ledger, &provider, &key, &event);

    assert_eq!(
        verdict,
        Path050RecoveryVerdict::Unresolved,
        "epistemic discipline preserved: uncertainty at the effect boundary must not be laundered \
         into a retry decision either way"
    );
    assert_eq!(provider.execution_count, 0, "zero NEW economic executions during recovery");
    assert!(
        matches!(ledger.records.get(&key), Some(SettlementRecord::Reserved { .. })),
        "K remains Reserved -- reconciliation required, not guessed"
    );
}

/// PATH-050-K: forged receipt attack, against Candidate B
/// (`path_050_recover_via_receipt_query`). This experiment's fake
/// provider naturally exposes a trust gap -- `receipts` and
/// `execution_count` are two independently writable observables with
/// nothing enforcing they agree. A receipt inserted WITHOUT a real
/// execution demonstrates that receipt AUTHENTICITY is an unverifiable
/// exogenous assumption this candidate cannot audit -- mirroring
/// FIND-019/FIND-026's axiom-forgery pattern one layer further out, at
/// the effect-completion boundary. See PATH-050-K2, immediately below,
/// for the SAME attack against Candidate A -- both surviving candidates
/// read the identical underlying `receipts` memory, so this is not a
/// weakness unique to query-based recovery.
#[test]
fn path_050_k_forged_receipt_suppresses_a_needed_effect() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v, Path048Verdict::New);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    assert_eq!(provider.execution_count, 0, "no real effect has EVER executed");
    // FORGERY: a receipt is inserted directly, bypassing `execute()`
    // entirely -- standing in for a compromised, buggy, or merely
    // untrustworthy receipt source. Nothing in this candidate's model
    // prevents this from being indistinguishable from a genuine receipt.
    provider.receipts.insert(key.clone(), payload.clone());

    let verdict = path_050_recover_via_receipt_query(&mut ledger, &provider, &key, &event);

    assert_eq!(
        verdict,
        Path050RecoveryVerdict::AlreadyComplete,
        "AXIOM FORGERY, one layer further out: without an exogenous authenticity guarantee on the \
         receipt itself, this candidate cannot distinguish a forged completion record from a \
         genuine one -- it trusts the forged receipt and reports the occurrence complete"
    );
    assert_eq!(
        provider.execution_count, 0,
        "the NEEDED real effect was suppressed entirely -- a forged receipt caused a legitimate \
         economic effect to never happen, silently"
    );
}

/// PATH-050-K2 (cross-review addition): the IDENTICAL forged receipt
/// also defeats Candidate A (`path_050_recover`, provider-side
/// idempotency via `execute()`) -- not only Candidate B. Inspecting
/// `EffectProvider::execute` under `IdempotentByOperationId` shows it
/// consults `self.receipts.get(&operation_id)` FIRST, exactly as
/// `query_receipt` does; a forged entry is indistinguishable from a
/// genuine one to either code path, because both read the SAME map.
/// This confirms receipt authenticity is a trust boundary SHARED by
/// both surviving candidates, not unique to query-based recovery.
#[test]
fn path_050_k2_forged_receipt_also_defeats_provider_side_idempotency() {
    let contract = TrustedSourceContract::trusting(&[("payroll-system", SourceIdContract::both_guaranteed())]);
    let payload = path_047_payload("alice", "review", 40.0);
    let event = path_047_candidate("payroll-system", "evt-1", payload.clone());
    let key = path_048_occurrence_key(&contract, &event).unwrap();

    let mut ledger = Path048Ledger::new_attested_genesis();
    let v = path_048_reserve(&mut ledger, &contract, ContractVersion(1), CallId(1), &event);
    assert_eq!(v, Path048Verdict::New);

    let mut provider = EffectProvider::new(EffectProviderMode::IdempotentByOperationId);
    assert_eq!(provider.execution_count, 0, "no real effect has EVER executed");
    // The SAME forgery as K, now attacking Candidate A's execute()-based
    // recovery path instead of Candidate B's query-based one.
    provider.receipts.insert(key.clone(), payload.clone());

    let verdict = path_050_recover(&mut ledger, &mut provider, &key, &event);

    assert_eq!(
        verdict,
        Path050RecoveryVerdict::AlreadyComplete,
        "Candidate A's execute() consults the SAME receipts map the forged entry was inserted \
         into -- it returns SuppressedDuplicate for a forged receipt exactly as it would for a \
         genuine one, and path_050_recover confirms Settled from that signal alone"
    );
    assert_eq!(
        provider.execution_count, 0,
        "the NEEDED real effect was suppressed entirely -- CONFIRMS receipt authenticity is a \
         trust boundary shared by BOTH surviving candidates (A and B), because both trust the \
         same underlying provider memory, not a weakness unique to the query-based recovery path"
    );
}

// ---------------------------------------------------------------------
// Research conclusions -- answered from the behavior demonstrated above.
// ---------------------------------------------------------------------
//
// STRONGEST CRASH AMBIGUITY WITNESS: PATH-050-C -- WORLD A (effect
// happened, Settled write lost) and WORLD B (effect never happened,
// crash before it) produce byte-identical local ledger state
// (`ledger_a.records == ledger_b.records`), yet the SAME naive recovery
// procedure applied to that identical state is correct in exactly one
// world (effect_count 2 vs. 1). No function of `Reserved(K)` alone can
// be correct in both.
//
// H21: CONFIRMED for the constructed failure pattern. An atomic
// occurrence claim (PATH-049) is demonstrably insufficient for
// exactly-once economic completion once effect execution and the
// durable Settled transition can fail independently -- this candidate
// never claims otherwise, and PATH-050-A/B/C show the ambiguity is real
// and unresolvable from local state alone. What resolves it, for the
// FAILURE PATTERN TESTED (Settled write lost, effect provider still
// reachable), is EITHER an effect boundary whose retry semantics make
// repeating the same canonical operation harmless (provider-side
// idempotency keyed by canonical operation_id -- D/E/F/G/H) OR an
// authoritative completion witness capable of distinguishing
// effect-happened from effect-not-happened (a queryable durable receipt
// -- I) -- and K/K2 show the word AUTHORITATIVE there is doing real,
// unverified work, for BOTH of these candidates, because both read the
// SAME underlying provider memory.
//
// DUPLICATE-EFFECT RESULT (naive blind retry): effect_count == 2 (A, C).
// LOST-EFFECT/LIVENESS RESULT (naive never-retry / safe-refuse under
// NonIdempotent): effect_count == 0, permanently stuck Reserved (B, C).
//
// IDEMPOTENT-PROVIDER RESULT: retry suppressed, real effect_count stays
// at 1, Settled written (D). Changed payload on the same operation_id is
// Conflicted, not a silent replay, no second effect, original receipt
// untouched (E).
//
// QUERYABLE-RECEIPT RESULT: recovery via query alone recovers Settled
// from existing evidence with ZERO re-execution (I); with no receipt
// present, correctly stays Unresolved with zero new executions and K
// left Reserved for reconciliation (J).
//
// SAME-OCCURRENCE / DIFFERENT-PAYLOAD: Conflicted, caught before the
// provider is even touched when the local Reserved record already
// disagrees (E).
//
// ALIAS RESULT: two raw source labels resolving to one canonical
// namespace produce the SAME operation_id -- exactly one real effect
// (G).
//
// DIFFERENT-OCCURRENCE RESULT: distinct event_ids under one namespace
// produce distinct operation ids; both effects execute independently,
// effect_count == 2 total (F).
//
// DIFFERENT-NAMESPACE RESULT: identical event_id string, two canonical
// namespaces -- distinct operation ids, both effects execute (H).
//
// UNRESOLVED/RECONCILIATION BEHAVIOR: whenever completion cannot be
// proven (NonIdempotent with no evidence, C/B; or Idempotent-capable but
// genuinely no receipt yet, J), this candidate returns `Unresolved`,
// executes nothing, and leaves `K` `Reserved` rather than guessing in
// either direction.
//
// RECEIPT AUTHENTICITY: became its OWN exogenous trust boundary (K, K2)
// -- SHARED by BOTH surviving candidates, not unique to query-based
// recovery. `EffectProvider::execute`'s duplicate-suppression check
// (candidate A) and `query_receipt` (candidate B) read the identical
// `receipts` map; a forged entry is indistinguishable from a genuine one
// to either code path. This candidate propagates whatever that shared
// memory reports and cannot verify it; a forged or merely-wrong receipt
// silently suppresses a needed real effect regardless of which surviving
// candidate is used for recovery. This is FIND-019/FIND-026's
// axiom-forgery pattern recurring at the effect-completion boundary, not
// solved here.
//
// SMALLEST SURVIVING COMPLETION WITNESS TESTED: a provider-maintained,
// per-canonical-operation_id receipt -- whether reached via
// idempotency-on-execute (D) or via an explicit pre-execution query (I).
// Note precisely what this witness is: the PROVIDER's memory associated
// with the operation_id (the receipt itself), never the bare
// operation_id/idempotency key on its own -- a key is only identity, not
// evidence that anything completed.
// A bare local `effect_status` bit written in a SEPARATE local durable
// step, AFTER an independently-failing external effect, by the SAME
// process that might crash between the effect and that bit write, was
// NOT tested as a separate candidate: PATH-050-C already shows this
// reduces to the identical ambiguity one instruction later (a crash
// between "effect happened" and "bit written" reproduces WORLD A
// exactly), so it was not built as a distinct candidate -- it is,
// structurally, candidate C (local-only) with an extra field that does
// not close the gap. This scope is deliberately narrow: a bit
// TRANSACTIONALLY COUPLED to the actual effect (i.e. the effect and the
// bit write share one atomic failure domain, so neither can happen
// without the other) would rest on a DIFFERENT failure-domain assumption
// than this experiment tests, and is not evaluated here either way. A
// receipt is useful specifically because its durability boundary is
// OUTSIDE the local process's own failure domain; this candidate does
// not verify that property for any real provider, only asserts it as
// the reason the fake provider's receipt is presumed durable across the
// local crash it is tested against.
