#![cfg(test)]

/// Import of the Token Vesting Manager Wasm code.
/// Needed to register the contract Wasm and deploy the contract.
mod token_vesting_manager_wasm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/token_vesting_manager.wasm"
    );
}

extern crate std;

use super::*;
use soroban_sdk::{bytesn, testutils::Address as TestAddress, vec, BytesN, Env};

#[test]
#[should_panic]
fn test_factory_double_initialization() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let wasm_hash: BytesN<32> = BytesN::from_array(&env, &[0; 32]);

    client.init(&owner, &wasm_hash);

    // Panics given that `init` can only be called once.
    client.init(&owner, &wasm_hash);
}

#[test]
fn test_deploy_token_vesting_manager_contract_from_factory() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let wasm_hash = env
        .deployer()
        .upload_contract_wasm(token_vesting_manager_wasm::WASM);

    let owner: Address = Address::generate(&env);

    client.init(&owner, &wasm_hash);

    let factory_caller = Address::generate(&env);
    let token_address = Address::generate(&env);

    client.new_token_vesting_manager(&vec![&env, factory_caller.to_val(), token_address.to_val()]);
}

#[test]
fn test_update_owner() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let wasm_hash: BytesN<32> = bytesn!(
        &env,
        0x89424fc9ff1cf53ab622eb1616ebe19ad3815d9d139736ec2a2d59e75b075c60
    );

    client.init(&owner, &wasm_hash);

    let new_owner: Address = Address::generate(&env);

    // Mocks calls to `require_auth`.
    env.mock_all_auths();

    client.update_owner(&owner, &new_owner);

    assert_eq!(client.get_owner(), new_owner);
}

#[test]
#[should_panic]
fn test_update_owner_with_same_address() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let wasm_hash: BytesN<32> = bytesn!(
        &env,
        0x89424fc9ff1cf53ab622eb1616ebe19ad3815d9d139736ec2a2d59e75b075c60
    );

    client.init(&owner, &wasm_hash);

    // Mocks calls to `require_auth`.
    env.mock_all_auths();

    // Panics because contract implementation prevents from updating with same value.
    client.update_owner(&owner, &owner);
}

#[test]
fn test_get_owner() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let wasm_hash: BytesN<32> = bytesn!(
        &env,
        0x89424fc9ff1cf53ab622eb1616ebe19ad3815d9d139736ec2a2d59e75b075c60
    );

    client.init(&owner, &wasm_hash);

    assert_eq!(client.get_owner(), owner);
}

#[test]
fn test_update_wasm_hash() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let wasm_hash: BytesN<32> = bytesn!(
        &env,
        0x89424fc9ff1cf53ab622eb1616ebe19ad3815d9d139736ec2a2d59e75b075c60
    );

    client.init(&owner, &wasm_hash);

    let new_wasm_hash: BytesN<32> = bytesn!(
        &env,
        0x89424fc9ff1cf53ab622eb1616ebe19ad3815d9d139736ec2a2d59e75b075c61
    );

    // Mocks calls to `require_auth`.
    env.mock_all_auths();

    client.update_vesting_manager_wasm_hash(&owner, &new_wasm_hash);

    assert_eq!(client.get_vesting_manager_wasm_hash(), new_wasm_hash);
}

#[test]
#[should_panic]
fn test_update_wasm_hash_with_same_hash() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let wasm_hash: BytesN<32> = bytesn!(
        &env,
        0x89424fc9ff1cf53ab622eb1616ebe19ad3815d9d139736ec2a2d59e75b075c60
    );

    client.init(&owner, &wasm_hash);

    // Mocks calls to `require_auth`.
    env.mock_all_auths();

    // Panics because contract implementation prevents from updating with same value.
    client.update_vesting_manager_wasm_hash(&owner, &wasm_hash);
}

#[test]
fn test_get_vesting_manager_wasm_hash() {
    let env = Env::default();
    let contract_id = env.register(TokenVestingFactory, ());
    let client = TokenVestingFactoryClient::new(&env, &contract_id);

    let owner: Address = Address::generate(&env);
    let wasm_hash: BytesN<32> = bytesn!(
        &env,
        0x89424fc9ff1cf53ab622eb1616ebe19ad3815d9d139736ec2a2d59e75b075c60
    );

    client.init(&owner, &wasm_hash);

    assert_eq!(client.get_vesting_manager_wasm_hash(), wasm_hash);
}
