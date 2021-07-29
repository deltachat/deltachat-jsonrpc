extern crate proc_macro;
use proc_macro::TokenStream;

use quote::quote;

#[cfg(test)]
mod test {
    use quote::quote;
    use syn;

    #[test]
    fn t1() -> anyhow::Result<()> {
        let mut input: syn::Type = syn::parse2(quote! {Vec::<u32>})?;

        println!("{:?}", &input);

        match &mut input {
            syn::Type::Path(syn::TypePath { path, .. }) => {
                let n_pseg = path.segments.last_mut();
                println!("{:?}", n_pseg);
            }
            _ => unimplemented!(),
        }

        assert!(false);
        Ok(())
    }
}

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

fn statify(t: syn::Type) -> syn::Type {
    // makes the static function call work even for generics such as vec

    let mut new_t = t.clone();
    // println!("{:?}", new_t);
    match &mut new_t {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            if let Some(ps) = path.segments.last_mut() {
                if let syn::PathArguments::AngleBracketed(pa) = &mut ps.arguments {
                    pa.colon2_token = Some(syn::Token!(::)(proc_macro2::Span::call_site()));

                    pa.args = pa
                        .args
                        .iter_mut()
                        .map(|a| match a {
                            syn::GenericArgument::Type(ty) => {
                                syn::GenericArgument::Type(statify(ty.clone()))
                            }
                            _ => a.clone(),
                        })
                        .collect();

                    new_t
                } else {
                    new_t
                }
            } else {
                unimplemented!("no last segment");
            }
        }
        _ => t,
    }
}

fn get_contained_arg_type(rtype: &syn::Type) -> syn::Type {
    if let syn::Type::Path(path) = rtype.clone() {
        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args,
            ..
        }) = path.path.segments.last().unwrap().arguments.clone()
        {
            if let Some(syn::GenericArgument::Type(rtype)) = args.first() {
                rtype.clone()
            } else {
                unimplemented!()
            }
        } else {
            unimplemented!()
        }
    } else {
        unimplemented!()
    }
}

fn get_2contained_args_type(rtype: &syn::Type) -> (syn::Type, syn::Type) {
    if let syn::Type::Path(path) = rtype.clone() {
        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args,
            ..
        }) = path.path.segments.last().unwrap().arguments.clone()
        {
            let mut arg_iter = args.iter();
            if let Some(syn::GenericArgument::Type(rtype1)) = arg_iter.next() {
                if let Some(syn::GenericArgument::Type(rtype2)) = arg_iter.next() {
                    (rtype1.clone(), rtype2.clone())
                } else {
                    unimplemented!()
                }
            } else {
                unimplemented!()
            }
        } else {
            unimplemented!()
        }
    } else {
        unimplemented!()
    }
}

fn custom_type_def(
    rtype: &syn::Type,
    custom_return_types: &mut Vec<(String, proc_macro2::TokenStream)>,
) -> proc_macro2::TokenStream {
    let type_as_string = quote! { #rtype }.to_string();

    // if wrapper, we need to extract type for custom_return_types
    if type_as_string.starts_with("Vec") | type_as_string.starts_with("Option") {
        custom_type_def(&get_contained_arg_type(rtype), custom_return_types);
    }

    if type_as_string.starts_with("HashMap") {
        let (key, value) = get_2contained_args_type(rtype);
        custom_type_def(&key, custom_return_types);
        custom_type_def(&value, custom_return_types);
    }

    let rt = statify(rtype.clone());

    // Add type to custom types if its not already in there
    if custom_return_types
        .iter()
        .find(|t| t.0 == type_as_string)
        .is_none()
    {
        custom_return_types.push((
            type_as_string,
            quote! {
                if  #rt::makes_use_of_custom_ts_type() && !#rt::IS_WRAPPER {
                    ts.push_str(&format!(
                        "export type {} = {};\n",
                        #rt::get_typescript_type_with_custom_type_support(),
                        #rt::get_typescript_type()
                    ));
                }
            },
        ))
    }
    return quote! { &#rt::get_typescript_type_with_custom_type_support() };
}

fn generate_get_typescript_function(
    struct_ident: &syn::Ident,
    functions: &[syn::ImplItemMethod],
) -> proc_macro2::TokenStream {
    let mut custom_return_types: Vec<(String, proc_macro2::TokenStream)> = Vec::new();
    let mut function_definitions = Vec::new();

    for method in functions {
        let method_name = method.sig.ident.to_string();

        let mut fn_inputs = Vec::new();

        for minput in &method.sig.inputs {
            if let syn::FnArg::Typed(arg) = minput.clone() {
                fn_inputs.push((*arg.pat, *arg.ty));
            }
        }

        let (args, params) = if fn_inputs.is_empty() {
            (quote! {"".to_owned()}, "undefined".to_owned())
        } else {
            let arg_parts_code: Vec<proc_macro2::TokenStream> = fn_inputs
                .iter()
                .map(|(pat, ty)| {
                    let pn = quote! {#pat}.to_string();
                    let ty = custom_type_def(ty, &mut custom_return_types);
                    quote! {
                        arg_parts.push(format!("{}: {}", #pn, #ty));
                    }
                })
                .collect();

            let params = fn_inputs
                .iter()
                .map(|(pat, _ty)| quote! {#pat}.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            (
                quote! {
                    {
                        let mut arg_parts:Vec<String> = Vec::new();
                        #(#arg_parts_code)*
                        arg_parts.join(", ").as_ref()
                    }
                },
                format!("{{{}}}", params),
            )
        };

        // find out the kind of return type
        //  case 1: anyhow::Result -> get content type
        //  case 2: where T:ReturnType
        let return_line = match &method.sig.output {
            syn::ReturnType::Default => {
                quote! {"void"}
            }
            syn::ReturnType::Type(_, ty) => {
                let rtype = *ty.clone();

                let is_result = if let syn::Type::Path(path) = rtype.clone() {
                    path.path.segments[0].ident == "Result"
                        || (path.path.segments[0].ident == "anyhow"
                            && path.path.segments[1].ident == "Result")
                } else {
                    false
                };

                if is_result {
                    // get the type inside of the result
                    let return_type = get_contained_arg_type(&rtype);

                    if quote! { #return_type }.to_string() == "()" {
                        quote! {"void"}
                    } else {
                        custom_type_def(&return_type, &mut custom_return_types)
                    }
                } else {
                    custom_type_def(&rtype, &mut custom_return_types)
                }
            }
        };

        // construction of output
        function_definitions.push(quote! {
            ts.push_str(&gen_ts_body(
                #method_name,
                &#args,
                #return_line,
                &#params
            ));
        });
    }

    let custom_return_types: Vec<proc_macro2::TokenStream> = custom_return_types
        .into_iter()
        .map(|(_name, typedef)| typedef)
        .collect();

    return quote! {
        impl #struct_ident {
            pub fn get_typescript() -> String {
                let mut ts = String::new();
                ts.push_str("// THIS FILE WAS AUTOGENERATED DO NOT EDIT MANUALLY!, unless you know what you are doing...\n");
                // ts.push_str("type todo = any;\n");
                // custom return types

                #(#custom_return_types)*

                // functions - prelude
                ts.push_str("\
                    export class RawApi {\n\
                        \t/**\n\
                        \t * @param json_transport function that executes a jsonrpc call and throws an error if one occured\n\
                        \t */\n\
                        \tconstructor (private json_transport: (method: string, params?: any) => Promise<any>) {}\n"
                );

                // functions
                fn gen_ts_body(method_name: &str, args: &str, return_type: &str, params: &str) -> String {
                    format!(
                        "\tpublic async {method_name}({args}):Promise<{return_type}>{{\n\
                            \t\treturn await this.json_transport(\"{method_name}\", {params});\n\
                            \t}}\n",
                        method_name = method_name,
                        args = args,
                        return_type = return_type,
                        params = params
                    )
                }

                #(#function_definitions)*

                ts.push_str("}");
                ts
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

        let typescript_function = generate_get_typescript_function(struct_ident, &methods);

        let expanded = quote! {
            impl #struct_ident {
                #(#methods)*
            }

            #json_api_function
            #typescript_function
        };

        TokenStream::from(expanded)
    } else {
        panic!("unexpected input");
    }
}
