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

Blocks generated in Aurora network are virtual blocks compatible with Ethereum that are derived from NEAR blocks. This API is a formal specification for a new Aurora block field: `block_hashchain`. Block hashchain is in the form of a hash, constructed from Aurora chain id, contract account id, block height, previous block hashchain, and block transactions, and it will serve for Aurora blockchain cryptographic verification.

## Motivation

As of today, Aurora blocks are populated with a “virtual” hash that has nothing to do with the contents of the block (transactions or state changes). The reason for this is because of coupling with the EVM opcode that can get block hash from height and the fact that we do not want to keep a history of blocks in the Engine contract itself.

Even if we cannot change the “official” blockhash of the Aurora blocks, we can still introduce an additional field that allows cryptographic verification that the data is not tampered with. To avoid confusion with the existing blockhash field, we will call this new field the “block hashchain".

The goal of the block hashchain is to enable someone running a non-archival NEAR node to verify Aurora data is not tampered with while minimizing the on-chain overhead.

## Specification

### Genesis Hashchain

Aurora genesis block `G` is the block with the transaction where `aurora` account was created. We will consider the block hashchain of blocks before `G` as zero (`[0u8; 32]`).

An independent off-chain process will precompute a hashchain seed starting from `G`, and will keep it up to date with the Engine to include recent transactions as soon as possible. For the Engine, at some block height `H` we will pause the Engine so no new write transactions are accepted. This pausing is required to be done by Aurora DAO. We will then insert the hashchian seed on the current block, which most be at height `H + k, k > 0`, compute the small hashchain gap for the `k - 1` blocks in between, and resume the Engine.

From this point the Engine and hashchian mechanism will accept and add all new relevant transactions, so the hashchain will reamin up to date with the contract.

### Computation

All calls to mutable functions in the Aurora Engine (i.e., the same functions that the Refiner includes in Aurora blocks) will contribute to the “transactions hash” of the block, which is computed using a binary Merkle Tree as follows:

- the “intrinsic transaction hash”, `tx_hash`, of a transaction will be `hash(method_name.len().to_be_bytes() || method_name || input_bytes.len().to_be_bytes() || input_bytes || output_bytes.len().to_be_bytes() || output_bytes)`, where `||` means bytes-concatenation;
- the “transactions hash”, `txs_hash`, will be the root hash value of the binary Merkle Tree constructed from the executed transactions of the block. Details about this procedure can be found later. In case there are no transactions on the block, the value will be zero (`[0u8; 32]`).

Also, calls to these functions that return a result including logs, will contribute to the "transactions logs bloom" filter of the block as follows:

- we will use bloom filters with an array of `2048` bits (`256` bytes) and `3` hash functions. I.e, `m = 2048` and `k = 3`.
- the "log bloom" of a log, is the result of accrue the log's address bytes and the log's topics bytes.
- the "intrinsic transaction logs bloom", `tx_logs_bloom`, of a transaction will be the binary `OR` (`|`) of the logs bloom of the transaction's logs.
- the "transactions logs bloom", `txs_logs_bloom`, will be the binary `OR` (`|`) of the `tx_logs_bloom` of the transactions of the block.

When there is a change in block height (the Engine will know this has happened by keeping the last seen height in its state), we will compute the overall block hashchain for the previous block as follows:

```block_hashchain = hash(chain_id || contract_account_id || block_height.to_be_bytes() || previous_block_hashchain || txs_hash || txs_logs_bloom)```

For example, the hashchain for the block at height `H` will be:

```block_hashchain_H = hash(chain_id || contract_account_id || H.to_be_bytes() || precomputed_hashchain || txs_hash_H || txs_logs_bloom_H)```

and the hashchain for the following block will be:

```block_hashchain_H_plus_1 = hash(chain_id || contract_account_id || (H + 1).to_be_bytes() || block_hashchain_H || txs_hash_H_plus_1 || txs_logs_bloom_H_plus_1)```

The standard `keccak256` hashing function will be used for all the hashes.

#### Merkle Tree

A binary Merkle Tree is constructed bottom-up combining couples of hash nodes from the current level to construct the next level. Combining a couple of hash nodes consist of creating a parent hash node, which hash is the hash of the concatenation of the hashes of its children. The starting point in this case is an initial list of hash leaf nodes, and the procedure continues combining level by level until there is only one hash node, the hash root node. If at a level, the final hash node does not have a pair hash node to be combined with, then the final hash node is combined with itself to produce the hash parent node.

We wanted to avoid this approach since the Engine will need to record all the block's transactions hashes until there is a change in block height. We created instead what we call a Stream Compact Merkle Tree to minimize memory usage while maintaining a good performance.

The Stream Compact Merkle Tree structure receives the hash leaf nodes dynamically from a stream, and stores only records of full compact Merkle binary subtrees, called Compact Merkle Subtrees. Every Compact Merkle Subtree record holds only two fields: height and hash. For the subtree that it represents, the height field is the height of the subtree, and the hash is the computed hash root node of the subtree.

The memory utilized by the Stream Compact Merkle Tree structure is just a list (stack) of Compact Merkle Subtree records. For `n` hash leaf nodes, the structure only holds in the list one record per every `1` in the binary representation of `n`. Thus, the space complexity is bounded by `O(log n)`. For example, for `152` hash leaf nodes, the binary representation of `152` is `10011000`, and this means that we only store three Compact Merkle Subtree records on the list:

```[{height: 7, hash: h1}, {height: 4, hash: h2}, {height: 3, hash: h1}]```

where `h1`, `h2`, and `h3` are the respective binary Merkle Trees roots hashes nodes, of the first `128` hashes, the next `16` hashes, and the next `8` hashes, that consitute the `152` hashes.

Adding a particular hash leaf node to the structure could take `O(log n)`, but the amortized complexity for the `n` hashes is `O(1)` because of the internal compaction of full binary subtrees on the list.

To get the global root hash of the structure, i.e. the `txs_hash` of a block when there is a change in block height, we use a computation procedure to combine the subtrees on the list that takes `O(log n)`. The standard self-combination occurs for hash nodes that do not have a pair hash node.

### Skip Blocks

Not every NEAR block must contain an Aurora transaction, or indeed it could be that a NEAR block skipped some height. Either way, we want to have an Aurora block hashchain for every height (because the Refiner produces blocks at all heights). So, if the Engine detects a height change of more than one, then it will need to compute the intermediate block hashchain before proceeding. For example, if consecutive NEAR blocks had heights `H'`, `H' + 2` then the following sequence of hashchains should be produced:

```text
block_hashchain_H_prime          = hash(chain_id || contract_account_id || (H').to_be_bytes() || block_hashchain_H_prime_minus_one || txs_hash_H_prime || txs_logs_bloom_H_prime)
block_hashchain_H_prime_plus_one = hash(chain_id || contract_account_id || (H' + 1).to_be_bytes() || block_hashchain_H_prime || [0u8; 32] || [0u8; 256])
block_hashchain_H_prime_plus_two = hash(chain_id || contract_account_id || (H' + 2).to_be_bytes() || block_hashchain_H_prime_plus_one || txs_hash_H_prime_plus_two || txs_logs_bloom_H_prime_plus_two)
```

### Cryptographic Verification

#### Blockchain Verification

The engine state will always contain the block hashchain of the last completed block. To verify a given Aurora stream is correct using a non-archival NEAR node, follow the scheme above to compute the hashchain for the block at some recent height, using a know block hashchain value of a previous block as a seed, and check that hashchain matches the value in the Engine state (obtained via a view call again).

If the two values match, then the data must match what happened on-chain. This is called “range verification” because it verifies a whole range of blocks by checking only the endpoint hashchains. The reason this works is that the hashchain includes all the transaction data and block heights in between, thus any change to the local copy of the data should produce an incorrect final hash.

#### Transaction Verification

To verify that a transaction belongs to a block, we can rely on the binary Merkle Tree to use Merkle Proofs. Light clients (verifiers) only need to hold block's headers to have all the block hash information, and require a Merkle Proof to verify a transaction.

Given a transaction in a block, the corresponding Merkle Proof from the corresponding Merkle Tree consists of the sequence of sibling hash nodes, of the branch hash nodes that join the transaction hash leaf node with the root hash node. This sequence allows the recreation of the computation of the root hash, computing the joining branch bottom up literately, using at each step the computed branch node from the previous step and its provided sibling on the proof.

If by the end of the process, the verifier ends up with a computed `txs_hash` (root hash) that corresponds to the one of the block, then the transaction should belong to the block. To check if the computed `txs_hash` corresponds to the one of the block, the verifier just needs to compute the block hashchain using the info in the block header and the computed `txs_hash`, and compare it with the block hashchain on the header.

The size of the siblings hash nodes sequence is then the height of the tree. As this is a binary balanced tree then for `n` transactions the sequence size is `O(log n)`. Given the transaction and the proof, the verification process takes then `O(log n)`.

## Rationale

The current design aims to compute the block hashchain, and facilitate a fast transaction proof verification, while minimizing the on-chain overhead.

Every time that a new transaction is received on the Engine, the intrinsic transaction hash will be computed and added to the Stream Compact Merkle Tree (tree). Both procedures are fast since they involve a single transaction hash, and the amortized `O(1)` tree addition. When there is a change in block height, the block hashchain will be computed from:
A. already know values: chain id hash, contract account id hash, and previous block hashchain.
B. computation of the hash of the block height.
C. computation of the tree hash, that takes `O(log n)` for `n` transactions on the block.

The space needed, as explained in the Merkle Tree section, is `O(log n)`. With this approach, we are distributing the block transactions hash computation between the `n` transactions with an amortized `O(1)` procedure time, plus a final `O(log n)` procedure at block height change. As explained before, because of the use of a binary Merkle Tree, the transaction proof verification procedure is also `O(log n)`.

Another possible approach would be to compute the transactions hash of a block using a lineal hash composition instead of a binary Merkle Tree. In this case, every transaction received will be hashed to get the intrinsic transaction hash, `tx_hash`, and the transactions hash of the block, `txs_hash`, will be updated as `txs_hash = hash(txs_hash || tx_hash)`.

This reduces the memory utilization to `O(1)` since we only need to maintain a fixed amount of hashes. The procedure per transaction will be faster since it is simpler but only by a constant factor because both are `O(1)`. The final procedure to compute the block hashchain will be faster since it will be `O(1)` given that the `txs_hash` is already computed. The downside here is that the transaction proof verification offered will be `O(n)` because of the linear composition to get `txs_hash`, so affecting considerably everyone who needs to run that process.

Paying the price of `O(log n)` in procedure and space to offer faster `O(log n)` transaction proof verification, is reasonable and focuses on the convenient use of our products by our users.

Another alternative would be to compute the block hashchain as a single operation when there is a change in block height. We will need then to store all the `n` hashes of block transactions when they are received; this will increase considerably the memory usage. When the block height changes, we will need then to run the `O(n)` procedure to compute the full tree hash, plus the final block hashchain computation. This could heavily impact the on-chain performance at that point depending on the number of transactions in the block.

It seems reasonable to distribute the load of the computation by transactions versus the bulk operation at the end.

## Backwards Compatibility

As this is an addition of a new field to the blocks there should not be any issues regarding backwards compatibility. Tools, indexers or consumers of Aurora blocks could ignore the field and continue to work as before or add the new field to have updated information.

## Test Cases

### Correctnes Tests
<!-- TODO: Fill following section -->
<!-- We could add the following test:
Include as part of the assets the Aurora genesis block 'G' and the next block 'N'.
Add here the result of the txs_hash of 'G' and the hashchain of 'G'.
Add then the result of the txs_hash of 'N' and the hashchain of 'N' using all the info.-->

### Benchmark Tests

To measure the impact on gas consumption of the block hashchain computation, we ran some tests against a base branch and the hashchain branch. After getting the raw gas consumption results from both, we found the delta (difference) between them and use some statistics to show the impact.

Base Branch:
https://github.com/Casuso/aurora-engine_bechmark/tree/block_txs_benchmark

Hashchain Branch:
https://github.com/Casuso/aurora-engine_bechmark/tree/aurora_block_hashchain_block_txs_benchmark

#### Tests Descriptions

There are two types of tests:

A. block_txs
Receiving a number of transactions on the same block and measuring the gas profiled on each.
The input is the number of transactions to insert.
The output is an array where each position `i` is the gas profile of the transaction `i`.

B. blocks_change_txs
Receiving a number of transactions on blocks and measuring the gas profiled by an extra transaction on each that changes the block height.
The input is an array where each position `i` is the number of transactions to insert on block `i`.
The output is an array where each position `i` is the gas profiled of the extra transaction to change the height.

For the A type, we did two tests:

1. Test1 input: `255` transactions.
This covers ending in an almost full binary tree.
2. Test2 input: `1024` transactions.
This covers ending in a full binary tree.

For the B type, we did two tests:

3. Test3 input: `[128 + 0, 128 + 1, 128 + 2, 128 + 4, 128 + 8, 128 + 16, 128 + 32, 128 + 64, 128 + 16 + 1, 255]`.
This covers having the bigger and minor heights (b - m) of subtrees as: 7 - 0, 7 - 1, 7 - 2, ..., 7 - 6. Also covers three heights subtrees with 7, 4, and 1, heights. Finally, it covers the full heights subtree as a result of 255 transactions.
4. Test4 input: `[129, 131, 133, 135, 137]`.
This covers odds number of transactions, that will result in two or more subtrees, one of max height 7 (128), the other of min height 1 (1), and some in between. This will force the worse hashchain computation from min height 1 to max height 7.

#### Tests Outputs and Deltas

In the file /assets/api-8/benchamark-tests you can find the test outputs per branch and the deltas. The deltas are the differences in gas consumption after subtracting the base branch outputs from the hashchain branch outputs.

#### Statistics

These are the statistics that we use per delta:
Average (Ave), Median (Med), Minimum (Min), Maximum (Max), Average Absolute Deviation (AAD), Variance (Var), and Standard Deviation (SD).

Test1 Delta:
Ave: 453177975598.3647
Med: 445037423505
Min: 425679273477
Max: 572577894612
AAD: 19802253247.994835
Var: 697395477625485500000
SD : 26408246394.364876

Test2 Delta:
Ave: 457749007212.3633
Med: 449238413991
Min: 426419287881
Max: 637521856101
AAD: 20644841491.551567
Var: 796410880964748800000
SD : 28220752664.74565

Test3 Delta:
Ave: 555763332385.5
Med: 571837554816
Min: 454648655718
Max: 622351133922
AAD: 45833283443.1
Var: 2829685596386189400000
SD : 53194789184.52624

Test4 Delta:
Ave: 610743227202
Med: 610706650236
Min: 608897310084
Max: 612698875218
AAD: 782259206.4
Var: 1447196558467091700
SD : 1202994828.944452

#### Analysis

Tests 1 and 2 are focused on executing transactions on the same block, which is the most common case as only one transaction triggers the change in block height. For these the average increase observed (delta ave) is less than 0.46 Tgas, and the maximums are less than 0.64 Tgas.

Tests 3 and 4 are focused on executing a transaction on a block height change, that triggers the block hashchain computation, which is the most expensive one. Both show average and max increases of less than 0.62 Tgas.

An increase of 0.64 or 0.62 Tgas is a relavite small amount so we should be fine. For reference, the transaction gas limit is 300 Tgas.

## Security Considerations

The security considerations of the block hashchain should be focused on the possible attempts to break the Aurora blockchain cryptographic verification that it aims to offer.

Given a seed block hashchain as well as the block hashchain of the last completed block from the Engine, we can verify an Aurora stream following the procedure described above on Cryptographic Verification. We show that any change to the stream would produce an incorrect final hash different from the one of the last completed block from the Engine.

We will describe some important properties and then proceed to test the possible break attempts. Through the text, we will use the word different in between quotes, "different", to denote a highly probably difference. This is to avoid repeatedly clarifications.

### Properties

The "Collision resistance" property of cryptographic hashing functions, and specifically of the `keccak256` function (`hash`), establishes the difficulty to find two different inputs `x1` and `x2` such `hash(x1) = hash(x2)`. Let us call this property `P0`.

We heavily rely on the security offered by the composition of hashing functions, so offering the same security model as known blockchains, having its roots on Merkle Trees and in `P0`. We will then assume safely, that any data point change in a chain or tree of compositions of a hash function, will result in a “different” output. Let us call this property `P1`.

Every parameter, on the transactions hash computation and on the final block hashchain computation, is concatenated with other parameters before performing the hashing. For the transaction hash computation, we preappend the lenght of each paramater to each of them. For the final block computation, all parameters but one (contract_account_id) are fixed size parameters. These procedures and caracteristics ensures that both concatenations operations are injective. This is important so a bad actor cannot shift bytes from one parameter to the next one to create another input with the same resulting concatenation. Let us call this property `P2`.

### Break Attempts

Without losing generality, let’s assume that a bad actor attempts to change at least the `input` value of a transaction `t_x` in a block `B`. Below are the original `_1` and new `_2` computations of the intrinsic transaction hash of `t_x`.

```text
intrinsic_tx_hash_1 = hash(method_name.len().to_be_bytes() || method_name || input_bytes_1.len().to_be_bytes() || input_bytes_1 || output_bytes.len().to_be_bytes() || output_bytes)
intrinsic_tx_hash_2 = hash(method_name.len().to_be_bytes() || method_name || input_bytes_1.len().to_be_bytes() || input_bytes_1 || output_bytes.len().to_be_bytes() || output_bytes)
```

Since at least the `input` parameter is different, then the concatenation operation will have a different result by `P2`. Then the general hash functions inputs would be different, and by `P0` the resulting intrinsic transaction hash output should be "different": `tx_hash_1 != tx_hash_2`.

This different intrinsic transaction hash `tx_hash_2` of `t_x` will cause a difference in the transactions hash of `B`, by `P1`, since its computation is a composition of hash functions that includes the intrinsic transaction hash of `t_x`. So, instead of having transactions hash `txs_hash_1` for `B`, we would have `txs_hash_2`.

In another case, let’s assume that the attempt is to add a transaction to `B`. Then, there will be also a change in the compositions of the hashing functions for the computation of the transactions hash, thus resulting in a "different" value because of `P1`. Similar would happen in the case of removing a transaction from `B`.

In summary, any change to the transactions of `B`, would result in a different value `txs_hash_2` of the transactions hash. Having a different value of the transactions hash, then the block hashchain of `B` should be "different", because of `P2` and `P0`. So `block_hashchain_B_1 != block_hashchain_B_2` (probabilistically speaking), by applying properties `P2` and `P0`.

It is worth also mentioning that any attempts to tamper with the block height, or to use a different previous block hashchain, or change the logs bloom, would result also in a "different" block hashchain value for `B`, because of `P2` and `P0` too.

Finally, once the block hashchain of a block in the stream has changed, then we can claim that the computed block hashchain of the last completed block of the stream would be "different" from the known one in the Engine. This is because the computation of the block hashchain is the composition of hashing functions, so `P1` applies:

```block_hashchain = hash(chain_id || contract_account_id || block_height.to_be_bytes() || previous_block_hashchain || txs_hash || txs_logs_bloom)```

## Copyright

Copyright and related rights waived via [CC0](https://creativecommons.org/publicdomain/zero/1.0/).
