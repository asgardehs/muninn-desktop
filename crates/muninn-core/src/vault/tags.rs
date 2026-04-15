use std::collections::HashMap;

/// A tag with its occurrence count across the vault.
#[derive(Debug, Clone)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

/// Collect and count tags from a set of tag lists.
pub fn collect_tags(tag_lists: &[Vec<String>]) -> Vec<TagCount> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for tags in tag_lists {
        for tag in tags {
            let normalized = tag.to_lowercase();
            *counts.entry(normalized).or_insert(0) += 1;
        }
    }

    let mut result: Vec<TagCount> = counts
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();

    result.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.tag.cmp(&b.tag)));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_tags() {
        let tag_lists = vec![
            vec!["rust".to_string(), "code".to_string()],
            vec!["Rust".to_string(), "test".to_string()],
            vec!["code".to_string()],
        ];

        let counts = collect_tags(&tag_lists);
        assert_eq!(counts[0].tag, "code");
        assert_eq!(counts[0].count, 2);
        assert_eq!(counts[1].tag, "rust");
        assert_eq!(counts[1].count, 2);
        assert_eq!(counts[2].tag, "test");
        assert_eq!(counts[2].count, 1);
    }

    #[test]
    fn empty_input() {
        let counts = collect_tags(&[]);
        assert!(counts.is_empty());
    }
}
