use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Result, Token, Type,
};

struct Field {
    name: Ident,
    _colon: Token![:],
    ty: Type,
    _eq: Token![=],
    default_expr: Expr,
}

impl Parse for Field {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Field {
            name: input.parse()?,
            _colon: input.parse()?,
            ty: input.parse()?,
            _eq: input.parse()?,
            default_expr: input.parse()?,
        })
    }
}

struct ConfigSpec {
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for ConfigSpec {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let fields = Punctuated::<Field, Token![,]>::parse_terminated(input)?;
        Ok(Self { fields })
    }
}

pub fn config_builder_impl(input: TokenStream) -> TokenStream {
    let ConfigSpec { fields } = syn::parse_macro_input!(input as ConfigSpec);

    let struct_name = format_ident!("Config", span = Span::call_site());

    // Generate struct fields and default initializers
    let field_idents: Vec<Ident> = fields.iter().map(|f| f.name.clone()).collect();
    let field_types: Vec<Type> = fields.iter().map(|f| f.ty.clone()).collect();
    let field_defaults: Vec<Expr> = fields.iter().map(|f| f.default_expr.clone()).collect();

    let expanded = quote! {
        #[derive(Clone, Debug)]
        #[freezable]
        struct #struct_name {
            #( #field_idents: #field_types, )*
        }

        impl ::core::default::Default for #struct_name {
            fn default() -> Self {
                Self {
                    #( #field_idents: #field_defaults, )*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
