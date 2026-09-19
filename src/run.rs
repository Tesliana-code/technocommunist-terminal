use std::collections::HashSet;

use crate::{
    contribution::{ContributionEvent, ContributionFactors, ContributionWeights},
    distribution::{ContributionAllocation, allocate_contribution_pool},
    ledger::MoneyLedger,
    protocol::{DistributionPools, PoolAmounts},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ContributorMeasurement {
    pub contributor_id: String,
    pub contribution_id: String,
    pub base_work_value: f64,
    pub factors: ContributionFactors,
}

impl ContributorMeasurement {
    pub fn validate_persistable(&self) -> Result<(), String> {
        if self.contributor_id.trim().is_empty() {
            return Err("contributor_id cannot be empty".into());
        }

        if self.contribution_id.trim().is_empty() {
            return Err("contribution_id cannot be empty".into());
        }

        if !self.base_work_value.is_finite() {
            return Err("base_work_value must be finite".into());
        }

        if self.base_work_value < 0.0 {
            return Err("base_work_value cannot be negative".into());
        }

        self.factors.validate().map_err(str::to_string)
    }

    fn to_event(&self) -> ContributionEvent {
        ContributionEvent {
            contributor_id: self.contributor_id.clone(),
            contribution_id: self.contribution_id.clone(),
            base_work_value: self.base_work_value,
            factors: self.factors,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistributionDraft {
    pub id: String,
    pub revision: u64,
    pub money: MoneyLedger,
    pub pools: DistributionPools,
    pub contribution_weights: ContributionWeights,
    pub contributors: Vec<ContributorMeasurement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DraftPreview {
    pub available_for_distribution: f64,
    pub pool_amounts: PoolAmounts,
    pub contribution_allocations: Vec<ContributionAllocation>,
}

impl DistributionDraft {
    pub fn validate_persistable(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("draft id cannot be empty".into());
        }

        for (label, value) in [
            ("revenue", self.money.revenue),
            ("direct_costs", self.money.direct_costs),
            ("shared_costs", self.money.shared_costs),
            ("labor", self.money.labor),
            ("reserve_reinvestment", self.money.reserve_reinvestment),
        ] {
            if !value.is_finite() {
                return Err(format!("{label} must be finite"));
            }
        }

        self.money.validate().map_err(str::to_string)?;

        for (label, value) in [
            ("contribution pool", self.pools.contribution),
            ("stewardship pool", self.pools.stewardship),
            ("capital pool", self.pools.capital),
            (
                "organizational reserve pool",
                self.pools.organizational_reserve,
            ),
            (
                "responsibility weight",
                self.contribution_weights.responsibility,
            ),
            ("criticality weight", self.contribution_weights.criticality),
            ("deal impact weight", self.contribution_weights.deal_impact),
            (
                "dependency reach weight",
                self.contribution_weights.dependency_reach,
            ),
        ] {
            validate_unit_interval(label, value)?;
        }

        let mut contribution_ids = HashSet::new();

        for measurement in &self.contributors {
            measurement.validate_persistable()?;

            if !contribution_ids.insert(measurement.contribution_id.as_str()) {
                return Err(format!(
                    "duplicate contribution_id: {}",
                    measurement.contribution_id
                ));
            }
        }

        Ok(())
    }

    pub fn preview(&self) -> Result<DraftPreview, String> {
        self.validate_persistable()?;

        let available = self.money.calculate_surplus().distributable_surplus;

        if !available.is_finite() {
            return Err("available for distribution must be finite".into());
        }

        if available <= 0.0 {
            return Err("available for distribution must be positive".into());
        }

        let pool_amounts = self.pools.allocate(available).map_err(str::to_string)?;

        self.contribution_weights
            .validate()
            .map_err(str::to_string)?;

        let events: Vec<ContributionEvent> = self
            .contributors
            .iter()
            .map(ContributorMeasurement::to_event)
            .collect();

        let contribution_allocations = allocate_contribution_pool(
            &events,
            &self.contribution_weights,
            pool_amounts.contribution,
        )
        .map_err(str::to_string)?;

        Ok(DraftPreview {
            available_for_distribution: available,
            pool_amounts,
            contribution_allocations,
        })
    }

    pub fn ensure_revision(&self, expected_revision: u64) -> Result<(), String> {
        if self.revision != expected_revision {
            return Err(format!(
                "draft revision mismatch: expected {expected_revision}, current {}",
                self.revision
            ));
        }

        Ok(())
    }
}

fn validate_unit_interval(label: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{label} must be finite"));
    }

    if !(0.0..=1.0).contains(&value) {
        return Err(format!("{label} must be between 0.0 and 1.0"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measurement(
        contributor_id: &str,
        contribution_id: &str,
        base_work_value: f64,
    ) -> ContributorMeasurement {
        ContributorMeasurement {
            contributor_id: contributor_id.into(),
            contribution_id: contribution_id.into(),
            base_work_value,
            factors: ContributionFactors {
                responsibility: 0.80,
                criticality: 0.90,
                deal_impact: 0.85,
                stewardship: 0.0,
                dependency_reach: 0.75,
                evidence_confidence: 1.0,
            },
        }
    }

    fn draft() -> DistributionDraft {
        DistributionDraft {
            id: "draft-001".into(),
            revision: 7,
            money: MoneyLedger {
                revenue: 120_000.0,
                direct_costs: 20_000.0,
                shared_costs: 10_000.0,
                labor: 40_000.0,
                reserve_reinvestment: 20_000.0,
            },
            pools: DistributionPools {
                contribution: 0.35,
                stewardship: 0.20,
                capital: 0.25,
                organizational_reserve: 0.20,
            },
            contribution_weights: ContributionWeights {
                responsibility: 0.20,
                criticality: 0.30,
                deal_impact: 0.25,
                dependency_reach: 0.25,
            },
            contributors: vec![
                measurement("operations", "operations-delivery", 1.0),
                measurement("backend", "backend-engineering", 1.2),
                measurement("integration", "integration-engineering", 1.1),
            ],
        }
    }

    #[test]
    fn valid_draft_previews_through_existing_engine() {
        let preview = draft().preview().unwrap();

        assert_eq!(preview.available_for_distribution, 30_000.0);
        assert_eq!(preview.pool_amounts.contribution, 10_500.0);

        let allocated: f64 = preview
            .contribution_allocations
            .iter()
            .map(|allocation| allocation.allocated_amount)
            .sum();

        assert!((allocated - preview.pool_amounts.contribution).abs() < 1e-9);
    }

    #[test]
    fn persistable_draft_may_be_temporarily_unreconciled() {
        let mut draft = draft();
        draft.contribution_weights.criticality = 0.31;

        assert!(draft.validate_persistable().is_ok());
        assert!(draft.preview().is_err());
    }

    #[test]
    fn duplicate_contribution_identity_is_rejected() {
        let mut draft = draft();
        draft.contributors[1].contribution_id = draft.contributors[0].contribution_id.clone();

        assert!(draft.validate_persistable().is_err());
    }

    #[test]
    fn non_finite_money_is_rejected_before_persistence() {
        let mut draft = draft();
        draft.money.revenue = f64::NAN;

        assert!(draft.validate_persistable().is_err());
    }

    #[test]
    fn stale_revision_is_rejected() {
        let draft = draft();

        assert!(draft.ensure_revision(7).is_ok());
        assert!(draft.ensure_revision(6).is_err());
    }
}
