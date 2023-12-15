// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `slog-error-chain` provides `Display` and `slog::Value` adapters to report
//! the full chain of error causes from `std::error::Error`s.

use slog::Value;
use slog::KV;
use std::error::Error;
use std::fmt;

#[cfg(feature = "nested-values")]
pub use erased_serde;
#[cfg(feature = "nested-values")]
mod nested_values;
#[cfg(feature = "nested-values")]
pub use nested_values::*;

#[cfg(all(feature = "derive", feature = "nested-values"))]
pub use slog_error_chain_derive::SlogArrayError;
#[cfg(feature = "derive")]
pub use slog_error_chain_derive::SlogInlineError;

/// Adapter for [`Error`]s that provides both [`std::fmt::Display`] and
/// [`slog::Value`] implementations that print the full chain of error sources,
/// separated by `: `.
pub struct InlineErrorChain<'a>(&'a dyn Error);

impl<'a> InlineErrorChain<'a> {
    /// Construct a new `InlineErrorChain` from an error.
    pub fn new(err: &'a dyn Error) -> Self {
        Self(err)
    }
}

impl KV for InlineErrorChain<'_> {
    // If slog is built with the `dynamic-keys` feature, we have to pass a `Key`
    // to `emit_arguments`, hence `"error".into()`. However, if slog is built
    // without `dynamic-keys`, `emit_arguments` expects a `&str`.
    // `"error".into()` is still valid, but clippy warns about the useless
    // `.into()` call. Suppress that warning so we can use the same code
    // regardless of the slog feature.
    #[allow(clippy::useless_conversion)]
    fn serialize(
        &self,
        _record: &slog::Record,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_arguments("error".into(), &format_args!("{self}"))
    }
}

impl Value for InlineErrorChain<'_> {
    fn serialize(
        &self,
        _record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_arguments(key, &format_args!("{self}"))
    }
}

impl fmt::Display for InlineErrorChain<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;
        let mut cause = self.0.source();
        while let Some(source) = cause {
            write!(f, ": {source}")?;
            cause = source.source();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ErrorA {
        #[error("error a")]
        A(#[source] io::Error),
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ErrorB {
        #[error("error b")]
        B(#[source] ErrorA),
    }

    #[test]
    fn inline_error_chain_formatting() {
        let err = io::Error::new(io::ErrorKind::Other, "test error");
        assert_eq!(InlineErrorChain::new(&err).to_string(), "test error");

        let err = ErrorA::A(err);
        assert_eq!(
            InlineErrorChain::new(&err).to_string(),
            "error a: test error"
        );

        let err = ErrorB::B(err);
        assert_eq!(
            InlineErrorChain::new(&err).to_string(),
            "error b: error a: test error"
        );
    }
}
