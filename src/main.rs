extern crate promise_rs;
use promise_rs::Promise;

fn main() {
    println!("1. Create Promise");
    let mut promise = Promise::new(|resolve, reject| {
        std::thread::sleep(std::time::Duration::from_millis(1));
        println!("3. Resolve resute in new thread");
        if true {
            resolve(Some("resolve result".to_string()));
        } else {
            reject(None);
        }
    });

    println!("2. Add then/catch handlers");
    promise
        .then(
            |value| {
                println!("4. On fulfilled - {:?}", &value);
                Some("changed result".to_string())
            },
            |reason| {
                println!("4. On rejected - {:?}", &reason);
                reason
            },
        )
        .then(
            |value| {
                println!("5. On fulfilled - {:?}", &value);
                value
            },
            |reason| reason,
        )
        .catch(|reason| {
            println!("5. On catche - {:?}", &reason);
            None
        });

    promise.a_await();
}
