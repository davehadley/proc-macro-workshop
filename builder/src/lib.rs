use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Field, Fields,
    GenericArgument, Ident, Lit, Meta, NestedMeta, PathArguments, Type,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let inputtree = parse_macro_input!(input as DeriveInput);
    let output = match generate_derive_code(&inputtree) {
        Ok(stream) => stream,
        Err(err) => err.to_compile_error(),
    };
    output.into()
}

fn generate_derive_code(inputtree: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let builderfactorycode = generate_builder_factory(inputtree)?;
    let builderstructcode = generate_builder_struct(inputtree)?;
    let builderimplcode = generate_builder_impl(inputtree)?;

    let output = join([builderfactorycode, builderstructcode, builderimplcode]);
    // eprintln!("DEBUG TOKENS: {}", output);
    Ok(output)
}

fn join(iter: impl IntoIterator<Item = TokenStream>) -> TokenStream {
    let mut output = TokenStream::new();
    for it in iter {
        output.extend(it)
    }
    output
}

fn generate_builder_factory(inputtree: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let structname = &inputtree.ident;
    let builderstructname = get_builder_struct_name(inputtree);
    let output = quote! {
        impl #structname {
            pub fn builder() -> #builderstructname { #builderstructname::new() }
        }
    };
    Ok(output)
}

fn generate_builder_struct(inputtree: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let builderstructname = get_builder_struct_name(inputtree);

    let fields = get_parsed_field(inputtree)?;
    let fields = fields.iter().map(|field| {
        let ty = get_option_type_inner(field.ty()).unwrap_or_else(|| field.ty());
        let name = field.name();
        quote! {
            #name: ::std::option::Option<#ty>
        }
    });

    let output = quote! {
        pub struct #builderstructname {
            #(#fields),*
        }
    };
    Ok(output)
}

fn generate_builder_impl(inputtree: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let builderstructname = get_builder_struct_name(inputtree);
    let newmethod = generate_builder_impl_new_method(inputtree)?;
    let fieldsetters = generate_builder_impl_field_setters(inputtree)?;
    let fieldelementsetters = generate_builder_impl_field_element_setters(inputtree)?;
    let buildmethod = generate_builder_impl_build_method(inputtree)?;

    let output = quote! {
        impl #builderstructname {
            #newmethod

            #(#fieldsetters)*

            #(#fieldelementsetters)*

            #buildmethod
        }
    };
    Ok(output)
}

fn generate_builder_impl_field_setters(
    inputtree: &DeriveInput,
) -> Result<Vec<TokenStream>, syn::Error> {
    let fields = get_parsed_field(inputtree)?;
    let fieldsetters = fields
        .iter()
        .filter(|field| field.should_have_set_method())
        .map(|field| {
            let ty = get_option_type_inner(field.ty()).unwrap_or_else(|| field.ty());
            let name = field.name();
            quote! {
                    fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name = ::std::option::Option::Some(#name);
                        self
                    }
            }
        })
        .collect();

    Ok(fieldsetters)
}

fn generate_builder_impl_field_element_setters(
    inputtree: &DeriveInput,
) -> Result<Vec<TokenStream>, syn::Error> {
    let fields = get_struct_vec_fields(inputtree)?;
    let fieldelementsetters = fields
        .iter()
        .map(|pf| {
            let ty = get_vec_type_inner(&pf.field.ty);
            let methodname = pf.vecattr.clone().unwrap().method;
            let fieldname = pf.field.ident.clone().unwrap();
            quote! {
                    fn #methodname(&mut self, #methodname: #ty) -> &mut Self {

                        self.#fieldname
                        .get_or_insert_with(|| ::std::vec::Vec::new())
                        .push(#methodname);
                        self
                    }
            }
        })
        .collect();

    Ok(fieldelementsetters)
}

fn generate_builder_impl_build_method(inputtree: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let fields = get_parsed_field(inputtree)?;
    let fieldnames = fields.iter().map(|field| field.name());
    let checkforunset = fields.iter().map(|field| {
        let name = field.name();
        if is_option_type(field.ty()) {
            quote! { let #name = self.#name.clone(); }
        } else {
            let msg = format!("{} must be set", name);
            quote! {
                let #name = match &self.#name {
                    ::std::option::Option::Some(inner) => inner.clone(),
                    ::std::option::Option::None => return ::std::result::Result::Err(#msg.into()),
                };
            }
        }
    });
    let structname = &inputtree.ident;
    let buildmethod = quote! {
        pub fn build(&mut self) -> ::std::result::Result<#structname, ::std::boxed::Box<dyn ::std::error::Error>> {
            #(#checkforunset)*
            ::std::result::Result::Ok(
                #structname {
                    #(#fieldnames),*
                }
            )
        }
    };

    Ok(buildmethod)
}

fn generate_builder_impl_new_method(inputtree: &DeriveInput) -> Result<TokenStream, syn::Error> {
    let fields = get_parsed_field(inputtree)?;
    let fields = fields.iter().map(|it| {
        let name = it.name();
        if it.has_vec_attribute() {
            quote! { #name: ::std::option::Option::Some(::std::vec::Vec::new()) }
        } else {
            quote! { #name: ::std::option::Option::None }
        }
    });
    let output = quote! {
        pub fn new() -> Self {
            Self {
                #(#fields),*
            }
        }
    };
    Ok(output)
}

fn get_parsed_field(inputtree: &DeriveInput) -> Result<Vec<ParsedField>, syn::Error> {
    let fields = if let Data::Struct(datastruct) = &inputtree.data {
        &datastruct.fields
    } else {
        unimplemented!()
    };
    let fields = if let Fields::Named(namedfields) = fields {
        namedfields.named.iter().map(ParsedField::new)
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

fn get_struct_vec_fields(inputtree: &DeriveInput) -> Result<Vec<ParsedField>, syn::Error> {
    let fields = if let Data::Struct(datastruct) = &inputtree.data {
        &datastruct.fields
    } else {
        unimplemented!()
    };
    let result = if let Fields::Named(namedfields) = fields {
        namedfields
            .named
            .iter()
            .map(ParsedField::new)
            .collect::<Result<Vec<_>, syn::Error>>()?
            .into_iter()
            .filter(|pf| pf.vecattr.is_some())
            .collect()
    } else {
        Vec::new()
    };
    Ok(result)
}

#[derive(Clone)]
struct ParsedField {
    field: Field,
    vecattr: Option<ParsedVecAttribute>,
    _name: Ident,
}

impl ParsedField {
    fn new(field: &Field) -> Result<Self, syn::Error> {
        let vecattr = field
            .attrs
            .iter()
            .map(parse_vec_attribute)
            .collect::<Result<Vec<Option<ParsedVecAttribute>>, syn::Error>>()?
            .into_iter()
            .find(|att| att.is_some())
            .flatten();
        Ok(Self {
            field: field.clone(),
            vecattr,
            _name: field
                .ident
                .clone()
                .expect("only named fields are supported"),
        })
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

impl TryFrom<&Field> for ParsedField {
    fn try_from(field: &Field) -> Result<Self, Self::Error> {
        ParsedField::new(field)
    }

    type Error = syn::Error;
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

fn parse_vec_attribute(attr: &Attribute) -> Result<Option<ParsedVecAttribute>, syn::Error> {
    let err = || syn::Error::new(attr.tokens.span(), "expected `builder(each = \"...\")`");
    if attr.path.segments.iter().last().ok_or_else(err)?.ident == "builder" {
        let meta = attr.parse_meta()?;
        if let Meta::List(metalist) = meta {
            if let NestedMeta::Meta(syn::Meta::NameValue(value)) =
                metalist.nested.iter().next().ok_or_else(err)?
            {
                if value.path.segments.iter().last().ok_or_else(err)?.ident == "each" {
                    if let Lit::Str(lit) = &value.lit {
                        return Ok(Some(ParsedVecAttribute::new(lit.value())));
                    }
                }
            };
        };
    } else {
        return Ok(None);
    }
    Err(err())
}
