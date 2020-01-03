pub struct TxnCheck<'a> {
    pub key: &'a str,
    pub version: i64,
}

pub enum TxnOp<'a> {
    Create { key: &'a str, value: &'a str, leased: bool },
    Set    { key: &'a str, value: &'a str},
    Erase  { key: &'a str },
}

pub struct Transaction<'a> {
    pub checks: Vec<TxnCheck<'a>>,
    pub ops:    Vec<TxnOp<'a>>,
}

pub enum TxnOpResult {
    Create(i64),
    Set(i64),
}
