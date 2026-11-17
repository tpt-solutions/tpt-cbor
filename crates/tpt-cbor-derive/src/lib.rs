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
//! * **Transparent struct**: encodes exactly as its single field.
//!
//! Decoding a map ignores unknown keys, which allows forward compatibility,
//! unless the type has `deny_unknown_fields`. Strict decoders still reject
//! out-of-order or duplicate keys.
//!
//! # Container attributes
//!
//! * `#[cbor(rename_all = "...")]` renames the keys of a struct's named fields,
//!   or the names of an enum's variants. The rules are `lowercase`,
//!   `UPPERCASE`, `PascalCase`, `camelCase`, `snake_case`,
//!   `SCREAMING_SNAKE_CASE`, `kebab-case` and `SCREAMING-KEBAB-CASE`. A field
//!   or variant's own `rename` takes precedence. The rule does not reach the
//!   fields of struct variants.
//! * `#[cbor(deny_unknown_fields)]` makes a map with an unknown key fail with
//!   `UnknownField`. Applies to named structs and struct variants.
//! * `#[cbor(transparent)]` on a struct with exactly one field. The struct
//!   encodes and decodes exactly as that field.
//!
//! # Field attributes
//!
//! * `#[cbor(rename = "key")]` on a named field or variant.
//! * `#[cbor(skip)]` on a named field. The field is not encoded, and is
//!   `Default::default()` when decoded.
//! * `#[cbor(default)]` on a named field. A missing key decodes to
//!   `Default::default()`. `Option<T>` fields behave this way automatically.
//! * `#[cbor(skip_serializing_if = "path")]` on a named field. The key is left
//!   out when `path(&field)` returns `true`. `path` is a function such as
//!   `"Option::is_none"`.
//! * `#[cbor(with = "module")]` on a named field. The field is encoded by
//!   `module::encode(&field, encoder)` and decoded by `module::decode(decoder)`.
//!   The field type needs no `Encode` or `Decode` impl.
//! * `#[cbor(bytes)]` on a `Vec<u8>` or `&[u8]` field, so it is a CBOR byte
//!   string rather than an array of integers.
//!
//! # Limitations
//!
//! At most one lifetime parameter is supported. It becomes the decode
//! lifetime. Unions are rejected. Attributes that do not apply to the item they
//! sit on are rejected at compile time.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::BTreeSet;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, Data, DataEnum, DeriveInput, Error, Fields, GenericArgument,
    Generics, Ident, Index, LitStr, Member, PathArguments, Type, Variant,
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
// Attributes
// ---------------------------------------------------------------------------

/// Every `#[cbor(...)]` key, from any position. Placement is checked later.
#[derive(Default)]
struct Attrs {
    rename: Option<String>,
    rename_all: Option<RenameRule>,
    skip: bool,
    default: bool,
    deny_unknown_fields: bool,
    transparent: bool,
    skip_serializing_if: Option<syn::Path>,
    with: Option<syn::Path>,
    bytes: bool,
}

impl Attrs {
    /// `true` when no `#[cbor(...)]` key was given.
    fn is_empty(&self) -> bool {
        self.rename.is_none()
            && self.rename_all.is_none()
            && !self.skip
            && !self.default
            && !self.deny_unknown_fields
            && !self.transparent
            && self.skip_serializing_if.is_none()
            && self.with.is_none()
            && !self.bytes
    }
}

fn parse_attrs(attrs: &[syn::Attribute]) -> syn::Result<Attrs> {
    let mut out = Attrs::default();
    for attr in attrs {
        if !attr.path().is_ident("cbor") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let key = meta
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            match key.as_str() {
                "rename" => {
                    let lit: LitStr = meta.value()?.parse()?;
                    out.rename = Some(lit.value());
                }
                "rename_all" => {
                    let lit: LitStr = meta.value()?.parse()?;
                    out.rename_all = Some(RenameRule::parse(&lit)?);
                }
                "skip" => out.skip = true,
                "default" => out.default = true,
                "deny_unknown_fields" => out.deny_unknown_fields = true,
                "transparent" => out.transparent = true,
                "bytes" => out.bytes = true,
                "skip_serializing_if" => {
                    let lit: LitStr = meta.value()?.parse()?;
                    out.skip_serializing_if = Some(lit.parse()?);
                }
                "with" => {
                    let lit: LitStr = meta.value()?.parse()?;
                    out.with = Some(lit.parse()?);
                }
                _ => {
                    return Err(meta.error(
                        "unknown cbor attribute; expected `rename`, `rename_all`, `skip`, \
                         `default`, `skip_serializing_if`, `with`, `bytes`, \
                         `deny_unknown_fields` or `transparent`",
                    ))
                }
            }
            Ok(())
        })?;
    }
    Ok(out)
}

/// Container attributes: only `rename_all`, `deny_unknown_fields` and
/// `transparent` make sense on a type.
fn container_attrs(input: &DeriveInput) -> syn::Result<Attrs> {
    let attrs = parse_attrs(&input.attrs)?;
    if attrs.rename.is_some()
        || attrs.skip
        || attrs.default
        || attrs.skip_serializing_if.is_some()
        || attrs.with.is_some()
        || attrs.bytes
    {
        return Err(Error::new_spanned(
            &input.ident,
            "`rename`, `skip`, `default`, `skip_serializing_if`, `with` and `bytes` \
             apply to fields or variants, not to a type",
        ));
    }
    Ok(attrs)
}

/// Field attributes, checked against where the field sits.
fn check_field_attrs(attrs: &Attrs, named: bool, field: &syn::Field) -> syn::Result<()> {
    if attrs.rename_all.is_some() || attrs.deny_unknown_fields || attrs.transparent {
        return Err(Error::new_spanned(
            field,
            "`rename_all`, `deny_unknown_fields` and `transparent` apply to a type, not a field",
        ));
    }
    if !named
        && (attrs.rename.is_some()
            || attrs.skip
            || attrs.default
            || attrs.skip_serializing_if.is_some()
            || attrs.with.is_some())
    {
        return Err(Error::new_spanned(
            field,
            "`rename`, `skip`, `default`, `skip_serializing_if` and `with` are only supported \
             on named fields",
        ));
    }
    Ok(())
}

/// Variant attributes: only `rename` applies.
fn check_variant_attrs(attrs: &Attrs, variant: &Variant) -> syn::Result<()> {
    if attrs.skip
        || attrs.default
        || attrs.skip_serializing_if.is_some()
        || attrs.with.is_some()
        || attrs.bytes
        || attrs.rename_all.is_some()
        || attrs.deny_unknown_fields
        || attrs.transparent
    {
        return Err(Error::new_spanned(
            variant,
            "only `rename` is supported on variants",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Renaming
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum RenameRule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

impl RenameRule {
    fn parse(lit: &LitStr) -> syn::Result<Self> {
        Ok(match lit.value().as_str() {
            "lowercase" => Self::Lower,
            "UPPERCASE" => Self::Upper,
            "PascalCase" => Self::Pascal,
            "camelCase" => Self::Camel,
            "snake_case" => Self::Snake,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            "kebab-case" => Self::Kebab,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebab,
            _ => {
                return Err(Error::new_spanned(
                    lit,
                    "unknown rename_all rule; expected one of `lowercase`, `UPPERCASE`, \
                     `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, \
                     `kebab-case` or `SCREAMING-KEBAB-CASE`",
                ))
            }
        })
    }

    fn apply(self, ident: &str) -> String {
        let words = split_words(ident);
        match self {
            Self::Lower => words.concat(),
            Self::Upper => words.concat().to_uppercase(),
            Self::Pascal => words.iter().map(|w| capitalize(w)).collect(),
            Self::Camel => words
                .iter()
                .enumerate()
                .map(|(i, w)| if i == 0 { w.clone() } else { capitalize(w) })
                .collect(),
            Self::Snake => words.join("_"),
            Self::ScreamingSnake => words.join("_").to_uppercase(),
            Self::Kebab => words.join("-"),
            Self::ScreamingKebab => words.join("-").to_uppercase(),
        }
    }
}

/// Splits an identifier into lowercase words. Words are separated by `_`, `-`
/// or an uppercase letter that follows a lowercase one or ends a run of
/// uppercase letters (`HTTPServer` gives `http`, `server`).
fn split_words(ident: &str) -> Vec<String> {
    let chars: Vec<char> = ident.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' || c == '-' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let starts_word = c.is_uppercase()
            && !current.is_empty()
            && (chars
                .get(i.wrapping_sub(1))
                .is_some_and(|p| p.is_lowercase())
                || chars.get(i + 1).is_some_and(|n| n.is_lowercase()));
        if starts_word {
            words.push(std::mem::take(&mut current));
        }
        current.extend(c.to_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The wire name of a variant: its `rename`, else the container rule applied
/// to its identifier, else the identifier itself.
fn variant_name(variant: &Variant, rule: Option<RenameRule>) -> syn::Result<String> {
    let attrs = parse_attrs(&variant.attrs)?;
    check_variant_attrs(&attrs, variant)?;
    Ok(match (attrs.rename, rule) {
        (Some(name), _) => name,
        (None, Some(rule)) => rule.apply(&variant.ident.to_string()),
        (None, None) => variant.ident.to_string(),
    })
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum BytesKind {
    /// `Vec<u8>`: decoded into an owned copy.
    Owned,
    /// `&[u8]`: borrowed from the input.
    Borrowed,
}

struct Field {
    member: Member,
    /// Text key for named fields.
    key: String,
    ty: Type,
    skip: bool,
    default: bool,
    skip_if: Option<syn::Path>,
    with: Option<syn::Path>,
    bytes: Option<BytesKind>,
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

fn is_ident_type(ty: &Type, name: &str) -> bool {
    matches!(ty, Type::Path(p) if p.path.is_ident(name))
}

/// Classifies a `bytes` field. Only `Vec<u8>` and `&[u8]` are byte strings.
fn bytes_kind(ty: &Type) -> Option<BytesKind> {
    match ty {
        Type::Reference(r) => match &*r.elem {
            Type::Slice(s) if is_ident_type(&s.elem, "u8") => Some(BytesKind::Borrowed),
            _ => None,
        },
        Type::Path(p) => {
            let seg = p.path.segments.last()?;
            if seg.ident != "Vec" {
                return None;
            }
            match &seg.arguments {
                PathArguments::AngleBracketed(args) => match args.args.first()? {
                    GenericArgument::Type(t) if is_ident_type(t, "u8") => Some(BytesKind::Owned),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Builds the field model for a struct or variant body. `rule` renames named
/// fields that have no explicit `rename`.
fn collect(fields: &Fields, rule: Option<RenameRule>) -> syn::Result<Body> {
    let (shape, list): (Shape, Vec<&syn::Field>) = match fields {
        Fields::Named(f) => (Shape::Named, f.named.iter().collect()),
        Fields::Unnamed(f) => (Shape::Tuple, f.unnamed.iter().collect()),
        Fields::Unit => (Shape::Unit, Vec::new()),
    };

    let mut out = Vec::with_capacity(list.len());
    for (i, field) in list.into_iter().enumerate() {
        let attrs = parse_attrs(&field.attrs)?;
        check_field_attrs(&attrs, shape == Shape::Named, field)?;
        let bytes = if attrs.bytes {
            Some(bytes_kind(&field.ty).ok_or_else(|| {
                Error::new_spanned(
                    &field.ty,
                    "`bytes` is supported only on `Vec<u8>` and `&[u8]` fields",
                )
            })?)
        } else {
            None
        };
        let (member, key) = match &field.ident {
            Some(ident) => {
                let key = match (&attrs.rename, rule) {
                    (Some(name), _) => name.clone(),
                    (None, Some(rule)) => rule.apply(&ident.to_string()),
                    (None, None) => ident.to_string(),
                };
                (Member::Named(ident.clone()), key)
            }
            None => (Member::Unnamed(Index::from(i)), String::new()),
        };
        out.push(Field {
            member,
            key,
            ty: field.ty.clone(),
            skip: attrs.skip,
            default: attrs.default || is_option_type(&field.ty),
            skip_if: attrs.skip_serializing_if,
            with: attrs.with,
            bytes,
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
/// Parameters used only by skipped or `with` fields get no bound, because the
/// derived code does not require a codec impl for them.
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

/// Types of the fields that the derived code encodes or decodes through their
/// own `Encode`/`Decode` impls. Skipped and `with` fields are excluded, as are
/// fields whose codec the derive does not call.
fn codec_field_types(input: &DeriveInput) -> syn::Result<Vec<Type>> {
    fn push_codec(fields: &Fields, out: &mut Vec<Type>) -> syn::Result<()> {
        for field in fields {
            let attrs = parse_attrs(&field.attrs)?;
            if !attrs.skip && attrs.with.is_none() && !attrs.bytes {
                out.push(field.ty.clone());
            }
        }
        Ok(())
    }

    let mut out = Vec::new();
    match &input.data {
        Data::Struct(s) => push_codec(&s.fields, &mut out)?,
        Data::Enum(e) => {
            for variant in &e.variants {
                push_codec(&variant.fields, &mut out)?;
            }
        }
        Data::Union(_) => {}
    }
    Ok(out)
}

/// Rejects two variants whose wire names collide. Without this, the second
/// arm of the generated `match` would be unreachable and that variant could
/// never be decoded.
fn check_variant_names(e: &DataEnum, rule: Option<RenameRule>) -> syn::Result<()> {
    let mut seen = BTreeSet::new();
    for variant in &e.variants {
        let name = variant_name(variant, rule)?;
        if !seen.insert(name.clone()) {
            return Err(Error::new_spanned(
                &variant.ident,
                format!("duplicate variant name `{name}` (after `rename` and `rename_all`)"),
            ));
        }
    }
    Ok(())
}

/// The one field of a `transparent` struct. Field attributes are not allowed.
fn transparent_field(fields: &Fields) -> syn::Result<(Member, Shape)> {
    let body = collect(fields, None)?;
    if body.fields.len() != 1 {
        return Err(Error::new(
            Span::call_site(),
            "`transparent` needs a struct with exactly one field",
        ));
    }
    let field = body
        .fields
        .first()
        .ok_or_else(|| Error::new(Span::call_site(), "`transparent` needs one field"))?;
    let attrs = match fields {
        Fields::Named(f) => f.named.first().map(|f| parse_attrs(&f.attrs)),
        Fields::Unnamed(f) => f.unnamed.first().map(|f| parse_attrs(&f.attrs)),
        Fields::Unit => None,
    };
    if let Some(attrs) = attrs {
        if !attrs?.is_empty() {
            return Err(Error::new(
                Span::call_site(),
                "a `transparent` struct's field cannot have `cbor` attributes",
            ));
        }
    }
    Ok((field.member.clone(), body.shape))
}

// ---------------------------------------------------------------------------
// Field codecs
// ---------------------------------------------------------------------------

/// Statement that encodes the field value `r` (a reference) for `f`.
fn encode_value(f: &Field, r: &TokenStream2) -> TokenStream2 {
    if let Some(path) = &f.with {
        quote!(#path::encode(#r, encoder)?;)
    } else if f.bytes.is_some() {
        quote!(encoder.encode_bytes(#r)?;)
    } else {
        quote!(::tpt_cbor::Encode::encode(#r, encoder)?;)
    }
}

/// Expression of type `Result<_, DecodeError>` that decodes the field `f`.
fn decode_value(f: &Field) -> TokenStream2 {
    if let Some(path) = &f.with {
        quote!(#path::decode(decoder))
    } else {
        match f.bytes {
            Some(BytesKind::Owned) => quote!(decoder.decode_bytes().map(|b| b.to_vec())),
            Some(BytesKind::Borrowed) => quote!(decoder.decode_bytes()),
            None => quote!(::tpt_cbor::Decode::decode(decoder)),
        }
    }
}

/// Local flag that records whether a `skip_serializing_if` field is left out.
fn skip_flag(f: &Field) -> Ident {
    format_ident!("{}_skip", f.binding)
}

// ---------------------------------------------------------------------------
// Encode
// ---------------------------------------------------------------------------

/// Statements that encode one struct-shaped body. Each one is a self-contained
/// container (it enters and leaves its own depth), so the caller can place it
/// inside an enum variant without extra handling.
fn encode_stmts(body: &Body, refs: &dyn Fn(&Field) -> TokenStream2) -> TokenStream2 {
    let live: Vec<&Field> = body.fields.iter().filter(|f| f.live()).collect();
    match body.shape {
        Shape::Named => {
            let mut sorted = live.clone();
            sorted.sort_by_key(|f| text_key_encoding(&f.key));

            // Fields with `skip_serializing_if` are tested once, before the
            // header, so the header count matches the entries written.
            let conditional: Vec<&Field> = sorted
                .iter()
                .copied()
                .filter(|f| f.skip_if.is_some())
                .collect();
            let decls = conditional.iter().filter_map(|f| {
                let path = f.skip_if.as_ref()?;
                let flag = skip_flag(f);
                let r = refs(f);
                Some(quote!(let #flag: bool = #path(#r);))
            });
            let fixed = sorted.len() - conditional.len();
            let counts = conditional.iter().map(|f| {
                let flag = skip_flag(f);
                quote!(+ if #flag { 0usize } else { 1usize })
            });

            let writes = sorted.iter().map(|f| {
                let key = &f.key;
                let r = refs(f);
                let value = encode_value(f, &r);
                let entry = quote! {
                    encoder.encode_str(#key)?;
                    #value
                };
                if f.skip_if.is_some() {
                    let flag = skip_flag(f);
                    quote!(if !#flag { #entry })
                } else {
                    entry
                }
            });
            quote! {
                #(#decls)*
                let __count: usize = #fixed #(#counts)*;
                encoder.enter_container()?;
                encoder.encode_map_header(__count)?;
                #(#writes)*
                encoder.leave_container();
            }
        }
        Shape::Tuple => {
            let writes = live.iter().map(|f| {
                let r = refs(f);
                encode_value(f, &r)
            });
            let n = live.len();
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
    let container = container_attrs(input)?;
    let mut generics = input.generics.clone();
    add_type_bounds(
        &mut generics,
        &quote!(::tpt_cbor::Encode),
        &codec_field_types(input)?,
    );
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(s) if container.transparent => {
            let (member, _) = transparent_field(&s.fields)?;
            quote!(::tpt_cbor::Encode::encode(&self.#member, encoder))
        }
        Data::Struct(s) => {
            let body = collect(&s.fields, container.rename_all)?;
            let stmts = encode_stmts(&body, &|f: &Field| {
                let m = &f.member;
                quote!(&self.#m)
            });
            quote! {
                #stmts
                Ok(())
            }
        }
        Data::Enum(_) if container.transparent => {
            return Err(Error::new_spanned(
                &input.ident,
                "`transparent` applies to structs, not enums",
            ))
        }
        Data::Enum(e) => {
            if e.variants.is_empty() {
                quote!(match *self {})
            } else {
                check_variant_names(e, container.rename_all)?;
                let mut arms = Vec::new();
                for v in &e.variants {
                    let vname = variant_name(v, container.rename_all)?;
                    let vident = &v.ident;
                    let body = collect(&v.fields, None)?;
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
/// body and builds `ctor` from it. `deny` makes unknown map keys an error.
fn decode_expr(body: &Body, ctor: &TokenStream2, deny: bool) -> TokenStream2 {
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
                let value = decode_value(f);
                quote! {
                    ::core::option::Option::Some(#key) => {
                        if #b.is_some() {
                            return Err(::tpt_cbor::DecodeError::DuplicateKey);
                        }
                        #b = ::core::option::Option::Some(#value?);
                    }
                }
            });
            let unknown = if deny {
                quote!(_ => return Err(::tpt_cbor::DecodeError::UnknownField),)
            } else {
                quote!(_ => decoder.skip_item()?,)
            };
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
                        #unknown
                    }
                }
                decoder.leave_container();
                Ok(#ctor { #(#build),* })
            }
        }
        Shape::Tuple => {
            let binds = live.iter().map(|f| {
                let b = &f.binding;
                let value = decode_value(f);
                quote!(let #b = #value?;)
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
    let container = container_attrs(input)?;

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
        &codec_field_types(input)?,
    );
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(s) if container.transparent => {
            let (member, shape) = transparent_field(&s.fields)?;
            match shape {
                Shape::Named => quote!({
                    ::core::result::Result::Ok(Self {
                        #member: ::tpt_cbor::Decode::decode(decoder)?,
                    })
                }),
                _ => quote!({
                    ::core::result::Result::Ok(Self(::tpt_cbor::Decode::decode(decoder)?))
                }),
            }
        }
        Data::Struct(s) => {
            let body = collect(&s.fields, container.rename_all)?;
            let expr = decode_expr(&body, &quote!(Self), container.deny_unknown_fields);
            quote!({ #expr })
        }
        Data::Enum(_) if container.transparent => {
            return Err(Error::new_spanned(
                &input.ident,
                "`transparent` applies to structs, not enums",
            ))
        }
        Data::Enum(e) => {
            if e.variants.is_empty() {
                quote!(Err(::tpt_cbor::DecodeError::UnsupportedType))
            } else {
                check_variant_names(e, container.rename_all)?;
                let mut text_arms = Vec::new();
                let mut map_arms = Vec::new();
                for v in &e.variants {
                    let vname = variant_name(v, container.rename_all)?;
                    let vident = &v.ident;
                    let body = collect(&v.fields, None)?;
                    let ctor = quote!(Self::#vident);
                    if body.shape == Shape::Unit {
                        text_arms.push(quote!(#vname => Ok(#ctor)));
                    } else {
                        let expr = decode_expr(&body, &ctor, container.deny_unknown_fields);
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
