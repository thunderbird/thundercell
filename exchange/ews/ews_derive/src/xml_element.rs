use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, DataEnum, DataStruct, Expr, Ident, Meta, Token,
};

const MACRO_ATTRIBUTE: &str = "xml_serialize";

/// Generates an implementation of `XmlElement` for a struct and its fields.
///
/// The struct is serialized as an element with the same name as the type, with
/// fields serialized either as attributes on the element, with the field name
/// transformed to PascalCase as attribute name, or as child elements named
/// based on their type as appropriate.
pub(super) fn write_element_derivation_for_struct(
    ident: Ident,
    data: DataStruct,
    options: TypeOptions,
) -> proc_macro::TokenStream {
    let fields: Vec<_> = match data.fields {
        syn::Fields::Named(fields) => fields
            .named
            .into_iter()
            .map(|field| {
                let call_accessor = {
                    let ident = field.ident.clone().unwrap();
                    quote!(self.#ident)
                };

                let verify_accessor = quote!(&#call_accessor);

                Ok(Field {
                    ident: field.ident,
                    verify_accessor,
                    call_accessor,
                    options: FieldOptions::try_from(field.attrs)?,
                })
            })
            .collect::<Result<Vec<_>, &str>>()
            .expect("msg"),

        syn::Fields::Unnamed(fields) => fields
            .unnamed
            .into_iter()
            .enumerate()
            .map(|(index, field)| {
                let call_accessor = {
                    let positional = Literal::usize_unsuffixed(index);
                    quote!(self.#positional)
                };
                let verify_accessor = quote!(&#call_accessor);

                let options = FieldOptions::try_from(field.attrs)?;
                if options.is_attribute {
                    // Field names are used as the name of the attribute to
                    // write, so there is no clear way to derive an attribute
                    // name from an unnamed field.
                    panic!("Unnamed fields may not be XML attributes");
                }

                Ok(Field {
                    ident: field.ident,
                    verify_accessor,
                    call_accessor,
                    options,
                })
            })
            .collect::<Result<Vec<_>, &str>>()
            .expect("msg"),

        syn::Fields::Unit => Default::default(),
    };

    let element_name_decl = build_element_name_declaration(&ident, &options.ns_prefix);
    let xmlns_calls = build_calls_for_namespaces(options.namespaces);
    let (verify_calls, (attribute_calls, element_calls)) = build_calls_for_fields(fields);

    quote!(
        // Ensure that the `XmlAttribute` trait is in scope so that consumers
        // don't need to worry about it. It's fine for this to show up multiple
        // times in one file.
        use crate::xml::XmlAttribute as _;

        #[automatically_derived]
        impl crate::xml::XmlElement for #ident {
            fn write_as_element<W: std::io::Write>(
                &self,
                writer: &mut xml::EventWriter<W>,
            ) -> Result<(), xml::writer::Error> {
                #element_name_decl

                // These calls are no-ops solely for good compiler errors, so
                // they must occur before anything else which depends on fields
                // implementing our traits.
                #(#verify_calls)*

                let builder = xml::writer::events::XmlEvent::start_element(ELEMENT_NAME);

                // Namespaces and attributes are calls on the `StartElement`
                // builder and so need to happen before passing to the writer.
                #(#xmlns_calls)*
                #(#attribute_calls)*

                writer.write(builder)?;

                // Element fields are written as children of the struct element.
                #(#element_calls)*

                writer.write(xml::writer::events::XmlEvent::end_element())
            }
        }
    )
    .into()
}

/// Generates an implementation of `XmlElement` for an enum and its fields.
///
/// See [`write_element_derivation_for_unit_enum`] and
/// [`write_element_derivation_for_structured_enum`] for details on
/// serialization.
pub(super) fn write_element_derivation_for_enum(
    ident: Ident,
    data: DataEnum,
    enum_options: TypeOptions,
) -> proc_macro::TokenStream {
    assert!(
        !data.variants.is_empty(),
        "Deriving `XmlElement` is not supported for zero-variant enums"
    );

    // We treat enums with unit variants differently from those with non-unit
    // variants, and do not support enums with both.
    match data.variants[0].fields {
        syn::Fields::Named(_) | syn::Fields::Unnamed(_) => {
            write_element_derivation_for_structured_enum(ident, data, enum_options)
        }

        syn::Fields::Unit => write_element_derivation_for_unit_enum(ident, data, enum_options),
    }
}

/// Generates an implementation of `XmlElement` for an enum with unit variants.
///
/// The enum is serialized as an element with the same name as the type, with
/// the variant name written as child [PCDATA].
///
/// [PCDATA]: https://en.wikipedia.org/wiki/PCDATA
fn write_element_derivation_for_unit_enum(
    ident: Ident,
    data: DataEnum,
    options: TypeOptions,
) -> proc_macro::TokenStream {
    let variant_arms: Vec<TokenStream> = data
        .variants
        .into_iter()
        .map(|variant| {
            match variant.fields {
                syn::Fields::Unit => (),

                _ => panic!("Mixing unit and non-unit variants in an enum is not supported"),
            }

            let variant_name = variant.ident;
            let as_string = variant_name.to_string();

            quote!(
                Self::#variant_name => #as_string
            )
        })
        .collect();

    let element_name_decl = build_element_name_declaration(&ident, &options.ns_prefix);
    let xmlns_calls = build_calls_for_namespaces(options.namespaces);

    quote!(
        // Ensure that the `XmlAttribute` trait is in scope so that consumers
        // don't need to worry about it. It's fine for this to show up multiple
        // times in one file.
        use crate::xml::XmlAttribute as _;

        #[automatically_derived]
        impl crate::xml::XmlElement for #ident {
            fn write_as_element<W: std::io::Write>(
                &self,
                writer: &mut xml::EventWriter<W>,
            ) -> Result<(), xml::writer::Error> {
                #element_name_decl

                let builder = xml::writer::events::XmlEvent::start_element(ELEMENT_NAME);
                #(#xmlns_calls)*
                writer.write(builder)?;

                let characters = match self {
                    #(#variant_arms),*
                };
                writer.write(xml::writer::events::XmlEvent::characters(characters))?;

                writer.write(xml::writer::events::XmlEvent::end_element())
            }
        }
    )
    .into()
}

/// Generates an implementation of `XmlElement` for an enum with structured
/// (non-unit) variants.
///
/// Variants with named fields are serialized as an element with the same name
/// as the type, with fields serialized as though they were the fields of a
/// struct.
///
/// Variants with unnamed fields are serialized with each field serialized as an
/// element with a name derived from its type. No containing element is
/// serialized.
///
/// In both cases, the variant name does not affect the serialized output.
fn write_element_derivation_for_structured_enum(
    ident: Ident,
    data: DataEnum,
    options: TypeOptions,
) -> proc_macro::TokenStream {
    let xmlns_calls = build_calls_for_namespaces(options.namespaces);

    let variant_arms: TokenStream = data
        .variants
        .into_iter()
        .map(|variant| {
            // Because each variant has its own internal structure, each variant
            // has a separate implementation of serialization.
            let ident = variant.ident;
            match variant.fields {
                syn::Fields::Named(fields) => {
                    let fields = fields
                        .named
                        .into_iter()
                        .map(|field| {
                            let accessor = {
                                let ident = field.ident.clone().unwrap();
                                quote!(#ident)
                            };

                            Ok(Field {
                                ident: field.ident,
                                verify_accessor: accessor.clone(),
                                call_accessor: accessor,
                                options: FieldOptions::try_from(field.attrs)?,
                            })
                        })
                        .collect::<Result<Vec<_>, &str>>()
                        .expect("Unable to to process enum variant field");

                    let pattern = {
                        let accessors = fields.iter().map(|field| &field.call_accessor);

                        quote!(Self::#ident { #(#accessors),* })
                    };

                    let element_name_decl = build_element_name_declaration(&ident, &options.ns_prefix);
                    let (verify_calls, (attribute_calls, element_calls)) = build_calls_for_fields(fields);

                    quote!(
                        #pattern => {
                            #element_name_decl

                            #(#verify_calls)*

                            let builder = xml::writer::events::XmlEvent::start_element(ELEMENT_NAME);
                            #(#xmlns_calls)*
                            #(#attribute_calls)*
                            writer.write(builder)?;

                            #(#element_calls)*

                            writer.write(xml::writer::events::XmlEvent::end_element())
                        }
                    )
                }

                syn::Fields::Unnamed(fields) => {
                    if !xmlns_calls.is_empty() || options.ns_prefix.is_some() {
                        panic!("Namespace properties may not be applied to enums with variants containing unnamed fields");
                    }

                    let fields = fields
                        .unnamed
                        .into_iter()
                        .enumerate()
                        .map(|(index, field)| {
                            let accessor = {
                                let accessor = format_ident!("field{index}");
                                quote!(#accessor)
                            };

                            let options = FieldOptions::try_from(field.attrs)?;
                            if options.is_attribute {
                                panic!("Unnamed fields may not be XML attributes");
                            }

                            Ok(Field {
                                ident: field.ident,
                                verify_accessor: accessor.clone(),
                                call_accessor: accessor,
                                options,
                            })
                        })
                        .collect::<Result<Vec<_>, &str>>()
                        .expect("Unable to to process enum variant field");

                    let pattern = {
                        let idents = fields.iter().map(|field| &field.call_accessor);

                        quote!(Self::#ident(#(#idents),*))
                    };

                    let (verify_calls, (_, element_calls)) = build_calls_for_fields(fields);

                    quote!(
                        #pattern => {
                            #(#verify_calls)*

                            #(#element_calls)*

                            Ok(())
                        }
                    )
                }

                syn::Fields::Unit => panic!("Mixing unit and non-unit variants in an enum is not supported"),
            }
        })
        .collect();

    quote!(
        // Ensure that the `XmlAttribute` trait is in scope so that consumers
        // don't need to worry about it. It's fine for this to show up multiple
        // times in one file.
        use crate::xml::XmlAttribute as _;

        #[automatically_derived]
        impl crate::xml::XmlElement for #ident {
            fn write_as_element<W: std::io::Write>(
                &self,
                writer: &mut xml::EventWriter<W>,
            ) -> Result<(), xml::writer::Error> {
                match self {
                    #variant_arms
                }
            }
        }
    )
    .into()
}

/// Builds the element name to serialize as a `const` `&str`.
///
/// The namespace prefix, if any, is prepended to the element name at
/// compile-time.
fn build_element_name_declaration(ident: &Ident, prefix: &Option<TokenStream>) -> TokenStream {
    let ident = ident.to_string();
    match prefix {
        Some(prefix) => {
            let ident = format!(":{ident}");

            // We use some relatively complex `const` machinery here to allow
            // for specifying the prefix as a `const` `&str` variable in
            // addition to allowing for string literals. This function is
            // copied into each derived serialize function to avoid collision,
            // but the function is executed at compile-time and deleted from the
            // final executable.
            quote!(
                const LEN: usize = #prefix.len() + #ident.len();

                const fn copy_bytes_into(input: &[u8], mut output: [u8; LEN], offset: usize) -> [u8; LEN] {
                    // Copy the input byte-by-byte into the output buffer at the
                    // specified offset.
                    let mut index = 0;
                    loop {
                        output[offset + index] = input[index];
                        index += 1;
                        if index == input.len() {
                            break;
                        }
                    }

                    // We must return the buffer, as `const` functions cannot
                    // take a mutable reference, so it's passed to us by value.
                    output
                }

                const fn dirty_concat(prefix: &'static str, value: &'static str) -> [u8; LEN] {
                    let mut output = [0u8; LEN];
                    output = copy_bytes_into(prefix.as_bytes(), output, 0);
                    output = copy_bytes_into(value.as_bytes(), output, prefix.len());

                    output
                }

                // As of writing this comment, Rust does not provide a standard
                // macro for compile-time string concatenation, so we exploit
                // the fact that `str::as_bytes()` and `std::str::from_utf8()`
                // are `const` and simply copy the bytes for each string into a
                // shared buffer and
                const BYTES: [u8; LEN] = dirty_concat(#prefix, #ident);
                const ELEMENT_NAME: &'static str = match std::str::from_utf8(&BYTES) {
                    Ok(value) => &value,

                    // Given that both the prefix and ident strings are stored
                    // as Rust strings, they should both be valid UTF-8 and
                    // therefore their concatenation should have no way of being
                    // invalid. If that occurs, it's likely a bug in one of the
                    // above functions.
                    Err(_) => panic!("Unable to create element name string"),
                };
            )
        }
        None => quote!(
            const ELEMENT_NAME: &'static str = #ident;
        ),
    }
}

/// Builds lists of calls for each field in a struct or enum.
///
/// Each field has two function calls in the final expanded macro: one to a
/// no-op function which serves only to provide useful compiler messages in the
/// case that the field's type does not implement the appropriate trait, and one
/// to serialize the field.
///
/// Because serialization calls must happen at different times for attributes
/// and elements, serialization calls are split into two lists.
fn build_calls_for_fields(
    fields: Vec<Field>,
) -> (Vec<TokenStream>, (Vec<TokenStream>, Vec<TokenStream>)) {
    let (verify_calls, (attribute_calls, element_calls)): (
        Vec<TokenStream>,
        (Vec<TokenStream>, Vec<TokenStream>),
    ) = fields
        .into_iter()
        .map(|field| {
            let verify_accessor = field.verify_accessor;
            let call_accessor = field.call_accessor;

            match field.options.is_attribute {
                true => {
                    let ident = field.ident.unwrap();
                    let attr_name = ident_to_pascal_case_string(ident);

                    (
                        quote!(crate::xml::verify_attribute_field(#verify_accessor);),
                        Either::Left(quote!(
                            let builder = #call_accessor.write_as_attribute(builder, #attr_name);
                        )),
                    )
                }
                false => (
                    quote!(crate::xml::verify_element_field(#verify_accessor);),
                    Either::Right(quote!(
                        #call_accessor.write_as_element(writer)?;
                    )),
                ),
            }
        })
        .unzip();

    (verify_calls, (attribute_calls, element_calls))
}

/// `TypeOptions` is a collection of values which affect the serialization of an
/// XML element tag.
#[derive(Default)]
pub(super) struct TypeOptions {
    /// The namespace prefix to apply to the serialized XML element tag name.
    ns_prefix: Option<TokenStream>,

    /// The list of namespaces to be declared on the serialized XML element
    /// corresponding to this type.
    namespaces: Vec<XmlNamespace>,
}

impl TryFrom<Vec<Attribute>> for TypeOptions {
    type Error = &'static str;

    fn try_from(value: Vec<Attribute>) -> Result<Self, Self::Error> {
        let meta = try_get_type_meta(value)?;

        let mut ns_prefix = None;

        let mut has_set_default = false;
        let namespaces = meta
            .into_iter()
            .filter_map(|meta| match meta {
                Meta::NameValue(name_value) => {
                    if name_value.path.is_ident("default_ns") {
                        // The value of `default_ns` must be a single string,
                        // representing a namespace URI. There can be at most a
                        // single `default_ns` per type.
                        if has_set_default {
                            return Some(Err(
                                "there must be at most one `default_ns` declaration per type",
                            ));
                        }

                        has_set_default = true;

                        Some(Ok(XmlNamespace::Default(
                            name_value.value.into_token_stream(),
                        )))
                    } else if name_value.path.is_ident("ns") {
                        // The value of `ns` must be a tuple of strings,
                        // representing a namespace prefix and associated URI.
                        // There can be many `ns` attributes per type.
                        match name_value.value {
                            Expr::Tuple(tuple) if tuple.elems.len() == 2 => {
                                let mut elems = tuple.elems.into_iter();
                                Some(Ok(XmlNamespace::Prefixed(
                                    elems.next().unwrap().into_token_stream(),
                                    elems.next().unwrap().into_token_stream(),
                                )))
                            }

                            _ => Some(Err("`ns` takes a single tuple of two elements as argument")),
                        }
                    } else if name_value.path.is_ident("ns_prefix") {
                        // The value of `ns_prefix` must be a single string,
                        // representing a namespace prefix to apply to the
                        // element name. There can be at most a single
                        // `ns_prefix` per type.
                        if ns_prefix.is_some() {
                            return Some(Err(
                                "there must be at most one `ns_prefix` declaration per type",
                            ));
                        }

                        ns_prefix = Some(name_value.value.into_token_stream());

                        None
                    } else {
                        Some(Err("unrecognized attribute for type"))
                    }
                }

                _ => Some(Err("unrecognized attribute for type")),
            })
            .collect::<Result<_, _>>()?;

        Ok(TypeOptions {
            ns_prefix,
            namespaces,
        })
    }
}

/// `XmlNamespace` contains details necessary for generating XML namespace
/// declaration calls in the serializer function.
enum XmlNamespace {
    Default(TokenStream),
    Prefixed(TokenStream, TokenStream),
}

/// Builds a list of calls to `xml-rs` namespace declaration functions.
fn build_calls_for_namespaces(namespaces: Vec<XmlNamespace>) -> Vec<TokenStream> {
    namespaces
        .into_iter()
        .map(|xmlns| match xmlns {
            XmlNamespace::Default(uri) => quote!(let builder = builder.default_ns(#uri);),
            XmlNamespace::Prefixed(prefix, uri) => {
                quote!(let builder = builder.ns(#prefix, #uri);)
            }
        })
        .collect()
}

/// `Field` is a collection of values necessary for building calls to serialize
/// a struct or enum field.
struct Field {
    /// The name of the field, if any.
    ident: Option<Ident>,

    /// The tokens necessary to pass the field's value to a verifier call as a
    /// reference.
    verify_accessor: TokenStream,

    /// The tokens necessary to create a call to the field's serializer method.
    call_accessor: TokenStream,

    /// Options affecting the field's serialization.
    options: FieldOptions,
}

/// `FieldOptions` encapsulates options specified by Rust attributes applied to
/// a struct or enum field.
struct FieldOptions {
    /// `true` if the field should be serialized as an attribute instead of as
    /// an element.
    is_attribute: bool,
}

impl TryFrom<Vec<Attribute>> for FieldOptions {
    type Error = &'static str;

    fn try_from(value: Vec<Attribute>) -> Result<Self, Self::Error> {
        let meta = try_get_type_meta(value)?;

        let is_xml_attribute = meta.into_iter().try_fold(false, |value, meta| match meta {
            // At present, the only option for a single field is to serialize it
            // as an XML attribute instead of an XML element.
            Meta::Path(path) => Ok(value || path.is_ident("attribute")),

            _ => Err("unrecognized XML field attribute"),
        })?;

        Ok(Self {
            is_attribute: is_xml_attribute,
        })
    }
}

/// Converts a standard snake_case identifier into a PascalCase string.
///
/// This function may fail if used on non-ASCII identifiers.
fn ident_to_pascal_case_string(ident: Ident) -> String {
    let mut capitalize_next = true;
    ident
        .to_string()
        .chars()
        .filter_map(|character| {
            if character == '_' {
                // Consume the underscore and capitalize the next
                capitalize_next = true;

                None
            } else if capitalize_next {
                capitalize_next = false;

                // Rust supports non-ASCII identifiers, so this could
                // technically fail, but this macro is not expected to handle
                // the general XML case, and so supporting full case mapping is
                // out of scope.
                Some(character.to_ascii_uppercase())
            } else {
                Some(character)
            }
        })
        .collect()
}

/// `Either` is a convenience enum for splitting a single iterator into two
/// collections.
///
/// Although more sophisticated versions of this enum exist in the ecosystem, we
/// really only need a very simple version.
enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R, ExtendL, ExtendR> Extend<Either<L, R>> for (ExtendL, ExtendR)
where
    ExtendL: Extend<L>,
    ExtendR: Extend<R>,
{
    fn extend<T: IntoIterator<Item = Either<L, R>>>(&mut self, iter: T) {
        for value in iter {
            // Using `Extend` with a single-element iterator is probably not the
            // most efficient way to accomplish this. `extend_one()` exists in
            // unstable. We could also just do this explicitly for `Vec`. It's
            // unlikely to be a build performance issue, though, given the
            // quantity of fields we're dealing with.
            match value {
                Either::Left(l) => self.0.extend(std::iter::once(l)),
                Either::Right(r) => self.1.extend(std::iter::once(r)),
            }
        }
    }
}

/// Parses the macro's helper attribute, if any, into `syn` structures.
fn try_get_type_meta(attrs: Vec<Attribute>) -> Result<Punctuated<Meta, Comma>, &'static str> {
    let mut applicable_attrs = attrs.into_iter().filter_map(|attr| {
        if attr.path().is_ident(MACRO_ATTRIBUTE) {
            Some(attr)
        } else {
            None
        }
    });

    let attr_to_parse = match applicable_attrs.next() {
        Some(attr) => attr,

        // No applicable attributes, nothing to do.
        None => return Ok(Default::default()),
    };

    if applicable_attrs.next().is_some() {
        return Err("multiple applicable attributes specified for component");
    }

    attr_to_parse
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .map_err(|_| "illegal attribute syntax")
}
