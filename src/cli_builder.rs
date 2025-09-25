use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, ExprArray, Ident, Token,
};

/// cli_runtime!( [ CLICommand { ... }, CLICommand { ... } ] )
/// cli_runtime!( default = help; [ CLICommand { ... }, ... ] )
struct MacroInput {
    default_ident: Option<Ident>,
    array: ExprArray,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let look = input.lookahead1();
        let default_ident = if look.peek(Token![default]) || input.peek(Ident) && input.fork().parse::<Ident>().is_ok() {
            // Support `default = ident;` only if it starts with the literal word `default`
            // to avoid accidental captures. We'll parse strictly: `default = <Ident> ;`
            // If it doesn't match, we assume the input starts with the array.
            if input.peek(Token![default]) {
                // parse `default`
                let _: Token![default] = input.parse()?;
                // parse `=`
                let _eq: Token![=] = input.parse()?;
                // parse ident
                let ident: Ident = input.parse()?;
                // parse `;`
                let _semi: Token![;] = input.parse()?;
                Some(ident)
            } else {
                None
            }
        } else {
            None
        };

        // Now expect `[ ... ]` as an array expression
        let array: ExprArray = input.parse()?;
        Ok(Self { default_ident, array })
    }
}

pub fn cli_builder_impl(input: TokenStream) -> TokenStream {
    let MacroInput { default_ident, array } = parse_macro_input!(input as MacroInput);

    let elems = array.elems; // Punctuated<Expr, Comma>
    let default_ident = default_ident
        .map(|id| quote! { Some(#id) })
        .unwrap_or_else(|| quote! { Some(help) });

    // We define the CLICommand and Runtime, then plug the user-provided entries directly.
    let expanded = quote! {
        #[derive(Clone)]
        pub struct CLICommand {
            pub short_flag: char,
            pub long_flag: String,
            pub command: fn(),
            pub description: String,
        }

        pub struct Runtime {
            pub commands: ::std::vec::Vec<CLICommand>,
            pub default_command: ::std::option::Option<fn()>,
        }

        impl Runtime {
            pub fn new() -> Self {
                Self {
                    commands: ::std::vec![
                        #(#elems ,)*
                    ],
                    default_command: #default_ident,
                }
            }

            pub fn run(&self, arg: ::std::string::String) {
                for command in &self.commands {
                    if let Some(long_arg) = arg.strip_prefix("--") {
                        if command.long_flag == long_arg {
                            (command.command)()
                        }
                    } else if let Some(short_arg) = arg.strip_prefix("-")
                        && command.short_flag.to_string() == short_arg {
                        (command.command)()
                    }
                }
            }

            pub fn gen_help(&self) -> ::std::string::String {
                let mut message: ::std::string::String = "Help:\n".into();
                for command in &self.commands {
                    let helpmsg = ::std::format!(
                        "\t -{}, --{}: {}\n",
                        command.short_flag,
                        command.long_flag,
                        command.description
                    );
                    message.push_str(&helpmsg);
                }
                message
            }
        }
    };

    expanded.into()
}
