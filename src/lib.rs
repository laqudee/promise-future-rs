use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Pending,
    Fulfilled,
    Rejected,
}

pub struct Handler {
    pub resolve: bool,
    pub handler: Box<dyn Fn(Option<String>) -> Option<String> + Send>,
}

pub struct Promise {
    pub value: Arc<Mutex<Option<String>>>,
    pub status: Arc<Mutex<Option<Status>>>,
    pub handlers: Arc<Mutex<Option<Vec<Handler>>>>,
    pub thread: std::thread::JoinHandle<()>,
}

impl Promise {
    pub fn new<F>(executor: F) -> Promise
    where
        F: Send + 'static + Fn(&dyn Fn(Option<String>), &dyn Fn(Option<String>)),
    {
        let result = Arc::new(Mutex::new(None));
        let result_resolve = result.clone();
        let result_reject = result.clone();

        let status = Arc::new(Mutex::new(Some(Status::Pending)));
        let status_resolve = status.clone();
        let status_reject = status.clone();

        let handlers = Arc::new(Mutex::new(Some(Vec::new())));
        let handlers_resolve = handlers.clone();
        let handlers_reject = handlers.clone();

        let thread = thread::spawn(move || {
            let resolve = move |value| {
                let mut prev_value: Option<String> = value;
                for handler in handlers_resolve.lock().unwrap().take().unwrap().into_iter() {
                    let handler: Handler = handler;
                    if handler.resolve == true {
                        prev_value = (handler.handler)(prev_value.clone());
                    }
                }
                let mut result_resolve = result_resolve.lock().unwrap();
                *result_resolve = prev_value;
                let mut state_guard = status_resolve.lock().unwrap();
                let state = state_guard.as_mut().unwrap();
                *state = Status::Fulfilled;
            };
            let reject = move |reason| {
                let mut prev_reason: Option<String> = reason;
                for handler in handlers_reject.lock().unwrap().take().unwrap().into_iter() {
                    let handler: Handler = handler;
                    if handler.resolve == false {
                        prev_reason = (handler.handler)(prev_reason.clone());
                    }
                }
                let mut result_reject = result_reject.lock().unwrap();
                *result_reject = prev_reason;
                let mut state_guard = status_reject.lock().unwrap();
                let state = state_guard.as_mut().unwrap();
                *state = Status::Rejected;
            };

            executor(&resolve, &reject);
        });

        Promise {
            handlers,
            status,
            value: result,
            thread,
        }
    }

    pub fn then<F1, F2>(&mut self, on_fulfilled: F1, on_rejected: F2) -> &mut Promise
    where
        F1: Send + 'static + Fn(Option<String>) -> Option<String>,
        F2: Send + 'static + Fn(Option<String>) -> Option<String>,
    {
        let status = self.status.lock().unwrap().clone().unwrap();
        match status {
            Status::Fulfilled => {
                let result_resolve = self.value.clone();
                let mut value = result_resolve.lock().unwrap();
                let prev_value = value.clone();
                *value = (on_fulfilled)(prev_value);
            }
            Status::Rejected => {
                let result_reject = self.value.clone();
                let mut reason = result_reject.lock().unwrap();
                let prev_reason = reason.clone();
                *reason = (on_rejected)(prev_reason);
            }
            Status::Pending => {
                let handler_fulfilled = Handler {
                    resolve: true,
                    handler: Box::new(on_fulfilled),
                };
                let handler_rejected = Handler {
                    resolve: false,
                    handler: Box::new(on_rejected),
                };
                self.handlers
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .push(handler_fulfilled);
                self.handlers
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .push(handler_rejected);
            }
        }
        self
    }

    pub fn catch<F>(&mut self, on_rejected: F) -> &mut Promise
    where
        F: Send + 'static + Fn(Option<String>) -> Option<String>,
    {
        let status = self.status.lock().unwrap().clone().unwrap();
        match status {
            Status::Fulfilled => {}
            Status::Rejected => {
                let result_reject = self.value.clone();
                let mut reason = result_reject.lock().unwrap();
                let prev_reason = reason.clone();
                *reason = (on_rejected)(prev_reason);
            }
            Status::Pending => {
                let handler = Handler {
                    resolve: false,
                    handler: Box::new(on_rejected),
                };
                self.handlers
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .push(handler);
            }
        }
        self
    }

    pub fn a_await(self) {
        let _ = self.thread.join();
    }

    pub fn resolve(value: Option<String>) -> Promise {
        Promise::new(move |resolve, _| {
            resolve(value.clone());
        })
    }

    pub fn reject(reason: Option<String>) -> Promise {
        Promise::new(move |_, reject| {
            reject(reason.clone());
        })
    }

    pub fn all(promises: Vec<Promise>) -> Promise {
        Promise::all_ex(promises, ";")
    }

    pub fn all_ex(promises: Vec<Promise>, delimeter: &str) -> Promise {
        let mut rejected = false;
        let mut resolved_result: Vec<String> = vec![];
        let mut first_reject_reason = String::new();
        for promise in promises.into_iter() {
            let _ = promise.thread.join();
            let status = promise.status.lock().unwrap().clone().unwrap();
            let value = promise
                .value
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(String::new());
            match status {
                Status::Fulfilled => {
                    resolved_result.push(value);
                }
                Status::Rejected => {
                    rejected = true;
                    first_reject_reason = value;
                }
                Status::Pending => {}
            }
        }
        if rejected {
            return Promise::reject(Some(first_reject_reason));
        } else {
            return Promise::resolve(Some(resolved_result.join(delimeter)));
        }
    }
}
