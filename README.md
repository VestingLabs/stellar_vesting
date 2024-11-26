# Token Vesting and Grant Distribution Contracts for Stellar Network

This set of contracts provides a flexible and extensible framework for managing
token vesting schedules and grant distributions. The contracts are designed to
support a wide range of use cases, including employee compensation, advisor
grants, community rewards, vestings, and more.


## Key Features 🔑

### Vesting Schedules (`TokenVestingManager`)

Fundamental building block for managing token vesting schedules.

- **Flexible Vesting Schedules**: Create custom vesting schedules for token
  recipients, with support for timelock, cliff, Unlock-Cliff vesting models.
- **Controlled Token Release**: Recipients can withdraw their vested tokens
  according to the vesting schedule, ensuring a controlled release of
  tokens over time.
- **Revocation and Reclamation**: The contract owner has the ability to revoke
  a vesting schedule and reclaim any unvested tokens.
- **Administrative Functions**: The contract owner can also withdraw
  unallocated tokens and tokens of other types, providing additional
  flexibility and control.
- **Multiple Vestings**: Multiple vestings per address.
- **Initial Unlock**: Optional initial unlock of tokens at the
  start of the vesting.
- **Timelock**: Optional timelock on top of vesting schedule to
  prevent premature withdrawals.

# Development
This project uses soroban-sdk 21.0.0. You will need to install Rust and Stellar CLI in order to build the project and run tests.

## Prerequisites
Install Rust and Stellar CLI

## Build and run
1. Compile

```shell
make build
```

2. Test

```shell
make test
```