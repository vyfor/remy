use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{FnArg, ItemFn, Pat, Path, Type, parse2};

enum CxKind {
    Value,
    Ref,
    MutRef,
}

fn cx_kind(ty: &Type) -> Option<CxKind> {
    match ty {
        Type::Path(path) if is_ident(&path.path, "Cx") => Some(CxKind::Value),
        Type::Reference(reference) => match reference.elem.as_ref() {
            Type::Path(path) if is_ident(&path.path, "Cx") => {
                if reference.mutability.is_some() {
                    Some(CxKind::MutRef)
                } else {
                    Some(CxKind::Ref)
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn is_ident(path: &Path, ident: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == ident)
}

fn is_id_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => is_ident(&path.path, "Id"),
        Type::ImplTrait(impl_trait) => impl_trait.bounds.iter().any(|bound| {
            let syn::TypeParamBound::Trait(tr) = bound else {
                return false;
            };
            let Some(seg) = tr.path.segments.last() else {
                return false;
            };
            if seg.ident != "Into" {
                return false;
            }
            let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
                return false;
            };
            matches!(
                args.args.first(),
                Some(syn::GenericArgument::Type(Type::Path(p))) if is_ident(&p.path, "Id")
            )
        }),
        _ => false,
    }
}

fn make_cx_binding(name: &Ident, kind: CxKind) -> TokenStream {
    match kind {
        CxKind::Value => quote! {
            let #name = ::remy::Cx::new(__owner_id);
        },
        CxKind::Ref => quote! {
            let __remy_owner_cx = ::remy::Cx::new(__owner_id);
            let #name = &__remy_owner_cx;
        },
        CxKind::MutRef => quote! {
            let mut __remy_owner_cx = ::remy::Cx::new(__owner_id);
            let #name = &mut __remy_owner_cx;
        },
    }
}

struct ParamAnalysis {
    cx_binding: Option<TokenStream>,
    id_param: Option<(Ident, Type)>,
    props: Vec<(Ident, Type)>,
}

fn analyze_params(func: &mut ItemFn) -> Result<ParamAnalysis, syn::Error> {
    let mut cx_binding = None;
    let mut id_param = None;
    let mut props = Vec::new();

    let inputs = std::mem::take(&mut func.sig.inputs);
    let mut kept_inputs = syn::punctuated::Punctuated::new();

    for (index, arg) in inputs.into_iter().enumerate() {
        let FnArg::Typed(pat_type) = &arg else {
            kept_inputs.push(arg);
            continue;
        };

        let Pat::Ident(pat_ident) = &*pat_type.pat else {
            kept_inputs.push(arg);
            continue;
        };
        let name = pat_ident.ident.clone();

        if let Some(kind) = cx_kind(&pat_type.ty) {
            if index != 0 {
                return Err(syn::Error::new_spanned(
                    &arg,
                    "Cx must be the first parameter",
                ));
            }
            cx_binding = Some(make_cx_binding(&name, kind));
            continue;
        }

        if is_id_type(&pat_type.ty) {
            if id_param.is_some() {
                return Err(syn::Error::new_spanned(
                    &arg,
                    "component takes one Id at most",
                ));
            }
            id_param = Some((name, (*pat_type.ty).clone()));
            kept_inputs.push(arg);
            continue;
        }

        props.push((name, (*pat_type.ty).clone()));
        kept_inputs.push(arg);
    }

    func.sig.inputs = kept_inputs;

    Ok(ParamAnalysis {
        cx_binding,
        id_param,
        props,
    })
}

pub fn expand_component(attr: TokenStream, input: TokenStream) -> TokenStream {
    let no_cache = if attr.is_empty() {
        false
    } else {
        let attr_str = attr.to_string();
        if attr_str == "no_cache" {
            true
        } else {
            return syn::Error::new_spanned(attr, "#[component] only accepts `no_cache`")
                .to_compile_error();
        }
    };

    let mut func: ItemFn = match parse2(input) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    let analysis = match analyze_params(&mut func) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };
    let cx_binding = analysis.cx_binding.unwrap_or_else(|| {
        quote! {
            let cx = ::remy::Cx::new(__owner_id);
        }
    });

    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let existing_stmts = &func.block.stmts;

    if !no_cache {
        func.sig.output = syn::parse2(quote! { -> ::remy::Instance }).unwrap();
    }

    let has_id = analysis.id_param.is_some();
    let prop_count = analysis.props.len();

    let new_body = if no_cache {
        quote! {
            {
                static __OWNER_ID: ::std::sync::OnceLock<::remy::core::tracking::OwnerId> =
                    ::std::sync::OnceLock::new();
                let __owner_id = *__OWNER_ID.get_or_init(|| {
                    ::remy::core::runtime::register_owner(
                        concat!(module_path!(), "::", #fn_name_str)
                    )
                });
                let __prev_active = ::remy::core::runtime::set_active_owner(Some(__owner_id));
                struct __FrameworkOwnerGuard(::std::option::Option<::remy::core::tracking::OwnerId>);
                impl ::std::ops::Drop for __FrameworkOwnerGuard {
                    fn drop(&mut self) {
                        ::remy::core::runtime::set_active_owner(self.0);
                    }
                }
                let __active_guard = __FrameworkOwnerGuard(__prev_active);
                #cx_binding
                let __res = { #(#existing_stmts)* };
                ::std::mem::drop(__active_guard);
                ::remy::core::CachedView::new(__owner_id, __res)
            }
        }
    } else if !has_id && prop_count == 0 {
        quote! {
            {
                static __OWNER_ID: ::std::sync::OnceLock<::remy::core::tracking::OwnerId> =
                    ::std::sync::OnceLock::new();
                let __owner_id = *__OWNER_ID.get_or_init(|| {
                    ::remy::core::runtime::register_owner(
                        concat!(module_path!(), "::", #fn_name_str)
                    )
                });

                static __SETUP: ::std::sync::OnceLock<
                    ::std::sync::Arc<dyn ::remy::View + ::std::marker::Send + ::std::marker::Sync>,
                > = ::std::sync::OnceLock::new();

                let __view = __SETUP.get_or_init(|| {
                    let __prev_active = ::remy::core::runtime::set_active_owner(Some(__owner_id));
                    struct __Guard(::std::option::Option<::remy::core::tracking::OwnerId>);
                    impl ::std::ops::Drop for __Guard {
                        fn drop(&mut self) {
                            ::remy::core::runtime::set_active_owner(self.0);
                        }
                    }
                    let __active_guard = __Guard(__prev_active);

                    #cx_binding

                    let __res = { #(#existing_stmts)* };

                    ::std::mem::drop(__active_guard);
                    ::std::sync::Arc::new(::remy::core::cached::CachedView::new(__owner_id, __res))
                });

                ::remy::Instance::__new_raw(__owner_id, ::std::sync::Arc::clone(__view))
            }
        }
    } else if !has_id && prop_count > 0 {
        let prop_names = analysis.props.iter().map(|(id, _)| id);

        quote! {
            {
                static __OWNER_ID: ::std::sync::OnceLock<::remy::core::tracking::OwnerId> =
                    ::std::sync::OnceLock::new();
                let __owner_id = *__OWNER_ID.get_or_init(|| {
                    ::remy::core::runtime::register_owner(
                        concat!(module_path!(), "::", #fn_name_str)
                    )
                });

                static __SETUP: ::std::sync::Mutex<
                    ::std::option::Option<(
                        u64,
                        ::std::sync::Arc<dyn ::remy::View + ::std::marker::Send + ::std::marker::Sync>,
                    )>,
                > = ::std::sync::Mutex::new(None);

                let __incoming_hash = ::remy::hash_props(&(#(#prop_names,)*));

                let mut __cache = __SETUP.lock().unwrap();

                if let ::std::option::Option::Some((__cached_hash, __view)) = __cache.as_ref() {
                    if *__cached_hash == __incoming_hash {
                        return ::remy::Instance::__new_raw(__owner_id, ::std::sync::Arc::clone(__view));
                    }
                }

                let __prev_active = ::remy::core::runtime::set_active_owner(Some(__owner_id));
                struct __Guard(::std::option::Option<::remy::core::tracking::OwnerId>);
                impl ::std::ops::Drop for __Guard {
                    fn drop(&mut self) {
                        ::remy::core::runtime::set_active_owner(self.0);
                    }
                }
                let __active_guard = __Guard(__prev_active);

                #cx_binding

                let __res = { #(#existing_stmts)* };

                ::std::mem::drop(__active_guard);

                let __view: ::std::sync::Arc<dyn ::remy::View + ::std::marker::Send + ::std::marker::Sync> =
                    ::std::sync::Arc::new(::remy::core::cached::CachedView::new(__owner_id, __res));

                *__cache = ::std::option::Option::Some((__incoming_hash, ::std::sync::Arc::clone(&__view)));

                ::remy::Instance::__new_raw(__owner_id, __view)
            }
        }
    } else if has_id && prop_count == 0 {
        let (id_name, _) = analysis.id_param.as_ref().unwrap();
        quote! {
            {
                static __OWNER_ID: ::std::sync::OnceLock<::remy::core::tracking::OwnerId> =
                    ::std::sync::OnceLock::new();
                let __base_owner_id = *__OWNER_ID.get_or_init(|| {
                    ::remy::core::runtime::register_owner(
                        concat!(module_path!(), "::", #fn_name_str)
                    )
                });

                static __INSTANCES: ::std::sync::Mutex<
                    ::std::collections::HashMap<
                        ::remy::Id,
                        (
                            ::remy::core::tracking::OwnerId,
                            ::std::sync::Arc<dyn ::remy::View + ::std::marker::Send + ::std::marker::Sync>,
                        ),
                    >,
                > = ::std::sync::Mutex::new(::std::collections::HashMap::new());

                let __id: ::remy::Id = #id_name.into();
                let mut __instances = __INSTANCES.lock().unwrap();

                if let ::std::option::Option::Some((__owner_id, __view)) = __instances.get(&__id) {
                    return ::remy::Instance::__new_raw(*__owner_id, ::std::sync::Arc::clone(__view));
                }

                let __owner_id = ::remy::core::runtime::spawn_owner(
                    concat!(module_path!(), "::", #fn_name_str)
                );

                let __prev_active = ::remy::core::runtime::set_active_owner(Some(__owner_id));
                struct __Guard(::std::option::Option<::remy::core::tracking::OwnerId>);
                impl ::std::ops::Drop for __Guard {
                    fn drop(&mut self) {
                        ::remy::core::runtime::set_active_owner(self.0);
                    }
                }
                let __active_guard = __Guard(__prev_active);

                #cx_binding

                let __res = { #(#existing_stmts)* };

                ::std::mem::drop(__active_guard);

                let __view: ::std::sync::Arc<dyn ::remy::View + ::std::marker::Send + ::std::marker::Sync> =
                    ::std::sync::Arc::new(::remy::core::cached::CachedView::new(__owner_id, __res));

                __instances.insert(__id, (__owner_id, ::std::sync::Arc::clone(&__view)));

                ::remy::Instance::__new_raw(__owner_id, __view)
            }
        }
    } else {
        let (id_name, _) = analysis.id_param.as_ref().unwrap();
        let prop_names = analysis.props.iter().map(|(id, _)| id);

        quote! {
            {
                static __OWNER_ID: ::std::sync::OnceLock<::remy::core::tracking::OwnerId> =
                    ::std::sync::OnceLock::new();
                let __base_owner_id = *__OWNER_ID.get_or_init(|| {
                    ::remy::core::runtime::register_owner(
                        concat!(module_path!(), "::", #fn_name_str)
                    )
                });

                static __INSTANCES: ::std::sync::Mutex<
                    ::std::collections::HashMap<
                        ::remy::Id,
                        (
                            ::remy::core::tracking::OwnerId,
                            ::std::option::Option<(
                                u64,
                                ::std::sync::Arc<dyn ::remy::View + ::std::marker::Send + ::std::marker::Sync>,
                            )>,
                        ),
                    >,
                > = ::std::sync::Mutex::new(::std::collections::HashMap::new());

                let __id: ::remy::Id = #id_name.into();
                let __incoming_hash = ::remy::hash_props(&(#(#prop_names,)*));

                let mut __instances = __INSTANCES.lock().unwrap();

                let (__owner_id, __cached) = __instances
                    .entry(__id)
                    .or_insert_with(|| {
                        (::remy::core::runtime::spawn_owner(
                            concat!(module_path!(), "::", #fn_name_str)
                        ), None)
                    });

                if let ::std::option::Option::Some((__cached_hash, __view)) = __cached {
                    if *__cached_hash == __incoming_hash {
                        return ::remy::Instance::__new_raw(*__owner_id, ::std::sync::Arc::clone(__view));
                    }
                }

                let __prev_active = ::remy::core::runtime::set_active_owner(Some(*__owner_id));
                struct __Guard(::std::option::Option<::remy::core::tracking::OwnerId>);
                impl ::std::ops::Drop for __Guard {
                    fn drop(&mut self) {
                        ::remy::core::runtime::set_active_owner(self.0);
                    }
                }
                let __active_guard = __Guard(__prev_active);

                #cx_binding

                let __res = { #(#existing_stmts)* };

                ::std::mem::drop(__active_guard);

                let __view: ::std::sync::Arc<dyn ::remy::View + ::std::marker::Send + ::std::marker::Sync> =
                    ::std::sync::Arc::new(::remy::core::cached::CachedView::new(*__owner_id, __res));

                *__cached = ::std::option::Option::Some((__incoming_hash, ::std::sync::Arc::clone(&__view)));

                ::remy::Instance::__new_raw(*__owner_id, __view)
            }
        }
    };

    func.block = syn::parse2(new_body).unwrap();

    let component_name = quote! { concat!(module_path!(), "::", #fn_name_str) };
    let reg_static_name = quote::format_ident!("__REG_OWNER_{}", fn_name);

    quote! {
        #[allow(non_snake_case)]
        #func

        #[::remy::linkme::distributed_slice(::remy::core::OWNER_REGISTRY)]
        #[linkme(crate = ::remy::linkme)]
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #reg_static_name: &'static str = #component_name;
    }
}
