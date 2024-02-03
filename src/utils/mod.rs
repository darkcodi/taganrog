use itertools::Itertools;
use crate::utils::str_utils::StringExtensions;

pub mod vec_utils;
pub mod hash_utils;
pub mod str_utils;

pub fn normalize_query(query: &str) -> String {
    let mut tags = query.split(" ")
        .map(|x| x.trim()) // remove leading and trailing whitespaces
        .filter(|x| !x.is_empty()) // remove empty strings (e.g. multiple spaces)
        .map(|x| x.slugify().to_string()) // convert to slug
        .unique() // filter out duplicates
        .collect::<Vec<String>>();
    if tags.len() > 0 && query.ends_with(" ") {
        tags.push("".to_string());
    }
    tags.join(" ")
}
