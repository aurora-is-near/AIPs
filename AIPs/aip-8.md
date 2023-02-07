---
aip: 8
title: Aurora Block Hashchain
description: Introduction of an additional block field for Aurora blockchain cryptographic verification.
author: Michael Birch (@birchmd), Leandro Casuso Montero (@Casuso)
discussions-to: https://hackmd.io/@birchmd/rJXGBMnoj
status: Draft
type: Standards Track
category (*only required for Standards Track): Aurora-Engine
created: 2023-02-07
---

## Abstract

Blocks generated in Aurora network are virtual blocks compatible to Ethereum that are derived from NEAR blocks. This API is a formal specification for a new Aurora block field: `block hashchain`. Block hashchain is in the form of a hash, constructed from Aurora block transactions, block height and previous block hashchain, and it will serve for Aurora blockchain cryptographic verification.

## Motivation

As of today, Aurora blocks are populated with a “virtual” hash that has nothing to do with the contents of the block (transactions or state changes). The reason for this is because of coupling with the EVM opcode that can get block hash from height and the fact that we do not want to keep a history of blocks in the Engine contract itself.

Even if we cannot change the “official” blockhash of the Aurora blocks, we can still introduce an additional field that allows cryptographic verification that the data is not tampered with. To avoid confusion with the exiting blockhash field, we will call this new field the “block hashchain".

The goal of the block hashchain is to enable someone running a non-archival NEAR node to verify Aurora data is not tampered with, while minimizing the on-chain overhead.

## Specification

### Genesis Hashchain

Aurora genesis block `G` is the block with the transaction where `aurora` account was created. We will consider the block hashchain of blocks before `G` as zero (`0x00`). 

For the Engine, at some block height `H` we seed the process with the precomputed hashchain for the block at height `H - 1` using the Refiner (see the note below). This genesis hashchain value will be stored in the Aurora Engine state permanently as a constant (this allows anyone with a non-archival node to view the genesis hashchain because it is always present in the engine state).

We will compute the hashchain for the block at height `H - 1` off-chain using the Refiner and push the value to the Engine as part of the upgrade that includes this functionality. The precomputed hashchain will follow the scheme outlined below starting from `G`.

### Computation

All calls to mutable functions in the Aurora Engine (i.e. the same functions that the Refiner includes in Aurora blocks) will contribute to a “transactions hash” in the following way:

- the “intrinsic transaction hash” will be `hash(hash(method_name) || hash(input_bytes) || hash(output_bytes))`, where `||` means bytes-concatenation;
- the “transactions hash” will be updated as `txs_hash = hash(txs_hash || intrinsic_tx_hash)`, where the initial value of txs_hash is `0x00` if there have not yet been any transactions in this block.

When there is a change in block height (the Engine will know this has happened by keeping the last seen height in its state), it will compute the overall block hashchain for the previous block as follows:

```block_hashchain = hash(previous_block_hashchain || hash(block_height.to_le_bytes()) || txs_hash)```

For example, the hashchain for the block at height `H` will be

```block_hashchain_H = hash(precomputed_hashchain || hash(H.to_le_bytes()) || txs_hash_H)```

and the hashchain for the following block will be

```block_hashchain_H_plus_1 = hash(block_hashchain_H || hash((H + 1).to_le_bytes()) || txs_hash_H_plus_1)```

The standard `keccak256` hashing function will be used for all the hashes.

### Skip blocks

It could be that not every NEAR block contains an Aurora transaction, or indeed it could be that a NEAR block skipped some height; either way we want to have an Aurora block hashchain for every height (because the refiner produces blocks at all heights). So if the Engine detects a height change of more than one then it will need to compute the intermediate block hashchain before proceeding. For example, if consecutive NEAR blocks had heights `H'`, `H' + 2` then the following sequence of hashchains should be produced:

```
block_hashchain_H_prime          = hash(block_hashchain_H_prime_minus_one || hash((H').to_le_bytes()) || txs_hash_H_prime)
block_hashchain_H_prime_plus_one = hash(block_hashchain_H_prime || hash((H' + 1).to_le_bytes()) || 0x00)
block_hashchain_H_prime_plus_two = hash(block_hashchain_H_prime_plus_one || hash((H' + 2).to_le_bytes()) || txs_hash_H_prime_plus_two)
```

### Optional extension

Depending on the viability of computing the EVM logs bloom filter in the engine itself (need to figure out how much gas that would cost), then the block hashchain should include that as well.

```block_hashchain = hash(previous_block_hashchain || hash(block_height.to_le_bytes()) || txs_hash || hash(logs_bloom))```

### Cryptographic Verification

The engine state will always contain the genesis hashchain as well as the hashchain of the last completed block. To verify a given Aurora stream is correct using a non-archival NEAR node do the following:

1. Check the genesis hashchain in the Engine (obtained via a view call to the NEAR node) matches the hashchain in block `G`.
2. Using the data in the stream, follow the scheme above to compute the hashchain for the block at some recent height and check that hashchain matches the value in the Engine state (obtained via a view call again).

If the two values matches, then the data must match what happened on-chain. This is called “range verification” because it verifies a whole range of blocks by checking only the endpoint hashchains. The reason this works is because the hashchain includes all the transaction data and block heights in between, thus any change to the local copy of the data would produce an incorrect final hash.

## Rationale

The current design aims to compute the bock hashchain while minimizing the on-chain overhead.

Every time that a new transaction is received on the Engine, the intrinsic transaction hash and the transactions hash will be computed. Both computations should be fast since they only depend on a single transaction. When there is a change in block height, the overall block hashchain will be computed, but this only depends on already know values: previous block hashchain, block height and transactions hash. With this approach we are effectively distributing the block hashchain computation between every transaction that Engine receives.

An alternative approach would be to compute the block hashchain as a single operation when there is a change in block height. Besides memory considerations, we will need to compute a hash for a considerable amount of data that includes all the transactions in the block. This could impact the on-chain performance at that point depending on the number of transactions in the block.

It seems more reasonable to distribute the load of the computation by transactions versus the bulk operation at the end.

## Backwards Compatibility

As this is an addition of a new field to the blocks there should not be any issues regarding backwards compatibility. Tools, indexers or consumers of Aurora blocks could ignore the field and continue to work as before or add the new field to have updated information.

## Test Cases

<!-- TODO: Fill following section -->
<!-- Maybe we can add the following test:
Include as part of the assets the Aurora genesis block 'G' and the next block 'N'.
Add here the result of the txs_hash of 'G' and the hashchain of 'G'.
Add then the result of the txs_hash of 'N' and the hashchain of 'N' using all the info.-->

Test cases for an implementation are mandatory for AIPs that are affecting consensus changes. If the test suite is too large to reasonably be included inline, then consider adding it as one or more files in `../assets/aip-####/`.

## Security Considerations

The security considerations of the block hashchain should be focused on the possible attempts to break the Aurora blockchain cryptographic verification that it aims to offer.

Given the genesis block hashchain as well as the block hashchain of the last completed block from the Engine, we can verify an Aurora stream following the procedure described above on Cryptographic Verification. We show that any change to the stream would produce an incorrect final hash different from the one of the last completed block from the Engine.

We will describe some important properties and then proceed to test the possible break attempts. Through the text, we will use the word different in between quotes, "different", to denote a highly probably difference. This is to avoid repeatedly clarifications.

### Properties

The "Collision resistance" property of cryptographic hashing functions, and specifically of the `keccak256` function (`hash`), establishes the difficulty to find two different inputs `x1` and `x2` such `hash(x1) = hash(x2)`. Let us call this property `P0`.

We heavily rely on the security offered by the composition of a hashing functions, so offering the same security model as known blockchains, having its roots on Merkle Trees and in `P0`. We will then assume safely, that any data point change in a chain of compositions of a hash function, will result on a “different” output. Let us call this property `P1`.

Every parameter on the transactions hash computation and on the final block hashchain computation, is the result of hash operation. This ensures that the bytes-concatenation `||` operations that are applied to the parameters on both computations are injective, since the parameters have the same fixed size. This is important so a bad actor cannot shift bytes from one parameter to the next one to create another input with the same resulting concatenation. Let us call this property `P2`.

### Break attempts

Without losing generality, lets assume that a bad actor attempts to change at least the `input` value of a transaction `t_x` in a block `B`. Below are the original `_1` and new `_2` computations of the intrinsic transaction hash of `t_x`.

```
intrinsic_tx_hash_1 = hash(hash(method_name) || hash(input_bytes_1) || hash(output_bytes))
intrinsic_tx_hash_2 = hash(hash(method_name) || hash(input_bytes_2) || hash(output_bytes))
```

Since at least the `input` it’s different, then its respective hash value should be "different" by `P0`. By `P2`, we can then claim that the general hash functions inputs would be different after the bytes-concatenation. Then again, by `P0` the resulting hash output should be "different" so `intrinsic_tx_hash_1 != intrinsic_tx_hash_2`.

This different intrinsic transaction hash `intrinsic_tx_hash_2` of `t_x` will cause a difference in the transactions hash of `B`, by `P1`, since its computation is a compositions of hash functions that includes the intrinsic transaction hash of `t_x`.

```txs_hash = hash(txs_hash || intrinsic_tx_hash)```   (we use `0x00` as an initial value for `txs_hash`)

So, instead of having transactions hash `txs_hash_1` for `B`, we would have `txs_hash_2`.

In another case, let’s assume that the attempt is to add a transaction to `B`. Then, there will be also a change in the compositions of the hashing functions for the computation of the transactions hash, thus resulting in a "different" value because of `P1`. Similar would happen in case of removing a transaction from `B`.

In summary, any change to the transactions of `B` would result in a different value `txs_hash_2` of the transactions hash. Having a different value of the transactions hash, then the block hashchain of `B` should be "different", because of `P2` and `P0`. Let us assume that `b` is the height of `B`.

```
block_hashchain = hash(previous_block_hashchain || hash(block_height.to_le_bytes()) || txs_hash)

block_hashchain_B_1 = hash(b_minus_1_block_hashchain || hash(b.to_le_bytes()) || txs_hash_1)
block_hashchain_B_2 = hash(b_minus_1_block_hashchain || hash(b.to_le_bytes()) || txs_hash_2)
```

So `block_hashchain_B_1 != block_hashchain_B_2` (probabilistically speaking), by applying properties `P2` and `P0`.

It worth also to mention that any attempts to tamper with the block height or to use a different previous block hashchain, would result also in a "different" block hashchain value for `B`, because of `P2` and `P0` too.

Finally, once the block hashchain of a block in the stream has changed, then we can claim that the computed block hashchain of the last completed block of the stream would be "different" from the known one in the Engine. This is because the computation of the block hashchain is the compositions of hashing functions, so `P1` applies:

```block_hashchain = hash(previous_block_hashchain || hash(block_height.to_le_bytes()) || txs_hash)```

## Copyright

Copyright and related rights waived via [CC0](https://creativecommons.org/publicdomain/zero/1.0/).