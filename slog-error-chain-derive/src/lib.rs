// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Provides derive macros to attach `slog::Value` implementations to errors.
//!
//! The `SlogInlineError` macro provides a `slog::Value` implementation that
//! will log errors using `slog_error_chain::InlineErrorChain`; i.e., as a
//! single string with each cause in the error chain separated by colons.
//!
//! If the `nested-values` feature is enabled, the `SlogArrayError` macro
//! provides implementations for `slog::Value`, `slog::SerdeValue`, and
//! `serde::Serialize` that will log errors as an array of strings (one element
//! for each cause), if the logger in use itself supports nested values via
//! `serde`.

use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

#[proc_macro_derive(SlogInlineError)]
pub fn derive_slog_inline_error(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::slog::Value for #name #ty_generics #where_clause {
            fn serialize(
                &self,
                record: &::slog::Record,
                key: ::slog::Key,
                serializer: &mut dyn ::slog::Serializer,
            ) -> ::slog::Result {
                ::slog_error_chain::InlineErrorChain::new(self).serialize(
                    record,
                    key,
                    serializer,
                )
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[cfg(feature = "nested-values")]
#[proc_macro_derive(SlogArrayError)]
pub fn derive_slog_array_error(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::slog::Value for #name #ty_generics #where_clause {
            fn serialize(
                &self,
                record: &::slog::Record,
                key: ::slog::Key,
                serializer: &mut dyn ::slog::Serializer,
            ) -> ::slog::Result {
                ::slog_error_chain::ArrayErrorChain::new(self).serialize(
                    record,
                    key,
                    serializer,
                )
            }
        }

        impl #impl_generics ::serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                ::slog_error_chain::ArrayErrorChain::new(self).serialize(
                    serializer
                )
            }
        }

        impl #impl_generics ::slog::SerdeValue for #name #ty_generics #where_clause {
            fn as_serde(&self) -> &dyn ::slog_error_chain::erased_serde::Serialize {
                self
            }

            fn to_sendable(&self) -> Box<dyn ::slog::SerdeValue + Send + 'static> {
                Box::new(::slog_error_chain::OwnedErrorChain::new(self))
            }

            fn serialize_fallback(
                &self,
                key: ::slog::Key,
                serializer: &mut dyn ::slog::Serializer,
            ) -> slog::Result<()> {
                ::slog_error_chain::ArrayErrorChain::new(self)
                    .serialize_fallback(key, serializer)
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}
