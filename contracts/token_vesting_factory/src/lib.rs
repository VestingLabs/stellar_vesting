#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec,
    U256,
};

#[contract]
pub struct TokenVestingFactory;

#[contractimpl]
impl TokenVestingFactory {}

mod test;
