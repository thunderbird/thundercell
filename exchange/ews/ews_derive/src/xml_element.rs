use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, DataEnum, DataStruct,
    Expr, Ident, Meta, Token,
};

const MACRO_ATTRIBUTE: &str = "xml_serialize";

pub(super) fn write_element_derivation_for_struct(
    ident: Ident,
    data: DataStruct,
    options: ComponentOptions,
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

    let (verify_calls, (attribute_calls, element_calls)) = fields_to_calls(fields);

    let xmlns_calls: TokenStream = namespaces_to_calls(options.namespaces);

    let element_name = get_component_name(&ident, &options.prefix);
    quote!(
        #[automatically_derived]
        impl crate::xml::XmlElement for #ident {
            fn write_as_element<W: std::io::Write>(
                &self,
                writer: &mut xml::EventWriter<W>,
            ) -> Result<(), xml::writer::Error> {
                #verify_calls

                let builder = xml::writer::events::XmlEvent::start_element(#element_name);
                #xmlns_calls
                #(#attribute_calls)*
                writer.write(builder)?;

                #(#element_calls)*

                writer.write(xml::writer::events::XmlEvent::end_element())
            }
        }
    )
    .into()
}

pub(super) fn write_element_derivation_for_enum(
    ident: Ident,
    data: DataEnum,
    enum_options: ComponentOptions,
) -> proc_macro::TokenStream {
    let is_unit_enum = matches!(data.variants[0].fields, syn::Fields::Unit);
    if is_unit_enum {
        write_element_derivation_for_unit_enum(ident, data, enum_options)
    } else {
        write_element_derivation_for_structured_enum(ident, data, enum_options)
    }
}

fn write_element_derivation_for_unit_enum(
    ident: Ident,
    data: DataEnum,
    options: ComponentOptions,
) -> proc_macro::TokenStream {
    let variant_arms: TokenStream = data
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
                Self::#variant_name => #as_string,
            )
        })
        .collect();

    let xmlns_calls = namespaces_to_calls(options.namespaces);

    let element_name = get_component_name(&ident, &options.prefix);
    quote!(
        #[automatically_derived]
        impl crate::xml::XmlElement for #ident {
            fn write_as_element<W: std::io::Write>(
                &self,
                writer: &mut xml::EventWriter<W>,
            ) -> Result<(), xml::writer::Error> {
                let builder = xml::writer::events::XmlEvent::start_element(#element_name);
                #xmlns_calls
                writer.write(builder)?;

                let characters = match self {
                    #variant_arms
                };
                writer.write(xml::writer::events::XmlEvent::characters(characters))?;

                writer.write(xml::writer::events::XmlEvent::end_element())
            }
        }
    )
    .into()
}

fn write_element_derivation_for_structured_enum(
    ident: Ident,
    data: DataEnum,
    options: ComponentOptions,
) -> proc_macro::TokenStream {
    let xmlns_calls: TokenStream = namespaces_to_calls(options.namespaces);

    let variant_arms: TokenStream = data
        .variants
        .into_iter()
        .map(|variant| {
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

                    let matcher = {
                        let accessors = fields.iter().map(|field| &field.call_accessor);

                        quote!(Self::#ident { #(#accessors),* })
                    };

                    let (verify_calls, (attribute_calls, element_calls)) = fields_to_calls(fields);

                    let element_name = get_component_name(&ident, &options.prefix);
                    quote!(#matcher => {
                        #verify_calls

                        let builder = xml::writer::events::XmlEvent::start_element(#element_name);
                        #xmlns_calls
                        #(#attribute_calls)*
                        writer.write(builder)?;

                        #(#element_calls)*

                        writer.write(xml::writer::events::XmlEvent::end_element())
                    })
                }

                syn::Fields::Unnamed(fields) => {
                    if !xmlns_calls.is_empty() || options.prefix.is_some() {
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

                    let matcher = {
                        let idents = fields.iter().map(|field| &field.call_accessor);

                        quote!(Self::#ident(#(#idents),*))
                    };

                    let (verify_calls, (_, element_calls)) = fields_to_calls(fields);

                    quote!(#matcher => {
                        #verify_calls

                        #(#element_calls)*

                        Ok(())
                    })
                }

                syn::Fields::Unit => panic!("Mixing unit and non-unit variants in an enum is not supported"),
            }
        })
        .collect();

    quote!(
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

fn get_component_name(ident: &Ident, prefix: &Option<TokenStream>) -> TokenStream {
    let ident = ident.to_string();
    match prefix {
        Some(prefix) => quote!(const_format::formatcp!("{}:{}", #prefix, #ident)),
        None => ident.into_token_stream(),
    }
}

fn fields_to_calls(fields: Vec<Field>) -> (TokenStream, (Vec<TokenStream>, Vec<TokenStream>)) {
    let (verify_calls, (attribute_calls, element_calls)): (
        TokenStream,
        (Vec<Option<TokenStream>>, Vec<Option<TokenStream>>),
    ) = fields
        .into_iter()
        .map(|field| {
            let verify_accessor = field.verify_accessor;
            let call_accessor = field.call_accessor;
            match field.options.is_attribute {
                true => {
                    let ident = field.ident.as_ref().unwrap();
                    let attr_name = snake_to_pascal(ident);
                    (
                        quote!(crate::xml::verify_attribute_field(#verify_accessor);),
                        (
                            Some(quote!(
                                let builder = #call_accessor.write_as_attribute(builder, #attr_name);
                            )),
                            None,
                        ),
                    )
                }
                false => (
                    quote!(crate::xml::verify_element_field(#verify_accessor);),
                    (
                        None,
                        Some(quote!(
                            #call_accessor.write_as_element(writer)?;
                        )),
                    ),
                ),
            }
        })
        .unzip();

    (
        verify_calls,
        (
            attribute_calls.into_iter().flatten().collect(),
            element_calls.into_iter().flatten().collect(),
        ),
    )
}

fn namespaces_to_calls(namespaces: Vec<XmlNamespace>) -> TokenStream {
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

#[derive(Default, Debug)]
pub(super) struct ComponentOptions {
    prefix: Option<TokenStream>,
    namespaces: Vec<XmlNamespace>,
}

impl TryFrom<Vec<Attribute>> for ComponentOptions {
    type Error = &'static str;

    fn try_from(value: Vec<Attribute>) -> Result<Self, Self::Error> {
        let meta = try_get_serialize_meta(value)?;

        let mut prefix = None;
        let mut encountered_default = false;
        let namespaces = meta
            .into_iter()
            .map(|meta| match meta {
                Meta::NameValue(name_value) => {
                    if name_value.path.is_ident("default_ns") {
                        if encountered_default {
                            return Err(
                                "there must be at most one `default_ns` declaration per component",
                            );
                        }

                        encountered_default = true;

                        Ok(Some(XmlNamespace::Default(
                            name_value.value.into_token_stream(),
                        )))
                    } else if name_value.path.is_ident("ns") {
                        match name_value.value {
                            Expr::Tuple(tuple) if tuple.elems.len() == 2 => {
                                let mut elems = tuple.elems.into_iter();
                                Ok(Some(XmlNamespace::Prefixed(
                                    elems.next().unwrap().into_token_stream(),
                                    elems.next().unwrap().into_token_stream(),
                                )))
                            }

                            _ => Err("`ns` takes a single tuple of two elements as argument"),
                        }
                    } else if name_value.path.is_ident("ns_prefix") {
                        prefix = Some(name_value.value.into_token_stream());

                        Ok(None)
                    } else {
                        Err("unrecognized XML component attribute")
                    }
                }

                _ => Err("unrecognized XML component attribute"),
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(ComponentOptions { prefix, namespaces })
    }
}

fn try_get_serialize_meta(attrs: Vec<Attribute>) -> Result<Punctuated<Meta, Comma>, &'static str> {
    let mut parseable_attrs = attrs.into_iter().filter_map(|attr| {
        if attr.path().is_ident(MACRO_ATTRIBUTE) {
            Some(attr)
        } else {
            None
        }
    });

    let attr_to_parse = match parseable_attrs.clone().count() {
        0 => return Ok(Default::default()),
        1 => parseable_attrs.next().unwrap(),

        _ => return Err("multiple attributes specified for component"),
    };

    attr_to_parse
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .map_err(|_| "illegal attribute syntax")
}

struct FieldOptions {
    is_attribute: bool,
}

impl TryFrom<Vec<Attribute>> for FieldOptions {
    type Error = &'static str;

    fn try_from(value: Vec<Attribute>) -> Result<Self, Self::Error> {
        let meta = try_get_serialize_meta(value)?;

        let is_xml_attribute = meta.into_iter().try_fold(false, |value, meta| match meta {
            Meta::Path(path) => Ok(value || path.is_ident("is_attribute")),

            _ => Err("unrecognized XML field attribute"),
        })?;

        Ok(Self {
            is_attribute: is_xml_attribute,
        })
    }
}

#[derive(Debug)]
enum XmlNamespace {
    Default(TokenStream),
    Prefixed(TokenStream, TokenStream),
}

struct Field {
    ident: Option<Ident>,
    verify_accessor: TokenStream,
    call_accessor: TokenStream,
    options: FieldOptions,
}

// impl TryFrom<FieldsNamed> for Field {}

// impl TryFrom<FieldsUnnamed> for Field {}

fn snake_to_pascal(ident: &Ident) -> String {
    let mut capitalize_next = true;
    ident
        .to_string()
        .chars()
        .filter_map(|character| {
            if character == '_' {
                capitalize_next = true;

                None
            } else if capitalize_next {
                capitalize_next = false;

                Some(character.to_ascii_uppercase())
            } else {
                Some(character)
            }
        })
        .collect()
}
