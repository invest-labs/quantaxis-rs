use quantaxis_rs::qaaccount::QA_Account;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tokio::runtime;
use tokio::runtime::Builder;
use tokio::sync::oneshot;
use tokio::task;

async fn some_computation() -> String {
    "represents the result of the computation".to_string()
}

#[tokio::main]
async fn main() {
    let code = "RB2005".to_string();
    let mut acc = QA_Account::new("RustT01B2_RBL8", "test", "admin", 100000.0, false, "real");

    let order = acc
        .send_order_async(&code, 10.0, "2020-01-20 22:10:00", 2, 3500.0, "BUY_OPEN")
        .await;
    println!("{:#?}", order.unwrap());
}
