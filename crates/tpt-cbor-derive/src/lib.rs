//! Derive macros for [`tpt-cbor`](https://docs.rs/tpt-cbor).
//!
//! Do not depend on this crate directly. Enable the `derive` feature of
//! `tpt-cbor` and use `tpt_cbor::{Encode, Decode}`.
//!
//! # Wire format
//!
//! * **Named struct**: a canonical map from text keys to fields. Keys are the
//!   field names, or their `rename` values. Entries are emitted in RFC 8949
//!   §4.2.1 bytewise key order, which is fixed at compile time.
//! * **Tuple struct**: an array of its fields, in declaration order.
//! * **Unit struct**: an empty array.
//! * **Enum** (externally tagged): a unit variant is its name as a text string.
//!   A tuple variant is `{"Name": [fields...]}`. A struct variant is
//!   `{"Name": {fields...}}`.
//!
//! Decoding a map ignores unknown keys, which allows forward compatibility.
//! Strict decoders still reject out-of-order or duplicate keys.
//!
//! # Attributes
//!
//! * `#[cbor(rename = "key")]` on a named field or variant.
//! * `#[cbor(skip)]` on a named field. The field is not encoded, and is
//!   `Default::default()` when decoded.
//! * `#[cbor(default)]` on a named field. A missing key decodes to
//!   `Default::default()`. `Option<T>` fields behave this way automatically.
//!
//! # Limitations
//!
//! At most one lifetime parameter is supported. It becomes the decode
//! lifetime. Unions are rejected.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeSet;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, Data, DataEnum, DeriveInput, Error, Fields, Generics, Ident,
    Index, LitStr, Member, Type,
};

/// Derives `tpt_cbor::Encode`. See the crate documentation for the format.
#[proc_macro_derive(Encode, attributes(cbor))]
pub fn derive_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_encode(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derives `tpt_cbor::Decode`. See the crate documentation for the format.
#[proc_macro_derive(Decode, attributes(cbor))]
pub fn derive_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_decode(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

// ---------------------------------------------------------------------------
// Parsed model
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Named,
    Tuple,
    Unit,
}

struct Field {
    member: Member,
    /// Text key for named fields.
    key: String,
    ty: Type,
    skip: bool,
    default: bool,
    /// Local identifier used while decoding or matching on enum variants.
    binding: Ident,
}

impl Field {
    fn live(&self) -> bool {
        !self.skip
    }
}

struct Body {
    shape: Shape,
    fields: Vec<Field>,
}

#[derive(Default)]
struct Attrs {
    rename: Option<String>,
    skip: bool,
    default: bool,
}

fn parse_attrs(attrs: &[syn::Attribute]) -> syn::Result<Attrs> {
    let mut out = Attrs::default();
    for attr in attrs {
        if !attr.path().is_ident("cbor") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let lit: LitStr = meta.value()?.parse()?;
                out.rename = Some(lit.value());
                Ok(())
            } else if meta.path.is_ident("skip") {
                out.skip = true;
                Ok(())
            } else if meta.path.is_ident("default") {
                out.default = true;
                Ok(())
            } else {
                Err(meta.error("unknown cbor attribute; expected `rename`, `skip` or `default`"))
            }
        })?;
    }
    Ok(out)
}

fn is_option_type(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => p
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "Option"),
        _ => false,
    }
}

fn collect(fields: &Fields) -> syn::Result<Body> {
    let (shape, list): (Shape, Vec<&syn::Field>) = match fields {
        Fields::Named(f) => (Shape::Named, f.named.iter().collect()),
        Fields::Unnamed(f) => (Shape::Tuple, f.unnamed.iter().collect()),
        Fields::Unit => (Shape::Unit, Vec::new()),
    };

    let mut out = Vec::with_capacity(list.len());
    for (i, field) in list.into_iter().enumerate() {
        let attrs = parse_attrs(&field.attrs)?;
        if shape != Shape::Named && (attrs.rename.is_some() || attrs.skip || attrs.default) {
            return Err(Error::new_spanned(
                field,
                "`rename`, `skip` and `default` are only supported on named fields",
            ));
        }
        let (member, key) = match &field.ident {
            Some(ident) => (
                Member::Named(ident.clone()),
                attrs.rename.clone().unwrap_or_else(|| ident.to_string()),
            ),
            None => (Member::Unnamed(Index::from(i)), String::new()),
        };
        out.push(Field {
            member,
            key,
            ty: field.ty.clone(),
            skip: attrs.skip,
            default: attrs.default || is_option_type(&field.ty),
            binding: format_ident!("__cbor_f{}", i),
        });
    }

    if shape == Shape::Named {
        let mut seen = BTreeSet::new();
        for f in out.iter().filter(|f| f.live()) {
            if !seen.insert(f.key.clone()) {
                return Err(Error::new(
                    Span::call_site(),
                    format!("duplicate CBOR key `{}`", f.key),
                ));
            }
        }
    }
    Ok(Body { shape, fields: out })
}

/// Canonical encoding of a text string used as a map key. Only used to sort
/// keys at compile time, so it must match the runtime encoder exactly.
fn text_key_encoding(s: &str) -> Vec<u8> {
    let n = s.len() as u64;
    let base = 0x60u8; // major type 3
    let mut out = if n < 24 {
        vec![base | n as u8]
    } else if n <= 0xff {
        vec![base | 24, n as u8]
    } else if n <= 0xffff {
        let mut v = vec![base | 25];
        v.extend_from_slice(&(n as u16).to_be_bytes());
        v
    } else if n <= 0xffff_ffff {
        let mut v = vec![base | 26];
        v.extend_from_slice(&(n as u32).to_be_bytes());
        v
    } else {
        let mut v = vec![base | 27];
        v.extend_from_slice(&n.to_be_bytes());
        v
    };
    out.extend_from_slice(s.as_bytes());
    out
}

// ---------------------------------------------------------------------------
// Generics
// ---------------------------------------------------------------------------

/// Adds `bound` for each type parameter that appears in one of `used_by`.
///
/// Parameters used only by skipped fields get no bound, because the derived
/// code never encodes or decodes those fields.
fn add_type_bounds(generics: &mut Generics, bound: &TokenStream2, used_by: &[Type]) {
    let mut used = BTreeSet::new();
    for ty in used_by {
        collect_idents(ty.to_token_stream(), &mut used);
    }
    let idents: Vec<Ident> = generics
        .type_params()
        .map(|p| p.ident.clone())
        .filter(|ident| used.contains(&ident.to_string()))
        .collect();
    let where_clause = generics.make_where_clause();
    for ident in idents {
        where_clause.predicates.push(parse_quote!(#ident: #bound));
    }
}

/// Collects every identifier in `ts`, descending into delimited groups.
fn collect_idents(ts: TokenStream2, out: &mut BTreeSet<String>) {
    for tt in ts {
        match tt {
            TokenTree::Ident(ident) => {
                out.insert(ident.to_string());
            }
            TokenTree::Group(group) => collect_idents(group.stream(), out),
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }
}

/// Types of the fields that the derived code encodes or decodes (all fields
/// that are not `skip`), across every variant for enums.
fn live_field_types(input: &DeriveInput) -> syn::Result<Vec<Type>> {
    fn push_live(fields: &Fields, out: &mut Vec<Type>) -> syn::Result<()> {
        for field in fields {
            if !parse_attrs(&field.attrs)?.skip {
                out.push(field.ty.clone());
            }
        }
        Ok(())
    }

    let mut out = Vec::new();
    match &input.data {
        Data::Struct(s) => push_live(&s.fields, &mut out)?,
        Data::Enum(e) => {
            for variant in &e.variants {
                push_live(&variant.fields, &mut out)?;
            }
        }
        Data::Union(_) => {}
    }
    Ok(out)
}

/// Rejects two variants whose names collide after `rename`. Without this,
/// the second arm of the generated `match` would be unreachable and the
/// second variant could never be decoded.
fn check_variant_names(e: &DataEnum) -> syn::Result<()> {
    let mut seen = BTreeSet::new();
    for variant in &e.variants {
        let name = parse_attrs(&variant.attrs)?
            .rename
            .unwrap_or_else(|| variant.ident.to_string());
        if !seen.insert(name.clone()) {
            return Err(Error::new_spanned(
                &variant.ident,
                format!("duplicate variant name `{name}` (after `rename`)"),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

/// Statements that encode one struct-shaped body. Each one is a self-contained
/// container (it enters and leaves its own depth), so the caller can place it
/// inside an enum variant without extra handling.
fn encode_stmts(body: &Body, refs: &dyn Fn(&Field) -> TokenStream2) -> TokenStream2 {
    let live: Vec<&Field> = body.fields.iter().filter(|f| f.live()).collect();
    let n = live.len();
    match body.shape {
        Shape::Named => {
            let mut sorted = live.clone();
            sorted.sort_by_key(|f| text_key_encoding(&f.key));
            let writes = sorted.iter().map(|f| {
                let key = &f.key;
                let r = refs(f);
                quote! {
                    encoder.encode_str(#key)?;
                    ::tpt_cbor::Encode::encode(#r, encoder)?;
                }
            });
            quote! {
                encoder.enter_container()?;
                encoder.encode_map_header(#n)?;
                #(#writes)*
                encoder.leave_container();
            }
        }
        Shape::Tuple => {
            let writes = live.iter().map(|f| {
                let r = refs(f);
                quote! { ::tpt_cbor::Encode::encode(#r, encoder)?; }
            });
            quote! {
                encoder.enter_container()?;
                encoder.encode_array_header(#n)?;
                #(#writes)*
                encoder.leave_container();
            }
        }
        Shape::Unit => quote! {
            encoder.enter_container()?;
            encoder.encode_array_header(0usize)?;
            encoder.leave_container();
        },
    }
}

fn expand_encode(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let mut generics = input.generics.clone();
    add_type_bounds(
        &mut generics,
        &quote!(::tpt_cbor::Encode),
        &live_field_types(input)?,
    );
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(s) => {
            let body = collect(&s.fields)?;
            let stmts = encode_stmts(&body, &|f: &Field| {
                let m = &f.member;
                quote!(&self.#m)
            });
            quote! {
                #stmts
                Ok(())
            }
        }
        Data::Enum(e) => {
            if e.variants.is_empty() {
                quote!(match *self {})
            } else {
                check_variant_names(e)?;
                let mut arms = Vec::new();
                for v in &e.variants {
                    let attrs = parse_attrs(&v.attrs)?;
                    if attrs.skip || attrs.default {
                        return Err(Error::new_spanned(
                            v,
                            "`skip` and `default` are not supported on variants",
                        ));
                    }
                    let vname = attrs.rename.unwrap_or_else(|| v.ident.to_string());
                    let vident = &v.ident;
                    let body = collect(&v.fields)?;
                    let arm = match body.shape {
                        Shape::Unit => quote! {
                            Self::#vident => encoder.encode_str(#vname),
                        },
                        shape => {
                            let pattern = match shape {
                                Shape::Named => {
                                    let binds = body.fields.iter().filter(|f| f.live()).map(|f| {
                                        let m = &f.member;
                                        let b = &f.binding;
                                        quote!(#m: #b)
                                    });
                                    quote!(Self::#vident { #(#binds,)* .. })
                                }
                                _ => {
                                    let pats = body.fields.iter().map(|f| {
                                        let b = &f.binding;
                                        quote!(#b)
                                    });
                                    quote!(Self::#vident(#(#pats),*))
                                }
                            };
                            let payload = encode_stmts(&body, &|f: &Field| {
                                let b = &f.binding;
                                quote!(#b)
                            });
                            quote! {
                                #pattern => {
                                    encoder.enter_container()?;
                                    encoder.encode_map_header(1usize)?;
                                    encoder.encode_str(#vname)?;
                                    #payload
                                    encoder.leave_container();
                                    Ok(())
                                }
                            }
                        }
                    };
                    arms.push(arm);
                }
                quote!(match self { #(#arms)* })
            }
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                &input.ident,
                "tpt-cbor cannot derive Encode for unions",
            ))
        }
    };

    Ok(quote! {
        impl #impl_generics ::tpt_cbor::Encode for #name #ty_generics #where_clause {
            #[allow(unused_variables)]
            fn encode(
                &self,
                encoder: &mut ::tpt_cbor::Encoder<'_>,
            ) -> ::core::result::Result<(), ::tpt_cbor::EncodeError> {
                #body
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Decode
// ---------------------------------------------------------------------------

/// An expression of type `Result<_, DecodeError>` that decodes one struct-shaped
/// body and builds `ctor` from it.
fn decode_expr(body: &Body, ctor: &TokenStream2) -> TokenStream2 {
    let live: Vec<&Field> = body.fields.iter().filter(|f| f.live()).collect();
    let n = live.len();
    match body.shape {
        Shape::Named => {
            let decls = live.iter().map(|f| {
                let b = &f.binding;
                let ty = &f.ty;
                quote!(let mut #b: ::core::option::Option<#ty> = ::core::option::Option::None;)
            });
            let arms = live.iter().map(|f| {
                let key = &f.key;
                let b = &f.binding;
                quote! {
                    ::core::option::Option::Some(#key) => {
                        if #b.is_some() {
                            return Err(::tpt_cbor::DecodeError::DuplicateKey);
                        }
                        #b = ::core::option::Option::Some(::tpt_cbor::Decode::decode(decoder)?);
                    }
                }
            });
            let build = body.fields.iter().map(|f| {
                let m = &f.member;
                let b = &f.binding;
                let value = if f.skip {
                    quote!(::core::default::Default::default())
                } else if f.default {
                    quote!(#b.unwrap_or_default())
                } else {
                    quote!(#b.ok_or(::tpt_cbor::DecodeError::MissingField)?)
                };
                quote!(#m: #value)
            });
            quote! {
                let __len = decoder.decode_map_len()?;
                decoder.enter_container()?;
                #(#decls)*
                let mut __prev: ::core::option::Option<(usize, usize)> = ::core::option::Option::None;
                for _ in 0..__len {
                    let __start = decoder.position();
                    let __key = if decoder.peek_kind()? == ::tpt_cbor::Kind::Text {
                        ::core::option::Option::Some(decoder.decode_str()?)
                    } else {
                        // Non-text keys are unknown; skip the key, then the value below.
                        decoder.skip_item()?;
                        ::core::option::Option::None
                    };
                    let __span = (__start, decoder.position());
                    decoder.check_key_order(__prev, __span)?;
                    __prev = ::core::option::Option::Some(__span);
                    match __key {
                        #(#arms)*
                        _ => decoder.skip_item()?,
                    }
                }
                decoder.leave_container();
                Ok(#ctor { #(#build),* })
            }
        }
        Shape::Tuple => {
            let binds = live.iter().map(|f| {
                let b = &f.binding;
                quote!(let #b = ::tpt_cbor::Decode::decode(decoder)?;)
            });
            let idents = live.iter().map(|f| {
                let b = &f.binding;
                quote!(#b)
            });
            quote! {
                let __n = decoder.decode_array_len()?;
                if __n != #n {
                    return Err(::tpt_cbor::DecodeError::TypeMismatch);
                }
                decoder.enter_container()?;
                #(#binds)*
                decoder.leave_container();
                Ok(#ctor(#(#idents),*))
            }
        }
        Shape::Unit => quote! {
            if decoder.decode_array_len()? != 0usize {
                return Err(::tpt_cbor::DecodeError::TypeMismatch);
            }
            Ok(#ctor)
        },
    }
}

fn expand_decode(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;

    // Decode lifetime: reuse the type's single lifetime, or introduce `'__de`.
    let lifetimes: Vec<_> = input.generics.lifetimes().collect();
    let mut generics = input.generics.clone();
    let de = match lifetimes.len() {
        0 => {
            generics.params.insert(0, parse_quote!('__de));
            parse_quote!('__de)
        }
        1 => lifetimes[0].lifetime.clone(),
        _ => {
            return Err(Error::new_spanned(
                &input.ident,
                "tpt-cbor supports at most one lifetime parameter on derived types",
            ))
        }
    };
    add_type_bounds(
        &mut generics,
        &quote!(::tpt_cbor::Decode<#de>),
        &live_field_types(input)?,
    );
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(s) => {
            let body = collect(&s.fields)?;
            let expr = decode_expr(&body, &quote!(Self));
            quote!({ #expr })
        }
        Data::Enum(e) => {
            if e.variants.is_empty() {
                quote!(Err(::tpt_cbor::DecodeError::UnsupportedType))
            } else {
                check_variant_names(e)?;
                let mut text_arms = Vec::new();
                let mut map_arms = Vec::new();
                for v in &e.variants {
                    let attrs = parse_attrs(&v.attrs)?;
                    if attrs.skip || attrs.default {
                        return Err(Error::new_spanned(
                            v,
                            "`skip` and `default` are not supported on variants",
                        ));
                    }
                    let vname = attrs.rename.unwrap_or_else(|| v.ident.to_string());
                    let vident = &v.ident;
                    let body = collect(&v.fields)?;
                    let ctor = quote!(Self::#vident);
                    if body.shape == Shape::Unit {
                        text_arms.push(quote!(#vname => Ok(#ctor)));
                    } else {
                        let expr = decode_expr(&body, &ctor);
                        map_arms.push(quote!(#vname => { #expr }));
                    }
                }
                quote! {
                    match decoder.peek_kind()? {
                        ::tpt_cbor::Kind::Text => {
                            let __name = decoder.decode_str()?;
                            match __name {
                                #(#text_arms,)*
                                _ => Err(::tpt_cbor::DecodeError::UnknownVariant),
                            }
                        }
                        ::tpt_cbor::Kind::Map => {
                            let __len = decoder.decode_map_len()?;
                            if __len != 1usize {
                                return Err(::tpt_cbor::DecodeError::TypeMismatch);
                            }
                            decoder.enter_container()?;
                            let __name = decoder.decode_str()?;
                            let __value = match __name {
                                #(#map_arms,)*
                                _ => Err(::tpt_cbor::DecodeError::UnknownVariant),
                            };
                            decoder.leave_container();
                            __value
                        }
                        _ => Err(::tpt_cbor::DecodeError::TypeMismatch),
                    }
                }
            }
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                &input.ident,
                "tpt-cbor cannot derive Decode for unions",
            ))
        }
    };

    Ok(quote! {
        impl #impl_generics ::tpt_cbor::Decode<#de> for #name #ty_generics #where_clause {
            #[allow(unused_mut, unused_variables)]
            fn decode(
                decoder: &mut ::tpt_cbor::Decoder<#de>,
            ) -> ::core::result::Result<Self, ::tpt_cbor::DecodeError> {
                #body
            }
        }
    })
}
