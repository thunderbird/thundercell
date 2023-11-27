use quote::quote;
use syn::{DataEnum, Ident};

/// Generates an implementation of `XmlAttribute` for an enum with unit
/// variants.
///
/// Variant identifiers are stringified and passed as the attribute value.
pub(super) fn write_attribute_derivation(ident: Ident, data: DataEnum) -> proc_macro::TokenStream {
    let variant_arms: Vec<_> = data
        .variants
        .into_iter()
        .map(|variant| {
            if !matches!(variant.fields, syn::Fields::Unit) {
                // There's no clear derivation of `XmlAttribute` for a non-unit
                // variant. We could potentially handle single-element tuple
                // variants, but it's not clear that there's any advantage to
                // doing so.
                panic!("`XmlAttribute` derivation is only supported for unit enums");
            }

            let ident = &variant.ident;
            let value = variant.ident.to_string();

            quote!(Self::#ident => #value)
        })
        .collect();

    quote!(
        #[automatically_derived]
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
    )
    .into()
}
