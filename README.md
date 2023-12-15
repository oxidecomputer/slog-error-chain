## Overview

`slog-error-chain` provides `Display` and `slog::Value` adapters to report
the full chain of error causes from `std::error::Error`s.

### Background

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

See the [`basic`](./examples/basic.rs) example.

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

Doing so will make the `Display` implementation of `MyError` _look_ correct:

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
  applied to error types; it provides an implementation of `slog::Value` that
  delegates to `InlineErrorChain`.
* `nested-values`: Provides the `ArrayErrorChain` type, which is similar to
  `InlineErrorChain` except that it also implements `slog::SerdeValue`, and for
  loggers that support nested values, the error will be logged as an array of
  strings (one element per error in the chain).

If both `derive` and `nested-values` are enabled, the
`#[derive(SlogArrayError)]` proc macro is provided. This gives an implementation
of `slog::SerdeValue` for the error type that delegates to `ArrayErrorChain`.
However, implementing `slog::SerdeValue` also requires implementing
`serde::Serialize`, so this proc macro cannot be used with error types that
already implement `serde::Serialize`.

See the [`derive`](./examples/derive.rs) example, which demonstrates all of the
above features.
