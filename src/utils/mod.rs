use crate::utils::str_utils::StringExtensions;

pub mod vec_utils;
pub mod hash_utils;
pub mod str_utils;

pub fn normalize_query(query: &str) -> String {
    let mut tags = query.split(" ").map(|x| x.trim()).filter(|x| !x.is_empty()).map(|x| x.slugify().to_string()).collect::<Vec<String>>();
    if tags.len() > 0 && query.ends_with(" ") {
        tags.push("".to_string());
    }
    tags.join(" ")
}
