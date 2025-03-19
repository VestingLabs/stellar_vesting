#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token::TokenClient, Address, Env, Map,
    Symbol, Vec,
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
const ADMIN_WITHDRAWN_OTHER: Symbol = symbol_short!("WITHOTHER");

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
    pub initial_unlock: i128,
    pub cliff_amount: i128,
    pub linear_vest_amount: i128,
    pub claimed_amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateVestingBatchParams {
    pub recipients: Vec<Address>,
    pub start_timestamps: Vec<u64>,
    pub end_timestamps: Vec<u64>,
    pub timelocks: Vec<u64>,
    pub initial_unlocks: Vec<i128>,
    pub cliff_release_timestamps: Vec<u64>,
    pub cliff_amounts: Vec<i128>,
    pub release_interval_secs: Vec<u64>,
    pub linear_vest_amounts: Vec<i128>,
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
        admins.set(factory_caller.clone(), true);
        env.storage().persistent().set(&ADMINS, &admins);
        env.events()
            .publish((ADMIN_ACCESS_SET,), (factory_caller, true));

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
            .unwrap_or_else(|| Map::new(&env));

        // Access control check
        Self::admin_check(caller.clone(), admins.clone());

        assert!(
            admins.get(admin.clone()).unwrap_or(false) != is_enabled,
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
            .unwrap_or_else(|| Map::new(&env));

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
        initial_unlock: i128,
        cliff_release_timestamp: u64,
        cliff_amount: i128,
        release_interval_secs: u64,
        linear_vest_amount: i128,
    ) -> u64 {
        let admins: Map<Address, bool> = env.storage().persistent().get(&ADMINS).unwrap();

        // Access control check
        Self::admin_check(caller.clone(), admins.clone());

        Self::create_vesting_internal(
            env.clone(),
            caller.clone(),
            recipient.clone(),
            start_timestamp,
            end_timestamp,
            timelock,
            initial_unlock,
            cliff_release_timestamp,
            cliff_amount,
            release_interval_secs,
            linear_vest_amount,
        )
    }

    /// Creates vesting schedules in batch for multiple recipients.
    pub fn create_vesting_batch(
        env: Env,
        caller: Address,
        create_vesting_batch_params: CreateVestingBatchParams,
    ) -> Vec<u64> {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or_else(|| Map::new(&env));

        // Access control check
        Self::admin_check(caller.clone(), admins.clone());

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

        let mut vesting_ids: Vec<u64> = Vec::new(&env);

        for i in 0..length {
            vesting_ids.insert(
                i,
                Self::create_vesting_internal(
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
    pub fn claim(env: Env, caller: Address, vesting_id: u64) {
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
        let claimable = vest_amount - vesting.claimed_amount;

        assert!(claimable != 0, "Insufficient balance to claim");

        vesting.claimed_amount = vesting.claimed_amount + claimable;

        let mut vesting_by_id: Map<u64, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or_else(|| Map::new(&env));

        vesting_by_id.set(vesting_id, vesting.clone());
        env.storage()
            .persistent()
            .set(&VESTING_BY_ID, &vesting_by_id);

        let reserved_tokens: i128 = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(0)
            - claimable;

        env.storage()
            .persistent()
            .set(&TOKENS_RESERVED_FOR_VESTING, &reserved_tokens);

        env.events().publish(
            (CLAIMED,),
            (vesting_id.clone(), caller.clone(), claimable.clone()),
        );

        let token_address: Address = env.storage().persistent().get(&TOKEN_ADDRESS).unwrap();

        TokenClient::new(&env, &token_address).transfer(
            &env.current_contract_address(),
            &caller,
            &claimable,
        );
    }

    /// Revokes a vesting arrangement before it has been fully claimed.
    pub fn revoke_vesting(env: Env, caller: Address, vesting_id: u64) {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or_else(|| Map::new(&env));

        // Access control check
        Self::admin_check(caller.clone(), admins.clone());

        let mut vesting = Self::get_vesting_info(env.clone(), vesting_id);
        assert!(vesting.deactivation_timestamp == 0, "Vesting not active");

        let final_vest_amount =
            Self::calculate_vested_amount(env.clone(), vesting.clone(), vesting.end_timestamp);
        assert!(
            final_vest_amount != vesting.claimed_amount,
            "All vested amount already claimed"
        );

        vesting.deactivation_timestamp = env.ledger().timestamp();

        let mut vesting_by_id: Map<u64, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or_else(|| Map::new(&env));

        vesting_by_id.set(vesting_id.clone(), vesting.clone());
        env.storage()
            .persistent()
            .set(&VESTING_BY_ID, &vesting_by_id);

        let vested_amount_now =
            Self::calculate_vested_amount(env.clone(), vesting.clone(), env.ledger().timestamp());
        let amount_remaining = final_vest_amount - vested_amount_now;

        let reserved_tokens = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(0)
            - amount_remaining;

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
    pub fn calculate_vested_amount(_env: Env, vesting: Vesting, reference_timestamp: u64) -> i128 {
        let mut adjusted_reference_timestamp = reference_timestamp;

        if vesting.deactivation_timestamp != 0
            && adjusted_reference_timestamp > vesting.deactivation_timestamp
        {
            adjusted_reference_timestamp = vesting.deactivation_timestamp;
        }

        let mut vesting_amount: i128 = 0;

        if adjusted_reference_timestamp >= vesting.end_timestamp {
            adjusted_reference_timestamp = vesting.end_timestamp;
        }

        if adjusted_reference_timestamp >= vesting.cliff_release_timestamp {
            vesting_amount = vesting_amount + vesting.cliff_amount;
        }

        if vesting.initial_unlock > 0 && reference_timestamp >= vesting.start_timestamp {
            vesting_amount = vesting_amount + vesting.initial_unlock;
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

            let final_vesting_duration_secs: i128 =
                (vesting.end_timestamp - start_timestamp).into();

            let truncated_current_vesting_duration_secs: i128 =
                truncated_current_vesting_duration_secs.into();

            let linear_vest_amount: i128 = (vesting.linear_vest_amount
                * truncated_current_vesting_duration_secs)
                / final_vesting_duration_secs;

            vesting_amount = vesting_amount + linear_vest_amount;
        }

        vesting_amount
    }

    /// Allows the admin to withdraw ERC20 tokens not locked in vesting.
    pub fn withdraw_admin(env: Env, caller: Address, amount_requested: i128) {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or_else(|| Map::new(&env));

        // Access control check
        Self::admin_check(caller.clone(), admins.clone());

        let amount_remaining = Self::amount_to_withdraw_by_admin(env.clone());
        assert!(amount_remaining >= amount_requested, "Insuffisance balance");

        let token_address: Address = env.storage().persistent().get(&TOKEN_ADDRESS).unwrap();

        TokenClient::new(&env, &token_address).transfer(
            &env.current_contract_address(),
            &caller,
            &amount_requested,
        );

        env.events()
            .publish((ADMIN_WITHDRAWN,), (caller, amount_requested));
    }

    /// Withdraws other ERC20 tokens accidentally sent to the contract's address.
    pub fn withdraw_other_token(env: Env, caller: Address, other_token_address: Address) {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or_else(|| Map::new(&env));

        // Access control check
        Self::admin_check(caller.clone(), admins.clone());

        assert!(
            other_token_address != Self::get_token_address(env.clone()),
            "Invalid other token"
        );

        let balance =
            TokenClient::new(&env, &other_token_address).balance(&env.current_contract_address());

        TokenClient::new(&env, &other_token_address).transfer(
            &env.current_contract_address(),
            &caller,
            &balance,
        );

        env.events()
            .publish((ADMIN_WITHDRAWN_OTHER,), (caller, balance));
    }

    /// Returns the amount of tokens that are available for the admin to withdraw.
    pub fn amount_to_withdraw_by_admin(env: Env) -> i128 {
        let token_address: Address = env.storage().persistent().get(&TOKEN_ADDRESS).unwrap();

        let balance =
            TokenClient::new(&env, &token_address).balance(&env.current_contract_address());

        let reserved_tokens: i128 = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(0);

        balance - reserved_tokens
    }

    /// Retrieves information about a specific vesting arrangement.
    pub fn get_vesting_info(env: Env, vesting_id: u64) -> Vesting {
        let vesting_by_id: Map<u64, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or_else(|| Map::new(&env));

        // This will panic if there is no vesting associated with a given id.
        vesting_by_id.get(vesting_id).unwrap()
    }

    /// Returns all recipient addresses which have at least one vesting schedule set.
    pub fn get_all_recipients(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&RECIPIENTS)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns the list of recipients in a specific range, `from` being inclusive and `to` being exclusive.
    pub fn get_all_recipients_sliced(env: Env, from: u32, to: u32) -> Vec<Address> {
        let recipients: Vec<Address> = env
            .storage()
            .persistent()
            .get(&RECIPIENTS)
            .unwrap_or_else(|| Vec::new(&env));

        recipients.slice(from..to)
    }

    /// Returns the number of recipients.
    pub fn get_all_recipients_len(env: Env) -> u32 {
        let recipients: Vec<Address> = env
            .storage()
            .persistent()
            .get(&RECIPIENTS)
            .unwrap_or_else(|| Vec::new(&env));

        recipients.len()
    }

    /// Returns the list of vestings for the recipient.
    pub fn get_all_recipient_vestings(env: Env, recipient: Address) -> Vec<u64> {
        let recipient_vestings: Map<Address, Vec<u64>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or_else(|| Map::new(&env));

        recipient_vestings
            .get(recipient)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns the list of vestings for the recipient in a specific range, `from` being inclusive and
    /// `to` being exclusive.
    pub fn get_all_recipient_vesting_sliced(
        env: Env,
        from: u32,
        to: u32,
        recipient: Address,
    ) -> Vec<u64> {
        let recipient_vestings: Map<Address, Vec<u64>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or_else(|| Map::new(&env));

        let vestings: Vec<u64> = recipient_vestings
            .get(recipient)
            .unwrap_or_else(|| Vec::new(&env));

        vestings.slice(from..to)
    }

    /// Returns the length of all vestings for the recipient.
    pub fn get_all_recipient_vestings_len(env: Env, recipient: Address) -> u32 {
        let recipient_vestings: Map<Address, Vec<u64>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or_else(|| Map::new(&env));

        recipient_vestings
            .get(recipient)
            .unwrap_or_else(|| Vec::new(&env))
            .len()
    }

    /// Checks if a given address is a recipient of any vesting schedule.
    pub fn is_recipient(env: Env, recipient: Address) -> bool {
        let recipient_vestings: Map<Address, Vec<u64>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or_else(|| Map::new(&env));

        let recipient_ids: Vec<u64> = recipient_vestings
            .get(recipient)
            .unwrap_or_else(|| Vec::new(&env));

        recipient_ids.len() != 0
    }

    /// Returns the address of the token used in the vesting contract.
    pub fn get_token_address(env: Env) -> Address {
        env.storage().persistent().get(&TOKEN_ADDRESS).unwrap()
    }

    /// Returns the amount of token reserved for vesting in the contract.
    pub fn get_tokens_reserved_for_vesting(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(0)
    }

    /// Internal version of `create_vesting`, used for `create_vesting_batch`.
    /// Same but without authentication, required to make `create_vesting_batch` work properly.
    ///
    /// Creates a vesting schedule for a recipient and returns a vesting ID.
    fn create_vesting_internal(
        env: Env,
        caller: Address,
        recipient: Address,
        start_timestamp: u64,
        end_timestamp: u64,
        timelock: u64,
        initial_unlock: i128,
        cliff_release_timestamp: u64,
        cliff_amount: i128,
        release_interval_secs: u64,
        linear_vest_amount: i128,
    ) -> u64 {
        assert!(
            initial_unlock >= 0 && cliff_amount >= 0 && linear_vest_amount >= 0,
            "Invalid amount"
        );
        assert!(
            linear_vest_amount + cliff_amount != 0,
            "Invalid vested amount"
        );
        assert!(
            start_timestamp != 0 && start_timestamp < end_timestamp,
            "Invalid start timestamp"
        );
        assert!(release_interval_secs != 0, "Invalid release interval");

        if cliff_release_timestamp == 0 {
            assert!(cliff_amount == 0, "invalid cliff amount");
            assert!(
                (end_timestamp - start_timestamp) % release_interval_secs == 0,
                "Invalid interval length"
            );
        } else {
            assert!(cliff_amount != 0, "Invalid cliff amount");
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

        let total_expected_amount = initial_unlock + cliff_amount + linear_vest_amount;

        let reserved_tokens = env
            .storage()
            .persistent()
            .get(&TOKENS_RESERVED_FOR_VESTING)
            .unwrap_or(0_i128)
            + total_expected_amount;

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
            claimed_amount: 0,
        };

        let vesting_id: u64 = env.storage().persistent().get(&NONCE).unwrap_or(0);
        let new_vesting_id: u64 = vesting_id + 1;
        env.storage().persistent().set(&NONCE, &new_vesting_id);

        if !Self::is_recipient(env.clone(), recipient.clone()) {
            let mut recipients: Vec<Address> = env
                .storage()
                .persistent()
                .get(&RECIPIENTS)
                .unwrap_or_else(|| Vec::new(&env));

            recipients.push_back(recipient.clone());
            env.storage().persistent().set(&RECIPIENTS, &recipients);
        }

        let mut vesting_by_id: Map<u64, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or_else(|| Map::new(&env));

        vesting_by_id.set(vesting_id, vesting.clone());
        env.storage()
            .persistent()
            .set(&VESTING_BY_ID, &vesting_by_id);

        let mut recipient_vestings: Map<Address, Vec<u64>> = env
            .storage()
            .persistent()
            .get(&RECIPIENT_VESTINGS)
            .unwrap_or_else(|| Map::new(&env));

        let mut recipient_ids: Vec<u64> = recipient_vestings
            .get(recipient.clone())
            .unwrap_or_else(|| Vec::new(&env));
        recipient_ids.push_back(vesting_id);
        recipient_vestings.set(recipient.clone(), recipient_ids);

        env.storage()
            .persistent()
            .set(&RECIPIENT_VESTINGS, &recipient_vestings);

        env.events()
            .publish((VESTING_CREATED,), (vesting_id.clone(), recipient, vesting));

        let token_address: Address = env.storage().persistent().get(&TOKEN_ADDRESS).unwrap();

        TokenClient::new(&env, &token_address).transfer_from(
            &env.current_contract_address(),
            &caller,
            &env.current_contract_address(),
            &total_expected_amount,
        );

        vesting_id
    }

    /// Access control check for admin functions.
    fn admin_check(caller: Address, admins: Map<Address, bool>) {
        caller.require_auth();
        if !admins.get(caller.clone()).unwrap_or(false) {
            panic!("Not an admin");
        }
    }
}

mod test;
