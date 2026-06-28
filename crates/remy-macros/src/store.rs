use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprCall, ItemFn, Local, Pat, Stmt, Type, parse2};

fn type_name_is(ty: &Type, name: &str) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        return seg.ident == name;
    }
    false
}

fn is_memo_type(ty: &Type) -> bool {
    type_name_is(ty, "Memo")
}

fn is_query_type(ty: &Type) -> bool {
    type_name_is(ty, "Query")
}

fn is_state_type(ty: &Type) -> bool {
    type_name_is(ty, "State")
}

pub fn expand_store(input: TokenStream) -> TokenStream {
    let func: ItemFn = match parse2(input) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    let vis = &func.vis;
    let fn_name = &func.sig.ident;
    let mod_name = quote::format_ident!("__store_{}", fn_name);
    let mod_name_str = func.sig.ident.to_string();

    let mut slot_declarations = Vec::new();
    let mut slot_inits = Vec::new();
    let mut sync_body = None;

    for stmt in &func.block.stmts {
        match stmt {
            Stmt::Local(local) => {
                if let Some((var_name, var_type, init_expr)) = parse_let(local) {
                    if is_state_type(&var_type) {
                        let var_name_str = var_name.to_string();
                        let slot_id_expr = quote! {
                            ::remy::core::const_slot_id(
                                concat!(module_path!(), "::", #mod_name_str),
                                #var_name_str
                            )
                        };
                        let reg_static_name = quote::format_ident!("__SLOT_REG_{}", var_name);

                        slot_declarations.push(quote! {
                            #[allow(non_upper_case_globals)]
                            pub static #var_name: #var_type =
                                <#var_type>::new(#slot_id_expr);

                            #[::remy::linkme::distributed_slice(::remy::core::SLOT_REGISTRY)]
                            #[linkme(crate = ::remy::linkme)]
                            #[doc(hidden)]
                            #[allow(non_upper_case_globals)]
                            static #reg_static_name: (&'static str, &'static str, ::remy::core::SlotId) = (
                                concat!(module_path!(), "::", #mod_name_str),
                                #var_name_str,
                                #slot_id_expr,
                            );
                        });

                        slot_inits.push(quote! {
                            ::remy::core::install(&#var_name, #init_expr, __cx.clone());
                        });
                    } else if is_resource_type(&var_type) {
                        slot_declarations.push(quote! {
                            #[allow(non_upper_case_globals)]
                            pub static #var_name: #var_type =
                                ::remy::core::Resource::uninit();
                        });
                        slot_inits.push(quote! {
                            ::remy::core::install(&#var_name, #init_expr, __cx.clone());
                        });
                    } else if is_query_type(&var_type) {
                        slot_declarations.push(quote! {
                            #[allow(non_upper_case_globals)]
                            pub static #var_name: #var_type =
                                ::remy::core::Query::uninit();
                        });
                        slot_inits.push(quote! {
                            ::remy::core::install(&#var_name, #init_expr, __cx.clone());
                        });
                    } else if is_memo_type(&var_type) {
                        slot_declarations.push(quote! {
                            #[allow(non_upper_case_globals)]
                            pub static #var_name: #var_type =
                                ::remy::core::Memo::uninit();
                        });
                        slot_inits.push(quote! {
                            ::remy::core::install(&#var_name, #init_expr, __cx.clone());
                        });
                    } else {
                        let var_name_str = var_name.to_string();
                        let slot_id_expr = quote! {
                            ::remy::core::const_slot_id(
                                concat!(module_path!(), "::", #mod_name_str),
                                #var_name_str
                            )
                        };
                        let reg_static_name = quote::format_ident!("__SLOT_REG_{}", var_name);

                        slot_declarations.push(quote! {
                            #[allow(non_upper_case_globals)]
                            pub static #var_name: ::remy::core::State<#var_type> =
                                ::remy::core::State::new(#slot_id_expr);

                            #[::remy::linkme::distributed_slice(::remy::core::SLOT_REGISTRY)]
                            #[linkme(crate = ::remy::linkme)]
                            #[doc(hidden)]
                            #[allow(non_upper_case_globals)]
                            static #reg_static_name: (&'static str, &'static str, ::remy::core::SlotId) = (
                                concat!(module_path!(), "::", #mod_name_str),
                                #var_name_str,
                                #slot_id_expr,
                            );
                        });

                        slot_inits.push(quote! {
                            let __initial: #var_type = #init_expr;
                            ::remy::core::runtime::allocate_slot(#var_name.id(), __initial);
                        });
                    }
                }
            }
            Stmt::Expr(expr, _semi) => {
                if let Some(body) = extract_sync_block(expr) {
                    sync_body = Some(body);
                }
            }
            _ => {}
        }
    }

    let sync_init = sync_body
        .map(|body| {
            quote! {
                {
                    let app = ::remy::core::App::with_owner(__store_owner());
                    #body
                }
            }
        })
        .unwrap_or_default();

    quote! {
        #vis mod #mod_name {
            use super::*;

            static __STORE_OWNER: ::std::sync::Mutex<::std::option::Option<::std::sync::Arc<::remy::core::Owner>>> =
                ::std::sync::Mutex::new(::std::option::Option::None);

            fn __store_owner() -> ::std::sync::Arc<::remy::core::Owner> {
                let mut owner = __STORE_OWNER
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                owner
                    .get_or_insert_with(|| ::std::sync::Arc::new(::remy::core::Owner::new()))
                    .clone()
            }
        #(#slot_declarations)*

            #[doc(hidden)]
            pub fn __init_store(__cx: ::remy::core::App) {
                #(#slot_inits)*
                #sync_init
            }

            #[::remy::linkme::distributed_slice(::remy::core::STORE_REGISTRY)]
            #[linkme(crate = ::remy::linkme)]
            #[doc(hidden)]
            static __STORE_REG: fn(::remy::core::App) = __init_store;
        }

        #vis use #mod_name as #fn_name;
    }
}

fn parse_let(local: &Local) -> Option<(syn::Ident, Type, Expr)> {
    let pat = &local.pat;
    let init = local.init.as_ref()?;
    let expr = &*init.expr;

    if let Pat::Type(pat_type) = pat
        && let Pat::Ident(pat_ident) = &*pat_type.pat
    {
        let var_name = pat_ident.ident.clone();
        let var_type = *pat_type.ty.clone();
        return Some((var_name, var_type, expr.clone()));
    }

    None
}

fn is_resource_type(ty: &Type) -> bool {
    type_name_is(ty, "Resource")
}

fn extract_sync_block(expr: &Expr) -> Option<TokenStream> {
    if let Expr::Call(call) = expr
        && is_sync_call(call)
        && let Some(arg) = call.args.first()
        && let Expr::Closure(closure) = arg
    {
        let body = &closure.body;
        return Some(quote! { #body });
    }
    None
}

fn is_sync_call(call: &ExprCall) -> bool {
    let func = &call.func;
    let func_str = quote!(#func).to_string();
    func_str.contains("sync")
}
