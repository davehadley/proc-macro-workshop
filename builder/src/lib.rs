use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, GenericArgument, Ident, PathArguments, Type,
};

#[proc_macro_derive(Builder, attributes(builder))]
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
        let ty = get_option_type_inner(ty).unwrap_or(ty);
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
    let structname = &inputtree.ident;
    let builderstructname = get_builder_struct_name(inputtree);

    let fields: Vec<_> = get_struct_field_names_and_types(inputtree).collect();
    let fieldnames: Vec<_> = fields.iter().map(|(name, _)| *name).collect();
    let fieldsetters = fields.iter().map(|(name, ty)| {
        let ty = get_option_type_inner(ty).unwrap_or(ty);
        quote! {
                fn #name(&mut self, #name: #ty) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
        }
    });

    let checkforunset = fields.iter().map(|(name, ty)| {
        if is_option_type(ty) {
            quote! { let #name = self.#name.clone(); }
        } else {
            let msg = format!("{} must be set", name);
            quote! {
                let #name = match &self.#name {
                    Some(inner) => inner.clone(),
                    None => return Err(#msg.into()),
                };
            }
        }
    });
    let buildmethod = quote! {
        pub fn build(&mut self) -> Result<#structname, Box<dyn std::error::Error>> {
            #(#checkforunset)*
            Ok(
                #structname {
                    #(#fieldnames),*
                }
            )
        }
    };

    let output = quote! {
        impl #builderstructname {
            pub fn new() -> Self {
                Self {
                    #(#fieldnames: None),*
                }
            }

            #(#fieldsetters)*

            #buildmethod
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

// fn get_struct_field_names(inputtree: &DeriveInput) -> impl Iterator<Item = &Ident> {
//     get_struct_field_names_and_types(inputtree).map(|(name, _)| name)
// }

fn get_builder_struct_name(inputtree: &DeriveInput) -> Ident {
    format_ident!("{}{}", inputtree.ident, "Builder")
}

fn get_option_type_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(typepath) = ty {
        if let Some(seg) = typepath.path.segments.iter().last() {
            if seg.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    };
    None
}

fn is_option_type(ty: &Type) -> bool {
    get_option_type_inner(ty).is_some()
}
