use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};
use std::hash::{Hash, Hasher};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Result, Token, Type,
};

struct Field {
    name: Ident,
    ty: Type,
    default_expr: Option<Expr>,
}

impl Parse for Field {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        let default_expr = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Self {
            name,
            ty,
            default_expr,
        })
    }
}

struct Section {
    name: Ident,
    members: Punctuated<Member, Token![,]>,
}

impl Parse for Section {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        let content;
        braced!(content in input);
        let members = Punctuated::parse_terminated(&content)?;
        Ok(Self { name, members })
    }
}

enum Member {
    Field(Field),
    Section(Section),
}

impl Parse for Member {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            let fork = input.fork();
            let _: Ident = fork.parse()?;
            if fork.peek(syn::token::Brace) {
                return input.parse().map(Self::Section);
            }
            return input.parse().map(Self::Field);
        }
        Err(lookahead.error())
    }
}

struct ConfigSpec {
    members: Punctuated<Member, Token![,]>,
}

impl Parse for ConfigSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            members: Punctuated::parse_terminated(input)?,
        })
    }
}

fn validate_sections(
    members: &Punctuated<Member, Token![,]>,
) -> Result<()> {
    for member in members {
        let Member::Section(section) = member else {
            continue;
        };

        if section.name.to_string().starts_with("r#") {
            return Err(syn::Error::new_spanned(
                &section.name,
                "section names cannot use raw identifiers",
            ));
        }

        validate_sections(&section.members)?;
    }

    Ok(())
}

fn field_declaration(field: &Field, visible_to_parent: bool) -> proc_macro2::TokenStream {
    let Field {
        name,
        ty,
        default_expr,
    } = field;
    let default = default_expr
        .as_ref()
        .map(|expr| quote!(#[default(#expr)]))
        .unwrap_or_default();
    let visibility = if visible_to_parent {
        quote!(pub(super))
    } else {
        quote!()
    };
    quote!(#default #visibility #name: #ty,)
}

fn generate_section(
    section: &Section,
    module_name: &Ident,
    is_root_section: bool,
    next_type_index: &mut usize,
    generated: &mut Vec<proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let section_type = format_ident!(
        "{}Section{}",
        module_name,
        *next_type_index,
        span = Span::mixed_site()
    );
    *next_type_index += 1;
    let mut declarations = Vec::new();

    for member in &section.members {
        match member {
            Member::Field(field) => declarations.push(field_declaration(field, true)),
            Member::Section(nested) => {
                declarations.push(generate_section(
                    nested,
                    module_name,
                    false,
                    next_type_index,
                    generated,
                ));
            }
        }
    }

    generated.push(quote! {
        #[freezable_trait::freezable]
        pub(super) struct #section_type {
            #( #declarations )*
        }
    });

    let name = &section.name;
    if is_root_section {
        quote!(#name: #module_name::#section_type,)
    } else {
        quote!(pub(super) #name: super::#module_name::#section_type,)
    }
}

pub fn config_builder_impl(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.to_string().hash(&mut hasher);
    let module_name = format_ident!(
        "__configurator_sections_{:016x}",
        hasher.finish(),
        span = Span::mixed_site()
    );
    let ConfigSpec { members } = match syn::parse2::<ConfigSpec>(input) {
        Ok(spec) => spec,
        Err(error) => return error.to_compile_error().into(),
    };
    if let Err(error) = validate_sections(&members) {
        return error.to_compile_error().into();
    }

    let configurator_path = match crate_name("toml-configurator") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let name = Ident::new(&name, Span::call_site());
            quote!(::#name)
        }
        Err(error) => {
            return syn::Error::new(Span::call_site(), error.to_string())
                .to_compile_error()
                .into();
        }
    };

    let struct_name = format_ident!("Config", span = Span::call_site());
    let mut generated_sections = Vec::new();
    let mut field_declarations = Vec::new();
    let mut next_type_index = 0;

    for member in &members {
        match member {
            Member::Field(field) => field_declarations.push(field_declaration(field, false)),
            Member::Section(section) => {
                let declaration = generate_section(
                    section,
                    &module_name,
                    true,
                    &mut next_type_index,
                    &mut generated_sections,
                );
                field_declarations.push(declaration);
            }
        }
    }

    let expanded = quote! {
        static CONFIG: std::sync::LazyLock<Config> = std::sync::LazyLock::new(|| #configurator_path::get_config::<Config>(env!("CARGO_PKG_NAME").to_string()));

        mod #module_name {
            use super::*;
            #( #generated_sections )*
        }

        #[freezable_trait::freezable]
        struct #struct_name {
            #( #field_declarations )*
        }
    };

    TokenStream::from(expanded)
}
