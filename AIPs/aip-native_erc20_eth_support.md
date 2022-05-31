---
aip: 3
title: ERC20 interface support for ETH
description: Support ERC20 interface for native ETH on Aurora without a wrapper.
author: Marcelo Fornet (@mfornet)
discussions-to: https://forum.aurora.dev/t/native-erc20-support-for-eth/309
status: Draft
type: Standards Track
category (*only required for Standards Track): Aurora-Engine
created: 2022-05-29
---

## Abstract

Add a precompile that implements the [ERC20](https://eips.ethereum.org/EIPS/eip-20) interface over native ETH on Aurora. This precompile will be perceived on the network as a regular contract similar to a wrapped Ethereum contract, with the main benefit that no wrap or unwrap is needed. It is expected that applications that wants to interact with ETH tokens via an ERC20, use the provided contract rather than a custom wETH token.

## Motivation

Historically all applications that deal with multiple tokens implementing the ERC20 interface don't interact directly with ETH token, but instead use a wrapper. This wrapper is a smart contract that implements the ERC20 interface, and allows user to wrap/unwrap ETH token at a 1:1 rate. One popular contract on Ethereum is [wETH](https://etherscan.io/address/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2#code). Having a wrapper around ETH has multiple problems with regard to usability. It increases the cognitive load on the users, fragment the ETH through multiple contracts, it can't be used to pay network gas fees, and it is not advisable to sent them over bridges (where it gets double wrapped). Aurora uses ETH as the base token, so a similar problems exist in the ecosystem.

Aurora is in a unique position, where it is possible to implement a contract via a precompile, such that users and applications can choose their preferred way to interact with the tokens (either by using the ERC20 interface or low level EVM primitives).

## Specification

A new precompile `Erc20Eth` is introduced at address: `keccak("Erc20Eth")[12:] = 1f6392791e1654ac3f13d9f026a77463d872d8c4`. This precompile will allow users to manage their ETH balance using the ERC20 interface.

When users adds this token to their wallets, they must expect to see the same amount of tokens as the total ETH they hold. In case of reverts all interactions with this contract must be reverted as well as expected.

### Input / Output

Since this precompile will be perceived as a regular smart contract by other contracts and users, it should decode/encode the inputs and outputs using the [ABI](https://docs.soliditylang.org/en/v0.8.14/abi-spec.html) format.

```rust
impl Precompile for ERC20Eth {
    fn run(
        &self,
        input: &[u8],
        target_gas: Option<EthGas>,
        context: &Context,
        is_static: bool,
    ) -> EvmPrecompileResult {
        // It's not allowed to call exit precompiles in static mode
        if is_static {
            return Err(ExitError::Other(Cow::from("ERR_INVALID_IN_STATIC")));
        }

        // Decode ABI (https://docs.soliditylang.org/en/v0.8.14/abi-spec.html)
        let (selector, args) = decode_abi(input, ERC20_ABI_SCHEMA);

        match selector {
            // Selector for transfer(address,uint256)
            "a9059cbb" => self.transfer(args.to, args.balance),
            // Every method from the interface must be implemented.
            // See reference implementation section
            ...
        }
    }
}
```

### State mutability

This precompiles mutates the state of the EVM. In particular it has access to an allowances map, which is used to track the amount of tokens that are allowed to be spent by a given address; and to the ETH balance of the accounts. Any modification done to the state must be done in a way that allows:

-   Reverting the change in the case the whole context where the call to the precompile was made is reverted.
-   Make sure access to account balance and allowance in the same transaction that previously modified this part of the state, fetch the right values.

Eager modifications of the state should not be applied, as this will result in a non-reversible state change.

### Storage

The allowances should be stored in a map on the storage associated to the address of the precompile. Balances should not be stored, since the EVM is already keeping track of these values globally.

## Rationale

Notice that this contracts needs to be implemented as a precompile, since it will have read/write access to the internal balance of each user.

### Simpler precompile

One alternative is having a simpler precompile that performs ETH transfers between two accounts on Aurora. Similar to `internal_transfer` function described in [Reference Implementation Section](#reference-implementation). Only one account will be able to invoke this precompiles, other attempts to call it will result in failure. On this address a verified Wrapped Ethereum implementation is deployed, that make use of the precompile to perform the transfers.

This is a reasonable approach that should be considered. Most importantly the engine code will be less complex. It could be argued that this is not entirely true, given that the whitelisted code, should be considered part of the engine.

The main benefit of not using this approach, is about performance and granular control. Having the whole contract implemented builtin allows a more efficient use of the state and the computation. A secondary benefit is that it will create a baseline for a WASM native ERC20 contract. This can be used as a reference of how much gas can be saved in a native ERC20 implementation versus vanilla EVM ERC20 contracts.

### Why not using wETH

Wrapped tokens are already used to manage ETH through the ERC20 interface. While this is a simpler solution to the problem, arguably this proposal brings more value to the ecosystem. It is expected that all wETH deployments gets deprecated quickly after the deployment of this precompile.

## Backwards Compatibility

Previous contracts are not affected. While internally this is a non-breaking change, it might affect the way explorers and some external tools handle ETH balance. For example, an external tool that only looks at balance changes during contract executions and aggregate the values to compute the current balance, will have the wrong value for addresses that operates using this interface.

## Test Cases

TODO

## Reference Implementation

```rust
struct ERC20Eth {
    /// Map with all allowances created. This is stored on the storage associated to the address
    /// of the precompile.
    allowances: BTreeMap<Address, BTreeMap<Address, u256>>,
}

/// ERC20 interface implementation. The precompile should support all methods from the ERC20 interface.
impl ERC20Eth {
    /// The name of the token is ETH (no wrapping required).
    fn name(&self) -> String {
        "ETH".to_string()
    }

    /// The symbol of the token is ETH (no wrapping required).
    fn symbol(&self) -> String {
        "ETH".to_string()
    }

    /// Same amount of decimals as in regular ETH.
    /// https://ethereum.stackexchange.com/questions/363/why-is-ether-divisible-to-18-decimal-places
    fn decimals(&self) -> u8 {
        18
    }

    /// ETH total supply in Aurora. This should be the same as `ft_total_supply`
    fn totalSupply(&self) -> u256 {
        self.ft_total_supply() as u256
    }

    /// Balance of the current address.
    /// Similar to BALANCE [0x31](https://www.evm.codes/#31) opcode.
    fn balanceOf(&self, owner: Address) -> u256;

    /// Transfer tokens from the msg.sender to the `to` address.
    /// Similar to CALL [0xF1](https://www.evm.codes/#f1) opcode.
    /// However no CALL is actually performed, and only the tokens are transferred.
    ///
    /// - It should throw if the message sender doesn't have enough balance.
    ///
    /// A `Transfer` event must be emitted in case of success, even for 0 amount.
    fn transfer(&mut self, to: Address, value: u256) -> bool;

    /// Transfer tokens from the `from` address to the `to` address.
    /// Similar to CALL [0xF1](https://www.evm.codes/#f1) opcode.
    /// However no CALL is actually performed, and only the tokens are transferred.
    ///
    /// - It should throw if the message sender doesn't have enough allowance.
    /// - It should throw if the `from` address doesn't have enough balance.
    ///
    /// A `Transfer` event must be emitted in case of success, even for 0 amount.
    fn transferFrom(&mut self, from: Address, to: Address, value: u256) -> bool;

    /// Approve `spender` address to spend `value` tokens on behalf of the sender.
    /// Sets the allowance to the `spender` address to `value`.
    ///
    /// An `Approval` event must be emitted in case of success, even for 0 amount.
    fn approve(&mut self, spender: Address, value: u256) -> bool;

    /// Returns the amount of tokens that an owner allowed to a spender.
    fn allowance(&self, owner: Address, spender: Address) -> u256;
}


impl ERC20Eth {
    /// Function used to modify accounts balance. This must be the only interface that allows
    /// modifying balance accounts. All functions that require balance modifications (transfer / transferFrom)
    /// should use this function.
    fn internal_transfer(&mut self, from: Address, to: Address, value: u256) -> bool;
}
```

## Security Considerations

Following are some considerations taken into account about what could go wrong with the addition of this precompile.

-   **State mutability:** This precompile should performs lazy mutation to the state of the EVM during its execution. This will allow accessing the write values at every moment, and rolling back state updates if required. See [State Mutability Section](#state-mutability) for more details.
-   **Internal transfer:** Internal transfer will perform "unusual" changes to the balance of each account. Access to this function should be properly managed in the code, so it can't be invoked by mistake in any place that it is not supposed to be used. Rust compiler should help to achieve this.
-   **Extra balance attached:** The user might attach some balance to the invocation of the precompile. Balance of sender and receiver must be applied before executing the precompile.
-   **Invalid input:** If the user calls the precompile with an invalid input (invalid selector or invalid arguments), the precompile should fail early without applying any update. In particular no fallback function is supported, so it is not possible to send ETH to this account.
-   **Implicit Account Ids (NEP-141):** Aurora Engine currently handle internal balance for accounts inside aurora, and for NEAR Accounts using the NEP-141 interface. The introduction of this precompile should not affect the dual management of ETH balance.

## Copyright

Copyright and related rights waived via [CC0](https://creativecommons.org/publicdomain/zero/1.0/).
