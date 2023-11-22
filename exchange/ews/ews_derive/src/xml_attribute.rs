use quote::quote;
use syn::{DataEnum, Ident};

pub(super) fn write_attribute_derivation(ident: Ident, data: DataEnum) -> proc_macro::TokenStream {
    let variant_arms: Vec<_> = data.variants.into_iter().map(|variant| {
        let ident = &variant.ident;
        let value = variant.ident.to_string();

        quote!(Self::#ident => #value)
    }).collect();

    quote!(
        impl crate::xml::XmlAttribute for #ident {
            fn write_as_attribute<'a>(
                &'a self,
                builder: xml::writer::events::StartElementBuilder<'a>,
                attr_name: &'a str,
            ) -> xml::writer::events::StartElementBuilder<'a> {
                let value = match self {
                    #(#variant_arms),*
                };

                builder.attr(attr_name, value)
            }
        }
    ).into()
}
