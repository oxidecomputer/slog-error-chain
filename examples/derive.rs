// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use slog::info;
use slog::o;
use slog::Drain;
use slog::Logger;
use slog_error_chain::SlogInlineError;
use std::io;

#[derive(Debug, thiserror::Error, SlogInlineError)]
enum OuterError {
    #[error("outer error")]
    Outer(#[source] InnerError),
}

#[derive(Debug, thiserror::Error, SlogInlineError)]
enum InnerError {
    #[error("inner error")]
    Inner(#[source] io::Error),
}

fn main() {
    let plain = slog_term::PlainSyncDecorator::new(io::stdout());
    let log =
        Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());

    let err = OuterError::Outer(InnerError::Inner(io::Error::new(
        io::ErrorKind::Other,
        "custom I/O error",
    )));

    info!(
        log, "slog-term inline error formatting, explicit key";
        "my-key" => &err,
    );
    info!(
        log, "slog-term inline error formatting, implicit key";
        &err,
    );
}
