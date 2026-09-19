use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoneyLedger {
    pub revenue: f64,
    pub direct_costs: f64,
    pub shared_costs: f64,
    pub labor: f64,
    pub reserve_reinvestment: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurplusResult {
    pub distributable_surplus: f64,
    pub is_profitable: bool,
}

impl MoneyLedger {
    pub fn calculate_surplus(&self) -> SurplusResult {
        let distributable_surplus = self.revenue
            - self.direct_costs
            - self.shared_costs
            - self.labor
            - self.reserve_reinvestment;

        SurplusResult {
            distributable_surplus,
            is_profitable: distributable_surplus > 0.0,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.revenue < 0.0 {
            return Err("revenue cannot be negative");
        }

        if self.direct_costs < 0.0
            || self.shared_costs < 0.0
            || self.labor < 0.0
            || self.reserve_reinvestment < 0.0
        {
            return Err("cost, labor, and reserve values cannot be negative");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ncr_example_produces_expected_surplus() {
        let ledger = MoneyLedger {
            revenue: 120_000.0,
            direct_costs: 20_000.0,
            shared_costs: 10_000.0,
            labor: 40_000.0,
            reserve_reinvestment: 20_000.0,
        };

        let result = ledger.calculate_surplus();

        assert_eq!(result.distributable_surplus, 30_000.0);
        assert!(result.is_profitable);
    }

    #[test]
    fn negative_surplus_is_preserved_as_economic_fact() {
        let ledger = MoneyLedger {
            revenue: 50_000.0,
            direct_costs: 20_000.0,
            shared_costs: 10_000.0,
            labor: 25_000.0,
            reserve_reinvestment: 5_000.0,
        };

        let result = ledger.calculate_surplus();

        assert_eq!(result.distributable_surplus, -10_000.0);
        assert!(!result.is_profitable);
    }

    #[test]
    fn rejects_negative_inputs() {
        let ledger = MoneyLedger {
            revenue: 120_000.0,
            direct_costs: -1.0,
            shared_costs: 10_000.0,
            labor: 40_000.0,
            reserve_reinvestment: 20_000.0,
        };

        assert!(ledger.validate().is_err());
    }
}
