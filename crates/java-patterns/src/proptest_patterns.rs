//! Phase 3: JUnit5 → proptest / cargo test 変換パターン
//!
//! 変換ルール:
//!   @Test                              → #[test]
//!   @ParameterizedTest + @ValueSource  → proptest! { #[test] fn ... (x in ...) }
//!   @ParameterizedTest + @CsvSource    → proptest! テーブルスタイル
//!   @BeforeEach                        → テスト関数内で直接初期化
//!   @BeforeAll / @AfterAll            → once_cell::sync::Lazy / Drop impl
//!   assertEquals(a, b)                → assert_eq!(a, b)
//!   assertTrue(cond)                  → assert!(cond)
//!   assertThrows(Type, () -> ...)     → std::panic::catch_unwind
//!   @Nested                           → mod {} ネスト
//!   assumeTrue(cond)                  → prop_assume!(cond)  (proptest内)

// ───────────────────────────────────────────────
// JUnit5 基本アサーション → cargo test
// ───────────────────────────────────────────────

/// Java: assertEquals(expected, actual)
/// Rust: assert_eq!(actual, expected)   ← 順番注意
pub fn junit_assert_equals<T: PartialEq + std::fmt::Debug>(expected: T, actual: T) {
    assert_eq!(actual, expected);
}

/// Java: assertTrue(condition)
pub fn junit_assert_true(condition: bool) {
    assert!(condition);
}

/// Java: assertFalse(condition)
pub fn junit_assert_false(condition: bool) {
    assert!(!condition);
}

/// Java: assertNull(obj) → assert!(opt.is_none())
pub fn junit_assert_null<T>(value: Option<T>) {
    assert!(value.is_none());
}

/// Java: assertNotNull(obj) → assert!(opt.is_some())
pub fn junit_assert_not_null<T>(value: Option<T>) {
    assert!(value.is_some());
}

// ───────────────────────────────────────────────
// @BeforeEach 相当のセットアップ
// ───────────────────────────────────────────────

/// Java: @BeforeEach void setUp() { counter = new Counter(); }
/// Rust: テスト関数内で直接初期化する
pub struct Counter {
    pub value: i32,
}

impl Counter {
    pub fn new() -> Self {
        Counter { value: 0 }
    }

    pub fn increment(&mut self) {
        self.value += 1;
    }

    pub fn get(&self) -> i32 {
        self.value
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

// ───────────────────────────────────────────────
// 変換されたユーティリティ関数群（テスト対象）
// ───────────────────────────────────────────────

/// Java: StringUtils.isEmpty(s) 相当（テスト対象として再定義）
pub fn is_empty_str(s: Option<&str>) -> bool {
    s.map(|s| s.is_empty()).unwrap_or(true)
}

/// Java: int factorial(int n)
pub fn factorial(n: u64) -> u64 {
    (1..=n).product()
}

/// Java: boolean isPalindrome(String s)
pub fn is_palindrome(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let rev: Vec<char> = chars.iter().rev().cloned().collect();
    chars == rev
}

/// Java: int add(int a, int b)
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Java: String repeat(String s, int n)
pub fn repeat_str(s: &str, n: usize) -> String {
    s.repeat(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── @Test 変換: 通常テスト ────────────────────

    /// Java:
    ///   @Test void testIsEmpty() {
    ///     assertTrue(StringUtils.isEmpty(""));
    ///     assertTrue(StringUtils.isEmpty(null));
    ///     assertFalse(StringUtils.isEmpty("hello"));
    ///   }
    #[test]
    fn test_is_empty_basic() {
        assert!(is_empty_str(Some("")));
        assert!(is_empty_str(None));
        assert!(!is_empty_str(Some("hello")));
    }

    /// Java:
    ///   @Test void testFactorial() {
    ///     assertEquals(1, factorial(0));
    ///     assertEquals(120, factorial(5));
    ///   }
    #[test]
    fn test_factorial_known_values() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(10), 3_628_800);
    }

    /// Java:
    ///   @Test void testCounter() {
    ///     Counter c = new Counter();  // @BeforeEach
    ///     c.increment();
    ///     assertEquals(1, c.getCount());
    ///   }
    #[test]
    fn test_counter_increment() {
        let mut counter = Counter::new(); // @BeforeEach の代わり
        counter.increment();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_counter_multiple_increments() {
        let mut c = Counter::new();
        for _ in 0..5 {
            c.increment();
        }
        assert_eq!(c.get(), 5);
    }

    // ── @ParameterizedTest + @ValueSource → proptest ───────

    use proptest::prelude::*;

    proptest! {
        /// Java:
        ///   @ParameterizedTest
        ///   @ValueSource(strings = {"hello", "world", "rust"})
        ///   void testStringLength(String s) {
        ///     assertTrue(s.length() > 0);
        ///   }
        #[test]
        fn test_string_length_positive(s in "[a-z]{1,20}") {
            prop_assert!(s.len() > 0, "empty string generated");
        }

        /// Java:
        ///   @ParameterizedTest
        ///   @ValueSource(ints = {1, 2, 3, 4, 5})
        ///   void testFactorialPositive(int n) {
        ///     assertTrue(factorial(n) >= 1);
        ///   }
        #[test]
        fn test_factorial_always_positive(n in 0u64..=10u64) {
            prop_assert!(factorial(n) >= 1);
        }

        /// Java:
        ///   @ParameterizedTest
        ///   void testAddCommutativity(int a, int b) {
        ///     assertEquals(add(a, b), add(b, a));
        ///   }
        #[test]
        fn test_add_commutativity(a in -1000i32..1000i32, b in -1000i32..1000i32) {
            prop_assert_eq!(add(a, b), add(b, a));
        }

        /// Java: テスト: isEmpty(repeat(s, n)) は n > 0 かつ s が空でなければ false
        #[test]
        fn test_repeat_nonempty_when_base_nonempty(
            s in "[a-z]{1,5}",
            n in 1usize..=5
        ) {
            let result = repeat_str(&s, n);
            prop_assert!(!result.is_empty());
            prop_assert!(result.len() == s.len() * n);
        }

        /// Java: Palindrome の対称性テスト
        ///   assertTrue(isPalindrome(s + reverse(s)))
        #[test]
        fn test_palindrome_constructed(s in "[a-z]{1,10}") {
            let palindrome = {
                let rev: String = s.chars().rev().collect();
                format!("{}{}", s, rev)
            };
            prop_assert!(is_palindrome(&palindrome));
        }

        /// Java: 空文字列の繰り返しは常に空
        #[test]
        fn test_repeat_empty_stays_empty(n in 0usize..=10) {
            prop_assert_eq!(repeat_str("", n), "");
        }
    }

    // ── @Nested 変換: mod {} ─────────────────────

    /// Java:
    ///   @Nested class WhenCounterIsZero {
    ///     @Test void isZero() { assertEquals(0, counter.get()); }
    ///   }
    mod when_counter_is_zero {
        use super::*;

        #[test]
        fn initial_value_is_zero() {
            let c = Counter::new();
            assert_eq!(c.get(), 0);
        }

        #[test]
        fn is_not_negative() {
            let c = Counter::new();
            assert!(c.get() >= 0);
        }
    }

    mod when_counter_has_value {
        use super::*;

        fn setup() -> Counter {
            let mut c = Counter::new();
            c.increment();
            c.increment();
            c
        }

        #[test]
        fn value_is_two() {
            let c = setup();
            assert_eq!(c.get(), 2);
        }

        #[test]
        fn is_positive() {
            let c = setup();
            assert!(c.get() > 0);
        }
    }
}
