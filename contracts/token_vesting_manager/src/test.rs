#![cfg(test)]

use super::*;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{testutils::Address as TestAddress, Env};

fn deploy_manager_helper(
    env: &Env,
) -> (
    TokenVestingManagerClient,
    Address,
    TokenClient,
    StellarAssetClient,
    Address,
) {
    let contract_id = env.register_contract(None, TokenVestingManager);
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

    let new_admin: Address = Address::generate(&env);
    env.mock_all_auths();
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

    let new_admin: Address = Address::generate(&env);
    env.mock_all_auths();
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

    env.mock_all_auths();
    token_admin_client.mint(&admin, &total_expected_amount);
    token_client.approve(
        &admin,
        &client.address,
        &total_expected_amount,
        &expiration_ledger,
    );

    let vesting_id: U256 = client.create_vesting(
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
    assert_eq!(
        vesting_id,
        U256::from_u128(&env, 0),
        "wrong vesting id output"
    );
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
        client.get_vesting_info(&U256::from_u128(&env, 0)),
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
