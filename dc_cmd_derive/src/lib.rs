extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;

fn generate_json_rpc_api(
    struct_ident: &syn::Ident,
    functions: &[syn::ImplItemMethod],
) -> proc_macro2::TokenStream {
    let mut parameter_structs: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut methods: Vec<proc_macro2::TokenStream> = Vec::new();

    for method in functions {
        let method_name = method.sig.ident.to_string();

        let parameter_struct_ident = quote::format_ident!("{}_parameters", method.sig.ident);

        let mut fn_inputs = Vec::new();
        let mut fn_input_args = Vec::new();

        for minput in &method.sig.inputs {
            if let syn::FnArg::Typed(arg) = minput.clone() {
                let (pat, ty) = (*arg.pat, *arg.ty);
                fn_inputs.push(quote! {#pat: #ty,});
                let arg_name = quote::format_ident!("{}", (quote! {#pat}).to_string()); //ugly hack
                fn_input_args.push(quote! {
                    parameters.#arg_name
                });
            }
        }

        if !fn_inputs.is_empty() {
            parameter_structs.push(quote! {
                #[allow(non_camel_case_types)]
                #[derive(Deserialize)]
                struct #parameter_struct_ident {
                    #(#fn_inputs)*
                }
            });
        }

        // find out the kind of return type
        //  case 1: anyhow::Result
        //  case 2: where T:ReturnType
        let return_line = match &method.sig.output {
            syn::ReturnType::Default => {
                quote! {
                    jsonrpc_core::Result::Ok(().into_json_value())
                }
            }
            syn::ReturnType::Type(_, ty) => {
                let rtype = *ty.clone();

                let is_result = if let syn::Type::Path(path) = rtype {
                    path.path.segments[0].ident == "Result"
                        || (path.path.segments[0].ident == "anyhow"
                            && path.path.segments[1].ident == "Result")
                } else {
                    false
                };

                if is_result {
                    quote! {
                        result_convert_anyhow_into_json_rpc(result)
                    }
                } else {
                    quote! {
                        jsonrpc_core::Result::Ok(result.into_json_value())
                    }
                }
            }
        };

        let method_ident = &method.sig.ident;

        // construction of output
        if fn_inputs.is_empty() {
            methods.push(quote! {
                let self_ref = self.clone();
                io.add_method(#method_name, move |_: jsonrpc_core::Params| {
                    let self_ref = self_ref.clone();
                    async move {
                        let result = self_ref.#method_ident().await;
                        #return_line
                    }
                });
            });
        } else {
            methods.push(quote! {
                    let self_ref = self.clone();
                    io.add_method(#method_name, move |param: jsonrpc_core::Params| {
                        let self_ref = self_ref.clone();
                        async move {
                            let parameters: #parameter_struct_ident = param.parse()?;
                            let result = self_ref.#method_ident(#(#fn_input_args),*).await;
                            #return_line
                        }
                    });
            });
        }
    }

    return quote! {
        #(#parameter_structs)*

        impl #struct_ident {
            pub fn get_json_rpc_io(&self) -> jsonrpc_core::IoHandler {
                let mut io = jsonrpc_core::IoHandler::new();
                    #(#methods)*
                    io
            }
        }
    };
}

#[proc_macro_attribute]
pub fn gen_command_api(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemImpl);

    // println!("{:?}", input.items);

    if let syn::Type::Path(syn::TypePath {
        path: struct_path, ..
    }) = *input.self_ty.clone()
    {
        let struct_ident = struct_path.get_ident().unwrap();

        let mut methods: Vec<syn::ImplItemMethod> = Vec::new();

        for item in input.items {
            if let syn::ImplItem::Method(m) = item {
                if let Some(syn::token::Async { .. }) = m.sig.asyncness {
                    methods.push(m);
                } else {
                    panic!("sync methods are not supported yet")
                }
            }
        }

        let json_api_function = generate_json_rpc_api(struct_ident, &methods);

        let expanded = quote! {
            impl #struct_ident {
                #(#methods)*
            }

            #json_api_function
        };

        TokenStream::from(expanded)
    } else {
        panic!("unexpected input");
    }
}
