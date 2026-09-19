#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolVersion {
    pub id: String,
}

impl ProtocolVersion {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DistributionPools {
    pub contribution: f64,
    pub stewardship: f64,
    pub capital: f64,
    pub organizational_reserve: f64,
}

impl DistributionPools {
    pub fn validate(&self) -> Result<(), &'static str> {
        let values = [
            self.contribution,
            self.stewardship,
            self.capital,
            self.organizational_reserve,
        ];

        if values.iter().any(|value| *value < 0.0 || *value > 1.0) {
            return Err("pool percentages must be between 0.0 and 1.0");
        }

        let total: f64 = values.iter().sum();

        if (total - 1.0).abs() > 1e-9 {
            return Err("distribution pools must sum to exactly 1.0");
        }

        Ok(())
    }

    pub fn allocate(&self, surplus: f64) -> Result<PoolAmounts, &'static str> {
        self.validate()?;

        if surplus < 0.0 {
            return Err("cannot distribute negative surplus");
        }

        Ok(PoolAmounts {
            contribution: surplus * self.contribution,
            stewardship: surplus * self.stewardship,
            capital: surplus * self.capital,
            organizational_reserve: surplus * self.organizational_reserve,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolAmounts {
    pub contribution: f64,
    pub stewardship: f64,
    pub capital: f64,
    pub organizational_reserve: f64,
}

impl PoolAmounts {
    pub fn total(&self) -> f64 {
        self.contribution + self.stewardship + self.capital + self.organizational_reserve
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Protocol {
    pub version: ProtocolVersion,
    pub pools: DistributionPools,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn pp_01_pool_percentages_sum_to_one() {
        let protocol = pp_01_draft();

        assert!(protocol.pools.validate().is_ok());
    }

    #[test]
    fn rejects_pool_configuration_below_one_hundred_percent() {
        let pools = DistributionPools {
            contribution: 0.30,
            stewardship: 0.20,
            capital: 0.20,
            organizational_reserve: 0.20,
        };

        assert!(pools.validate().is_err());
    }

    #[test]
    fn rejects_pool_configuration_above_one_hundred_percent() {
        let pools = DistributionPools {
            contribution: 0.40,
            stewardship: 0.30,
            capital: 0.25,
            organizational_reserve: 0.20,
        };

        assert!(pools.validate().is_err());
    }

    #[test]
    fn ncr_surplus_allocates_into_expected_pool_amounts() {
        let protocol = pp_01_draft();

        let amounts = protocol.pools.allocate(30_000.0).unwrap();

        assert_eq!(amounts.contribution, 10_500.0);
        assert_eq!(amounts.stewardship, 6_000.0);
        assert_eq!(amounts.capital, 7_500.0);
        assert_eq!(amounts.organizational_reserve, 6_000.0);

        assert!((amounts.total() - 30_000.0).abs() < 1e-9);
    }

    #[test]
    fn negative_surplus_cannot_be_distributed() {
        let protocol = pp_01_draft();

        assert!(protocol.pools.allocate(-1.0).is_err());
    }
}
