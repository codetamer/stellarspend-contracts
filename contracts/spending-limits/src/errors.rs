use soroban_sdk::contracterror;

/// Errors returned by budget-management operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BudgetError {
    /// The spend would exceed the budget's configured cap.
    ///
    /// This error is returned when an attempt is made to spend an amount
    /// that would cause the total spending to exceed the maximum limit
    /// configured for the budget.
    BudgetExceeded = 1,
    /// No budget exists for the requested identifier.
    ///
    /// This error is returned when attempting to perform operations on a
    /// budget that has not been created or has been deleted.
    BudgetNotFound = 2,
    /// The caller is not authorized to perform this operation.
    ///
    /// This error is returned when the caller does not have the required
    /// permissions to execute the requested operation, such as when a
    /// non-administrator attempts to modify budget settings.
    Unauthorized = 3,
    /// An attempt was made to pause a budget that is already paused.
    ///
    /// This error is returned when trying to pause a budget that is
    /// already in a paused state, indicating a redundant operation.
    BudgetAlreadyPaused = 4,
    /// An attempt was made to activate a budget that is already active.
    ///
    /// This error is returned when trying to activate or unpause a budget
    /// that is already in an active state, indicating a redundant operation.
    BudgetAlreadyActive = 5,
}
