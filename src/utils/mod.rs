use itertools::Itertools;
use crate::utils::str_utils::StringExtensions;

pub mod hash_utils;
pub mod str_utils;

pub fn normalize_query(query: &str) -> String {
    let initial_tags = query.split(' ')
        .map(|x| x.trim()) // remove leading and trailing whitespaces
        .filter(|x| !x.is_empty()) // remove empty strings (e.g. multiple spaces)
        .collect::<Vec<&str>>();
    let tags_to_exclude = initial_tags.iter()
        .filter(|x| x.starts_with('-')) // find tags to exclude
        .map(|x| x.slugify()) // and slugify them
        .filter(|x| !x.is_empty()) // remove empty strings
        .collect::<Vec<String>>();
    let mut final_tags = initial_tags.iter()
        .map(|x| x.slugify().to_string()) // slugify all tags
        .filter(|x| !x.is_empty()) // remove empty strings
        .filter(|x| !tags_to_exclude.contains(x)) // remove tags to exclude
        .unique() // filter out duplicates
        .collect::<Vec<String>>();

    if final_tags.len() > 1 && final_tags.iter().any(|x| x == "all" || x == "no-thumbnail") {
        final_tags.retain(|x| x != "all" && x != "no-thumbnail");
    }

    let mut normalized_query = final_tags.join(" ");

    // append ' ' if query ends with a space
    if !normalized_query.is_empty() && query.ends_with(' ') {
        normalized_query.push(' ');
    }

    normalized_query
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_query() {
        assert_eq!(normalize_query("  "), "");
        assert_eq!(normalize_query("  -  "), "");
        assert_eq!(normalize_query("  -  -  "), "");
        assert_eq!(normalize_query("tag1"), "tag1");
        assert_eq!(normalize_query("tag1 tag2"), "tag1 tag2");
        assert_eq!(normalize_query("tag1 tag2 tag1"), "tag1 tag2");
        assert_eq!(normalize_query("tag1   tag2"), "tag1 tag2");
        assert_eq!(normalize_query("tag1   tag2 "), "tag1 tag2 ");
        assert_eq!(normalize_query("tag1   tag2   "), "tag1 tag2 ");
    }
}
