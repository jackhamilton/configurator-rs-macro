use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, ExprArray, Ident, Token,
};

/// cli_runtime!( [ Command { ... }, Command { ... } ] )
/// cli_runtime!( default = help; [ Command { ... }, ... ] )
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

        let array: ExprArray = input.parse()?;
        Ok(Self { default_ident, array })
    }
}

pub fn config_builder_impl(input: TokenStream) -> TokenStream {
    let MacroInput { default_ident, array } = parse_macro_input!(input as MacroInput);

    let elems = array.elems.iter();
    let default_ident = default_ident
        .map(|id| quote! { Some(#id) })
        .unwrap_or_else(|| quote! { Some(help) });

    let expanded = quote! {
        #[derive(Clone)]
        pub struct Command {
            pub short_flag: char,
            pub long_flag: &'static str,
            pub command: fn(),
            pub description: &'static str,
        }

        pub struct Runtime {
            pub commands: ::std::vec::Vec<Command>,
            pub default_command: ::std::option::Option<fn()>,
        }

        impl Runtime {
            pub fn new() -> Self {
                Self {
                    commands: ::std::vec![
                        Command {
                            short_flag: 'h',
                            long_flag: "help",
                            command: help,
                            description: "Explains available commands."
                        },
                        Command {
                            short_flag: 'v',
                            long_flag: "version",
                            command: version,
                            description: "Outputs tool version."
                        },
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

        fn help() {
            // I'm not losing sleep over the fact that this will likely be invoked from an existing runtime
            // object. Mind, I am losing sleep, just for different reasons.
            let runtime = Runtime::new();
            println!("{}", runtime.gen_help());
            std::process::exit(0);
        }

        fn version() {
            println!("{} version {} by Jack Hamilton", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        fn main() {
            let runtime = Runtime::new();
            let args: Vec<String> = env::args().collect();
            // First arg is junk
            if args.is_empty() || args.len() == 1 {
                if let Some(command) = runtime.default_command {
                    command()
                } else {
                    help()
                }
            } else if args.len() > 2 {
                println!("Too many arguments!");
                help()
            } else {
                let arg = args[1].clone();
                runtime.run(arg);
            }
        }
    };

    expanded.into()
}
