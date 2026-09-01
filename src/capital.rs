#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapitalClaim {
    pub claimant_id: &'static str,
    pub contributed_capital: f64,
    pub eligible_return_cap: f64,
    pub evidence_confidence: f64,
}

impl CapitalClaim {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.contributed_capital < 0.0 {
            return Err("contributed capital cannot be negative");
        }

        if self.eligible_return_cap < 0.0 {
            return Err("eligible return cap cannot be negative");
        }

        if !(0.0..=1.0).contains(&self.evidence_confidence) {
            return Err("evidence confidence must be between 0.0 and 1.0");
        }

        Ok(())
    }

    pub fn verified_claim_value(&self) -> Result<f64, &'static str> {
        self.validate()?;

        Ok(self.contributed_capital.min(self.eligible_return_cap) * self.evidence_confidence)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapitalAllocation {
    pub claimant_id: &'static str,
    pub verified_claim_value: f64,
    pub normalized_share: f64,
    pub allocated_amount: f64,
}

pub fn allocate_capital_pool(
    claims: &[CapitalClaim],
    capital_pool: f64,
) -> Result<Vec<CapitalAllocation>, &'static str> {
    if capital_pool < 0.0 {
        return Err("capital pool cannot be negative");
    }

    let mut verified = Vec::with_capacity(claims.len());

    for claim in claims {
        verified.push((claim, claim.verified_claim_value()?));
    }

    let total_verified: f64 = verified.iter().map(|(_, value)| *value).sum();

    if total_verified <= 0.0 {
        if capital_pool == 0.0 {
            return Ok(Vec::new());
        }

        return Err("cannot allocate positive capital pool with zero verified capital claims");
    }

    let distributable_capital = capital_pool.min(total_verified);

    Ok(verified
        .into_iter()
        .map(|(claim, value)| {
            let normalized_share = value / total_verified;

            CapitalAllocation {
                claimant_id: claim.claimant_id,
                verified_claim_value: value,
                normalized_share,
                allocated_amount: distributable_capital * normalized_share,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capital_claim_is_bounded_by_return_cap() {
        let claim = CapitalClaim {
            claimant_id: "founder",
            contributed_capital: 50_000.0,
            eligible_return_cap: 30_000.0,
            evidence_confidence: 1.0,
        };

        assert_eq!(claim.verified_claim_value().unwrap(), 30_000.0);
    }

    #[test]
    fn evidence_confidence_reduces_unverified_capital_claim() {
        let claim = CapitalClaim {
            claimant_id: "investor",
            contributed_capital: 20_000.0,
            eligible_return_cap: 20_000.0,
            evidence_confidence: 0.5,
        };

        assert_eq!(claim.verified_claim_value().unwrap(), 10_000.0);
    }

    #[test]
    fn capital_pool_allocates_by_verified_claim() {
        let claims = vec![
            CapitalClaim {
                claimant_id: "alice",
                contributed_capital: 30_000.0,
                eligible_return_cap: 30_000.0,
                evidence_confidence: 1.0,
            },
            CapitalClaim {
                claimant_id: "bob",
                contributed_capital: 10_000.0,
                eligible_return_cap: 10_000.0,
                evidence_confidence: 1.0,
            },
        ];

        let allocations = allocate_capital_pool(&claims, 8_000.0).unwrap();

        assert!((allocations[0].allocated_amount - 6_000.0).abs() < 1e-9);

        assert!((allocations[1].allocated_amount - 2_000.0).abs() < 1e-9);
    }

    #[test]
    fn capital_does_not_create_authority_or_extra_weight() {
        let claims = vec![
            CapitalClaim {
                claimant_id: "founder",
                contributed_capital: 10_000.0,
                eligible_return_cap: 10_000.0,
                evidence_confidence: 1.0,
            },
            CapitalClaim {
                claimant_id: "worker-investor",
                contributed_capital: 10_000.0,
                eligible_return_cap: 10_000.0,
                evidence_confidence: 1.0,
            },
        ];

        let allocations = allocate_capital_pool(&claims, 7_500.0).unwrap();

        assert!((allocations[0].allocated_amount - allocations[1].allocated_amount).abs() < 1e-9);
    }
}
