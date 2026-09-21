use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContributionWeights {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

impl ContributionWeights {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.responsibility,
            self.criticality,
            self.deal_impact,
            self.dependency_reach,
        ];

        if values.iter().any(|value| !value.is_finite()) {
            return Err("contribution weights must be finite");
        }

        if values.iter().any(|value| *value < 0.0 || *value > 1.0) {
            return Err("contribution weights must be between 0.0 and 1.0");
        }

        let total: f64 = values.iter().sum();

        if (total - 1.0).abs() > 1e-9 {
            return Err("contribution weights must sum to exactly 1.0");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContributionFactors {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub stewardship: f64,
    pub dependency_reach: f64,
    pub evidence_confidence: f64,
}

impl ContributionFactors {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.responsibility,
            self.criticality,
            self.deal_impact,
            self.stewardship,
            self.dependency_reach,
            self.evidence_confidence,
        ];

        if values.iter().any(|value| !value.is_finite()) {
            return Err("contribution factors must be finite");
        }

        if values.iter().any(|value| *value < 0.0 || *value > 1.0) {
            return Err("contribution factors must be between 0.0 and 1.0");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContributionEvent {
    pub contributor_id: String,
    pub contribution_id: String,
    pub base_work_value: f64,
    pub factors: ContributionFactors,
}

/// Each term of the weighted-factor sum, computed once and preserved so the calculation trace
/// that produced `evidence_adjusted_value` can be reconstructed later without recomputation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContributionFactorTerms {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContributionScore {
    pub factor_terms: ContributionFactorTerms,
    pub weighted_factors: f64,
    pub evidence_adjusted_value: f64,
}

impl ContributionEvent {
    pub fn calculate(
        &self,
        weights: &ContributionWeights,
    ) -> Result<ContributionScore, &'static str> {
        weights.validate()?;
        self.factors.validate()?;

        if !self.base_work_value.is_finite() {
            return Err("base work value must be finite");
        }

        if self.base_work_value < 0.0 {
            return Err("base work value cannot be negative");
        }

        let factor_terms = ContributionFactorTerms {
            responsibility: weights.responsibility * self.factors.responsibility,
            criticality: weights.criticality * self.factors.criticality,
            deal_impact: weights.deal_impact * self.factors.deal_impact,
            dependency_reach: weights.dependency_reach * self.factors.dependency_reach,
        };

        let weighted_factors = factor_terms.responsibility
            + factor_terms.criticality
            + factor_terms.deal_impact
            + factor_terms.dependency_reach;

        let evidence_adjusted_value =
            self.base_work_value * weighted_factors * self.factors.evidence_confidence;

        Ok(ContributionScore {
            factor_terms,
            weighted_factors,
            evidence_adjusted_value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft_weights() -> ContributionWeights {
        ContributionWeights {
            responsibility: 0.20,
            criticality: 0.30,
            deal_impact: 0.25,
            dependency_reach: 0.25,
        }
    }

    #[test]
    fn draft_weights_sum_to_one() {
        assert!(draft_weights().validate().is_ok());
    }

    #[test]
    fn rejects_invalid_weight_total() {
        let weights = ContributionWeights {
            responsibility: 0.20,
            criticality: 0.20,
            deal_impact: 0.20,
            dependency_reach: 0.10,
        };

        assert!(weights.validate().is_err());
    }

    #[test]
    fn calculates_evidence_adjusted_contribution_value() {
        let event = ContributionEvent {
            contributor_id: "operator-trust-contributor".to_string(),
            contribution_id: "evidence-provenance-kernel".to_string(),
            base_work_value: 1.0,
            factors: ContributionFactors {
                responsibility: 0.70,
                criticality: 0.90,
                deal_impact: 0.80,
                stewardship: 0.40,
                dependency_reach: 0.75,
                evidence_confidence: 1.00,
            },
        };

        let score = event.calculate(&draft_weights()).unwrap();

        let expected = 0.20 * 0.70 + 0.30 * 0.90 + 0.25 * 0.80 + 0.25 * 0.75;

        assert!((score.weighted_factors - expected).abs() < 1e-9);
        assert!((score.evidence_adjusted_value - expected).abs() < 1e-9);
    }

    #[test]
    fn zero_evidence_confidence_produces_zero_economic_claim() {
        let event = ContributionEvent {
            contributor_id: "unknown".to_string(),
            contribution_id: "unverified-claim".to_string(),
            base_work_value: 1.0,
            factors: ContributionFactors {
                responsibility: 1.0,
                criticality: 1.0,
                deal_impact: 1.0,
                stewardship: 1.0,
                dependency_reach: 1.0,
                evidence_confidence: 0.0,
            },
        };

        let score = event.calculate(&draft_weights()).unwrap();

        assert_eq!(score.evidence_adjusted_value, 0.0);
    }

    #[test]
    fn contribution_can_exist_with_zero_deal_impact() {
        let event = ContributionEvent {
            contributor_id: "historical-contributor".to_string(),
            contribution_id: "future-capability".to_string(),
            base_work_value: 1.0,
            factors: ContributionFactors {
                responsibility: 0.80,
                criticality: 0.90,
                deal_impact: 0.0,
                stewardship: 0.40,
                dependency_reach: 0.85,
                evidence_confidence: 1.0,
            },
        };

        let score = event.calculate(&draft_weights()).unwrap();

        assert!(score.evidence_adjusted_value > 0.0);
    }

    #[test]
    fn factor_terms_and_weighted_sum_satisfy_the_calculation_trace_equations() {
        let weights = draft_weights();
        let factors = ContributionFactors {
            responsibility: 0.70,
            criticality: 0.90,
            deal_impact: 0.80,
            stewardship: 0.40,
            dependency_reach: 0.75,
            evidence_confidence: 0.85,
        };

        let event = ContributionEvent {
            contributor_id: "trace-proof".to_string(),
            contribution_id: "trace-proof-contribution".to_string(),
            base_work_value: 2.5,
            factors,
        };

        let score = event.calculate(&weights).unwrap();

        assert!(
            (score.factor_terms.responsibility - weights.responsibility * factors.responsibility)
                .abs()
                < 1e-12
        );
        assert!(
            (score.factor_terms.criticality - weights.criticality * factors.criticality).abs()
                < 1e-12
        );
        assert!(
            (score.factor_terms.deal_impact - weights.deal_impact * factors.deal_impact).abs()
                < 1e-12
        );
        assert!(
            (score.factor_terms.dependency_reach
                - weights.dependency_reach * factors.dependency_reach)
                .abs()
                < 1e-12
        );

        let term_sum = score.factor_terms.responsibility
            + score.factor_terms.criticality
            + score.factor_terms.deal_impact
            + score.factor_terms.dependency_reach;

        assert!((term_sum - score.weighted_factors).abs() < 1e-12);

        let expected_evidence_adjusted_value =
            event.base_work_value * score.weighted_factors * factors.evidence_confidence;

        assert!((score.evidence_adjusted_value - expected_evidence_adjusted_value).abs() < 1e-12);
    }

    #[test]
    fn rejects_factor_outside_zero_to_one() {
        let event = ContributionEvent {
            contributor_id: "bad-input".to_string(),
            contribution_id: "bad-factor".to_string(),
            base_work_value: 1.0,
            factors: ContributionFactors {
                responsibility: 1.2,
                criticality: 0.5,
                deal_impact: 0.5,
                stewardship: 0.5,
                dependency_reach: 0.5,
                evidence_confidence: 1.0,
            },
        };

        assert!(event.calculate(&draft_weights()).is_err());
    }
}
