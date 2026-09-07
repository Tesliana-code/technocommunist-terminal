use crate::contribution::{ContributionEvent, ContributionWeights};

#[derive(Debug, Clone, PartialEq)]
pub struct ContributionAllocation {
    pub contributor_id: String,
    pub contribution_id: String,
    pub contribution_value: f64,
    pub normalized_share: f64,
    pub allocated_amount: f64,
}

pub fn allocate_contribution_pool(
    events: &[ContributionEvent],
    weights: &ContributionWeights,
    contribution_pool: f64,
) -> Result<Vec<ContributionAllocation>, &'static str> {
    if !contribution_pool.is_finite() {
        return Err("contribution pool must be finite");
    }

    if contribution_pool < 0.0 {
        return Err("contribution pool cannot be negative");
    }

    let mut scored = Vec::with_capacity(events.len());

    for event in events {
        let score = event.calculate(weights)?;

        scored.push((event, score.evidence_adjusted_value));
    }

    let total_contribution: f64 = scored.iter().map(|(_, value)| *value).sum();

    if total_contribution <= 0.0 {
        if contribution_pool == 0.0 {
            return Ok(Vec::new());
        }

        return Err("cannot allocate a positive contribution pool with zero verified contribution");
    }

    let allocations = scored
        .into_iter()
        .map(|(event, contribution_value)| {
            let normalized_share = contribution_value / total_contribution;

            ContributionAllocation {
                contributor_id: event.contributor_id.clone(),
                contribution_id: event.contribution_id.clone(),
                contribution_value,
                normalized_share,
                allocated_amount: contribution_pool * normalized_share,
            }
        })
        .collect();

    Ok(allocations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contribution::{ContributionFactors, ContributionWeights};

    fn weights() -> ContributionWeights {
        ContributionWeights {
            responsibility: 0.20,
            criticality: 0.30,
            deal_impact: 0.25,
            dependency_reach: 0.25,
        }
    }

    fn event(contributor_id: &str, contribution_id: &str, value: f64) -> ContributionEvent {
        ContributionEvent {
            contributor_id: contributor_id.to_string(),
            contribution_id: contribution_id.to_string(),
            base_work_value: value,
            factors: ContributionFactors {
                responsibility: 1.0,
                criticality: 1.0,
                deal_impact: 1.0,
                stewardship: 1.0,
                dependency_reach: 1.0,
                evidence_confidence: 1.0,
            },
        }
    }

    #[test]
    fn equal_contributions_split_pool_equally() {
        let events = vec![
            event("alice", "capability-a", 1.0),
            event("bob", "capability-b", 1.0),
        ];

        let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

        assert_eq!(allocations.len(), 2);

        assert!((allocations[0].normalized_share - 0.5).abs() < 1e-9);

        assert!((allocations[1].normalized_share - 0.5).abs() < 1e-9);

        assert!((allocations[0].allocated_amount - 5_000.0).abs() < 1e-9);

        assert!((allocations[1].allocated_amount - 5_000.0).abs() < 1e-9);
    }

    #[test]
    fn larger_verified_contribution_receives_larger_share() {
        let events = vec![
            event("alice", "capability-a", 3.0),
            event("bob", "capability-b", 1.0),
        ];

        let allocations = allocate_contribution_pool(&events, &weights(), 10_000.0).unwrap();

        assert!((allocations[0].normalized_share - 0.75).abs() < 1e-9);

        assert!((allocations[1].normalized_share - 0.25).abs() < 1e-9);

        assert!((allocations[0].allocated_amount - 7_500.0).abs() < 1e-9);

        assert!((allocations[1].allocated_amount - 2_500.0).abs() < 1e-9);
    }

    #[test]
    fn allocated_amounts_reconcile_to_entire_pool() {
        let events = vec![
            event("alice", "a", 1.0),
            event("bob", "b", 2.0),
            event("carol", "c", 3.0),
        ];

        let allocations = allocate_contribution_pool(&events, &weights(), 10_500.0).unwrap();

        let total: f64 = allocations.iter().map(|a| a.allocated_amount).sum();

        assert!((total - 10_500.0).abs() < 1e-9);
    }

    #[test]
    fn zero_evidence_claim_receives_zero_allocation() {
        let mut unverified = event("alice", "unverified", 1.0);

        unverified.factors.evidence_confidence = 0.0;

        let verified = event("bob", "verified", 1.0);

        let allocations =
            allocate_contribution_pool(&[unverified, verified], &weights(), 10_000.0).unwrap();

        let alice = allocations
            .iter()
            .find(|a| a.contributor_id == "alice")
            .unwrap();

        let bob = allocations
            .iter()
            .find(|a| a.contributor_id == "bob")
            .unwrap();

        assert_eq!(alice.contribution_value, 0.0);
        assert_eq!(alice.normalized_share, 0.0);
        assert_eq!(alice.allocated_amount, 0.0);

        assert!((bob.allocated_amount - 10_000.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_positive_pool_when_no_verified_contribution_exists() {
        let mut event = event("alice", "unverified", 1.0);

        event.factors.evidence_confidence = 0.0;

        let result = allocate_contribution_pool(&[event], &weights(), 10_000.0);

        assert!(result.is_err());
    }

    #[test]
    fn zero_pool_with_zero_contribution_is_valid_empty_distribution() {
        let result = allocate_contribution_pool(&[], &weights(), 0.0).unwrap();

        assert!(result.is_empty());
    }
}

use crate::stewardship::{StewardshipClaim, StewardshipWeights};

#[derive(Debug, Clone, PartialEq)]
pub struct StewardshipAllocation {
    pub steward_id: String,
    pub stewardship_id: String,
    pub stewardship_value: f64,
    pub normalized_share: f64,
    pub allocated_amount: f64,
}

pub fn allocate_stewardship_pool(
    claims: &[StewardshipClaim],
    weights: &StewardshipWeights,
    stewardship_pool: f64,
) -> Result<Vec<StewardshipAllocation>, &'static str> {
    if stewardship_pool < 0.0 {
        return Err("stewardship pool cannot be negative");
    }

    let mut scored = Vec::with_capacity(claims.len());

    for claim in claims {
        let score = claim.calculate(weights)?;

        scored.push((claim, score.evidence_adjusted_value));
    }

    let total_stewardship: f64 = scored.iter().map(|(_, value)| *value).sum();

    if total_stewardship <= 0.0 {
        if stewardship_pool == 0.0 {
            return Ok(Vec::new());
        }

        return Err("cannot allocate a positive stewardship pool with zero verified stewardship");
    }

    Ok(scored
        .into_iter()
        .map(|(claim, stewardship_value)| {
            let normalized_share = stewardship_value / total_stewardship;

            StewardshipAllocation {
                steward_id: claim.steward_id.clone(),
                stewardship_id: claim.stewardship_id.clone(),
                stewardship_value,
                normalized_share,
                allocated_amount: stewardship_pool * normalized_share,
            }
        })
        .collect())
}
