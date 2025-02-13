#![no_std]
use manager::{TokenVestingManagerClient, Vesting};
use soroban_sdk::{contract, contractimpl, symbol_short, Bytes, Env, Symbol};

mod komet;
mod manager;

pub const MANAGER_ADDR: &[u8; 32] = b"mngr_ctr________________________";
pub const MANAGER_KEY: Symbol = symbol_short!("MNGR");


#[contract]
pub struct TestManagerContract;

#[contractimpl]
impl TestManagerContract {
    pub fn init(env: Env, manager_hash: Bytes) {
        let manager = komet::create_contract(&env, &Bytes::from_array(&env, &MANAGER_ADDR), &manager_hash);
        env.storage().instance().set(&MANAGER_KEY, &manager);
    }

    /// Tests that all `linear_amt` tokens are fully vested at the end of the schedule.
    /// 
    /// # Parameters
    /// - `start_t`: The start time of the vesting schedule.
    /// - `end_t`: The end time of the vesting schedule.
    /// - `linear_amt`: The total amount of tokens to be vested linearly over time.
    /// - `interval`: The interval at which vesting progresses.
    /// 
    /// This test verifies that by `end_t`, the entire `linear_amt` has been vested.
    pub fn test_all_vested_at_the_end(
        env: Env,
        start_t: u64,
        end_t: u64,
        linear_amt: i128,
        interval: u64,
    ) -> bool {

        // Assume that the arguments satisfies the requirements made by the protocol

        if end_t <= start_t // start_t should be less than end_t
        || linear_amt <= 0  // linear amount should be positive
        || interval == 0    // interval should be non-zero
        {
            return true;
        }

        let duration = end_t - start_t;
        if duration % interval != 0 {   // duration should be a multiple of the interval
            return true;
        }

        // // overflow check
        // if linear_amt.checked_mul(duration as i128).is_none() {
        //     return true;
        // }

        // Create a client for calling the vesting manager
        let manager = env.storage().instance().get(&MANAGER_KEY).unwrap();
        let manager_client = TokenVestingManagerClient::new(&env, &manager);

        // Create a vesting with the given arguments
        let vesting = Vesting {
            recipient: env.current_contract_address(),
            start_timestamp: start_t,
            end_timestamp: end_t,
            release_interval_secs: interval,
            linear_vest_amount: linear_amt,
            // default parameters. no cliff, no initial unlock, no timelock...
            deactivation_timestamp: 0,
            timelock: 0,
            cliff_release_timestamp: 0,
            initial_unlock: 0,
            cliff_amount: 0,
            claimed_amount: 0,
        };

        // Calculate the vested amount by the end of the schedule
        let vested_amt = manager_client.calculate_vested_amount(&vesting, &end_t);

        // Check that all the amount is vested
        linear_amt == vested_amt
    }
}
