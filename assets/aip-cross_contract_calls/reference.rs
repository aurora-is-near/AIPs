//! Reference implementation of contracts for Aurora Cross Contract Calls
//! See more details on [aip-cross_contract_calls]

/// Promise arguments passed to the router.
struct Promise {
    /// Target account id on NEAR to be called
    target: AccountId,
    /// Method to be called
    method: String,
    /// Bytes to use as input to the function
    payload: Bytes,
    /// Amount of gas to attach to the promise
    near_gas: Gas,
    /// Amount of NEAR to attach to the promise
    near_balance: Balance,
    /// Description to combine this promise with previous promises
    combinator: Option<Combinator>,
    /// Indicates if the promise needs to be initiated using the authentication standard
    with_authentication: bool,
}

/// Promise arguments passed to Async Aurora
struct PromiseOutput {
    /// Same fields as Promise
    ...Promise,
    /// Address of the message sender. Computed dynamically on the Router
    sender: Address,
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
    /// Selector: sha3("schedule()") = 0xb0604a26
    pub fn schedule(&mut self, promises: Vec<Promise>) {
        for mut promise in promises {
            promise.set_sender(aurora_context::message_sender());
        }
    }

    /// Pull all the prmoises
    ///
    /// Selector: sha3("pull()") = 0x329eb839
    pub fn pull(&mut self, indices: Vec<u64>, total_gas: Gas, total_balance: Balance) -> Vec<Vec<PromiseOutput>> {

    }
}

/// Async Aurora
///
/// This is a smart contract on NEAR
struct AsyncAurora {}

impl AsyncAurora {
    pub fn schedule_and_execute(&mut self, aurora: Address, submit_payload: Address) {

    }

    pub fn pull_and_execute(&mut self, aurora: Address, index: u64);
}

// """
// Precompile in Aurora.

// Stores all promises that needs to be created, and allows AsyncAurora to pull promises for execution.
// """

// def __init__(self, async_aurora: Address):
//     """
//     There is no explicit constructor in the precompile. This is executed on Aurora deployment & initialization.

//     @async_aurora: Implicit address for async_aurora account id.
//     """
//     self.async_aurora = async_aurora

//     # Dynamic array. Removing one element from the array doesn't change the position
//     # of other elements, but frees the storage.
//     self.store = []


// def schedule(version: u8, promises: Vec<Promise>):
//     """
//     Store a batch of promises to be executed.


//     """
//     index = len(self.store)
//     self.store.push()

//     emit Event()

// def pull():
//     pass

// def run(self, address: H160, payload: [u8], attach_msg_sender: bool):
//     assert(self.expected_predecessor == env::predecessor_account_id())

//     if attach_msg_sender:
//         payload = concat(payload, msg.sender)

//     ret = call(address, value, payload)
//     return ret
