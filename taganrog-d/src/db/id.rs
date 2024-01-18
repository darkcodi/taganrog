use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::string::ToString;
use anyhow::anyhow;
use nanoid::nanoid;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;

const ID_LENGTH: usize = 8;
const DELIMETER: char = ':';
const ALPHABET: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', // 0-9
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', // A-Z
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', // a-z
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Id<const T: &'static str> {
    entity_type: &'static str,
    entity_id: heapless::String<ID_LENGTH>,
}

#[derive(thiserror::Error, Debug)]
pub enum IdError {
    #[error("invalid fixed-string length for id")]
    InvalidLength,
    #[error("id contains characters that are not in alphabet")]
    InvalidAlphabet,
    #[error("failed to parse id string")]
    StringParseError,
}

impl<const T: &'static str> Id<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_fixed_str(s: heapless::String<ID_LENGTH>) -> Result<Self, IdError> {
        if s.len() != ID_LENGTH {
            return Err(IdError::InvalidLength);
        }
        if s.chars().any(|c| !ALPHABET.contains(&c)) {
            return Err(IdError::InvalidAlphabet);
        }

        Ok(Self {
            entity_type: T,
            entity_id: s,
        })
    }

    pub fn just_id(&self) -> String {
        self.entity_id.to_string()
    }
}

impl<const T: &'static str> Default for Id<T> {
    fn default() -> Self {
        let id_str = nanoid!(ID_LENGTH, &ALPHABET);

        Self {
            entity_type: T,
            entity_id: heapless::String::from_str(&id_str).unwrap(),
        }
    }
}

impl<const T: &'static str> FromStr for Id<T> {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut str = s;
        if str.starts_with(T) {
            let maybe_delimiter_index = str.find(DELIMETER);
            if let Some(delimiter_index) = maybe_delimiter_index {
                if delimiter_index == T.len() {
                    str = &str[delimiter_index + 1..];
                }
            }
        }
        if str.len() != ID_LENGTH {
            return Err(IdError::InvalidLength)
        }
        let fixed_str = heapless::String::<ID_LENGTH>::from_str(str)
            .map_err(|_| IdError::StringParseError)?;
        Id::<T>::from_fixed_str(fixed_str)
    }
}

impl<const T: &'static str> Display for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.entity_type, DELIMETER, self.entity_id)
    }
}

impl<const T: &'static str> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de, const T: &'static str> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let str: String = Deserialize::deserialize(deserializer)?;
        let id = Id::<T>::from_str(str.as_str())
            .map_err(|e| D::Error::custom(e.to_string()))?;
        Ok(id)
    }
}

#[cfg(test)]
mod id_tests {
    use std::str::FromStr;
    use crate::db::id::Id;

    #[test]
    fn from_str() {
        assert_eq!(Id::<"tag">::from_str("n5yv4r").unwrap().to_string(), "tag:n5yv4r");
        assert_eq!(Id::<"tag">::from_str("tag:n5yv4r").unwrap().to_string(), "tag:n5yv4r");
        assert_eq!(Id::<"tag">::from_str("tagara").unwrap().to_string(), "tag:tagara");
        assert_eq!(Id::<"tag">::from_str("tag:tagara").unwrap().to_string(), "tag:tagara");
    }

    #[test]
    fn serialize() {
        let id = Id::<"tag">::from_str("n5yv4r").unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"tag:n5yv4r\"")
    }

    #[test]
    fn deserialize() {
        let id: Id::<"tag"> = serde_json::from_str("\"tag:n5yv4r\"").unwrap();
        assert_eq!(id.to_string(), "tag:n5yv4r")
    }
}
