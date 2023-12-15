// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! [`ArrayErrorChain`] supports logging error chains as arrays of strings, one
//! element per cause, via [`slog::SerdeValue`] for loggers that support
//! structured values (aka `nested-values`), such as `slog-json`.

use crate::InlineErrorChain;
use serde::ser::SerializeSeq;
use serde::Serialize;
use slog::KV;
use slog::SerdeValue;
use slog::Value;
use std::error::Error;
use std::fmt;

/// An owned, `'static` version of an [`ArrayErrorChain`].
///
/// `OwnedErrorChain` is relatively expensive to construct, as it always
/// allocates a `String` for the initial error and additionally allocates a
/// `Vec<String>` for any causes in the error's chain. This type exists
/// primarily to allow [`ArrayErrorChain`] to implement [`slog::SerdeValue`],
/// which requires the ability to convert to an owned value (e.g., to offload to
/// another thread for serialization, such as when `slog-async` is used).
#[derive(Debug, Clone)]
pub struct OwnedErrorChain {
    first: String,
    rest: Vec<String>,
}

impl OwnedErrorChain {
    /// Construct a new `OwnedErrorChain` from an error.
    pub fn new(err: &dyn Error) -> Self {
        let mut causes = vec![];
        let mut source = err.source();
        while let Some(cause) = source {
            causes.push(cause.to_string());
            source = cause.source();
        }
        Self { first: err.to_string(), rest: causes }
    }
}

impl fmt::Display for OwnedErrorChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.first)?;
        for source in &self.rest {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl Serialize for OwnedErrorChain {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(1 + self.rest.len()))?;
        seq.serialize_element(self.first.as_str())?;
        for s in &self.rest {
            seq.serialize_element(s.as_str())?;
        }
        seq.end()
    }
}

impl KV for OwnedErrorChain {
    #[allow(clippy::useless_conversion)] // see InlineErrorChain's KV impl
    fn serialize(
        &self,
        _record: &slog::Record,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_serde("error".into(), self)
    }
}

impl Value for OwnedErrorChain {
    fn serialize(
        &self,
        _record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_serde(key, self)
    }
}

impl SerdeValue for OwnedErrorChain {
    fn as_serde(&self) -> &dyn erased_serde::Serialize {
        self
    }

    fn to_sendable(&self) -> Box<dyn SerdeValue + Send + 'static> {
        Box::new(self.clone())
    }

    fn serialize_fallback(
        &self,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result<()> {
        serializer.emit_arguments(key, &format_args!("{self}"))
    }
}

/// Adapter for [`Error`]s that provides a [`slog::SerdeValue`] implementation
/// that serializes the chain of errors as an array of strings.
///
/// `ArrayErrorChain`'s `Display` implementation and its fallback `SerdeValue`
/// format when using a logger that does not support nested values matches the
/// behavior of [`InlineErrorChain`]: the chain of errors is printed as a single
/// string with the causes separated by `: `.
pub struct ArrayErrorChain<'a>(&'a dyn Error);

impl<'a> ArrayErrorChain<'a> {
    /// Construct a new `ArrayErrorChain` from an error.
    pub fn new(err: &'a dyn Error) -> Self {
        Self(err)
    }
}

impl fmt::Display for ArrayErrorChain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        InlineErrorChain::new(self.0).fmt(f)
    }
}

impl Serialize for ArrayErrorChain<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&format_args!("{}", self.0))?;
        let mut source = self.0.source();
        while let Some(cause) = source {
            seq.serialize_element(&format_args!("{cause}"))?;
            source = cause.source();
        }
        seq.end()
    }
}

impl KV for ArrayErrorChain<'_> {
    #[allow(clippy::useless_conversion)] // see InlineErrorChain's KV impl
    fn serialize(
        &self,
        _record: &slog::Record,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_serde("error".into(), self)
    }
}

impl Value for ArrayErrorChain<'_> {
    fn serialize(
        &self,
        _record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_serde(key, self)
    }
}

impl SerdeValue for ArrayErrorChain<'_> {
    fn as_serde(&self) -> &dyn erased_serde::Serialize {
        self
    }

    fn to_sendable(&self) -> Box<dyn SerdeValue + Send + 'static> {
        Box::new(OwnedErrorChain::new(self.0))
    }

    fn serialize_fallback(
        &self,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result<()> {
        serializer.emit_arguments(key, &format_args!("{self}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{ErrorA, ErrorB};
    use slog::{b, record, Level};
    use std::io;

    #[derive(Default, Debug)]
    struct StringSerializer(String);

    impl slog::Serializer for StringSerializer {
        fn emit_arguments(
            &mut self,
            _key: slog::Key,
            val: &core::fmt::Arguments,
        ) -> slog::Result {
            self.0.push_str(&format!("{val}"));
            Ok(())
        }

        fn emit_serde(
            &mut self,
            _key: slog::Key,
            value: &dyn SerdeValue,
        ) -> slog::Result {
            let mut json = serde_json::Serializer::new(Vec::<u8>::new());
            {
                let mut json: Box<dyn erased_serde::Serializer> =
                    Box::new(<dyn erased_serde::Serializer>::erase(&mut json));
                value.erased_serialize(&mut json).unwrap();
            }
            let json = String::from_utf8(json.into_inner()).unwrap();
            self.0.push_str(&json);
            Ok(())
        }
    }

    #[test]
    fn owned_error_chain_formatting() {
        let dummy_args = format_args!("dummy");
        let dummy_record = record!(Level::Info, "dummy", &dummy_args, b!());

        let err = io::Error::new(io::ErrorKind::Other, "test error");

        // Check `Display` and non-serde serialization
        let chain = OwnedErrorChain::new(&err);
        assert_eq!(chain.to_string(), "test error");

        let mut out = StringSerializer::default();
        chain.serialize_fallback("unused", &mut out).unwrap();
        assert_eq!(out.0, "test error");

        // Check serde serialization
        let mut out = StringSerializer::default();
        Value::serialize(&chain, &dummy_record, "unused", &mut out).unwrap();
        assert_eq!(out.0, r#"["test error"]"#);

        let err = ErrorA::A(err);
        let chain = OwnedErrorChain::new(&err);
        assert_eq!(chain.to_string(), "error a: test error");

        let mut out = StringSerializer::default();
        chain.serialize_fallback("unused", &mut out).unwrap();
        assert_eq!(out.0, "error a: test error");

        let mut out = StringSerializer::default();
        Value::serialize(&chain, &dummy_record, "unused", &mut out).unwrap();
        assert_eq!(out.0, r#"["error a","test error"]"#);

        let err = ErrorB::B(err);
        let chain = OwnedErrorChain::new(&err);
        assert_eq!(chain.to_string(), "error b: error a: test error");

        let mut out = StringSerializer::default();
        chain.serialize_fallback("unused", &mut out).unwrap();
        assert_eq!(out.0, "error b: error a: test error");

        let mut out = StringSerializer::default();
        Value::serialize(&chain, &dummy_record, "unused", &mut out).unwrap();
        assert_eq!(out.0, r#"["error b","error a","test error"]"#);
    }

    #[test]
    fn array_error_chain_formatting() {
        let dummy_args = format_args!("dummy");
        let dummy_record = record!(Level::Info, "dummy", &dummy_args, b!());

        let err = io::Error::new(io::ErrorKind::Other, "test error");

        // Check `Display` and non-serde serialization
        let chain = ArrayErrorChain::new(&err);
        assert_eq!(chain.to_string(), "test error");

        let mut out = StringSerializer::default();
        chain.serialize_fallback("unused", &mut out).unwrap();
        assert_eq!(out.0, "test error");

        // Check serde serialization
        let mut out = StringSerializer::default();
        Value::serialize(&chain, &dummy_record, "unused", &mut out).unwrap();
        assert_eq!(out.0, r#"["test error"]"#);

        let err = ErrorA::A(err);
        let chain = ArrayErrorChain::new(&err);
        assert_eq!(chain.to_string(), "error a: test error");

        let mut out = StringSerializer::default();
        chain.serialize_fallback("unused", &mut out).unwrap();
        assert_eq!(out.0, "error a: test error");

        let mut out = StringSerializer::default();
        Value::serialize(&chain, &dummy_record, "unused", &mut out).unwrap();
        assert_eq!(out.0, r#"["error a","test error"]"#);

        let err = ErrorB::B(err);
        let chain = ArrayErrorChain::new(&err);
        assert_eq!(chain.to_string(), "error b: error a: test error");

        let mut out = StringSerializer::default();
        chain.serialize_fallback("unused", &mut out).unwrap();
        assert_eq!(out.0, "error b: error a: test error");

        let mut out = StringSerializer::default();
        Value::serialize(&chain, &dummy_record, "unused", &mut out).unwrap();
        assert_eq!(out.0, r#"["error b","error a","test error"]"#);
    }
}
