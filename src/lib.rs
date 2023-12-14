// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! TODO

use slog::Value;
use std::error::Error;
use std::fmt;

#[cfg(feature = "nested-values")]
mod nested_values;
#[cfg(feature = "nested-values")]
pub use nested_values::*;

#[cfg(feature = "derive")]
pub use slog_error_chain_derive::{SlogArrayError, SlogInlineError};

/// TODO
pub struct InlineErrorChain<'a>(&'a dyn Error);

impl <'a> InlineErrorChain<'a> {
    /// TODO
    pub fn new(err: &'a dyn Error) -> Self {
        Self(err)
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
