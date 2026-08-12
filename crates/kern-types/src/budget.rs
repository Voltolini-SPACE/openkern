//! Execution budgets.
//!
//! A budget bounds how much a mission may consume. Charging is fallible and saturating:
//! once a limit is reached, further charges are refused rather than silently ignored.

use std::time::Duration;

/// Reason a budget charge was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetError {
    /// No operations remain in the budget.
    OperationsExhausted,
    /// The output-byte allowance would be exceeded.
    OutputExhausted,
}

impl core::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BudgetError::OperationsExhausted => f.write_str("operation budget exhausted"),
            BudgetError::OutputExhausted => f.write_str("output budget exhausted"),
        }
    }
}

impl std::error::Error for BudgetError {}

/// A consumable execution budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionBudget {
    operations_remaining: u32,
    output_bytes_remaining: u64,
    max_wall: Duration,
}

impl ExecutionBudget {
    /// Build a budget with an operation count, an output-byte allowance, and a wall clock
    /// ceiling.
    #[must_use]
    pub fn new(operations: u32, output_bytes: u64, max_wall: Duration) -> Self {
        Self {
            operations_remaining: operations,
            output_bytes_remaining: output_bytes,
            max_wall,
        }
    }

    /// Operations still available.
    #[must_use]
    pub fn operations_remaining(&self) -> u32 {
        self.operations_remaining
    }

    /// Output bytes still available.
    #[must_use]
    pub fn output_bytes_remaining(&self) -> u64 {
        self.output_bytes_remaining
    }

    /// The wall-clock ceiling for a single operation under this budget.
    #[must_use]
    pub fn max_wall(&self) -> Duration {
        self.max_wall
    }

    /// Charge one operation. Refused when none remain.
    pub fn charge_operation(&mut self) -> Result<(), BudgetError> {
        self.operations_remaining = self
            .operations_remaining
            .checked_sub(1)
            .ok_or(BudgetError::OperationsExhausted)?;
        Ok(())
    }

    /// Charge `bytes` of output. Refused when the allowance would be exceeded.
    pub fn charge_output(&mut self, bytes: u64) -> Result<(), BudgetError> {
        self.output_bytes_remaining = self
            .output_bytes_remaining
            .checked_sub(bytes)
            .ok_or(BudgetError::OutputExhausted)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_deplete_and_refuse() {
        let mut b = ExecutionBudget::new(2, 100, Duration::from_secs(5));
        assert!(b.charge_operation().is_ok());
        assert!(b.charge_operation().is_ok());
        assert_eq!(b.charge_operation(), Err(BudgetError::OperationsExhausted));
        assert_eq!(b.operations_remaining(), 0);
    }

    #[test]
    fn output_cannot_overspend() {
        let mut b = ExecutionBudget::new(10, 50, Duration::from_secs(5));
        assert!(b.charge_output(40).is_ok());
        assert_eq!(b.charge_output(20), Err(BudgetError::OutputExhausted));
        assert_eq!(b.output_bytes_remaining(), 10);
    }
}
