#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec,
    U256,
};

// Constants for storage keys.

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
    pub linear_vest_amount: Vec<U256>,
}

#[contract]
pub struct TokenVestingManager;

#[contractimpl]
impl TokenVestingManager {
    // ADMIN RELATED METHODS

    pub fn set_admin(env: Env, admin: Address, is_enabled: bool) {
        // Implementation
    }

    pub fn get_admins_count(env: Env) -> u32 {
        env.storage().persistent().get(&ADMIN_COUNT).unwrap_or(0)
    }

    pub fn is_admin(env: Env, address: Address) -> bool {
        let admins: Map<Address, bool> = env
            .storage()
            .persistent()
            .get(&ADMINS)
            .unwrap_or(Map::new(&env));

        admins.get(address).unwrap_or(false)
    }

    // TOKEN VESTING MANAGER METHODS

    pub fn create_vesting(
        env: Env,
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
        // Implementation
        U256::from_u32(&env, 0)
    }

    pub fn create_vesting_batch(
        env: Env,
        create_vesting_batch_params: CreateVestingBatchParams,
    ) -> Vec<U256> {
        // Implementation
        Vec::new(&env)
    }

    pub fn claim(env: Env, vesting_id: U256) {
        // Implementation
    }

    pub fn revoke_vesting(env: Env, vesting_id: U256) {
        // Implementation
    }

    pub fn calculate_vested_amount(env: Env, vesting: Vesting, reference_timestamp: u64) -> U256 {
        // Implementation
        U256::from_u32(&env, 0)
    }

    pub fn withdraw_admin(env: Env, amount_requested: U256) {
        // Implementation
    }

    pub fn withdraw_other_token(env: Env, other_token_address: Address) {
        // Implementation
    }

    pub fn amount_to_withdraw_by_admin(env: Env) -> U256 {
        // Implementation
        U256::from_u32(&env, 0)
    }

    /// Retrieves information about a specific vesting arrangement.
    pub fn get_vesting_info(env: Env, vesting_id: U256) -> Vesting {
        let vesting_by_id: Map<U256, Vesting> = env
            .storage()
            .persistent()
            .get(&VESTING_BY_ID)
            .unwrap_or(Map::new(&env));

        // This will panic is there is no vesting associated with a given id.
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
            .unwrap_or(Address::from_string(&String::from_str(&env, "0x0")))
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
