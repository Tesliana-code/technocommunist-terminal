#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StewardshipWeights {
    pub continuity_responsibility: f64,
    pub maintenance_burden: f64,
    pub failure_exposure: f64,
    pub dependency_reach: f64,
}

impl StewardshipWeights {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.continuity_responsibility,
            self.maintenance_burden,
            self.failure_exposure,
            self.dependency_reach,
        ];

        if values.iter().any(|value| *value < 0.0 || *value > 1.0) {
            return Err("stewardship weights must be between 0.0 and 1.0");
        }

        let total: f64 = values.iter().sum();

        if (total - 1.0).abs() > 1e-9 {
            return Err("stewardship weights must sum to exactly 1.0");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StewardshipFactors {
    pub continuity_responsibility: f64,
    pub maintenance_burden: f64,
    pub failure_exposure: f64,
    pub dependency_reach: f64,
    pub evidence_confidence: f64,
}

impl StewardshipFactors {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.continuity_responsibility,
            self.maintenance_burden,
            self.failure_exposure,
            self.dependency_reach,
            self.evidence_confidence,
        ];

        if values.iter().any(|value| *value < 0.0 || *value > 1.0) {
            return Err("stewardship factors must be between 0.0 and 1.0");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StewardshipClaim {
    pub steward_id: String,
    pub stewardship_id: String,
    pub base_stewardship_value: f64,
    pub factors: StewardshipFactors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StewardshipScore {
    pub weighted_factors: f64,
    pub evidence_adjusted_value: f64,
}

impl StewardshipClaim {
    pub fn calculate(
        &self,
        weights: &StewardshipWeights,
    ) -> Result<StewardshipScore, &'static str> {
        weights.validate()?;
        self.factors.validate()?;

        if self.base_stewardship_value < 0.0 {
            return Err("base stewardship value cannot be negative");
        }

        let weighted_factors = weights.continuity_responsibility
            * self.factors.continuity_responsibility
            + weights.maintenance_burden * self.factors.maintenance_burden
            + weights.failure_exposure * self.factors.failure_exposure
            + weights.dependency_reach * self.factors.dependency_reach;

        let evidence_adjusted_value =
            self.base_stewardship_value * weighted_factors * self.factors.evidence_confidence;

        Ok(StewardshipScore {
            weighted_factors,
            evidence_adjusted_value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft_weights() -> StewardshipWeights {
        StewardshipWeights {
            continuity_responsibility: 0.30,
            maintenance_burden: 0.25,
            failure_exposure: 0.20,
            dependency_reach: 0.25,
        }
    }

    #[test]
    fn draft_weights_sum_to_one() {
        assert!(draft_weights().validate().is_ok());
    }

    #[test]
    fn rejects_invalid_weight_total() {
        let weights = StewardshipWeights {
            continuity_responsibility: 0.30,
            maintenance_burden: 0.20,
            failure_exposure: 0.20,
            dependency_reach: 0.20,
        };

        assert!(weights.validate().is_err());
    }

    #[test]
    fn stronger_stewardship_claim_produces_higher_value() {
        let weak = StewardshipClaim {
            steward_id: "weak".to_string(),
            stewardship_id: "weak-stewardship".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 0.40,
                maintenance_burden: 0.40,
                failure_exposure: 0.40,
                dependency_reach: 0.40,
                evidence_confidence: 1.0,
            },
        };

        let strong = StewardshipClaim {
            steward_id: "strong".to_string(),
            stewardship_id: "strong-stewardship".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 0.90,
                maintenance_burden: 0.90,
                failure_exposure: 0.90,
                dependency_reach: 0.90,
                evidence_confidence: 1.0,
            },
        };

        let weak_score = weak.calculate(&draft_weights()).unwrap();
        let strong_score = strong.calculate(&draft_weights()).unwrap();

        assert!(strong_score.evidence_adjusted_value > weak_score.evidence_adjusted_value);
    }

    #[test]
    fn zero_evidence_produces_zero_stewardship_claim() {
        let claim = StewardshipClaim {
            steward_id: "unknown".to_string(),
            stewardship_id: "unverified".to_string(),
            base_stewardship_value: 1.0,
            factors: StewardshipFactors {
                continuity_responsibility: 1.0,
                maintenance_burden: 1.0,
                failure_exposure: 1.0,
                dependency_reach: 1.0,
                evidence_confidence: 0.0,
            },
        };

        let score = claim.calculate(&draft_weights()).unwrap();

        assert_eq!(score.evidence_adjusted_value, 0.0);
    }
}
