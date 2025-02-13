use soroban_sdk::{contractclient, contracttype, Address, Env};

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

#[allow(dead_code)]
#[contractclient(name = "TokenVestingManagerClient")]
pub trait TokenVestingManagerTrait {

    fn init(env: Env, factory_caller: Address, token_address: Address);

    fn calculate_vested_amount(_env: Env, vesting: Vesting, reference_timestamp: u64) -> i128;

}
