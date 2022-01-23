use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Field, Fields, GenericArgument, Ident, Lit,
    Meta, NestedMeta, PathArguments, Type,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let inputtree = parse_macro_input!(input as DeriveInput);

    let builderfactorycode = generate_builder_factory(&inputtree);
    let builderstructcode = generate_builder_struct(&inputtree);
    let builderimplcode = generate_builder_impl(&inputtree);

    let output = join([builderfactorycode, builderstructcode, builderimplcode]);
    // eprintln!("DEBUG TOKENS: {}", output);
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

    let fields = get_parsed_field(inputtree);
    let fields = fields.iter().map(|field| {
        let ty = get_option_type_inner(field.ty()).unwrap_or(field.ty());
        let name = field.name();
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
    let fieldelementsetters = generate_builder_impl_field_element_setters(inputtree);
    let buildmethod = generate_builder_impl_build_method(inputtree);

    let output = quote! {
        impl #builderstructname {
            #newmethod

            #(#fieldsetters)*

            #(#fieldelementsetters)*

            #buildmethod
        }
    };
    output
}

fn generate_builder_impl_field_setters(inputtree: &DeriveInput) -> Vec<TokenStream> {
    let fields = get_parsed_field(inputtree);
    let fieldsetters = fields
        .iter()
        .filter(|field| field.should_have_set_method())
        .map(|field| {
            let ty = get_option_type_inner(field.ty()).unwrap_or(field.ty());
            let name = field.name();
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
        .map(|pf| {
            let ty = get_vec_type_inner(&pf.field.ty);
            let methodname = pf.vecattr.clone().unwrap().method;
            let fieldname = pf.field.ident.clone().unwrap();
            quote! {
                    fn #methodname(&mut self, #methodname: #ty) -> &mut Self {

                        self.#fieldname
                        .get_or_insert_with(|| Vec::new())
                        .push(#methodname);
                        self
                    }
            }
        })
        .collect();

    fieldelementsetters
}

fn generate_builder_impl_build_method(inputtree: &DeriveInput) -> TokenStream {
    let fields = get_parsed_field(inputtree);
    let fieldnames = fields.iter().map(|field| field.name());
    let checkforunset = fields.iter().map(|field| {
        let name = field.name();
        if is_option_type(field.ty()) {
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
    let fields = get_parsed_field(inputtree);
    let fields = fields.iter().map(|it| {
        let name = it.name();
        if it.has_vec_attribute() {
            quote! { #name: Some(Vec::new()) }
        } else {
            quote! { #name: None }
        }
    });
    quote! {
        pub fn new() -> Self {
            Self {
                #(#fields),*
            }
        }
    }
}

fn get_parsed_field(inputtree: &DeriveInput) -> Vec<ParsedField> {
    let fields = if let Data::Struct(datastruct) = &inputtree.data {
        &datastruct.fields
    } else {
        unimplemented!()
    };
    let fields = if let Fields::Named(namedfields) = fields {
        namedfields.named.iter().map(|it| ParsedField::new(it))
    } else {
        unimplemented!()
    };

    fields.collect()
}

fn get_builder_struct_name(inputtree: &DeriveInput) -> Ident {
    format_ident!("{}{}", inputtree.ident, "Builder")
}

fn get_container_type_inner<'a>(ty: &'a Type, containername: &str) -> Option<&'a Type> {
    if let Type::Path(typepath) = ty {
        if let Some(seg) = typepath.path.segments.iter().last() {
            if seg.ident == containername {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(ref inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    };
    None
}

fn get_option_type_inner(ty: &Type) -> Option<&Type> {
    get_container_type_inner(ty, "Option")
}

fn get_vec_type_inner(ty: &Type) -> Option<&Type> {
    get_container_type_inner(ty, "Vec")
}

fn is_option_type(ty: &Type) -> bool {
    get_option_type_inner(ty).is_some()
}

fn get_struct_vec_fields(inputtree: &DeriveInput) -> Vec<ParsedField> {
    let fields = if let Data::Struct(datastruct) = &inputtree.data {
        &datastruct.fields
    } else {
        unimplemented!()
    };
    if let Fields::Named(namedfields) = fields {
        namedfields
            .named
            .iter()
            .map(|field| ParsedField::new(field))
            .filter(|pf| pf.vecattr.is_some())
            .collect()
    } else {
        Vec::new()
    }
}

#[derive(Clone)]
struct ParsedField {
    field: Field,
    vecattr: Option<ParsedVecAttribute>,
    _name: Ident,
}

impl ParsedField {
    fn new(field: &Field) -> Self {
        let vecattr = field
            .attrs
            .iter()
            .map(|att| parse_vec_attribute(att))
            .find(|att| att.is_some())
            .flatten();
        Self {
            field: field.clone(),
            vecattr,
            _name: field
                .ident
                .clone()
                .expect("only named fields are supported"),
        }
    }

    fn name(&self) -> &Ident {
        &self._name
    }

    fn ty(&self) -> &Type {
        &self.field.ty
    }

    fn should_have_set_method(&self) -> bool {
        match &self.vecattr {
            Some(att) => att.method != self._name,
            None => true,
        }
    }

    fn has_vec_attribute(&self) -> bool {
        self.vecattr.is_some()
    }
}

impl From<&Field> for ParsedField {
    fn from(field: &Field) -> Self {
        ParsedField::new(field)
    }
}

#[derive(Debug, Clone)]

struct ParsedVecAttribute {
    method: Ident,
}

impl ParsedVecAttribute {
    fn new(method_name: String) -> Self {
        let method = format_ident!("{}", method_name);
        Self { method }
    }
}

fn parse_vec_attribute(attr: &Attribute) -> Option<ParsedVecAttribute> {
    if attr.path.segments.iter().last()?.ident == "builder" {
        let meta = attr.parse_meta().ok()?;
        if let Meta::List(metalist) = meta {
            if let NestedMeta::Meta(nestedmeta) = metalist.nested.iter().next()? {
                if let syn::Meta::NameValue(value) = nestedmeta {
                    if value.path.segments.iter().last()?.ident == "each" {
                        if let Lit::Str(lit) = &value.lit {
                            return Some(ParsedVecAttribute::new(lit.value()));
                        }
                    }
                };
            };
        };
    }
    None
}
