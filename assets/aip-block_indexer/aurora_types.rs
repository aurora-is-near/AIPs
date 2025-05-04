struct AuroraBlock {
    /// Chain id of the current network.
    chain_id: u64,
    /// Hash of the block
    hash: H256,
    /// Hash of the parent block.
    /// TODO(below): It is guaranteed that heights from consecutive blocks will be
    /// consecutive. i.e: block(parent_hash).height + 1 == block(hash)
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
    /// TODO(below) Uses NEAR state root of the block. While this doesn't match Ethereum rules to compute
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

type Address = [u8; 20];
type H256 = [u8; 32];
type Bloom = [u8; 256];

enum NearBlock {
    /// No block is known at this height.
    SkipBlock,
    /// Metadata from an existing block.
    ExistingBlock(NearBlockHeader),
}

struct NearBlockHeader {
    /// Hash of the block on NEAR
    near_hash: H256,
    /// Hash of the parent of block on NEAR. Note that some blocks can be skipped.
    near_parent_hash: H256,
    /// Account id of the validator that produced this block
    author: AccountId,
}

struct AccessTuple {
    address: H160,
    storage_keys: Vec<H256>,
}

struct Log {
    address: Address,
    topics: Vec<H256>,
    data: Vec<u8>,
}

struct NearTransaction {
    /// Index of the action on action list
    action_index: u64,
    /// Receipt hash on NEAR
    receipt_hash: H256,
}
