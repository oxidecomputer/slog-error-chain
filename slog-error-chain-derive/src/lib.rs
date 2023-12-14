// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! TODO

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
            fn as_serde(&self) -> &dyn ::erased_serde::Serialize {
                self
            }

            fn to_sendable(&self) -> Box<dyn ::slog::SerdeValue + Send + 'static> {
                Box::new(::slog_error_chain::OwnedErrorChain::new(self))
            }

            fn serialize_fallback(
                &self,
                key: slog::Key,
                serializer: &mut dyn slog::Serializer,
            ) -> slog::Result<()> {
                ::slog_error_chain::ArrayErrorChain::new(self)
                    .serialize_fallback(key, serializer)
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}
