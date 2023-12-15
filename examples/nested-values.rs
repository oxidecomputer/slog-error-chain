// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use slog::info;
use slog::o;
use slog::Drain;
use slog::Logger;
use slog_error_chain::SlogArrayError;
use slog_error_chain::SlogInlineError;
use std::io;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error, SlogInlineError)]
enum InlineOuterError {
    #[error("outer error")]
    Outer(#[source] InlineInnerError),
}

#[derive(Debug, thiserror::Error, SlogInlineError)]
enum InlineInnerError {
    #[error("inner error")]
    Inner(#[source] io::Error),
}

#[derive(Debug, thiserror::Error, SlogArrayError)]
enum ArrayOuterError {
    #[error("outer error")]
    Outer(#[source] ArrayInnerError),
}

#[derive(Debug, thiserror::Error, SlogArrayError)]
enum ArrayInnerError {
    #[error("inner error")]
    Inner(#[source] io::Error),
}

fn main() {
    let plain = slog_term::PlainSyncDecorator::new(io::stdout());
    let log =
        Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());

    let inline_err = InlineOuterError::Outer(InlineInnerError::Inner(
        io::Error::new(io::ErrorKind::Other, "custom I/O error"),
    ));
    let array_err = ArrayOuterError::Outer(ArrayInnerError::Inner(
        io::Error::new(io::ErrorKind::Other, "custom I/O error"),
    ));

    info!(
        log, "slog-term inline error formatting, explicit key";
        "my-key" => &inline_err,
    );
    info!(
        log, "slog-term inline error formatting, implicit key";
        &inline_err,
    );
    info!(
        log, "slog-term structured error formatting, explicit key";
        "my-key" => &array_err,
    );
    info!(
        log, "slog-term structured error formatting, implicit key";
        &array_err,
    );

    let json = slog_json::Json::default(io::stdout());
    let log = Logger::root(Mutex::new(json).fuse(), o!());

    info!(
        log, "slog-json inline error formatting, explicit key";
        "my-key" => &inline_err,
    );
    info!(
        log, "slog-json inline error formatting, implicit key";
        &inline_err,
    );
    info!(
        log, "slog-json structured error formatting, explicit key";
        "my-key" => &array_err,
    );
    info!(
        log, "slog-json structured error formatting, implicit key";
        &array_err,
    );
}
