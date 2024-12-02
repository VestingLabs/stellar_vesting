#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, Bytes, Env, Map, String,
    Symbol, Val, Vec, U256,
};

/// Constants for storage keys.

// Maps the admins of the contract.
const ADMINS: Symbol = symbol_short!("ADMINS");
// Number of admins of the contract.
const ADMIN_COUNT: Symbol = symbol_short!("ADCOUNT");
// Address of the token to be vested.
const TOKEN_ADDRESS: Symbol = symbol_short!("TOKENADDR");
// Total amount of tokens reserved for vesting.
const TOKENS_RESERVED_FOR_VESTING: Symbol = symbol_short!("TRESERVED");
// Maps the vesting ids for each recipient.
const RECIPIENT_VESTINGS: Symbol = symbol_short!("RVESTINGS");
// Maps the vesting information for each vesting id.
const VESTING_BY_ID: Symbol = symbol_short!("VBYID");
// A nonce that is incremented to generate unique ids
const NONCE: Symbol = symbol_short!("NONCE");
// List of all recipients.
const RECIPIENTS: Symbol = symbol_short!("RECIPS");

/// Constants for events.

const ADMIN_ACCESS_SET: Symbol = symbol_short!("ADMINSET");
const VESTING_CREATED: Symbol = symbol_short!("VCREATED");
const CLAIMED: Symbol = symbol_short!("CLAIMED");
const VESTING_REVOKED: Symbol = symbol_short!("VREVOKED");
const ADMIN_WITHDRAWN: Symbol = symbol_short!("ADMINWITH");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vesting {
    pub recipient: Address,
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub deactivation_timestamp: u64,
    pub timelock: u64,
    pub release_interval_secs: u64,
    pub cliff_release_timestamp: u64,
    pub initial_unlock: U256,
    pub cliff_amount: U256,
    pub linear_vest_amount: U256,
    pub claimed_amount: U256,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateVestingBatchParams {
    pub recipients: Vec<Address>,
    pub start_timestamps: Vec<u64>,
    pub end_timestamps: Vec<u64>,
    pub timelocks: Vec<u64>,
    pub initial_unlocks: Vec<U256>,
    pub cliff_release_timestamps: Vec<u64>,
    pub cliff_amounts: Vec<U256>,
    pub release_interval_secs: Vec<u64>,
    pub linear_vest_amounts: Vec<U256>,
}

#[contract]
pub struct TokenVestingManager;

#[contractimpl]
impl TokenVestingManager {
    /// Initialization function.
    pub fn init(env: Env, factory_caller: Address, token_address: Address) {
        if env.storage().persistent().has(&ADMINS) {
            panic!("Already initialized");
        }

        let mut admins: Map<Address, bool> = Map::new(&env);
        admins.set(factory_caller, true);
        env.storage().persistent().set(&ADMINS, &admins);

        let admin_count: u32 = 1;
        env.storage().persistent().set(&ADMIN_COUNT, &admin_count);
        env.storage()
            .persistent()
            .set(&TOKEN_ADDRESS, &token_address);
    }

    /// Adds a new admin or remove an existing one for the Token Vesting Manager contract.
    pub fn set_admin(env: Env, caller: Address, admin: Address, is_enabled: bool) {
        let mut admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        // Access control check
        caller.require_auth();
        if !admins.get(caller).is_some() {
            panic!("Not an admin");
        }

        assert!(
            admins.get(admin.clone()).unwrap_or(!is_enabled) != is_enabled,
            "Flag provided already set"
        );

        let admin_count: u32 = env.storage().persistent().get(&ADMIN_COUNT).unwrap_or(0);

        if is_enabled {
            let new_admin_count: u32 = admin_count + 1;
            env.storage()
                .persistent()
                .set(&ADMIN_COUNT, &new_admin_count);
        } else {
            assert!(admin_count > 1, "There must always be at least 1 admin");
            let new_admin_count: u32 = admin_count - 1;
            env.storage()
                .persistent()
                .set(&ADMIN_COUNT, &new_admin_count);
        }

        admins.set(admin.clone(), is_enabled);
        env.storage().persistent().set(&ADMINS, &admins);

        env.events()
            .publish((ADMIN_ACCESS_SET,), (admin, is_enabled));
    }

    /// Returns the number of admins for the Token Vesting Manager contract.
    pub fn get_admins_count(env: Env) -> u32 {
        env.storage().persistent().get(&ADMIN_COUNT).unwrap_or(0)
    }

    /// Returns true if the given address is an admin, false otherwise.
    pub fn is_admin(env: Env, address: Address) -> bool {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        admins.get(address).unwrap_or(false)
    }

    /// Creates a vesting schedule for a recipient and returns a vesting ID.
    pub fn create_vesting(
        env: Env,
        caller: Address,
        recipient: Address,
        start_timestamp: u64,
        end_timestamp: u64,
        timelock: u64,
        initial_unlock: U256,
        cliff_release_timestamp: u64,
        cliff_amount: U256,
        release_interval_secs: u64,
        linear_vest_amount: U256,
    ) -> U256 {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        // Access control check
        caller.require_auth();
        if !admins.get(caller.clone()).is_some() {
            panic!("Not an admin");
        }

        assert!(
            linear_vest_amount.add(&cliff_amount) != U256::from_u32(&env, 0),
            "Invalid vested amount"
        );
        assert!(
            start_timestamp != 0 && start_timestamp < end_timestamp,
            "Invalid start timestamp"
        );
        assert!(release_interval_secs != 0, "Invalid release interval");

        if cliff_release_timestamp == 0 {
            assert!(
                cliff_amount == U256::from_u32(&env, 0),
                "invalid cliff amount"
            );
            assert!(
                (end_timestamp - start_timestamp) % release_interval_secs == 0,
                "Invalid interval length"
            );
        } else {
            assert!(
                cliff_amount != U256::from_u32(&env, 0),
                "Invalid cliff amount"
            );
            assert!(
                start_timestamp <= cliff_release_timestamp
                    && cliff_release_timestamp < end_timestamp,
                "Invalid cliff release"
            );
            assert!(
                (end_timestamp - cliff_release_timestamp) % release_interval_secs == 0,
                "Invalid interval length"
            );
        }

        let total_expected_amount = initial_unlock.add(&cliff_amount).add(&linear_vest_amount);

        let reserved_tokens = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(U256::from_u32(&env, 0))
            .add(&total_expected_amount);

        env.storage()
            .persistent()
            .set(&TOKENS_RESERVED_FOR_VESTING, &reserved_tokens);

        let vesting: Vesting = Vesting {
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
            claimed_amount: U256::from_u32(&env, 0),
        };

        let vesting_id: U256 = env
            .storage()
            .persistent()
            .get(&NONCE)
            .unwrap_or(U256::from_u32(&env, 0));
        let new_vesting_id: U256 = vesting_id.add(&U256::from_u32(&env, 1));
        env.storage().persistent().set(&NONCE, &new_vesting_id);

        if !Self::is_recipient(env.clone(), recipient.clone()) {
            let mut recipients: Vec<Address> = env
                .storage()
                .persistent()
                .get(&RECIPIENTS)
                .unwrap_or(Vec::new(&env));

            recipients.insert(recipients.len(), recipient.clone());
            env.storage().persistent().set(&RECIPIENTS, &recipients);
        }

        let mut vesting_by_id: Map<U256, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or(Map::new(&env));

        vesting_by_id.set(vesting_id.clone(), vesting.clone());
        env.storage()
            .persistent()
            .set(&VESTING_BY_ID, &vesting_by_id);

        let mut recipient_vestings: Vec<U256> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or(Vec::new(&env));

        recipient_vestings.insert(recipient_vestings.len(), vesting_id.clone());
        env.storage()
            .persistent()
            .set(&RECIPIENT_VESTINGS, &recipient_vestings);

        env.events()
            .publish((VESTING_CREATED,), (vesting_id.clone(), recipient, vesting));

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TOKEN_ADDRESS)
            .unwrap_or(Address::from_string(&String::from_str(&env, "0")));

        let _: Val = env.invoke_contract(
            &token_address,
            &Symbol::new(&env, "transfer_from"),
            vec![
                &env,
                caller.to_val(),
                env.current_contract_address().to_val(),
                total_expected_amount.to_val(),
            ],
        );

        vesting_id
    }

    /// Creates vesting schedules in batch for multiple recipients.
    pub fn create_vesting_batch(
        env: Env,
        caller: Address,
        create_vesting_batch_params: CreateVestingBatchParams,
    ) -> Vec<U256> {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        // Access control check
        caller.require_auth();
        if !admins.get(caller.clone()).is_some() {
            panic!("Not an admin");
        }

        let length: u32 = create_vesting_batch_params.recipients.len();
        assert!(
            create_vesting_batch_params.start_timestamps.len() == length
                && create_vesting_batch_params.end_timestamps.len() == length
                && create_vesting_batch_params.timelocks.len() == length
                && create_vesting_batch_params.initial_unlocks.len() == length
                && create_vesting_batch_params.cliff_release_timestamps.len() == length
                && create_vesting_batch_params.cliff_amounts.len() == length
                && create_vesting_batch_params.release_interval_secs.len() == length
                && create_vesting_batch_params.linear_vest_amounts.len() == length,
            "Array length mismatch"
        );

        let mut vesting_ids: Vec<U256> = Vec::new(&env);

        for i in 0..length {
            vesting_ids.insert(
                i,
                Self::create_vesting(
                    env.clone(),
                    caller.clone(),
                    create_vesting_batch_params.recipients.get(i).unwrap(),
                    create_vesting_batch_params.start_timestamps.get(i).unwrap(),
                    create_vesting_batch_params.end_timestamps.get(i).unwrap(),
                    create_vesting_batch_params.timelocks.get(i).unwrap(),
                    create_vesting_batch_params.initial_unlocks.get(i).unwrap(),
                    create_vesting_batch_params
                        .cliff_release_timestamps
                        .get(i)
                        .unwrap(),
                    create_vesting_batch_params.cliff_amounts.get(i).unwrap(),
                    create_vesting_batch_params
                        .release_interval_secs
                        .get(i)
                        .unwrap(),
                    create_vesting_batch_params
                        .linear_vest_amounts
                        .get(i)
                        .unwrap(),
                ),
            )
        }

        vesting_ids
    }

    /// Allows a recipient to claim their vested tokens.
    pub fn claim(env: Env, caller: Address, vesting_id: U256) {
        let mut vesting = Self::get_vesting_info(env.clone(), vesting_id.clone());

        // Access control check
        caller.require_auth();
        if vesting.recipient != caller {
            panic!("Not vesting owner");
        }

        assert!(
            vesting.timelock <= env.ledger().timestamp(),
            "Timelock enabled"
        );

        let vest_amount =
            Self::calculate_vested_amount(env.clone(), vesting.clone(), env.ledger().timestamp());
        let claimable = vest_amount.sub(&vesting.claimed_amount);

        assert!(
            claimable != U256::from_u32(&env, 0),
            "Insufficient balance to claim"
        );

        vesting.claimed_amount = vesting.claimed_amount.add(&claimable);

        let mut vesting_by_id: Map<U256, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or(Map::new(&env));

        vesting_by_id.set(vesting_id.clone(), vesting.clone());
        env.storage()
            .persistent()
            .set(&VESTING_BY_ID, &vesting_by_id);

        let reserved_tokens = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(U256::from_u32(&env, 0))
            .sub(&claimable);

        env.storage()
            .persistent()
            .set(&TOKENS_RESERVED_FOR_VESTING, &reserved_tokens);

        env.events().publish(
            (CLAIMED,),
            (vesting_id.clone(), caller.clone(), claimable.clone()),
        );

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TOKEN_ADDRESS)
            .unwrap_or(Address::from_string(&String::from_str(&env, "0")));

        let _: Val = env.invoke_contract(
            &token_address,
            &Symbol::new(&env, "transfer"),
            vec![&env, caller.to_val(), claimable.to_val()],
        );
    }

    /// Revokes a vesting arrangement before it has been fully claimed.
    pub fn revoke_vesting(env: Env, caller: Address, vesting_id: U256) {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        // Access control check
        caller.require_auth();
        if !admins.get(caller.clone()).is_some() {
            panic!("Not an admin");
        }

        let mut vesting = Self::get_vesting_info(env.clone(), vesting_id.clone());
        assert!(vesting.deactivation_timestamp == 0, "Vesting not active");

        let final_vest_amount =
            Self::calculate_vested_amount(env.clone(), vesting.clone(), vesting.end_timestamp);
        assert!(
            final_vest_amount != vesting.claimed_amount,
            "All vested amount already claimed"
        );

        vesting.deactivation_timestamp = env.ledger().timestamp();

        let mut vesting_by_id: Map<U256, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or(Map::new(&env));

        vesting_by_id.set(vesting_id.clone(), vesting.clone());
        env.storage()
            .persistent()
            .set(&VESTING_BY_ID, &vesting_by_id);

        let vested_amount_now =
            Self::calculate_vested_amount(env.clone(), vesting.clone(), env.ledger().timestamp());
        let amount_remaining = final_vest_amount.sub(&vested_amount_now);

        let reserved_tokens = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(U256::from_u32(&env, 0))
            .sub(&amount_remaining);

        env.storage()
            .persistent()
            .set(&TOKENS_RESERVED_FOR_VESTING, &reserved_tokens);

        env.events().publish(
            (VESTING_REVOKED,),
            (
                vesting_id.clone(),
                vesting.clone().recipient,
                amount_remaining,
                vesting,
            ),
        );
    }

    /// Calculates the vested amount for a given Vesting, at a given timestamp.
    pub fn calculate_vested_amount(env: Env, vesting: Vesting, reference_timestamp: u64) -> U256 {
        let mut adjusted_reference_timestamp = reference_timestamp;

        if vesting.deactivation_timestamp != 0
            && adjusted_reference_timestamp > vesting.deactivation_timestamp
        {
            adjusted_reference_timestamp = vesting.deactivation_timestamp;
        }

        let mut vesting_amount: U256 = U256::from_u32(&env, 0);

        if adjusted_reference_timestamp >= vesting.end_timestamp {
            adjusted_reference_timestamp = vesting.end_timestamp;
        }

        if adjusted_reference_timestamp >= vesting.cliff_release_timestamp {
            vesting_amount = vesting_amount.add(&vesting.cliff_amount);
        }

        if vesting.initial_unlock > U256::from_u32(&env, 0)
            && reference_timestamp >= vesting.start_timestamp
        {
            vesting_amount = vesting_amount.add(&vesting.initial_unlock);
        }

        let start_timestamp: u64;

        if vesting.cliff_release_timestamp != 0 {
            start_timestamp = vesting.cliff_release_timestamp;
        } else {
            start_timestamp = vesting.start_timestamp;
        }

        if adjusted_reference_timestamp > start_timestamp {
            let current_vesting_duration_secs = adjusted_reference_timestamp - start_timestamp;
            let truncated_current_vesting_duration_secs = (current_vesting_duration_secs
                / vesting.release_interval_secs)
                * vesting.release_interval_secs;

            let final_vesting_duration_secs = vesting.end_timestamp - start_timestamp;

            let linear_vest_amount = vesting
                .linear_vest_amount
                .mul(&U256::from_u128(
                    &env,
                    truncated_current_vesting_duration_secs.into(),
                ))
                .div(&U256::from_u128(&env, final_vesting_duration_secs.into()));

            vesting_amount = vesting_amount.add(&linear_vest_amount);
        }

        vesting_amount
    }

    /// Allows the admin to withdraw ERC20 tokens not locked in vesting.
    pub fn withdraw_admin(env: Env, caller: Address, amount_requested: U256) {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        // Access control check
        caller.require_auth();
        if !admins.get(caller.clone()).is_some() {
            panic!("Not an admin");
        }

        let amount_remaining = Self::amount_to_withdraw_by_admin(env.clone());
        assert!(amount_remaining >= amount_requested, "Insuffisance balance");

        env.events().publish(
            (ADMIN_WITHDRAWN,),
            (caller.clone(), amount_requested.clone()),
        );

        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TOKEN_ADDRESS)
            .unwrap_or(Address::from_string(&String::from_str(&env, "0")));

        let _: Val = env.invoke_contract(
            &token_address,
            &Symbol::new(&env, "transfer"),
            vec![&env, caller.to_val(), amount_requested.to_val()],
        );
    }

    /// Withdraws other ERC20 tokens accidentally sent to the contract's address.
    pub fn withdraw_other_token(env: Env, caller: Address, other_token_address: Address) {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        // Access control check
        caller.require_auth();
        if !admins.get(caller.clone()).is_some() {
            panic!("Not an admin");
        }

        assert!(
            other_token_address != Self::get_token_address(env.clone()),
            "Invalid other token"
        );

        let balance: Val = env.invoke_contract(
            &other_token_address,
            &Symbol::new(&env, "balance"),
            vec![&env, env.current_contract_address().to_val()],
        );

        let _: Val = env.invoke_contract(
            &other_token_address,
            &Symbol::new(&env, "transfer"),
            vec![&env, caller.to_val(), balance],
        );
    }

    /// Returns the amount of tokens that are available for the admin to withdraw.
    pub fn amount_to_withdraw_by_admin(env: Env) -> U256 {
        let token_address: Address = env
            .storage()
            .persistent()
            .get(&TOKEN_ADDRESS)
            .unwrap_or(Address::from_string(&String::from_str(&env, "0")));

        let balance: U256 = env.invoke_contract(
            &token_address,
            &Symbol::new(&env, "balance"),
            vec![&env, env.current_contract_address().to_val()],
        );

        let reserved_tokens = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(U256::from_u32(&env, 0));

        balance.sub(&reserved_tokens)
    }

    /// Retrieves information about a specific vesting arrangement.
    pub fn get_vesting_info(env: Env, vesting_id: U256) -> Vesting {
        let vesting_by_id: Map<U256, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or(Map::new(&env));

        // This will panic if there is no vesting associated with a given id.
        vesting_by_id.get(vesting_id).unwrap()
    }

    /// Returns all recipient addresses which have at least one vesting schedule set.
    pub fn get_all_recipients(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&RECIPIENTS)
            .unwrap_or(Vec::new(&env))
    }

    /// Returns the list of recipients in a specific range, `from` being inclusive and `to` being exclusive.
    pub fn get_all_recipients_sliced(env: Env, from: u32, to: u32) -> Vec<Address> {
        let recipients: Vec<Address> = env
            .storage()
            .persistent()
            .get(&RECIPIENTS)
            .unwrap_or(Vec::new(&env));

        recipients.slice(from..to)
    }

    /// Returns the number of recipients.
    pub fn get_all_recipients_len(env: Env) -> u32 {
        let recipients: Vec<Address> = env
            .storage()
            .persistent()
            .get(&RECIPIENTS)
            .unwrap_or(Vec::new(&env));

        recipients.len()
    }

    /// Returns the list of vestings for the recipient.
    pub fn get_all_recipient_vestings(env: Env, recipient: Address) -> Vec<U256> {
        let recipient_vestings: Map<Address, Vec<U256>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or(Map::new(&env));

        recipient_vestings.get(recipient).unwrap_or(Vec::new(&env))
    }

    /// Returns the list of vestings for the recipient in a specific range, `from` being inclusive and
    /// `to` being exclusive.
    pub fn get_all_recipient_vesting_sliced(
        env: Env,
        from: u32,
        to: u32,
        recipient: Address,
    ) -> Vec<U256> {
        let recipient_vestings: Map<Address, Vec<U256>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or(Map::new(&env));

        let vestings: Vec<U256> = recipient_vestings.get(recipient).unwrap_or(Vec::new(&env));

        vestings.slice(from..to)
    }

    /// Returns the length of all vestings for the recipient.
    pub fn get_all_recipient_vestings_len(env: Env, recipient: Address) -> u32 {
        let recipient_vestings: Map<Address, Vec<U256>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or(Map::new(&env));

        recipient_vestings
            .get(recipient)
            .unwrap_or(Vec::new(&env))
            .len()
    }

    /// Checks if a given address is a recipient of any vesting schedule.
    pub fn is_recipient(env: Env, recipient: Address) -> bool {
        let recipient_vestings: Map<Address, Vec<U256>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or(Map::new(&env));

        let recipient_ids: Vec<U256> = recipient_vestings.get(recipient).unwrap_or(Vec::new(&env));

        recipient_ids.len() != 0
    }

    /// Returns the address of the token used in the vesting contract.
    pub fn get_token_address(env: Env) -> Address {
        env.storage()
            .persistent()
            .get(&TOKEN_ADDRESS)
            .unwrap_or(Address::from_string(&String::from_str(&env, "0")))
    }

    /// Returns the amount of token reserved for vesting in the contract.
    pub fn get_tokens_reserved_for_vesting(env: Env) -> U256 {
        env.storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(U256::from_u32(&env, 0))
    }
}

mod test;

/// Contract for deploying an asset contract for testing purpose.
#[contract]
pub struct SacDeployer;

#[contractimpl]
impl SacDeployer {
    pub fn deploy_sac(env: Env, serialized_asset: Bytes) -> Address {
        // Create the Deployer with Asset
        let deployer = env.deployer().with_stellar_asset(serialized_asset);
        let _ = deployer.deployed_address();
        // Deploy the Stellar Asset Contract
        let sac_address = deployer.deploy();

        sac_address
    }
}
