use itertools::Itertools;

pub trait StringExtensions {
    /// Convert a title string to a slug for identifying an article.
    /// E.g. `slugify("Doctests are the Bee's Knees") == "doctests-are-the-bees-knees"`
    fn slugify(&self) -> String;
}

impl StringExtensions for String {
    fn slugify(&self) -> String {
        const QUOTE_CHARS: &[char] = &['\'', '"'];

        self
            .split(|c: char| !(QUOTE_CHARS.contains(&c) || c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut s = s.replace(QUOTE_CHARS, "");
                s.make_ascii_lowercase();
                s
            })
            .join("-")
    }
}

#[test]
fn test_slugify() {
    assert_eq!(
        "Segfaults and You: When Raw Pointers Go Wrong".to_string().slugify(),
        "segfaults-and-you-when-raw-pointers-go-wrong"
    );

    assert_eq!(
        "Why are DB Admins Always Shouting?".to_string().slugify(),
        "why-are-db-admins-always-shouting"
    );

    assert_eq!(
        "Converting to Rust from C: It's as Easy as 1, 2, 3!".to_string().slugify(),
        "converting-to-rust-from-c-its-as-easy-as-1-2-3"
    )
}
