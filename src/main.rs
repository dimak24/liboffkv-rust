use std::{thread,time};


mod client;
mod txn;

use client::Client;
use txn::*;


fn main() {
    let c = Client::new("zk://localhost:2181", "/rust_test_key").unwrap();
    c.erase("/key", 0);
    c.create("/key", "value", false).unwrap();
    let v = c.set("/key", "new_value").unwrap();
    assert!(c.cas("/key", "kek", v).unwrap() != 0);

    thread::spawn(|| {
        let new_c = Client::new("zk://localhost:2181", "/rust_test_key").unwrap();
        let (_, value, watch_handle) =
            new_c.get("/key", true).unwrap();
        assert_eq!(value, String::from("kek"));

        watch_handle.unwrap().wait();
        let (_, value, _) =
            new_c.get("/key", false).unwrap();
        assert_eq!(value, String::from("lol"));
    });

    std::thread::sleep(time::Duration::from_secs(5));
    c.set("/key", "lol").unwrap();

    c.create("/key/key1", "value", true).unwrap();
    c.create("/key/key2", "value", true).unwrap();

    assert!(c.exists("/key/key1", false).unwrap().0 != 0);

    let (children, _) = c.get_children("/key", false).unwrap();
    for child in children {
        println!("{:?}", child);
    }

    println!("{:?}", c.exists("/key", false).unwrap().0);
    println!("{:?}", c.exists("/key/key1", false).unwrap().0);

    let result = c.commit(Transaction{
            checks: vec![
                TxnCheck{key: "/key", version: 0},
                TxnCheck{key: "/key/key1", version: 0},
            ],
            ops: vec![
                TxnOp::Create{key: "/new_key", value: "value", leased: true},
                TxnOp::Set{key: "/key/key1", value: "new_value"},
                TxnOp::Erase{key: "/key/key2"},
            ],
        }
    ).unwrap();

    assert_eq!(c.get("/key/key1", false).unwrap().1, String::from("new_value"));

    c.erase("/key", 0).unwrap();
}
