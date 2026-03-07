struct MyBox<T> {
    value: T,
}

impl<T> MyBox<T> {
    fn new(value: T) -> Self {
        Self { value }
    }

    fn value(&self) -> &T {
        &self.value
    }

    fn set_value(&mut self, value: T) {
        self.value = value;
    }
}
