use soroban_sdk::{Address, Env, Symbol, Vec as SorobanVec};

use crate::types::{CategoryAssignment, DataKey, Period};

const ADMIN: &str = "ADMIN";
const ALLOWED_CATEGORIES: &str = "ALLOWED_CATEGORIES";

pub fn read_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&ADMIN)
}

pub fn write_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&ADMIN, admin);
}

pub fn read_assignment(env: &Env, tx_id: u64) -> Option<CategoryAssignment> {
    env.storage().persistent().get(&DataKey::Assignment(tx_id))
}

pub fn write_assignment(env: &Env, tx_id: u64, assignment: &CategoryAssignment) {
    env.storage()
        .persistent()
        .set(&DataKey::Assignment(tx_id), assignment);
}

pub fn read_category_total(
    env: &Env,
    owner: &Address,
    category: &Symbol,
    period: Period,
    period_index: u64,
) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::CategoryTotal(
            owner.clone(),
            category.clone(),
            period,
            period_index,
        ))
        .unwrap_or(0)
}

pub fn write_category_total(
    env: &Env,
    owner: &Address,
    category: &Symbol,
    period: Period,
    period_index: u64,
    total: i128,
) {
    env.storage().persistent().set(
        &DataKey::CategoryTotal(owner.clone(), category.clone(), period, period_index),
        &total,
    );
}

pub fn read_allowed_categories(env: &Env) -> SorobanVec<Symbol> {
    env.storage()
        .instance()
        .get(&ALLOWED_CATEGORIES)
        .unwrap_or_else(|| SorobanVec::new(env))
}

pub fn write_allowed_categories(env: &Env, categories: &SorobanVec<Symbol>) {
    env.storage().instance().set(&ALLOWED_CATEGORIES, categories);
}

pub fn has_category_spend(
    env: &Env,
    owner: &Address,
    category: &Symbol,
) -> bool {
    for period in Period::all() {
        let idx = period.index(env);
        let total = read_category_total(env, owner, category, period, idx);
        if total > 0 {
            return true;
        }
    }
    false
}

pub fn remove_category_total(
    env: &Env,
    owner: &Address,
    category: &Symbol,
    period: Period,
    period_index: u64,
) {
    env.storage().persistent().remove(&DataKey::CategoryTotal(
        owner.clone(),
        category.clone(),
        period,
        period_index,
    ));
}

pub fn remove_assignment(env: &Env, tx_id: u64) {
    env.storage()
        .persistent()
        .remove(&DataKey::Assignment(tx_id));
}
