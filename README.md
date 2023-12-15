## Overview

`slog-error-chain` provides `std::fmt::Display`, `slog::KV`, and `slog::Value`
adapters to report the full chain of error causes from `std::error::Error`s, and
a proc macro to derive `slog::KV` and `slog::Value` implementations for error
types which will log the full chain of error causes.

This crate was born out of a use of `thiserror` to derive `std::error::Error`
implementations on error enums, although it does not depend on `thiserror` and
will work with any `Error`s. Error enums often wrap other error sources, such
as:

```rust
#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error("an I/O error occurred trying to open {}", .path.display())]
    OpeningFile {
        path: PathBuf,
        #[source]
        err: io::Error,
    },
}
```

The `Display` implementation produced by deriving `thiserror::Error` only prints
the topmost error, and does not print any causes. Given the example above, a
`MyError::OpeningFile { .. }` error will `Display`-format as

```text
# println!("{my_error}")
an I/O error occurred trying to open /some/path
```

This crate provides `InlineErrorChain`, which is an adapter that will print the
full error chain:

```text
# println!("{}", InlineErrorChain::new(&my_error))
an I/O error occurred trying to open /some/path: file not found
```

`InlineErrorChain` also implements `slog::Value` and `slog::KV`, allowing it to
be logged directly:

```rust
// explicit key
info!(
    log, "something happened"; "my-key" => InlineErrorChain::new(&err),
);

// key omitted; will log with the key "error"
info!(
    log, "something happened"; InlineErrorChain::new(&err),
);
```

With the `derive` feature enabled, error types can `#[derive(SlogInlineError)]`
to gain `slog::Value` and `slog::KV` implementations on themselves, allowing
them to be logged directly:

```rust
use slog_error_chain::SlogInlineError;

#[derive(Debug, thiserror::Error, SlogInlineError)]
enum MyError {
    #[error("an I/O error occurred trying to open {}", .path.display())]
    OpeningFile {
        path: PathBuf,
        #[source]
        err: io::Error,
    },
}

let err = MyError::OpeningFile { .. };

// explicit key; logs the full chain
info!(log, "something happened"; "my-key" => &err);

// implicit key; logs the full chain with the key "error"
info!(log, "something happened"; &err);
```

### Aside: Embedding Source Error Strings

An easy solution to reach for when encountering the "printing an error doesn't
show the underlying cause" problem is to embed the inner error in the outer
error's display string, such as by adding `: {err}` to the above example:

```rust
#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error("an I/O error occurred trying to open {}: {err}", .path.display())]
    OpeningFile {
        path: PathBuf,
        #[source]
        err: io::Error,
    },
}
```

Doing so will make the `Display` implementation of `MyError` _look_ reasonable:

```text
# println!("{my_error}")
an I/O error occurred trying to open /some/path: file not found
```

but this is incorrect! If you use an error adapter that knows how to walk the
full chain of errors (such as `InlineErrorChain` or `anyhow::Error`), you will
see "double-speak" along the chain:

```text
# println!("{}", InlineErrorChain::new(&my_error))
an I/O error occurred trying to open /some/path: file not found: file not found
```

The amount of doubled error text will compound as additional errors are added to
the chain, as each layer reprints the remainder of the chain starting from
itself.

### Cargo Features

`slog-error-chain` gates additional functionality behind two cargo features:

* `derive`: Provides the `#[derive(SlogInlineError)]` proc macro that can be
  applied to error types; it provides implementations of `slog::Value` and
  `slog::KV` that delegate to `InlineErrorChain`.
* `nested-values`: Provides the `ArrayErrorChain` type, which is similar to
  `InlineErrorChain` except that it also implements `slog::SerdeValue`, and for
  loggers that support nested values, the error will be logged as an array of
  strings (one element per error in the chain).

If both `derive` and `nested-values` are enabled, the
`#[derive(SlogArrayError)]` proc macro is provided. This gives implementations
of `slog::Value`, `slog::SerdeValue`, and `slog::KV` for the error type that
delegates to `ArrayErrorChain`. However, implementing `slog::SerdeValue` also
requires implementing `serde::Serialize`, so this proc macro cannot be used with
error types that already implement `serde::Serialize`.

### Examples

[`basic`](./examples/basic.rs) demonstrates raw `InlineErrorChain` usage:

```console
% cargo run --example basic
Dec 15 20:34:03.682 INFO logging error with Display impl, err: an I/O error occurred trying to open /some/path
Dec 15 20:34:03.682 INFO logging error with InlineErrorChain, explicit key, my-key: an I/O error occurred trying to open /some/path: custom I/O error
Dec 15 20:34:03.682 INFO logging error with InlineErrorChain, implicit key, error: an I/O error occurred trying to open /some/path: custom I/O error
```

[`derive`](./examples/derive.rs) demonstrates `#[derive(SlogInlineError)]`:

```console
% cargo run --example derive --features derive
Dec 15 20:44:45.976 INFO derived slog::Value with explicit key, my-key: outer error: inner error: custom I/O error
Dec 15 20:44:45.976 INFO derived slog::KV using implicit error key, error: outer error: inner error: custom I/O error
```

[`nested-values`](./examples/nested-values.rs) demonstrates the `nested-values`
feature (along with `#[derive(SlogArrayError)]`:

```console
% cargo run --example nested-values --features derive,nested-values
Dec 15 20:34:25.329 INFO slog-term inline error formatting, explicit key, my-key: outer error: inner error: custom I/O error
Dec 15 20:34:25.329 INFO slog-term inline error formatting, implicit key, error: outer error: inner error: custom I/O error
Dec 15 20:34:25.329 INFO slog-term structured error formatting, explicit key, my-key: outer error: inner error: custom I/O error
Dec 15 20:34:25.329 INFO slog-term structured error formatting, implicit key, error: outer error: inner error: custom I/O error
{"msg":"slog-json inline error formatting, explicit key","level":"INFO","ts":"2023-12-15T20:34:25.329726569Z","my-key":"outer error: inner error: custom I/O error"}
{"msg":"slog-json inline error formatting, implicit key","level":"INFO","ts":"2023-12-15T20:34:25.329768879Z","error":"outer error: inner error: custom I/O error"}
{"msg":"slog-json structured error formatting, explicit key","level":"INFO","ts":"2023-12-15T20:34:25.329805499Z","my-key":["outer error","inner error","custom I/O error"]}
{"msg":"slog-json structured error formatting, implicit key","level":"INFO","ts":"2023-12-15T20:34:25.329853429Z","error":["outer error","inner error","custom I/O error"]}
```
