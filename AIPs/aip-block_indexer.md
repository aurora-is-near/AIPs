---
aip: TODO:
title: Aurora Blocks Indexing Reference
description: Formal specification for indexing Aurora Blocks.
author: Marcelo Fornet (@mfornet), Michael Birch (@birchmd)
discussions-to: TODO:
status: Live
type: Standards Track
category: Interface
created: 2022-09-19
---

## Abstract

Blocks generated in Aurora network are virtual blocks compatible to Ethereum that are derived from NEAR Blocks. This AIP is a formal specification about how Aurora blocks should be derived.

## Motivation

One of Aurora's main proposition is to be compatible with Ethereum. Aurora on its core is a smart contract on NEAR Protocol that doesn't have blocks of its own. Instead, Ethereum "compatible" blocks are derived from NEAR blocks, and these are used as the basic component of information for indexing tools. Some example of applications relying on these tools are RPC providers, explorers, and oracles among others.

This AIP is in Live mode since the indexing standard is subject to transformations. Either for correcting bugs or to adequate new features added to Ethereum, Aurora or NEAR. In any case all changes to the specifications should be enumerated in the [change log section](#changes).

## Specification

The goal of this specification is to describe how Aurora blocks are derived from NEAR blocks. In particular how the following function can be built.

```python
def generate_aurora_block(
        height: int,
        nearcore: NearCoreInterface
    ) -> AuroraBlock:
    """
    height: Height of the Aurora block to be generated.
    nearcore: Interface to a nearcore instance with access to blocks, transactions and state from NEAR chain.
    """
```

### Genesis

There is a virtual Aurora block at each consecutive height starting from height `G` (genesis). `G` is the block height of the transaction where `aurora` account was created. This number might be different in different networks.

<!-- TODO: Fill data in following fields -->

| Network        | Genesis Height                          | Deployment Receipt                             |
| -------------- | --------------------------------------- | ---------------------------------------------- |
| Aurora Mainnet | [??](https://explorer.near.org/blocks/) | [??](https://explorer.near.org/transactions/#) |
| Aurora Testnet | [??](https://explorer.near.org/blocks/) | [??](https://explorer.near.org/transactions/#) |

This means that genesis height `G` is not necessarily `0` and blocks before `G` doesn't exist.

### Aurora Block

Following is the definition of Aurora blocks. Find below discussion about how each individual field is computed.

```rs
struct AuroraBlock {
    /// Chain id of the current network.
    chain_id: u64,
    /// Hash of the block
    hash: H256,
    /// Hash of the parent block.
    parent_hash: H256,
    /// Height of the block. This height matches the NEAR
    height: u64,
    /// Implicit account id of the NEAR validator that mined this block
    miner: Address,
    /// Timestamp where the block was generated.
    timestamp: u64,
    /// Gas limit will be always U256::MAX
    gas_limit: U256,
    /// Sum of the gas used on each tx included in the block.
    gas_used: U256,
    /// Log bloom of the block. Aggregation of transaction logs bloom.
    logs_bloom: Bloom,
    /// Size of the block in bytes.
    size: U256,
    /// Transaction root using Ethereum rules
    transactions_root: H256,
    /// State root:
    /// TODO(belo) Uses NEAR state root of the block. While this doesn't match Ethereum rules to compute
    /// proofs, it contains the relevant information to make any proof about any piece of state in Aurora.
    /// Note however that the state root included in block X matches the previous block. This means that
    /// at block X you can only build proofs of events that happened prior the execution of that block.
    state_root: H256,
    /// Receipts root of the merkle tree of all receipts.
    receipts_root: H256,
    /// List with all txs in the current block.
    //// TODO: Txs will be extracted from the receipts executed in
    /// a block. This means that potentially the original NEAR tx could have been created in an
    /// older block, but it was executed in the current block. For NEAR txs that create several
    /// contract calls, potentially hitting aurora several times, a different Ethereum tx will be
    /// created for each receipt.
    transactions: Vec<AuroraTransaction>,
    /// Metadata to recover the block on NEAR
    near_metadata: NearBlock,
}

struct AuroraTransaction {
    /// Transaction hash
    hash: H256,
    /// Hash of the block where the transaction was included.
    block_hash: H256,
    /// Height of the block where the transaction was included.
    block_height: u64,
    /// Chain id of the current network.
    chain_id: u64,
    /// Index of the transaction on the block.
    /// TODO: This index is computed after filtering out all
    /// transactions that are not relevant to current aurora chain id.
    transaction_index: u32,
    /// Sender of the transaction.
    /// TODO: If the transaction is not sent via submit, the sender will be
    /// derived using `near_account_to_evm_address`.
    from: Address,
    /// Target address of the transaction. It will be None in case it is a deploy transaction.
    to: Option<Address>,
    /// Nonce of the transaction.
    nonce: U256,
    /// Gas price for the transaction.
    /// TODO Related to Aurora Gas not NEAR Gas.
    gas_price: U256,
    /// Gas limit of the transaction.
    /// TODO: In the context of Aurora it should be U256::MAX
    gas_limit: U256,
    /// Gas used by the transaction
    gas_used: u64,
    /// Max priority (defined in EIP-1559)
    max_priority_fee_per_gas: U256,
    /// Max fee per unit of gas (defined in EIP-1559)
    max_fee_per_gas: U256,
    /// Amount of eth attached to the transaction.
    value: U256,
    /// Bytes passed to the target contract as input.
    input: Vec<u8>,
    /// Output of the transaction. The result from the execution.
    output: Vec<u8>,
    /// List of addresses that will be used during execution of the transaction. (defined EIP-TODO:)
    access_list: Vec<AccessTuple>,
    /// Type format of the transaction.
    tx_type: u8,
    /// Status of the transaction execution.
    status: bool,
    /// Logs recorded during transaction execution. For now they will be empty, since it can't be
    /// computed without access to the storage.
    logs: Vec<Log>,
    /// Logs bloom of the transaction. Aggregation of bloom filters among all logs.
    logs_bloom: Bloom,
    /// Address of the deployed contract. If will be different from `None` in case it is a deploy
    /// transaction.
    contract_address: Option<Address>,
    /// Signature data. Used to recover target address.
    v: u64,
    r: U256,
    s: U256,
    /// Metadata to recover the NEAR transaction/receipt associated with this transaction
    near_metadata: NearTransaction,
}
```

#### Useful data structures

In order to make this document self-contained, all inner data types used to define `AuroraBlock` and `AuroraTransaction` are defined in [`assets/aip-block_indexer/aurora_types.rs`](../assets/aip-block_indexer/aurora_types.rs). Types like `u<n>` should be interpreted as an unsigned integer of `n` bits.

#### Fields skipped

Some of the fields usually included in Ethereum blocks or transactions are omitted in their Aurora counterparts. RPCs or indexers looking to be 100% compatible can use sensible default values of their choice.

##### Block

-   baseFeePerGas: Gas is artificially computed, and real gas paid is denominated in NEAR. There is no base fee per gas as defined in EIP-1559.
-   difficulty: Related to PoW mining algorithm. There is no sensible counterpart in Aurora.
-   mixHash: Related to PoW mining algorithm. There is no sensible counterpart in Aurora.
-   nonce: Related to PoW mining algorithm. There is no sensible counterpart in Aurora.
-   totalDifficulty: Related to PoW mining algorithm. There is no sensible counterpart in Aurora.
-   uncles: Related to PoW mining algorithm. There is no sensible counterpart in Aurora.

##### Transaction

-   cumulativeGasUsed: Gas is artificially computed, and real gas paid is denominated in NEAR. Cumulative sum of gas in a block could be misleading.
-   effectiveGasPrice: // TODO

### Block Fields

#### Chain id

Chain id of the current network. It is constant per chain.

| Network        | Chain Id   |
| -------------- | ---------- |
| Aurora Mainnet | 1313161554 |
| Aurora Testnet | 1313161555 |

#### Hash

Hash of the block. The hash can be deterministically derived from the chain id and the height using the following function:

```python
def block_hash_preimage(height: int, chain_id: int) -> bytes:
    return bytes(25) + struct.pack('>Q', chain_id) + b'aurora' + struct.pack('>Q', height)

def compute_block_hash(height: int, chain_id: int) -> bytes:
    pre_image = block_hash_preimage(height, chain_id)
    return hashlib.sha256(pre_image).digest()
```

For example:

```python
assert block_hash_preimage(GENESIS, 1313161554).hex() == 'asdfasdf' # TODO Set proper value of genesis and compute results properly
assert compute_block_hash(GENESIS, 1313161554).hex() ==  'asdfasdf' # TODO
```

#### Parent hash

Hash of the parent block. Since hash is computed dynamically, hash of the parent block can be derived without access to previous block. In particular hash of the parent block of the genesis block is different from zero.

#### Height

Height `h` of the current block. It is guaranteed that heights from consecutive blocks will be consecutive. i.e: `block(parent_hash).height + 1 == block(hash)`.

#### Miner

In Aurora chain there is no miner in the same sense as in Ethereum an. In some sense NEAR validators, produce NEAR blocks, and a projection of this blocks conform the Aurora blocks. For this reason a sensible value for the miner is the implicit address of the NEAR account id that produced block at height `h`. Note that this miner is not receiving any reward from the point of view of Aurora.

```python
# TODO: Put function for computing implicit_address in Aurora for NEAR Account Id
```

#### Timestamp

Timestamp from NEAR block.

#### Gas limit

<!-- TODO: Continue here all missing sections -->

#### Gas used

#### Logs bloom

#### Size

#### Transactions root

#### state_root: H256,

#### Receipts root

#### Transactions

#### Near Metadata

### Skip Blocks

Every NEAR block is mapped to exactly one Aurora block. However it is possible that some blocks in [NEAR blockchain are skipped](TODO:Link). For example, block at height `N` can be built on top of block at height `N - 3`. In this case, two extra empty blocks are created in Aurora chain, one at height `N - 2` and other at height `N - 1` . This way there is exactly one block at every height from genesis. To compute each field, it is necessary to have access to the previous and next block.

Note: It makes sense to require access to next block, since the only way to figure out that a block was skipped is by seeing next produced block.

-   chain_id: `CHAIN_ID`
-   hash: `compute_block_hash(HEIGHT, CHAIN_ID)`
-   parent_hash: `compute_block_hash(HEIGHT - 1, CHAIN_ID)`
-   height: `HEIGHT`
-   miner: `near_account_to_evm_address(b"")`
-   timestamp: `NEXT_BLOCK.timestamp`
    The timestamp of the next block is used as the timestamp of all consecutive skipped blocks. Because of this it is possible
    that multiple blocks has the same timestamp. It is still the case that timestamp is non-decreasing.
-   gas_limit: `U256::MAX`
-   size: `0`
-   gas_used: `0`
-   transactions_root: `EMPTY_MERKLE_TREE_ROOT`
-   receipts_root: `EMPTY_MERKLE_TREE_ROOT`
-   transactions: `[]`
-   near_metadata: `NearBlock::SkipBlock`
-   state_root: `PREV_BLOCK.state_root`
    The state root of the previous block is used, since the state didn't change on skip blocks.
-   logs_bloom: `0`

<!-- TODO

-   Explain Skip blocks
-   Ignore transactions that failed at NEAR level
-   Each individual action per receipt hitting aurora is a different transaction
-   Blocks should only -->

### Receipts with older format

<!-- TODO: -->

### Requirements

<!-- TODO: What is required to generate an Aurora Block. Previous block and next block in case of skip blocks -->

### Divergence from Ethereum

-   Genesis is not necessarily at height 0
-   [Fields skipped](#fields-skipped)
-   Hash of the [parent block](#parent-hash) of the genesis block is not necessary the zero value.

<!-- TODO: Enumerate main divergence between Aurora blocks and Ethereum Blocks. -->

## Rationale

<!-- TODO: Fill following section -->

The rationale fleshes out the specification by describing what motivated the design and why particular design decisions were made. It should describe alternate designs that were considered and related work, e.g. how the feature is supported in other languages.

## Backwards Compatibility

There have been several tools for indexing Aurora network since early days. All applications that were consuming data from all indexer tools should move to new tools that follow the specifications stated in this AIP. For many of these applications, this means that a full network re-index is required. Notice also that this AIP is in Live mode, hence subject to changes. Every change proposed to this AIP must include a migration path for tools that were compliant with previous versions. Migration that involves re-indexing the whole network from scratch should be considered invasive, and should be avoided if possible.

## Test Cases

Test cases are included as part of each AIP, both for clarity on corner cases and to help developers build their own tools that transforms NEAR blocks into Aurora blocks.

Test cases are in json and have the following structure:

```json
{
    "name": "Short name, same as file name without .json",
    "description": "Description of what is being tested",
    "near_block": "NEAR Block. Could be empty for skip or non existent blocks | Input",
    "aurora_partial_state": "Partial state of aurora engine | Input",
    "aurora_block": "Aurora Block | Expected Output"
}
```

<!--
TODO: Update borealis-refiner-app to create such test cases.

TODO:
-   [ ] Examples should be valid NEAR-Blocks from mainnet / testnet
-   [ ] Skip block && blocks before genesis
-   [ ] Genesis block
-   [ ] Blocks where aurora is updated
-   [ ] Each method available on Aurora (submit, call, etc)... Even view methods that can be used to call
-   [ ] For every method a transaction that failed internally but was NEAR:Succeeded
-   [ ] Failing transactions directed to Aurora
-   [ ] Blocks with more than one transaction
-   [ ] Batched transactions (one failed and other succeeded)
-   [ ] Check in batched transactions output of transactions that succeeded and failed
-   [ ] Submit transactions from known contracts (ERC20 or DEX)
-   [ ] Submit to smart contract that if called and returns the block height and timestamp
-   [ ] Transactions such that their receipts span through multiple blocks (i.e cross contract calls) -->

## Security Considerations

The blocks generated will be used to present final information to users (through explorers), to populate indexing tools, to feed off-chain oracles, among others. This means that decisions will be taken with respect to the data they are receiving. Since Aurora blocks are a virtual component of Aurora network, it is important that consumers of this information understand how is it generated and which assumptions were made in the process. Most importantly see [difference with Ethereum blocks](#divergence-from-ethereum).

The recommended approach to generate validated Aurora blocks, is to fetch relevant pieces from a trusted NEAR node, and build the block from this information. See [requirements](#requirements) section above.

## Changes

Change log of all modifications made to this document. Include the migration proposed to support the new version of the AIP.

## Copyright

Copyright and related rights waived via [CC0](https://creativecommons.org/publicdomain/zero/1.0/).
