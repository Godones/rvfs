use spin::Mutex;
use std::sync::Arc;

fn main() {
    let val = 5;
    let t1 = Arc::new(Mutex::new(val));
    let t2 = t1.clone();
    println!("{}", Arc::ptr_eq(&t1, &t2));
}
