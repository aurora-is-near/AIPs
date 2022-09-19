/// See full definition of NEARBlock in StreamerMessage @ nearcore/chain/indexer-primitives/src/lib.rs
pub struct NEARBlock {
    pub block: BlockView,
    pub shards: Vec<Shard>,
}
