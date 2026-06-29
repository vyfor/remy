use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, GenericArgument, ItemFn, Pat, ReturnType, Type, TypePath, parse2};

pub fn expand_intent(input: TokenStream) -> TokenStream {
    let func: ItemFn = match parse2(input) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    let vis = &func.vis;
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let is_async = func.sig.asyncness.is_some();
    let body = &func.block;

    let mut user_params = Vec::new();
    let mut has_app = false;

    for (i, arg) in func.sig.inputs.iter().enumerate() {
        if let FnArg::Typed(pat_type) = arg {
            if i == 0 && is_app_param(pat_type) {
                has_app = true;
                continue;
            }
            user_params.push(pat_type.clone());
        }
    }

    let param_names: Vec<_> = user_params
        .iter()
        .filter_map(|p| {
            if let Pat::Ident(ident) = &*p.pat {
                Some(ident.ident.clone())
            } else {
                None
            }
        })
        .collect();

    let param_types: Vec<_> = user_params.iter().map(|p| &*p.ty).collect();

    let args_struct_name = format_ident!("__Intent_{}_Args", fn_name);
    let execute_fn_name = format_ident!("__execute_{}", fn_name);
    let intent_id_const = format_ident!("__INTENT_ID_{}", fn_name.to_string().to_uppercase());
    let reg_mod_name = format_ident!("__intent_reg_{}", fn_name);

    let struct_fields: Vec<_> = param_names
        .iter()
        .zip(param_types.iter())
        .map(|(name, ty)| quote! { pub #name: #ty })
        .collect();

    let struct_inits: Vec<_> = param_names.iter().map(|name| quote! { #name }).collect();
    let destructure: Vec<_> = param_names.iter().map(|name| quote! { #name }).collect();

    let app_binding = if has_app {
        quote! { let cx = ::remy::core::App::new(); }
    } else {
        quote! {}
    };

    let returns_result_unit =
        matches!(&func.sig.output, ReturnType::Type(_, ty) if is_result_unit(ty));
    let returns_unit = matches!(&func.sig.output, ReturnType::Default)
        || matches!(&func.sig.output, ReturnType::Type(_, ty) if is_unit_type(ty));

    if !returns_unit && !returns_result_unit {
        return syn::Error::new_spanned(
            &func.sig,
            "#[intent] only supports `()` and `Result<(), E: std::fmt::Display>`",
        )
        .to_compile_error();
    }

    let executor_body = if is_async {
        if returns_result_unit {
            quote! {
                #app_binding
                let fut = async move {
                    match async move { #body }.await {
                        Ok(()) => (),
                        Err(__e) => {
                            ::remy::core::runtime::report_error(
                                #fn_name_str,
                                &__e,
                            );
                        }
                    }
                };
                ::remy::core::runtime::Runtime::get()
                    .executor
                    .dispatch(#intent_id_const, fut);
            }
        } else {
            quote! {
                #app_binding
                let fut = async move {
                    #body
                };
                ::remy::core::runtime::Runtime::get()
                    .executor
                    .dispatch(#intent_id_const, fut);
            }
        }
    } else if returns_result_unit {
        quote! {
            #app_binding
            ::remy::core::batch! {
                let __result: () = match { #body } {
                    Ok(()) => (),
                    Err(__e) => {
                        ::remy::core::runtime::report_error(
                            #fn_name_str,
                            &__e,
                        );
                    }
                };
                __result
            }
        }
    } else {
        quote! {
            #app_binding
            ::remy::core::batch! { #body }
        }
    };

    quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        struct #args_struct_name {
            #(#struct_fields,)*
        }

        const #intent_id_const: u32 = ::remy::core::const_slot_id(
            concat!(module_path!(), "::intent"),
            #fn_name_str
        );

        #[doc(hidden)]
        fn #execute_fn_name(
            __cx: ::remy::core::App,
            __args: #args_struct_name,
        ) {
            let #args_struct_name { #(#destructure,)* } = __args;
            #executor_body
        }

        #vis fn #fn_name(#(#param_names: #param_types),*) {
            let __args = #args_struct_name { #(#struct_inits,)* };
            ::remy::core::runtime::dispatch_intent(
                #intent_id_const,
                Box::new(__args),
            );
        }

        #[::remy::linkme::distributed_slice(::remy::core::INTENT_REGISTRY)]
        #[linkme(crate = ::remy::linkme)]
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #reg_mod_name: (u32, fn(::remy::core::App, Box<dyn std::any::Any + Send>)) = (
            #intent_id_const,
            |cx, payload| {
                let args = *payload.downcast::<#args_struct_name>().unwrap();
                #execute_fn_name(cx, args);
            },
        );
    }
}

fn is_app_param(pat_type: &syn::PatType) -> bool {
    if let Type::Path(type_path) = &*pat_type.ty
        && let Some(seg) = type_path.path.segments.last()
    {
        return seg.ident == "App";
    }
    false
}

fn is_result_unit(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };
    let Some(last) = path.segments.last() else {
        return false;
    };
    if last.ident != "Result" {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return false;
    };
    let mut iter = args.args.iter();
    let Some(syn::GenericArgument::Type(Type::Tuple(tuple))) = iter.next() else {
        return false;
    };
    if !tuple.elems.is_empty() {
        return false;
    }
    matches!(iter.next(), Some(GenericArgument::Type(_)))
}

fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}
