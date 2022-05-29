//! Reference implementation of contracts for Aurora Cross Contract Calls
//! See more details on [aip-cross_contract_calls]

enum CombinatorMode {
    Then,
    And,
}

struct Combinator {
    /// Combinator that must be applied.
    mode: CombinatorMode,
    /// Reference to the previous promise in the same batch to apply the combinator with.
    index: u8,
}

/// Wrapper type that validates the data is a valid method.
type Method(String);

/// Promise arguments passed to the router.
struct Promise {
    /// Target account id on NEAR to be called
    target: AccountId,
    /// Method to be called
    method: Method,
    /// Bytes to use as input to the function
    payload: Vec<u8>,
    /// Amount of gas to attach to the promise
    near_gas: Gas,
    /// Amount of NEAR to attach to the promise
    near_balance: Balance,
    /// Description to combine this promise with previous promises
    combinator: Option<Combinator>,
}

/// Aurora Router
///
/// This is implemented as a precompile inside Aurora Engine.
struct AuroraRouter {
    /// Implicit address for Async Aurora account id on NEAR
    async_aurora: Address
    /// List of promises to be created
    promises: LookupMap<u64, Vec<Promise>>,
    /// Index of last promise inserted in `promises`
    last_used_index: u64,
}

/// All public functions to this contract will be referred to using a modification of ABI.
/// First four bytes will be the selector, computed as the sha3 of the name of the functions
/// with no arguments. The rest of the arguments is serialized using borsh for efficiency.
///
/// Rationale:
/// - We need to disambiguate between calls to different methods.
/// - Using ABI for selector allows explorer to display more information about the call
/// - Encoding reach types, such as `Promise` using ABI can only be done by using tuples.
/// - Borsh is more efficient gas-wise, and the precompile is written in rust.
///
/// Note: it is ok to add view functions to the contract allowing
impl AuroraRouter {

    /// There is no public constructor in the precompile. This function will be called during
    /// deployment of the contract.
    fn new(async_aurora: Address) -> Self {
        Self {
            async_aurora,
            promises: LookupMap::default(),
            last_used_index: 0,
        }
    }

    /// Stores the promises to be created and emit an event with the index where the promise is stored.
    ///
    /// NOTE: This change should not be applied to storage until the end of the transaction,
    /// to make sure it was not reverted before.
    ///
    /// Selector: sha3("schedule()") = 0xb0604a26
    pub fn schedule(&mut self, promises: Vec<Promise>) -> u64 {
        for mut promise in promises {
            // Extend payload with message sender
            promise.payload = [0, aurora_context::message_sender(), promise.payload].concat();
        }
        let index = self.last_used_index;

        self.promises.insert(index, promises);
        self.last_used_index += 1;

        index
    }

    /// Pull all the promises. The predecessor account id must be async aurora, this means this function can
    /// only be called using the `call` interface in `aurora` contract.
    ///
    /// Selector: sha3("pull()") = 0x329eb839
    pub fn pull(&mut self, indexes: Vec<u64>, total_gas: Gas, total_balance: Balance) -> Vec<Option<Vec<PromiseOutput>>> {
        // Only async aurora contract can pull promises out of the contract
        assert_eq!(env::predecessor_account_id(), self.async_aurora);

        let mut required_gas = 0;
        let mut required_balance = 0;

        indexes.iter().map(|index| {
            // Find and remove the promises at position index
            // If there is no promise, None is returned instead
            let promise = self.promises.pop(index)

            if let Some(promise) = promise {
                required_gas += promise.near_gas;
                required_balance += promise.near_balance;
            }

            promise
        }).collect()

        // The transaction should only be executed if there is enough resources for its execution.
        assert!(required_gas <= total_gas);
        assert!(required_balance <= total_balance);
    }
}

/// Async Aurora
///
/// This is a smart contract on NEAR
struct AsyncAurora {}

/// All gas amounts used here are boilerplate number and need to be properly adjusted.
const EXECUTE_GAS: Gas = tgas!(3);
const PULL_GAS: Gas = tgas!(3);
const PULL_AND_EXECUTE_GAS: Gas = tgas!(3) + PULL_GAS + EXECUTE_GAS;
const ON_SCHEDULE_GAS: Gas = tgas!(3) + PULL_AND_EXECUTE_GAS;

impl AsyncAurora {
    /// Runs a transaction on an Aurora engine, and create a callback to handle all scheduled promises
    pub fn schedule_and_execute(&mut self, aurora: AccountId, submit_payload: Vec<u8>, submit_gas: Gas, total_gas: Gas, total_balance: Balance){
        aurora_engine::submit(
            submit_payload,
            target: aurora,
            gas: submit_gas,
            balance: 0,
        ).then(remote_self::on_schedule(
            aurora,
            total_gas,
            total_balance,
            target: env::current_account_id(),
            gas: total_gas + ON_SCHEDULE_GAS,
            balance: total_balance,
        ))
    }

    /// Method used to parse promise indexes after sending a transaction to Aurora Engine.
    pub fn on_schedule(&mut self, aurora: AccountId, total_gas: Gas, total_balance: Balance) {
        // Parse promise index from the result of the call to aurora engine.
        let promise_index: Vec<u64> = parse_promise_indexes();
        self.pull_and_execute(aurora, promise_index, total_gas, total_balance);
    }

    /// Pull the promise values from Aurora Router, and schedule their execution.
    pub fn pull_and_execute(&mut self, aurora: AccountId, promise_index: Vec<u64>, total_gas: Gas, total_balance: Balance) {
        assert!(env::prepaid_gas() >= total_gas + PULL_AND_EXECUTE_GAS);

        // Note: To refund the remaining balance, the refund target must be passed as an extra argument to the function.
        assert!(env::attached_balance() >= total_balance);

        aurora_engine::call(
            "router::pull(${promise_index})",
            target: aurora,
            gas: PULL_GAS,
            balance: 0,
        ).then(remote_self::execute(
            aurora,
            target: env::current_account_id(),
            gas: total_gas + EXECUTE_GAS,
            balance: total_balance,
        ))
    }

    pub fn execute(&mut self, aurora: AccountId) {
        // Parse promises values from the result of the pull to aurora engine.
        let promises: Vec<Option<Vec<Promise>>> = parse_promises();
        // Keep only batches that has Some
        let promises: Vec<Vec<Promise>> = promises.filter(|promise_batch| promise_batch);

        for mut promise in promises.flatten() {
            // Parse and rebuild the payload according to the spec, where proper authentication information is provided.
            let (version, address, payload) = parse_payload(promise.payload);
            promise.payload = [0, aurora.len() as u8, aurora, address, payload].concat();
        }


        for promise_batch in promises {
            // Keep track of previous promises. Required for applying combinators.
            let mut all_promises_id = vec![];
            for promise in promise_batch {
                // Create a promise
                let mut promise_id = create_promise(promise);

                // Combine it with a previous promise if required
                if promise.combinator.is_some() {
                    promise_id = combine_promise(promise_id, all_promises_id);
                }

                // Store the promise reference
                all_promises_id.push(promise_id);
            }
        }
    }
}
