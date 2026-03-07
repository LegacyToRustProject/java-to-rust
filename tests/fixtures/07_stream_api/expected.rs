use std::collections::HashMap;

fn filter_and_transform(items: &[String]) -> Vec<String> {
    let mut result: Vec<String> = items
        .iter()
        .filter(|s| s.len() > 3)
        .map(|s| s.to_uppercase())
        .collect();
    result.sort();
    result
}

fn sum_of_squares(numbers: &[i32]) -> i32 {
    numbers.iter().map(|n| n * n).sum()
}

fn find_first(items: &[String], prefix: &str) -> Option<String> {
    items.iter().find(|s| s.starts_with(prefix)).cloned()
}

fn group_by_length(words: &[String]) -> HashMap<usize, Vec<String>> {
    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();
    for word in words {
        groups.entry(word.len()).or_default().push(word.clone());
    }
    groups
}
