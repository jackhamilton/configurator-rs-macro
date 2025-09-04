use quote::format_ident;
use proc_macro::TokenStream;
use quote::quote;
use syn::Fields::Named;
use syn::Fields::Unnamed;
use syn::Data::Struct;
use syn::DeriveInput;
use syn::parse_macro_input;

pub fn freezable_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = parse_macro_input!(input as DeriveInput);
    let where_clause = &generics.where_clause;

    match data {
        Struct(my_struct) => {
            match my_struct.fields {
                Named(fields) => {
                    let mut field_inits: Vec<proc_macro2::TokenStream> = vec![];
                    let field_stream: Vec<proc_macro2::TokenStream> = fields.named.into_iter().map(|field| {
                        let name = field.ident.as_ref().expect("Field is unnamed");
                        let func_name = format_ident!("_default_{}_{}", ident.to_string().to_lowercase(), name);
                        let field_type = &field.ty;
                        let field_func = quote! {
                            fn #func_name() -> #field_type { #ident::default().#name }
                        };
                        field_inits.push(field_func);
                        let func_name_str = format!("{}", func_name);
                        let literal_func_name = syn::LitStr::new(&func_name_str, func_name.span());
                        quote!{
                            #[serde(default = #literal_func_name)]
                            #field
                        }
                    }).collect();
                    quote!{
                        #[derive(Serialize, Deserialize)]
                        struct #generics #ident #generics #where_clause {
                            #(#field_stream),*
                        }
                        impl Freezable for #ident {}
                        #(#field_inits)*
                    }
                },
                Unnamed(_fields) => panic!("Fields should be named"),
                _unit => panic!("Should not be unit struct"),
            }
        }
        _ => panic!("Must be a struct.")
    }.into()
}
