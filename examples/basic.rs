// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use slog::info;
use slog::o;
use slog::Drain;
use slog::Logger;
use slog_error_chain::InlineErrorChain;
use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error("an I/O error occurred trying to open {}", .path.display())]
    OpeningFile {
        path: PathBuf,
        #[source]
        err: io::Error,
    },
}

fn main() {
    let plain = slog_term::PlainSyncDecorator::new(io::stdout());
    let log =
        Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());

    let err = MyError::OpeningFile {
        path: "/some/path".into(),
        err: io::Error::new(io::ErrorKind::Other, "custom I/O error"),
    };

    info!(log, "logging error with Display impl"; "err" => %err);
    info!(
        log, "logging error with InlineErrorChain, explicit key";
        "my-key" => InlineErrorChain::new(&err),
    );
    info!(
        log, "logging error with InlineErrorChain, implicit key";
        InlineErrorChain::new(&err),
    );
}
