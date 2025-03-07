#![cfg(test)]

use super::*;
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{testutils::Address as TestAddress, testutils::Ledger, Env};

fn deploy_manager_helper(
    env: &Env,
) -> (
    TokenVestingManagerClient,
    Address,
    TokenClient,
    StellarAssetClient,
    Address,
) {
    let contract_id = env.register(TokenVestingManager, ());
    let client = TokenVestingManagerClient::new(env, &contract_id);

    let admin: Address = Address::generate(env);
    let (token_client, token_admin_client, token_address) = deploy_token_helper(&env);
    client.init(&admin, &token_address);

    (
        client,
        admin,
        token_client,
        token_admin_client,
        token_address,
    )
}

fn deploy_token_helper(env: &Env) -> (TokenClient, StellarAssetClient, Address) {
    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_admin_client: StellarAssetClient<'_> =
        StellarAssetClient::new(&env, &token_contract_id.address());
    let token_client = TokenClient::new(&env, &token_contract_id.address());

    (
        token_client,
        token_admin_client,
        token_contract_id.address(),
    )
}

#[test]
#[should_panic]
fn test_double_initialization() {
    let env = Env::default();
    let (client, admin, _, _, token_address) = deploy_manager_helper(&env);

    // Panics given that init can only be called once.
    client.init(&admin, &token_address);
}

#[test]
fn test_set_admin() {
    let env = Env::default();
    let (client, admin, _, _, _) = deploy_manager_helper(&env);

    env.mock_all_auths();
    let new_admin: Address = Address::generate(&env);
    client.set_admin(&admin, &new_admin, &true);
    assert!(client.is_admin(&new_admin));

    client.set_admin(&admin, &new_admin, &false);
    assert!(!client.is_admin(&new_admin));
}

#[test]
fn test_get_admin_count() {
    let env = Env::default();
    let (client, admin, _, _, _) = deploy_manager_helper(&env);

    assert!(client.get_admins_count() == 1);

    env.mock_all_auths();
    let new_admin: Address = Address::generate(&env);
    client.set_admin(&admin, &new_admin, &true);
    assert!(client.get_admins_count() == 2);
}

#[test]
fn test_is_admin() {
    let env = Env::default();
    let (client, admin, _, _, _) = deploy_manager_helper(&env);

    assert!(client.is_admin(&admin));

    let non_admin: Address = Address::generate(&env);
    assert!(!client.is_admin(&non_admin));
}

#[test]
fn test_create_vesting() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, token_address) =
        deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    let expected_vesting: Vesting = Vesting {
        recipient: recipient.clone(),
        start_timestamp,
        end_timestamp,
        deactivation_timestamp: 0,
        timelock,
        release_interval_secs,
        cliff_release_timestamp,
        initial_unlock,
        cliff_amount,
        linear_vest_amount,
        claimed_amount: 0,
    };

    let vesting = client.get_vesting_info(&vesting_id);

    // Vesting struct checks.
    assert_eq!(vesting_id, 0, "wrong vesting id output");
    assert_eq!(vesting.recipient, recipient, "wrong recipient");
    assert_eq!(
        vesting.start_timestamp, start_timestamp,
        "Invalid start timestamp"
    );
    assert_eq!(
        vesting.end_timestamp, end_timestamp,
        "Invalid end timestamp"
    );
    assert_eq!(
        vesting.deactivation_timestamp, 0,
        "Invalid deactivation timestamp"
    );
    assert_eq!(vesting.timelock, timelock, "Invalid timelock");
    assert_eq!(
        vesting.release_interval_secs, release_interval_secs,
        "Invalid release interval"
    );
    assert_eq!(
        vesting.cliff_release_timestamp, cliff_release_timestamp,
        "Invalid cliff release"
    );
    assert_eq!(
        vesting.initial_unlock, initial_unlock,
        "Invalid initial unlock"
    );
    assert_eq!(vesting.cliff_amount, cliff_amount, "Invalid cliff amount");
    assert_eq!(
        vesting.linear_vest_amount, linear_vest_amount,
        "Invalid linear vest amount"
    );
    assert_eq!(vesting.claimed_amount, 0, "Invalid claimed amount");

    // Contract storage checks.
    assert_eq!(client.get_token_address(), token_address, "wrong token set");
    assert_eq!(
        client.get_all_recipients_len(),
        1,
        "wrong number of recipients"
    );
    assert_eq!(
        client.get_all_recipient_vestings_len(&recipient),
        1,
        "wrong vestings length for recipient"
    );
    assert_eq!(
        client.get_vesting_info(&0),
        expected_vesting,
        "wrong vesting setup for the corresponding vesting id"
    );
    assert_eq!(
        client.get_tokens_reserved_for_vesting(),
        2000,
        "wrong number of tokens reserved for vesting"
    );
    assert_eq!(
        client.amount_to_withdraw_by_admin(),
        0,
        "wrong amount available to withdraw by admin"
    );
    assert!(
        client.is_recipient(&recipient),
        "recipient not registered as an actual recipient"
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_creator_not_admin() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    // Cancel mocking.
    env.set_auths(&[]);
    // This will fail because only admin cn call `create_vesting`.
    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_funds_not_approved() {
    let env = Env::default();
    let (client, admin, _, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);

    // This will fail because `transfer_from` lacks allowance.
    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_vested_amount() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;

    // Invalid amounts because `linear_vest_amount + cliff_amount == 0`
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 0;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_start_timestamp() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    // Invalid `start_timestamp` because it needs to be > 0 .
    let start_timestamp: u64 = 0;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_release_interval() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    // Invalid `release_interval_secs` because it should be > 0.
    let release_interval_secs: u64 = 0;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_end_timestamp() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    // Invalid `end_timestamp` because it should be > `start_timestamp`.
    let end_timestamp: u64 = 500;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_cliff_timestamp() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    // Invalid `cliff_release_timestamp` because it should be > `start_timestamp`
    let cliff_release_timestamp: u64 = 200;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 10;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_cliff_amount() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 1500;
    let initial_unlock: i128 = 1000;
    // Invalid `cliff_amount` because it should be > 0 given that `cliff_release_timestamp != 0`.
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_cliff_amount_not_zero() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    // Invalid `cliff_amount` because it should be == 0 given that `cliff_release_timestamp == 0`.
    let cliff_amount: i128 = 10;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_interval_with_cliff_non_zero() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    // Invalid `release_interval_secs` because `(end_timestamp - cliff_release_timestamp) % release_interval_secs != 0`
    let release_interval_secs: u64 = 57;
    let cliff_release_timestamp: u64 = 1500;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 10;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_should_panic_if_invalid_interval_with_cliff_zero() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    // Invalid `release_interval_secs` because `(end_timestamp - cliff_release_timestamp) % release_interval_secs != 0`
    let release_interval_secs: u64 = 57;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
fn test_create_vesting_with_timelock() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 1500;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
fn test_create_vesting_with_no_initial_unlock() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 0;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );
}

#[test]
fn test_create_vesting_recipient_multiple_vestings() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 2000;
    let timelock: u64 = 1500;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 2000;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 1000;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = (initial_unlock + cliff_amount + linear_vest_amount) * 2;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id_1 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    let vesting_id_2 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    let vesting_1 = client.get_vesting_info(&vesting_id_1);

    // First vesting checks.
    assert_eq!(vesting_id_1, 0, "wrong vesting id output");
    assert_eq!(vesting_1.recipient, recipient, "wrong recipient");
    assert_eq!(
        vesting_1.start_timestamp, start_timestamp,
        "Invalid start timestamp"
    );
    assert_eq!(
        vesting_1.end_timestamp, end_timestamp,
        "Invalid end timestamp"
    );
    assert_eq!(
        vesting_1.deactivation_timestamp, 0,
        "Invalid deactivation timestamp"
    );
    assert_eq!(vesting_1.timelock, timelock, "Invalid timelock");
    assert_eq!(
        vesting_1.release_interval_secs, release_interval_secs,
        "Invalid release interval"
    );
    assert_eq!(
        vesting_1.cliff_release_timestamp, cliff_release_timestamp,
        "Invalid cliff release"
    );
    assert_eq!(
        vesting_1.initial_unlock, initial_unlock,
        "Invalid initial unlock"
    );
    assert_eq!(vesting_1.cliff_amount, cliff_amount, "Invalid cliff amount");
    assert_eq!(
        vesting_1.linear_vest_amount, linear_vest_amount,
        "Invalid linear vest amount"
    );
    assert_eq!(vesting_1.claimed_amount, 0, "Invalid claimed amount");

    let vesting_2 = client.get_vesting_info(&vesting_id_2);

    // Second vesting checks.
    assert_eq!(vesting_id_2, 1, "wrong vesting id output");
    assert_eq!(vesting_2.recipient, recipient, "wrong recipient");
    assert_eq!(
        vesting_2.start_timestamp, start_timestamp,
        "Invalid start timestamp"
    );
    assert_eq!(
        vesting_2.end_timestamp, end_timestamp,
        "Invalid end timestamp"
    );
    assert_eq!(
        vesting_2.deactivation_timestamp, 0,
        "Invalid deactivation timestamp"
    );
    assert_eq!(vesting_2.timelock, timelock, "Invalid timelock");
    assert_eq!(
        vesting_2.release_interval_secs, release_interval_secs,
        "Invalid release interval"
    );
    assert_eq!(
        vesting_2.cliff_release_timestamp, cliff_release_timestamp,
        "Invalid cliff release"
    );
    assert_eq!(
        vesting_2.initial_unlock, initial_unlock,
        "Invalid initial unlock"
    );
    assert_eq!(vesting_2.cliff_amount, cliff_amount, "Invalid cliff amount");
    assert_eq!(
        vesting_2.linear_vest_amount, linear_vest_amount,
        "Invalid linear vest amount"
    );
    assert_eq!(vesting_2.claimed_amount, 0, "Invalid claimed amount");
}

#[test]
fn test_create_vesting_batch() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipients = vec![
        &env,
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    let start_timestamps = vec![&env, 1000, 2000, 3000];
    let end_timestamps = vec![&env, 2000, 3000, 4000];
    let timelocks = vec![&env, 1000, 2000, 3000];
    let release_interval_secs = vec![&env, 10, 10, 10];
    let cliff_release_timestamps = vec![&env, 1000, 2000, 3000];
    let initial_unlocks = vec![&env, 1000, 2000, 3000];
    let cliff_amounts = vec![&env, 1000, 2000, 3000];
    let linear_vest_amounts = vec![&env, 1000, 2000, 3000];

    // Calculate total_expected_amount correctly
    let mut total_expected_amount: i128 = 0;
    for i in 0..recipients.len() {
        let initial_unlock = initial_unlocks.get(i).unwrap();
        let cliff_amount = cliff_amounts.get(i).unwrap();
        let linear_vest_amount = linear_vest_amounts.get(i).unwrap();

        total_expected_amount += initial_unlock + cliff_amount + linear_vest_amount;
    }

    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    // Create vesting params and call create_vesting_batch
    let vesting_params = CreateVestingBatchParams {
        recipients,
        start_timestamps,
        end_timestamps,
        timelocks,
        initial_unlocks,
        cliff_release_timestamps,
        cliff_amounts,
        release_interval_secs,
        linear_vest_amounts,
    };

    // Call the function to create the vesting batch
    client.create_vesting_batch(&admin, &vesting_params);
}

#[test]
fn test_claim() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp + 500);

    client.claim(&recipient, &vesting_id);
    assert_eq!(token_client.balance(&recipient), 1500);
}

#[test]
fn test_claim_fully_vested() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp + 1000);

    client.claim(&recipient, &vesting_id);
    assert_eq!(token_client.balance(&recipient), 2000);
}

#[test]
fn test_claim_initial_unlock() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp);

    client.claim(&recipient, &vesting_id);
    assert_eq!(token_client.balance(&recipient), 1000);
}

#[test]
#[should_panic]
fn test_claim_initial_unlock_before_start() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp - 1);

    client.claim(&recipient, &vesting_id);
}

#[test]
#[should_panic]
fn test_claim_not_recipient() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp + 500);

    let non_recipient_claimer: Address = Address::generate(&env);
    client.claim(&non_recipient_claimer, &vesting_id);
}

#[test]
fn test_claim_initial_unlock_and_cliff_amount() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = start_timestamp + 500;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 1000;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(cliff_release_timestamp);

    client.claim(&recipient, &vesting_id);
    assert_eq!(token_client.balance(&recipient), 2000);
}

#[test]
#[should_panic]
fn test_claim_before_timelock() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = start_timestamp + 500;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = start_timestamp + 500;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 1000;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(timelock - 1);

    client.claim(&recipient, &vesting_id);
}

#[test]
#[should_panic]
fn test_claim_zero_claimable() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = start_timestamp + 500;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = start_timestamp + 500;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 1000;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp);

    client.claim(&recipient, &vesting_id);
}

#[test]
#[should_panic]
fn test_claim_zero_duration() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp;
    let timelock: u64 = start_timestamp + 500;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = start_timestamp + 500;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 1000;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(end_timestamp + 1);

    client.claim(&recipient, &vesting_id);
}

#[test]
fn test_revoke() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp + 500);

    client.revoke_vesting(&admin, &vesting_id);

    let vesting = client.get_vesting_info(&vesting_id);
    assert!(vesting.deactivation_timestamp != 0);
}

#[test]
#[should_panic]
fn test_revoke_not_admin() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp + 500);

    let non_admin: Address = Address::generate(&env);

    client.revoke_vesting(&non_admin, &vesting_id);
}

#[test]
#[should_panic]
fn test_revoke_not_active() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp + 500);

    client.revoke_vesting(&admin, &vesting_id);

    let vesting = client.get_vesting_info(&vesting_id);
    assert!(vesting.deactivation_timestamp != 0);

    // This call will fail because the contract is also revoked and not active anymore.
    client.revoke_vesting(&admin, &vesting_id);
}

#[test]
fn test_revoke_fully_vested() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(end_timestamp + 1);

    client.revoke_vesting(&admin, &vesting_id);

    let vesting = client.get_vesting_info(&vesting_id);
    assert!(vesting.deactivation_timestamp != 0);
}

#[test]
#[should_panic]
fn test_revoke_fully_claimed() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(end_timestamp + 1);

    client.claim(&recipient, &vesting_id);

    // This will fail because all vested amount already claimed.
    client.revoke_vesting(&admin, &vesting_id);
}

#[test]
#[should_panic]
fn test_claim_revoke_claim() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp + 500);

    client.claim(&recipient, &vesting_id);
    client.revoke_vesting(&admin, &vesting_id);

    env.ledger().set_timestamp(end_timestamp);

    // This will fail because vesting is revoked and there is nothing more to claim after first claim.
    client.claim(&recipient, &vesting_id);
}

#[test]
fn test_withdraw_admin() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 0;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp);

    client.revoke_vesting(&admin, &vesting_id);

    env.ledger().set_timestamp(end_timestamp);

    assert_eq!(token_client.balance(&admin), 0);
    client.withdraw_admin(&admin, &999);
    assert_eq!(token_client.balance(&admin), 999);
    client.withdraw_admin(&admin, &1);
    assert_eq!(token_client.balance(&admin), 1000);
}

#[test]
#[should_panic]
fn test_withdraw_admin_insufficient_balance() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 0;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp);

    client.revoke_vesting(&admin, &vesting_id);

    env.ledger().set_timestamp(end_timestamp);

    assert_eq!(token_client.balance(&admin), 0);
    // This will fail because `transfer` fails for insufficient balance.
    client.withdraw_admin(&admin, &1001);
}

#[test]
#[should_panic]
fn test_withdraw_non_admin() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 0;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: u64 = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    env.ledger().set_timestamp(start_timestamp);

    client.revoke_vesting(&admin, &vesting_id);

    env.ledger().set_timestamp(end_timestamp);

    let non_admin: Address = Address::generate(&env);

    // This will fail because of access control panic.
    client.withdraw_admin(&non_admin, &1000);
}

#[test]
fn test_withdraw_other_token() {
    let env = Env::default();
    let (client, admin, _, _, _) = deploy_manager_helper(&env);

    let (other_token_client, other_token_admin_client, other_token_address) =
        deploy_token_helper(&env);

    let amount: i128 = 1000;

    // Mock the admin.
    env.mock_all_auths();
    other_token_admin_client.mint(&client.address, &amount);

    assert_eq!(other_token_client.balance(&admin), 0);
    client.withdraw_other_token(&admin, &other_token_address);
    assert_eq!(other_token_client.balance(&admin), 1000);
}

#[test]
#[should_panic]
fn test_withdraw_contract_token() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, token_address) =
        deploy_manager_helper(&env);

    let amount: i128 = 1000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&client.address, &amount);

    assert_eq!(token_client.balance(&admin), 0);
    assert_eq!(token_client.balance(&client.address), 1000);
    client.withdraw_other_token(&admin, &token_address);
}

#[test]
#[should_panic]
fn test_withdraw_other_token_non_admin() {
    let env = Env::default();
    let (client, admin, _, _, _) = deploy_manager_helper(&env);

    let (other_token_client, other_token_admin_client, other_token_address) =
        deploy_token_helper(&env);

    let amount: i128 = 1000;

    // Mock the admin.
    env.mock_all_auths();
    other_token_admin_client.mint(&client.address, &amount);

    assert_eq!(other_token_client.balance(&admin), 0);
    let non_admin: Address = Address::generate(&env);
    client.withdraw_other_token(&non_admin, &other_token_address);
    assert_eq!(other_token_client.balance(&admin), 1000);
}

#[test]
fn test_amount_available_to_withdraw_by_admin() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let amount: i128 = 1000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&client.address, &amount);

    let amount = client.amount_to_withdraw_by_admin();
    assert_eq!(amount, 1000);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = initial_unlock + cliff_amount + linear_vest_amount;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    let amount = client.amount_to_withdraw_by_admin();
    assert_eq!(amount, 1000);

    client.revoke_vesting(&admin, &vesting_id);

    let amount = client.amount_to_withdraw_by_admin();
    // initial_unlock + linear_vest_amount + initial mint
    assert_eq!(amount, 3000);
}

#[test]
fn test_get_all_recipients() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = (initial_unlock + cliff_amount + linear_vest_amount) * 5;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    for _ in 0..5 {
        let recipient: Address = Address::generate(&env);

        client.create_vesting(
            &admin,
            &recipient,
            &start_timestamp,
            &end_timestamp,
            &timelock,
            &initial_unlock,
            &cliff_release_timestamp,
            &cliff_amount,
            &release_interval_secs,
            &linear_vest_amount,
        );
    }

    assert_eq!(client.get_all_recipients_len(), 5);
    assert_eq!(client.get_all_recipients().len(), 5);
    assert_eq!(client.get_all_recipients_sliced(&0, &3).len(), 3);
}

#[test]
fn test_get_all_recipient_vestings() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = (initial_unlock + cliff_amount + linear_vest_amount) * 5;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    for _ in 0..5 {
        client.create_vesting(
            &admin,
            &recipient,
            &start_timestamp,
            &end_timestamp,
            &timelock,
            &initial_unlock,
            &cliff_release_timestamp,
            &cliff_amount,
            &release_interval_secs,
            &linear_vest_amount,
        );
    }

    assert_eq!(client.get_all_recipient_vestings(&recipient).len(), 5);
    assert_eq!(
        client
            .get_all_recipient_vesting_sliced(&0, &3, &recipient)
            .len(),
        3
    );
    assert_eq!(client.get_all_recipient_vestings_len(&recipient), 5);
}

#[test]
fn test_is_recipient() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = (initial_unlock + cliff_amount + linear_vest_amount) * 5;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    assert_eq!(client.is_recipient(&recipient), true);
}

#[test]
fn test_get_tokens_reserved_for_vesting() {
    let env = Env::default();
    let (client, admin, token_client, token_admin_client, _) = deploy_manager_helper(&env);

    let recipient: Address = Address::generate(&env);
    let start_timestamp: u64 = 1000;
    let end_timestamp: u64 = start_timestamp + 1000;
    let timelock: u64 = 0;
    let release_interval_secs: u64 = 10;
    let cliff_release_timestamp: u64 = 0;
    let initial_unlock: i128 = 1000;
    let cliff_amount: i128 = 0;
    let linear_vest_amount: i128 = 1000;

    let total_expected_amount: i128 = (initial_unlock + cliff_amount + linear_vest_amount) * 5;
    let expiration_ledger: u32 = 6300000;

    // Mock the admin.
    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id = client.create_vesting(
        &admin,
        &recipient,
        &start_timestamp,
        &end_timestamp,
        &timelock,
        &initial_unlock,
        &cliff_release_timestamp,
        &cliff_amount,
        &release_interval_secs,
        &linear_vest_amount,
    );

    assert_eq!(client.get_tokens_reserved_for_vesting(), 2000);

    env.ledger().set_timestamp(start_timestamp);
    client.claim(&recipient, &vesting_id);

    assert_eq!(client.get_tokens_reserved_for_vesting(), 1000);

    env.ledger().set_timestamp(start_timestamp + 500);
    client.claim(&recipient, &vesting_id);

    assert_eq!(client.get_tokens_reserved_for_vesting(), 500);

    client.revoke_vesting(&admin, &vesting_id);

    assert_eq!(client.get_tokens_reserved_for_vesting(), 0);
}

#[test]
fn test_get_token_address() {
    let env = Env::default();
    let (client, _, _, _, token_address) = deploy_manager_helper(&env);

    assert_eq!(client.get_token_address(), token_address);
}
