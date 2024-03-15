use itertools::Itertools;

pub trait StringExtensions<'a, T: Into<&'a str>> {
    /// Convert a title string to a slug for identifying an article.
    /// E.g. `slugify("Doctests are the Bee's Knees") == "doctests-are-the-bees-knees"`
    fn slugify(self) -> String;

    /// Convert an empty string to None.
    /// E.g. `empty_to_none("") == None`
    /// E.g. `empty_to_none("foo") == Some("foo")`
    fn empty_to_none(self) -> Option<String>;
}

impl<'a, T: Into<&'a str>> StringExtensions<'a, T> for T {
    fn slugify(self) -> String {
        const QUOTE_CHARS: &[char] = &['\'', '"'];

        self
            .into()
            .split(|c: char| !(QUOTE_CHARS.contains(&c) || c.is_alphanumeric()))
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut s = s.replace(QUOTE_CHARS, "");
                s.make_ascii_lowercase();
                s
            })
            .join("-")
    }

    fn empty_to_none(self) -> Option<String> {
        let str = self.into();
        if str.is_empty() {
            None
        } else {
            Some(str.to_owned())
        }
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

#[test]
fn test_empty_to_none() {
    assert_eq!("".to_string().empty_to_none(), None);
    assert_eq!("foo".to_string().empty_to_none(), Some("foo".to_string()));
}
