use crate::{raw, ValkeyError, ValkeyString};

/// Trait surface for code that wants to accept a real [`ValkeyString`] in
/// production and a [`MockValkeyString`] in tests.
#[cfg_attr(feature = "mockall", mockall::automock)]
pub trait ValkeyStringTrait {
    fn append(&mut self, s: &str) -> raw::Status;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn try_as_str(&self) -> Result<&'static str, ValkeyError>;
    fn as_slice(&self) -> &'static [u8];
    fn to_string_lossy(&self) -> String;
    fn parse_unsigned_integer(&self) -> Result<u64, ValkeyError>;
    fn parse_integer(&self) -> Result<i64, ValkeyError>;
    fn parse_float(&self) -> Result<f64, ValkeyError>;
}

impl ValkeyStringTrait for ValkeyString {
    fn append(&mut self, s: &str) -> raw::Status {
        self.append(s)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn try_as_str(&self) -> Result<&'static str, ValkeyError> {
        Self::from_ptr(self.inner).map_err(|_| ValkeyError::Str("Couldn't parse as UTF-8 string"))
    }

    fn as_slice(&self) -> &'static [u8] {
        Self::string_as_slice(self.inner)
    }

    fn to_string_lossy(&self) -> String {
        self.to_string_lossy()
    }

    fn parse_unsigned_integer(&self) -> Result<u64, ValkeyError> {
        self.parse_unsigned_integer()
    }

    fn parse_integer(&self) -> Result<i64, ValkeyError> {
        self.parse_integer()
    }

    fn parse_float(&self) -> Result<f64, ValkeyError> {
        self.parse_float()
    }
}

#[cfg(feature = "mockall")]
pub use MockValkeyStringTrait as MockValkeyString;

#[cfg(test)]
mod tests {
    use super::{raw, MockValkeyString, ValkeyStringTrait};
    use mockall::predicate::eq;

    #[test]
    fn test_mock_valkey_string() {
        let mut s = MockValkeyString::new();
        s.expect_len().returning(|| 5);
        s.expect_is_empty().returning(|| false);
        s.expect_try_as_str().returning(|| Ok("hello"));
        s.expect_as_slice().returning(|| b"hello");
        s.expect_to_string_lossy().returning(|| "hello".to_string());
        s.expect_parse_unsigned_integer().returning(|| Ok(42));
        s.expect_parse_integer().returning(|| Ok(-42));
        s.expect_parse_float().returning(|| Ok(42.5));
        s.expect_append()
            .with(eq("!"))
            .returning(|_| raw::Status::Ok);

        assert_eq!(ValkeyStringTrait::len(&s), 5);
        assert!(!ValkeyStringTrait::is_empty(&s));
        assert_eq!(ValkeyStringTrait::try_as_str(&s).unwrap(), "hello");
        assert_eq!(ValkeyStringTrait::as_slice(&s), b"hello");
        assert_eq!(ValkeyStringTrait::to_string_lossy(&s), "hello");
        assert_eq!(ValkeyStringTrait::parse_unsigned_integer(&s).unwrap(), 42);
        assert_eq!(ValkeyStringTrait::parse_integer(&s).unwrap(), -42);
        assert_eq!(ValkeyStringTrait::parse_float(&s).unwrap(), 42.5);
        assert_eq!(ValkeyStringTrait::append(&mut s, "!"), raw::Status::Ok);
    }
}
