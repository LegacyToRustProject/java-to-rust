trait Animal {
    fn name(&self) -> &str;
    fn speak(&self) -> String;
}

struct Dog {
    name: String,
}

impl Dog {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Animal for Dog {
    fn name(&self) -> &str {
        &self.name
    }
    fn speak(&self) -> String {
        "Woof!".to_string()
    }
}
