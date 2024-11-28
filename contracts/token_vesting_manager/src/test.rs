#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as TestAddress, Env};

#[test]
#[should_panic]
fn test_double_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, TokenVestingManager);
    let client = TokenVestingManagerClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let token_address: Address = Address::generate(&env);

    client.init(&owner, &token_address);

    // Panics given that init can only be called once.
    client.init(&owner, &token_address);
}
