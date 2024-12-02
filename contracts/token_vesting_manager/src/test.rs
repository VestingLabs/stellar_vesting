#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as TestAddress, Env};

fn deploy_manager_helper(env: &Env) -> (TokenVestingManagerClient, Address, Address) {
    let contract_id = env.register_contract(None, TokenVestingManager);
    let client = TokenVestingManagerClient::new(env, &contract_id);

    let admin: Address = Address::generate(env);
    let token_address: Address = Address::generate(env);
    client.init(&admin, &token_address);

    (client, admin, token_address)
}

#[test]
#[should_panic]
fn test_double_initialization() {
    let env = Env::default();
    let (client, admin, token_address) = deploy_manager_helper(&env);

    // Panics given that init can only be called once.
    client.init(&admin, &token_address);
}

#[test]
fn test_set_admin() {
    let env = Env::default();
    let (client, admin, _) = deploy_manager_helper(&env);

    let new_admin: Address = Address::generate(&env);
    env.mock_all_auths();
    client.set_admin(&admin, &new_admin ,&true);
    assert!(client.is_admin(&new_admin));

    client.set_admin(&admin, &new_admin ,&false);
    assert!(!client.is_admin(&new_admin));
}

#[test]
fn test_get_admin_count() {
    let env = Env::default();
    let (client, admin, _) = deploy_manager_helper(&env);

    assert!(client.get_admins_count() == 1);

    let new_admin: Address = Address::generate(&env);
    env.mock_all_auths();
    client.set_admin(&admin, &new_admin ,&true);
    assert!(client.get_admins_count() == 2);
}

#[test]
fn test_is_admin() {
    let env = Env::default();
    let (client, admin, _) = deploy_manager_helper(&env);

    assert!(client.is_admin(&admin));

    let non_admin: Address = Address::generate(&env);
    assert!(!client.is_admin(&non_admin));
}
