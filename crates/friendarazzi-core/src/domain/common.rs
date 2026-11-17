//! Small value types shared by more than one aggregate.

use std::fmt;
use std::str::FromStr;

/// A string with at least one visible character; surrounding whitespace is trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NonEmptyString(String);

#[derive(Debug, thiserror::Error)]
#[error("der Text darf nicht leer sein")]
pub struct NonEmptyStringFehler;

impl NonEmptyString {
    pub fn neu<S: AsRef<str>>(wert: S) -> Result<Self, NonEmptyStringFehler> {
        let wert = wert.as_ref().trim();
        if wert.is_empty() {
            return Err(NonEmptyStringFehler);
        }
        Ok(Self(wert.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for NonEmptyString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NonEmptyString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for NonEmptyString {
    type Err = NonEmptyStringFehler;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::neu(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn non_empty_string_trims_and_refuses_blank() {
        assert_eq!(NonEmptyString::neu("  Tim ").unwrap().as_str(), "Tim");
        assert!(NonEmptyString::neu("   ").is_err());
        assert!(NonEmptyString::neu("").is_err());
    }
}
