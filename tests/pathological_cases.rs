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
