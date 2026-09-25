#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, Address, Env};

mod storage;
#[cfg(test)]
mod test;
pub mod types;
pub mod validation;

/// Typed errors for the subscription contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Caller is not the administrator.
    Unauthorized = 2,
    /// Amount validation failed.
    InvalidAmount = 3,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Initializes the contract with an administrator.
    ///
    /// This function can only be called once. It sets up the contract configuration
    /// with the specified administrator address and an initial value of 0.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The address that will be granted administrator privileges
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization succeeds.
    ///
    /// # Errors
    ///
    /// Returns `Error::AlreadyInitialized` if the contract has already been initialized.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::read_config(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_config(&env, &types::Config { admin, value: 0 });
        Ok(())
    }

    /// Updates the contract value after authenticating the administrator.
    ///
    /// This function validates that the caller is the authorized administrator and
    /// that the new value is non-negative before updating the contract state.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The address claiming to be the administrator
    /// * `value` - The new value to set (must be non-negative)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the value is successfully updated.
    ///
    /// # Errors
    ///
    /// * `Error::Unauthorized` - If the caller is not the administrator or if the contract is not initialized
    /// * `Error::InvalidAmount` - If the provided value is negative
    pub fn set_value(env: Env, admin: Address, value: i128) -> Result<(), Error> {
        admin.require_auth();
        if value < 0 {
            return Err(Error::InvalidAmount);
        }
        let current = storage::read_config(&env).ok_or(Error::Unauthorized)?;
        if current.admin != admin {
            return Err(Error::Unauthorized);
        }
        storage::write_config(&env, &types::Config { admin, value });
        Ok(())
    }

    /// Returns the current configured value.
    ///
    /// Retrieves the value stored in the contract configuration. If the contract
    /// has not been initialized, returns 0.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// The current configured value, or 0 if the contract is not initialized.
    pub fn get_value(env: Env) -> i128 {
        storage::read_config(&env).map(|c| c.value).unwrap_or(0)
    }
}
