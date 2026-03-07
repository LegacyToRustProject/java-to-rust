use std::sync::Mutex;

struct Counter {
    count: Mutex<i32>,
}

impl Counter {
    fn new() -> Self {
        Self {
            count: Mutex::new(0),
        }
    }

    fn increment(&self) {
        *self.count.lock().unwrap() += 1;
    }

    fn decrement(&self) {
        *self.count.lock().unwrap() -= 1;
    }

    fn count(&self) -> i32 {
        *self.count.lock().unwrap()
    }
}
