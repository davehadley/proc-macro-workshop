use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Field, Fields, GenericArgument, Ident,
    PathArguments, Type,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let inputtree = parse_macro_input!(input as DeriveInput);

    let builderfactorycode = generate_builder_factory(&inputtree);
    let builderstructcode = generate_builder_struct(&inputtree);
    let builderimplcode = generate_builder_impl(&inputtree);

    let output = join([builderfactorycode, builderstructcode, builderimplcode]);

    output.into()
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
    output
}

fn generate_builder_struct(inputtree: &DeriveInput) -> TokenStream {
    let builderstructname = get_builder_struct_name(inputtree);

    let fields = get_struct_field_names_and_types(inputtree);
    let fields = fields.iter().map(|(name, ty)| {
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
    output
}

fn generate_builder_impl(inputtree: &DeriveInput) -> TokenStream {
    let builderstructname = get_builder_struct_name(inputtree);
    let newmethod = generate_builder_impl_new_method(inputtree);
    let fieldsetters = generate_builder_impl_field_setters(inputtree);
    //let fieldelementsetters = generate_builder_impl_field_element_setters(inputtree);
    let buildmethod = generate_builder_impl_build_method(inputtree);

    let output = quote! {
        impl #builderstructname {
            #newmethod

            #(#fieldsetters)*

            //#(fieldelementsetters)*

            #buildmethod
        }
    };
    output
}

fn generate_builder_impl_field_setters(inputtree: &DeriveInput) -> Vec<TokenStream> {
    let fields = get_struct_field_names_and_types(inputtree);
    let fieldsetters = fields
        .iter()
        .map(|(name, ty)| {
            let ty = get_option_type_inner(ty).unwrap_or(ty);
            quote! {
                    fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name = Some(#name);
                        self
                    }
            }
        })
        .collect();

    fieldsetters
}

fn generate_builder_impl_field_element_setters(inputtree: &DeriveInput) -> Vec<TokenStream> {
    let fields = get_struct_vec_fields(inputtree);
    let fieldelementsetters = fields
        .iter()
        .map(|(field, attr)| {
            let ty = get_vec_type_inner(&field.ty);
            let name = parse_vec_attribute(*attr).unwrap().method_name;
            quote! {
                    fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name.push(#name);
                        self
                    }
            }
        })
        .collect();

    fieldelementsetters
}

fn generate_builder_impl_build_method(inputtree: &DeriveInput) -> TokenStream {
    let fields = get_struct_field_names_and_types(inputtree);
    let fieldnames = fields.iter().map(|(name, _)| name);
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
    let structname = &inputtree.ident;
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

    buildmethod
}

fn generate_builder_impl_new_method(inputtree: &DeriveInput) -> TokenStream {
    let fieldnames = get_struct_field_names(inputtree);
    quote! {
        pub fn new() -> Self {
            Self {
                #(#fieldnames: None),*
            }
        }
    }
}

fn get_struct_field_names_and_types(inputtree: &DeriveInput) -> Vec<(&Ident, &Type)> {
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

    fields.collect()
}

fn get_struct_field_names(inputtree: &DeriveInput) -> Vec<&Ident> {
    get_struct_field_names_and_types(inputtree)
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

fn get_builder_struct_name(inputtree: &DeriveInput) -> Ident {
    format_ident!("{}{}", inputtree.ident, "Builder")
}

fn get_option_type_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(typepath) = ty {
        if let Some(seg) = typepath.path.segments.iter().last() {
            if seg.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(&inner);
                    }
                }
            }
        }
    };
    None
}

fn get_vec_type_inner(ty: &Type) -> Option<&Type> {
    todo!()
}

fn is_option_type(ty: &Type) -> bool {
    get_option_type_inner(ty).is_some()
}

fn get_struct_vec_fields(inputtree: &DeriveInput) -> Vec<(&Field, &Attribute)> {
    let fields = if let Data::Struct(datastruct) = &inputtree.data {
        &datastruct.fields
    } else {
        unimplemented!()
    };
    if let Fields::Named(namedfields) = fields {
        namedfields
            .named
            .iter()
            .map(|field| {
                let att = field.attrs.iter().filter(|att| todo!()).next();
                (field, att)
            })
            .filter(|(field, att)| att.is_some())
            .map(|(field, att)| (field, att.unwrap()))
            .collect()
    } else {
        Vec::new()
    }
}

struct ParsedVecAttribute {
    method_name: String,
}

fn parse_vec_attribute(attr: &Attribute) -> Option<ParsedVecAttribute> {
    None
    // Some(ParsedVecAttribute {

    // })
}
