/// Phase 1: Guava Collections → Rust 変換パターン
///
/// Java の Guava コレクションに対応する Rust イディオムを示す。
/// ImmutableList / ImmutableMap / ImmutableSet は Rust の標準コレクションで
/// 読み取り専用セマンティクスを実現する。
///
/// 変換ルール（PatternConverter 向け）:
///   ImmutableList<T>  → Vec<T>  (所有権モデルで不変を強制)
///   ImmutableMap<K,V> → HashMap<K,V>
///   ImmutableSet<T>   → HashSet<T>
///   ImmutableList.of(...)  → vec![...]
///   ImmutableMap.of(k, v)  → HashMap::from([(k, v)])
///   ImmutableMultimap<K,V> → HashMap<K, Vec<V>>
///   Optional<T> (Guava)    → Option<T>
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

// ───────────────────────────────────────────────
// ImmutableList<T> パターン
// ───────────────────────────────────────────────

/// Java: ImmutableList.of("a", "b", "c")
pub fn immutable_list_of<T: Clone>(items: &[T]) -> Vec<T> {
    items.to_vec()
}

/// Java: ImmutableList.copyOf(collection)
pub fn immutable_list_copy_of<T: Clone>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    items.into_iter().collect()
}

/// Java: ImmutableList.Builder<String> builder = ImmutableList.builder(); ...
pub fn immutable_list_builder<T>() -> Vec<T> {
    Vec::new()
}

// ───────────────────────────────────────────────
// ImmutableMap<K, V> パターン
// ───────────────────────────────────────────────

/// Java: ImmutableMap.of("key", 1)
pub fn immutable_map_of<K, V>(pairs: impl IntoIterator<Item = (K, V)>) -> HashMap<K, V>
where
    K: Eq + std::hash::Hash,
{
    pairs.into_iter().collect()
}

/// Java: ImmutableMap.copyOf(existingMap)
pub fn immutable_map_copy_of<K, V>(source: &HashMap<K, V>) -> HashMap<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    source.clone()
}

/// Java: ImmutableSortedMap.of("a", 1, "b", 2) → BTreeMap (ソート保証)
pub fn immutable_sorted_map<K: Ord, V>(pairs: impl IntoIterator<Item = (K, V)>) -> BTreeMap<K, V> {
    pairs.into_iter().collect()
}

// ───────────────────────────────────────────────
// ImmutableSet<T> パターン
// ───────────────────────────────────────────────

/// Java: ImmutableSet.of(1, 2, 3)
pub fn immutable_set_of<T>(items: impl IntoIterator<Item = T>) -> HashSet<T>
where
    T: Eq + std::hash::Hash,
{
    items.into_iter().collect()
}

/// Java: ImmutableSortedSet.of("a", "b", "c")
pub fn immutable_sorted_set<T: Ord>(items: impl IntoIterator<Item = T>) -> BTreeSet<T> {
    items.into_iter().collect()
}

// ───────────────────────────────────────────────
// Guava Optional → Rust Option
// ───────────────────────────────────────────────

/// Java: Optional.of(value) → Some(value)
pub fn guava_optional_of<T>(value: T) -> Option<T> {
    Some(value)
}

/// Java: Optional.fromNullable(value) → Option の直接表現
pub fn guava_optional_from_nullable<T>(value: Option<T>) -> Option<T> {
    value
}

/// Java: optional.or(defaultValue) → option.unwrap_or(default)
pub fn guava_optional_or<T>(opt: Option<T>, default: T) -> T {
    opt.unwrap_or(default)
}

/// Java: optional.transform(Function<T,R>) → option.map(|v| ...)
pub fn guava_optional_transform<T, R>(opt: Option<T>, f: impl Fn(T) -> R) -> Option<R> {
    opt.map(f)
}

// ───────────────────────────────────────────────
// Guava Multimap → HashMap<K, Vec<V>>
// ───────────────────────────────────────────────

/// Java: ArrayListMultimap<K, V>
pub struct Multimap<K, V> {
    inner: HashMap<K, Vec<V>>,
}

impl<K: Eq + std::hash::Hash, V> Multimap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Java: multimap.put(key, value)
    pub fn put(&mut self, key: K, value: V) {
        self.inner.entry(key).or_default().push(value);
    }

    /// Java: multimap.get(key) → List<V>
    pub fn get(&self, key: &K) -> &[V] {
        self.inner.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Java: multimap.values() → flat list
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.values().flatten()
    }

    pub fn len(&self) -> usize {
        self.inner.values().map(|v| v.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<K: Eq + std::hash::Hash, V> Default for Multimap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

// ───────────────────────────────────────────────
// Guava Strings / Joiner → Rust イディオム
// ───────────────────────────────────────────────

/// Java: Joiner.on(",").join(list)
pub fn joiner_on<'a>(sep: &str, items: impl IntoIterator<Item = &'a str>) -> String {
    items.into_iter().collect::<Vec<_>>().join(sep)
}

/// Java: Joiner.on(",").skipNulls().join(list)
pub fn joiner_skip_nulls<'a>(
    sep: &str,
    items: impl IntoIterator<Item = Option<&'a str>>,
) -> String {
    items.into_iter().flatten().collect::<Vec<_>>().join(sep)
}

/// Java: Strings.isNullOrEmpty(s) → s.is_none_or(|s| s.is_empty())
pub fn strings_is_null_or_empty(s: Option<&str>) -> bool {
    s.map(|s| s.is_empty()).unwrap_or(true)
}

/// Java: Strings.nullToEmpty(s) → s.unwrap_or("")
pub fn strings_null_to_empty(s: Option<&str>) -> &str {
    s.unwrap_or("")
}

/// Java: Strings.emptyToNull(s) → if s is "" then None
pub fn strings_empty_to_null(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

/// Java: Strings.repeat(s, count) → s.repeat(count)
pub fn strings_repeat(s: &str, count: usize) -> String {
    s.repeat(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ImmutableList テスト ──────────────────────

    #[test]
    fn test_immutable_list_of_strings() {
        // Java: ImmutableList.of("a", "b", "c")
        let list = immutable_list_of(&["a", "b", "c"]);
        assert_eq!(list, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_immutable_list_of_integers() {
        // Java: ImmutableList.of(1, 2, 3)
        let list: Vec<i32> = immutable_list_of(&[1, 2, 3]);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], 1);
    }

    #[test]
    fn test_immutable_list_copy_of() {
        // Java: ImmutableList.copyOf(existing)
        let source = vec![10, 20, 30];
        let copy = immutable_list_copy_of(source.iter().copied());
        assert_eq!(copy, vec![10, 20, 30]);
    }

    // ── ImmutableMap テスト ──────────────────────

    #[test]
    fn test_immutable_map_of_single() {
        // Java: ImmutableMap.of("key", 42)
        let map = immutable_map_of([("key".to_string(), 42i32)]);
        assert_eq!(map.get("key"), Some(&42));
    }

    #[test]
    fn test_immutable_map_of_multiple() {
        // Java: ImmutableMap.of("a", 1, "b", 2, "c", 3)
        let map = immutable_map_of([("a", 1), ("b", 2), ("c", 3)]);
        assert_eq!(map.len(), 3);
        assert_eq!(map["a"], 1);
        assert_eq!(map["c"], 3);
    }

    #[test]
    fn test_immutable_sorted_map() {
        // Java: ImmutableSortedMap.of("b", 2, "a", 1) → ordered by key
        let map = immutable_sorted_map([("b", 2), ("a", 1), ("c", 3)]);
        let keys: Vec<&&str> = map.keys().collect();
        assert_eq!(keys, vec![&"a", &"b", &"c"]); // sorted
    }

    // ── ImmutableSet テスト ──────────────────────

    #[test]
    fn test_immutable_set_of() {
        // Java: ImmutableSet.of(1, 2, 3)
        let set: HashSet<i32> = immutable_set_of([1, 2, 3]);
        assert!(set.contains(&1));
        assert!(set.contains(&3));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_immutable_set_deduplication() {
        // Java: ImmutableSet.of(1, 2, 2, 3) → {1, 2, 3} (重複除去)
        let set: HashSet<i32> = immutable_set_of([1, 2, 2, 3]);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_immutable_sorted_set() {
        // Java: ImmutableSortedSet.of("c", "a", "b") → ["a", "b", "c"]
        let set = immutable_sorted_set(["c", "a", "b"]);
        let items: Vec<&&str> = set.iter().collect();
        assert_eq!(items[0], &&"a");
    }

    // ── Guava Optional テスト ────────────────────

    #[test]
    fn test_guava_optional_of() {
        // Java: Optional.of("value").isPresent() == true
        let opt = guava_optional_of("value");
        assert!(opt.is_some());
        assert_eq!(opt.unwrap(), "value");
    }

    #[test]
    fn test_guava_optional_or() {
        // Java: Optional.absent().or("default") == "default"
        let opt: Option<&str> = None;
        assert_eq!(guava_optional_or(opt, "default"), "default");
    }

    #[test]
    fn test_guava_optional_transform() {
        // Java: Optional.of(5).transform(x -> x * 2) == Optional.of(10)
        let opt = guava_optional_of(5i32);
        let result = guava_optional_transform(opt, |x| x * 2);
        assert_eq!(result, Some(10));
    }

    // ── Multimap テスト ──────────────────────────

    #[test]
    fn test_multimap_put_get() {
        // Java: multimap.put("key", "v1"); multimap.put("key", "v2");
        let mut mm: Multimap<&str, &str> = Multimap::new();
        mm.put("key", "v1");
        mm.put("key", "v2");
        assert_eq!(mm.get(&"key"), &["v1", "v2"]);
        assert_eq!(mm.len(), 2);
    }

    #[test]
    fn test_multimap_missing_key() {
        // Java: multimap.get("absent") → empty list
        let mm: Multimap<&str, i32> = Multimap::new();
        assert!(mm.get(&"absent").is_empty());
    }

    // ── Strings テスト ───────────────────────────

    #[test]
    fn test_joiner_on() {
        // Java: Joiner.on(", ").join(["a", "b", "c"]) == "a, b, c"
        assert_eq!(joiner_on(", ", ["a", "b", "c"]), "a, b, c");
    }

    #[test]
    fn test_joiner_skip_nulls() {
        // Java: Joiner.on(",").skipNulls().join([Some("a"), None, Some("c")]) == "a,c"
        let items = vec![Some("a"), None, Some("c")];
        assert_eq!(joiner_skip_nulls(",", items), "a,c");
    }

    #[test]
    fn test_strings_is_null_or_empty() {
        // Java: Strings.isNullOrEmpty(null) == true
        assert!(strings_is_null_or_empty(None));
        // Java: Strings.isNullOrEmpty("") == true
        assert!(strings_is_null_or_empty(Some("")));
        // Java: Strings.isNullOrEmpty("hello") == false
        assert!(!strings_is_null_or_empty(Some("hello")));
    }

    #[test]
    fn test_strings_null_to_empty() {
        // Java: Strings.nullToEmpty(null) == ""
        assert_eq!(strings_null_to_empty(None), "");
        assert_eq!(strings_null_to_empty(Some("hi")), "hi");
    }

    #[test]
    fn test_strings_repeat() {
        // Java: Strings.repeat("ab", 3) == "ababab"
        assert_eq!(strings_repeat("ab", 3), "ababab");
    }
}
