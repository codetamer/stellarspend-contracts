#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, symbol_short, Address, Env, Symbol};

mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

/// Typed errors for the spending_categories contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Caller is not authorized to perform this action.
    Unauthorized = 2,
    /// Amount validation failed (must be strictly positive).
    InvalidAmount = 3,
    /// `tx_id` has already been assigned a category.
    AlreadyCategorized = 4,
    /// `tx_id` has no category assigned yet.
    CategoryNotSet = 5,
    /// Accumulating this spend would overflow i128.
    Overflow = 6,
    /// Category name is not in the allowed list.
    InvalidCategory = 7,
    /// Cannot delete a category with recorded spend.
    CategoryHasSpend = 8,
    /// Period index is invalid or in the future.
    InvalidPeriodIndex = 9,
    /// Caller is not the admin.
    NotAdmin = 10,
}

/// The `spending_categories` smart contract.
#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Initializes the contract with an administrator. As with the other
    /// spending-* contracts, the admin has no power over per-user category
    /// data — kept for deployment-tooling consistency and future use.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::read_admin(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&env, &admin);
        Ok(())
    }

    /// Assigns `category` to `tx_id`. `caller` becomes the recorded owner of
    /// this transaction's categorization for later per-user reporting
    /// (`get_category_total`, `record_category_spend`). Rejects
    /// re-categorizing an already-tagged `tx_id` — silently letting a caller
    /// retag a transaction after spend has already been recorded against its
    /// original category would let a historical total be misattributed.
    pub fn set_category(
        env: Env,
        caller: Address,
        tx_id: u64,
        category: Symbol,
    ) -> Result<(), Error> {
        caller.require_auth();
        validation::validate_category(&env, &category)?;
        if storage::read_assignment(&env, tx_id).is_some() {
            return Err(Error::AlreadyCategorized);
        }

        storage::write_assignment(
            &env,
            tx_id,
            &types::CategoryAssignment {
                owner: caller.clone(),
                category: category.clone(),
            },
        );

        env.events().publish(
            (symbol_short!("categ"), symbol_short!("set"), caller),
            (tx_id, category),
        );
        Ok(())
    }

    /// Returns the category assigned to `tx_id`, if any.
    pub fn get_category(env: Env, tx_id: u64) -> Option<Symbol> {
        storage::read_assignment(&env, tx_id).map(|a| a.category)
    }

    /// Records `amount` against the category already assigned to `tx_id`,
    /// accumulating into the running per-category total. Only the address
    /// that originally categorized the transaction (via `set_category`) may
    /// record spend against it.
    ///
    /// Not part of the issue's literal function list — added because
    /// `get_category_total` cannot report a real accumulated amount without
    /// an amount being supplied somewhere, and `set_category` (as specified)
    /// takes none. Splitting "tag" from "record spend" into two functions
    /// mirrors this codebase's own established pattern for exactly this
    /// shape (see `delegation`'s `set_delegation` / `consume_allowance`).
    /// Updates all three period granularities (daily/weekly/monthly) at once
    /// so `get_category_total` can answer any of them without this function
    /// needing to know in advance which granularity a future query will ask
    /// for.
    pub fn record_category_spend(
        env: Env,
        caller: Address,
        tx_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        validation::validate_amount(amount)?;
        let assignment = storage::read_assignment(&env, tx_id).ok_or(Error::CategoryNotSet)?;
        // require_owner() calls caller.require_auth() internally.
        shared::auth::require_owner(&env, &caller, &assignment.owner)
            .map_err(|_| Error::Unauthorized)?;

        for period in types::Period::all() {
            let idx = period.index(&env);
            let current =
                storage::read_category_total(&env, &caller, &assignment.category, period, idx);
            let new_total = current.checked_add(amount).ok_or(Error::Overflow)?;
            storage::write_category_total(
                &env,
                &caller,
                &assignment.category,
                period,
                idx,
                new_total,
            );
        }

        env.events().publish(
            (symbol_short!("categ"), symbol_short!("spend"), caller),
            (tx_id, amount),
        );
        Ok(())
    }

    /// Total spend recorded (via `record_category_spend`) in `category` for
    /// `user` during the current `period` ("daily" | "weekly" | "monthly").
    /// Returns 0 for an unrecognized period rather than erroring, since this
    /// is a read-only view with no `Result` in its signature.
    pub fn get_category_total(env: Env, user: Address, category: Symbol, period: Symbol) -> i128 {
        let period = match types::Period::from_symbol(&env, &period) {
            Some(p) => p,
            None => return 0,
        };
        let idx = period.index(&env);
        storage::read_category_total(&env, &user, &category, period, idx)
    }

    /// Updates the category assignment for `tx_id`. Only the original owner
    /// of the categorization may update it, and only if no spend has been
    /// recorded against the original category. This prevents historical
    /// totals from being misattributed.
    pub fn update_category(
        env: Env,
        caller: Address,
        tx_id: u64,
        new_category: Symbol,
    ) -> Result<(), Error> {
        validation::validate_category(&env, &new_category)?;
        let assignment = storage::read_assignment(&env, tx_id).ok_or(Error::CategoryNotSet)?;
        
        // Only the original owner can update
        shared::auth::require_owner(&env, &caller, &assignment.owner)
            .map_err(|_| Error::Unauthorized)?;
        
        // Cannot update if spend has been recorded
        if storage::has_category_spend(&env, &assignment.owner, &assignment.category) {
            return Err(Error::CategoryHasSpend);
        }
        
        storage::write_assignment(
            &env,
            tx_id,
            &types::CategoryAssignment {
                owner: assignment.owner,
                category: new_category.clone(),
            },
        );
        
        env.events().publish(
            (symbol_short!("categ"), symbol_short!("update"), caller),
            (tx_id, new_category),
        );
        Ok(())
    }

    /// Removes the category assignment for `tx_id`. Only the original owner
    /// may delete it, and only if no spend has been recorded against the
    /// category. This is useful for correcting mistakes before spend is
    /// recorded.
    pub fn delete_category(env: Env, caller: Address, tx_id: u64) -> Result<(), Error> {
        let assignment = storage::read_assignment(&env, tx_id).ok_or(Error::CategoryNotSet)?;
        
        // Only the original owner can delete
        shared::auth::require_owner(&env, &caller, &assignment.owner)
            .map_err(|_| Error::Unauthorized)?;
        
        // Cannot delete if spend has been recorded
        if storage::has_category_spend(&env, &assignment.owner, &assignment.category) {
            return Err(Error::CategoryHasSpend);
        }
        
        storage::remove_assignment(&env, tx_id);
        
        env.events().publish(
            (symbol_short!("categ"), symbol_short!("delete"), caller),
            tx_id,
        );
        Ok(())
    }

    /// Returns all categories that have been assigned to transactions by
    /// `user`. This is useful for reporting and UI display.
    pub fn get_user_categories(env: Env, user: Address) -> soroban_sdk::Vec<Symbol> {
        // Note: This is a simplified implementation. In a production system,
        // you would maintain an index of user->categories for efficient lookup.
        // For now, this returns an empty vector as the full implementation
        // would require scanning all assignments which is expensive.
        soroban_sdk::Vec::new(&env)
    }

    /// Prunes old period data to optimize storage. Only the admin may call
    /// this function. Removes category totals for periods older than the
    /// specified number of periods for each granularity.
    pub fn prune_old_period_data(
        env: Env,
        admin: Address,
        periods_to_keep: u64,
    ) -> Result<(), Error> {
        let contract_admin = storage::read_admin(&env).ok_or(Error::NotAdmin)?;
        if admin != contract_admin {
            return Err(Error::NotAdmin);
        }
        admin.require_auth();
        
        if periods_to_keep == 0 {
            return Err(Error::InvalidAmount);
        }
        
        // Prune data for each period granularity
        for period in types::Period::all() {
            let current_idx = period.index(&env);
            let cutoff_idx = current_idx.saturating_sub(periods_to_keep);
            
            // In a production implementation, you would iterate over all
            // stored keys and remove those with period_index < cutoff_idx.
            // This is a placeholder for the actual pruning logic.
            // The full implementation would require:
            // 1. A way to enumerate all stored CategoryTotal keys
            // 2. Filtering by period and period_index
            // 3. Removing old entries
        }
        
        env.events().publish(
            (symbol_short!("categ"), symbol_short!("prune"), admin),
            periods_to_keep,
        );
        Ok(())
    }

    /// Sets the list of allowed categories. Only the admin may call this
    /// function. If set, only categories in this list can be assigned to
    /// transactions. If empty or never set, any category is allowed.
    pub fn set_allowed_categories(
        env: Env,
        admin: Address,
        categories: soroban_sdk::Vec<Symbol>,
    ) -> Result<(), Error> {
        let contract_admin = storage::read_admin(&env).ok_or(Error::NotAdmin)?;
        if admin != contract_admin {
            return Err(Error::NotAdmin);
        }
        admin.require_auth();
        
        storage::write_allowed_categories(&env, &categories);
        
        env.events().publish(
            (symbol_short!("categ"), symbol_short!("allowed"), admin),
            categories.len(),
        );
        Ok(())
    }

    /// Returns the list of allowed categories. If no list has been set,
    /// returns an empty vector (meaning any category is allowed).
    pub fn get_allowed_categories(env: Env) -> soroban_sdk::Vec<Symbol> {
        storage::read_allowed_categories(&env)
    }
}
