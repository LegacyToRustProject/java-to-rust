use std::collections::{HashMap, HashSet};

fn process_items(items: &[String]) -> Vec<String> {
    items.iter().map(|item| item.to_uppercase()).collect()
}

fn word_count(words: &[String]) -> HashMap<String, i32> {
    let mut counts = HashMap::new();
    for word in words {
        *counts.entry(word.clone()).or_insert(0) += 1;
    }
    counts
}

fn unique_items(items: &[String]) -> HashSet<String> {
    items.iter().cloned().collect()
}
