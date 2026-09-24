use crate::{Error, storage};
use soroban_sdk::{Env, Symbol, Vec as SorobanVec};

/// Validates a spend amount recorded against a category: must be strictly
/// positive. Reuses `shared::validation::validate_positive_amount`.
pub fn validate_amount(amount: i128) -> Result<(), Error> {
    shared::validation::validate_positive_amount(amount).map_err(|_| Error::InvalidAmount)
}

/// Validates that a category is in the allowed list (if the list is set).
/// If no allowed categories are configured, any category is valid.
pub fn validate_category(env: &Env, category: &Symbol) -> Result<(), Error> {
    let allowed = storage::read_allowed_categories(env);
    if allowed.is_empty() {
        return Ok(());
    }
    
    for allowed_cat in allowed.iter() {
        if allowed_cat == category {
            return Ok(());
        }
    }
    
    Err(Error::InvalidCategory)
}

/// Validates that a period index is not in the future.
pub fn validate_period_index(env: &Env, period_index: u64) -> Result<(), Error> {
    let current_idx = env.ledger().timestamp();
    if period_index > current_idx {
        return Err(Error::InvalidPeriodIndex);
    }
    Ok(())
}
