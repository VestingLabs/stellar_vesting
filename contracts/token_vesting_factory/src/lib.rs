#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol, Val, Vec};

/// Constants for storage keys.

// Owner of the contract.
const OWNER: Symbol = symbol_short!("OWNER");
// Wasm hash of the TokenVestingManager contract.
const WASM_HASH: Symbol = symbol_short!("WASMHASH");

/// Constants for events.

const NEW_OWNER: Symbol = symbol_short!("NEWOWNER");
const NEW_WASM_HASH: Symbol = symbol_short!("NEWHASH");
const TOKEN_VESTING_MANAGER_CREATED: Symbol = symbol_short!("CREATED");

#[contract]
pub struct TokenVestingFactory;

#[contractimpl]
impl TokenVestingFactory {
    /// Initialization function.
    pub fn init(env: Env, owner: Address, wasm_hash: BytesN<32>) {
        if env.storage().persistent().has(&OWNER) {
            panic!("Already initialized");
        }

        env.storage().persistent().set(&OWNER, &owner);
        env.storage().persistent().set(&WASM_HASH, &wasm_hash);
    }

    /// Deploys a new TokenVestingManager contract and returns its address.
    pub fn new_token_vesting_manager(
        env: Env,
        init_args: Vec<Val>,
        salt: BytesN<32>,
    ) -> (Address, Val) {
        let wasm_hash: BytesN<32> = env.storage().persistent().get(&WASM_HASH).unwrap();

        // Deploy the contract.
        let deployed_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy(wasm_hash);

        // Invoke the init function with the given arguments.
        let res: Val = env.invoke_contract(&deployed_address, &symbol_short!("init"), init_args);

        // self.salt.write(self.salt.read() + 1);

        env.events()
            .publish((TOKEN_VESTING_MANAGER_CREATED,), deployed_address.clone());

        // Return the contract ID of the deployed contract and the result of
        // invoking the init result.
        (deployed_address, res)
    }

    /// Updates the owner of the factory.
    pub fn update_owner(env: Env, caller: Address, new_owner: Address) {
        let owner: Address = env.storage().persistent().get(&OWNER).unwrap();

        // Access control check
        caller.require_auth();
        if caller != owner {
            panic!("Not the owner");
        }

        assert!(new_owner != owner, "New owner wrongly set");

        env.storage().persistent().set(&OWNER, &new_owner);

        env.events().publish((NEW_OWNER,), new_owner);
    }

    /// Updates the Wasm hash of the TokenVestingManager contract.
    pub fn update_vesting_manager_wasm_hash(env: Env, caller: Address, new_wasm_hash: BytesN<32>) {
        let owner: Address = env.storage().persistent().get(&OWNER).unwrap();

        // Access control check
        caller.require_auth();
        if caller != owner {
            panic!("Not the owner");
        }

        let wasm_hash: BytesN<32> = env.storage().persistent().get(&WASM_HASH).unwrap();

        assert!(new_wasm_hash != wasm_hash, "New Wasm hash wrongly set");

        env.storage().persistent().set(&WASM_HASH, &new_wasm_hash);

        env.events().publish((NEW_WASM_HASH,), new_wasm_hash);
    }

    /// Returns the owner of the factory.
    pub fn get_owner(env: Env) -> Address {
        env.storage().persistent().get(&OWNER).unwrap()
    }

    /// Returns the Wasm hash of the TokenVestingManager contract.
    pub fn get_vesting_manager_wasm_hash(env: Env) -> BytesN<32> {
        env.storage().persistent().get(&WASM_HASH).unwrap()
    }
}

mod test;
