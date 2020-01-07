/// Transaction structure.
///
/// Each transaction consists of two parts:
/// a list of checks and a list of operations. Operations will be performed iff
/// all checks are satisfied. Checks and then operations are performed atomically.
pub struct Transaction<'a> {
    /// List of checks
    pub checks: Vec<TxnCheck<'a>>,

    /// List of operations
    pub ops:    Vec<TxnOp<'a>>,
}

/// Transaction check structure.
///
/// Each check is by a pair (key, version). It is satisfied if the specified key
/// has the specified version or if the version given is 0 and the key exists.
pub struct TxnCheck<'a> {
    /// key to check version
    pub key: &'a str,

    /// assumed version of the key
    pub version: i64,
}

/// Transaction operation.
///
/// There are 3 possible operations in rsoffkv transaction: Create, Set or Erase
pub enum TxnOp<'a> {
    /// Creates the key, rolls back if the key already exists or
    /// preceding entry does not exist.
    Create { key: &'a str, value: &'a str, leased: bool },


    /// Set - assigns new value to the given key, rolls back if the key does not exist.
    /// n.b. the behavior differs from the ordinary set
    Set    { key: &'a str, value: &'a str},

    /// Erase - deletes the key, rolls back if the key does not exist
    Erase  { key: &'a str },
}

/// Transaction operation result.
///
/// Result is returned only for operations affecting
/// versions (namely Create and Set). Result is represented with new version of the key.
pub enum TxnOpResult {
    /// initial version of newly created node
    Create(i64),

    /// new version after Set
    Set(i64),
}
