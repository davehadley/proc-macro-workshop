use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let inputtree = parse_macro_input!(input as DeriveInput);

    let builderfactorycode = generate_builder_factory(&inputtree);
    let builderstructcode = generate_builder_struct(&inputtree);
    let builderimplcode = generate_builder_impl(&inputtree);

    join([builderfactorycode, builderstructcode, builderimplcode])
}

fn join(iter: impl IntoIterator<Item = TokenStream>) -> TokenStream {
    let mut output = TokenStream::new();
    for it in iter {
        output.extend(it)
    }
    output
}

fn generate_builder_factory(inputtree: &DeriveInput) -> TokenStream {
    let structname = &inputtree.ident;
    let builderstructname = get_builder_struct_name(inputtree);
    let output = quote! {
        impl #structname {
            pub fn builder() -> #builderstructname { #builderstructname::new() }
        }
    };
    output.into()
}

fn generate_builder_struct(inputtree: &DeriveInput) -> TokenStream {
    let builderstructname = get_builder_struct_name(inputtree);

    let fields = get_struct_field_names_and_types(inputtree);
    let fields = fields.map(|(name, ty)| {
        let name = name;
        quote! {
            #name: Option<#ty>
        }
    });

    let output = quote! {
        pub struct #builderstructname {
            #(#fields),*
        }
    };
    output.into()
}

fn generate_builder_impl(inputtree: &DeriveInput) -> TokenStream {
    let builderstructname = get_builder_struct_name(inputtree);

    let fields = get_struct_field_names(inputtree);
    let fields = fields.map(|name| {
        quote! {
            #name: None
        }
    });

    let output = quote! {
        impl #builderstructname {
            pub fn new() -> Self {
                Self {
                    #(#fields),*
                }
            }
        }
    };
    output.into()
}

fn get_struct_field_names_and_types(
    inputtree: &DeriveInput,
) -> impl Iterator<Item = (&Ident, &Type)> {
    let fields = if let Data::Struct(datastruct) = &inputtree.data {
        &datastruct.fields
    } else {
        unimplemented!()
    };
    let fields = if let Fields::Named(namedfields) = fields {
        namedfields
            .named
            .iter()
            .map(|it| (it.ident.as_ref().expect("fields must be named"), &it.ty))
    } else {
        unimplemented!()
    };

    fields
}

fn get_struct_field_names(inputtree: &DeriveInput) -> impl Iterator<Item = &Ident> {
    get_struct_field_names_and_types(inputtree).map(|(name, _)| name)
}

fn get_builder_struct_name(inputtree: &DeriveInput) -> Ident {
    format_ident!("{}{}", inputtree.ident, "Builder")
}
