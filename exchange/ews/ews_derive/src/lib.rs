use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod xml_attribute;
use xml_attribute::write_attribute_derivation;

mod xml_element;
use xml_element::{
    write_element_derivation_for_enum, write_element_derivation_for_struct, ComponentOptions,
};

#[proc_macro_derive(XmlAttribute)]
pub fn derive_xml_attribute(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        syn::Data::Enum(enum_input) => write_attribute_derivation(input.ident, enum_input),

        _ => panic!("`XmlAttribute` derivation is only supported for unit enums"),
    }
}

#[proc_macro_derive(XmlElement, attributes(xml_serialize))]
pub fn derive_xml_write(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let options =
        ComponentOptions::try_from(input.attrs).expect("Unable to parse component attributes");

    match input.data {
        syn::Data::Struct(struct_input) => {
            write_element_derivation_for_struct(input.ident, struct_input, options)
        }
        syn::Data::Enum(enum_input) => {
            write_element_derivation_for_enum(input.ident, enum_input, options)
        }
        syn::Data::Union(_) => panic!("Using unions as XML elements is not supported"),
    }
}
