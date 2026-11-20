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
    default_expr: Option<Expr>,
}

impl Parse for Field {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        let colon = input.parse()?;
        let ty = input.parse()?;
        let default_expr = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(Field {
            name,
            _colon: colon,
            ty,
            default_expr,
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

    let field_declarations = fields.iter().map(|field| {
        let Field {
            name,
            ty,
            default_expr,
            ..
        } = field;
        let default = default_expr
            .as_ref()
            .map(|expr| quote!(= #expr))
            .unwrap_or_default();
        quote!(#name: #ty #default,)
    });

    let expanded = quote! {
        static CONFIG: std::sync::LazyLock<Config> = std::sync::LazyLock::new(|| get_config::<Config>(env!("CARGO_PKG_NAME").to_string()));

        #[freezable_trait::freezable]
        struct #struct_name {
            #( #field_declarations )*
        }
    };

    TokenStream::from(expanded)
}
