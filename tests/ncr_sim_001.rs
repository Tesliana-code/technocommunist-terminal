use distribution_machine::contribution::{
    ContributionEvent, ContributionFactors, ContributionWeights,
};
use distribution_machine::distribution::allocate_contribution_pool;
use distribution_machine::ledger::MoneyLedger;
use distribution_machine::protocol::{DistributionPools, Protocol, ProtocolVersion};

fn pp_01_draft() -> Protocol {
    Protocol {
        version: ProtocolVersion::new("PP-0.1-draft"),
        pools: DistributionPools {
            contribution: 0.35,
            stewardship: 0.20,
            capital: 0.25,
            organizational_reserve: 0.20,
        },
    }
}

fn contribution_weights() -> ContributionWeights {
    ContributionWeights {
        responsibility: 0.20,
        criticality: 0.30,
        deal_impact: 0.25,
        dependency_reach: 0.25,
    }
}

fn contribution(
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
fn ncr_sim_001_runs_end_to_end_and_reconciles() {
    let ledger = MoneyLedger {
        revenue: 120_000.0,
        direct_costs: 20_000.0,
        shared_costs: 10_000.0,
        labor: 40_000.0,
        reserve_reinvestment: 20_000.0,
    };

    ledger.validate().unwrap();

    let surplus = ledger.calculate_surplus();

    assert_eq!(surplus.distributable_surplus, 30_000.0);
    assert!(surplus.is_profitable);

    let protocol = pp_01_draft();

    assert_eq!(protocol.version.id, "PP-0.1-draft");

    let pool_amounts = protocol
        .pools
        .allocate(surplus.distributable_surplus)
        .unwrap();

    assert_eq!(pool_amounts.contribution, 10_500.0);
    assert_eq!(pool_amounts.stewardship, 6_000.0);
    assert_eq!(pool_amounts.capital, 7_500.0);
    assert_eq!(pool_amounts.organizational_reserve, 6_000.0);

    let events = vec![
        contribution(
            "founder-product",
            "product-architecture",
            1.20,
            0.90,
            0.90,
            0.80,
            0.60,
            0.85,
            1.00,
        ),
        contribution(
            "sales-bd",
            "ncr-commercial-close",
            1.00,
            0.80,
            0.60,
            1.00,
            0.30,
            0.50,
            1.00,
        ),
        contribution(
            "backend-engineer",
            "core-platform-capability",
            1.10,
            0.85,
            0.95,
            0.80,
            0.60,
            0.90,
            1.00,
        ),
        contribution(
            "integration-contributor",
            "connector-reconciliation",
            1.00,
            0.80,
            0.95,
            0.90,
            0.70,
            0.95,
            1.00,
        ),
        contribution(
            "operator-trust-contributor",
            "evidence-provenance",
            0.90,
            0.70,
            0.90,
            0.80,
            0.40,
            0.75,
            1.00,
        ),
        contribution(
            "qa-security",
            "production-validation",
            0.80,
            0.75,
            0.90,
            0.85,
            0.60,
            0.70,
            1.00,
        ),
        contribution(
            "operations",
            "deployment-continuity",
            0.75,
            0.70,
            0.80,
            0.75,
            0.95,
            0.80,
            1.00,
        ),
    ];

    let allocations =
        allocate_contribution_pool(&events, &contribution_weights(), pool_amounts.contribution)
            .unwrap();

    assert_eq!(allocations.len(), 7);

    let total_share: f64 = allocations.iter().map(|a| a.normalized_share).sum();

    let total_contribution_allocated: f64 = allocations.iter().map(|a| a.allocated_amount).sum();

    assert!((total_share - 1.0).abs() < 1e-9);

    assert!((total_contribution_allocated - pool_amounts.contribution).abs() < 1e-9);

    let full_reconciliation = ledger.direct_costs
        + ledger.shared_costs
        + ledger.labor
        + ledger.reserve_reinvestment
        + pool_amounts.contribution
        + pool_amounts.stewardship
        + pool_amounts.capital
        + pool_amounts.organizational_reserve;

    assert!((full_reconciliation - ledger.revenue).abs() < 1e-9);

    println!();
    println!("NCR-SIM-001");
    println!("Protocol: {}", protocol.version.id);
    println!("Revenue: €{:.2}", ledger.revenue);
    println!(
        "Distributable surplus: €{:.2}",
        surplus.distributable_surplus
    );
    println!("Contribution pool: €{:.2}", pool_amounts.contribution);
    println!();

    for allocation in &allocations {
        println!(
            "{} | {} | share {:.2}% | €{:.2}",
            allocation.contributor_id,
            allocation.contribution_id,
            allocation.normalized_share * 100.0,
            allocation.allocated_amount
        );
    }

    println!();
    println!("All money reconciled: PASS");
    println!("Contribution pool reconciled: PASS");
    println!("Protocol version preserved: PASS");
}

#[test]
fn ncr_sim_001_stewardship_pool_allocates_independently() {
    use distribution_machine::distribution::allocate_stewardship_pool;
    use distribution_machine::stewardship::{
        StewardshipClaim, StewardshipFactors, StewardshipWeights,
    };

    let protocol = pp_01_draft();
    let pool_amounts = protocol.pools.allocate(30_000.0).unwrap();

    let weights = StewardshipWeights {
        continuity_responsibility: 0.30,
        maintenance_burden: 0.25,
        failure_exposure: 0.20,
        dependency_reach: 0.25,
    };

    let claims = vec![
        StewardshipClaim {
            steward_id: "founder-product".to_string(),
            stewardship_id: "architecture-continuity".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 0.70,
                maintenance_burden: 0.45,
                failure_exposure: 0.70,
                dependency_reach: 0.85,
                evidence_confidence: 1.0,
            },
        },
        StewardshipClaim {
            steward_id: "backend-engineer".to_string(),
            stewardship_id: "platform-continuity".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 0.85,
                maintenance_burden: 0.80,
                failure_exposure: 0.90,
                dependency_reach: 0.90,
                evidence_confidence: 1.0,
            },
        },
        StewardshipClaim {
            steward_id: "integration-contributor".to_string(),
            stewardship_id: "connector-continuity".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 0.85,
                maintenance_burden: 0.85,
                failure_exposure: 0.85,
                dependency_reach: 0.90,
                evidence_confidence: 1.0,
            },
        },
        StewardshipClaim {
            steward_id: "operations".to_string(),
            stewardship_id: "production-continuity".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 1.0,
                maintenance_burden: 0.95,
                failure_exposure: 1.0,
                dependency_reach: 0.95,
                evidence_confidence: 1.0,
            },
        },
    ];

    let allocations =
        allocate_stewardship_pool(&claims, &weights, pool_amounts.stewardship).unwrap();

    let total: f64 = allocations.iter().map(|a| a.allocated_amount).sum();

    assert!((total - 6_000.0).abs() < 1e-9);

    println!();
    println!("NCR-SIM-001 — STEWARDSHIP");

    for allocation in &allocations {
        println!(
            "{} | {} | share {:.2}% | €{:.2}",
            allocation.steward_id,
            allocation.stewardship_id,
            allocation.normalized_share * 100.0,
            allocation.allocated_amount
        );
    }

    println!("Stewardship pool reconciled: PASS");
}
