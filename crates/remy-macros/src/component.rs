use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, Path, Type, parse2};

fn cx_binding(func: &mut ItemFn) -> TokenStream {
    let Some(first) = func.sig.inputs.first().cloned() else {
        return quote! {};
    };

    let FnArg::Typed(pat_type) = first else {
        return quote! {};
    };
    let Pat::Ident(pat_ident) = &*pat_type.pat else {
        return quote! {};
    };

    let Some(cx_kind) = cx_type(&pat_type.ty) else {
        return quote! {};
    };

    let inputs = std::mem::take(&mut func.sig.inputs);
    let mut kept = syn::punctuated::Punctuated::new();
    for (idx, arg) in inputs.into_iter().enumerate() {
        if idx != 0 {
            kept.push(arg);
        }
    }
    func.sig.inputs = kept;

    let name = &pat_ident.ident;
    match cx_kind {
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

enum CxKind {
    Value,
    Ref,
    MutRef,
}

fn cx_type(ty: &Type) -> Option<CxKind> {
    match ty {
        Type::Path(path) if is_cx(&path.path) => Some(CxKind::Value),
        Type::Reference(reference) => match reference.elem.as_ref() {
            Type::Path(path) if is_cx(&path.path) => {
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

fn is_cx(path: &Path) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == "Cx")
}

pub fn expand_component(attr: TokenStream, input: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new_spanned(
            attr,
            "#[component] does not accept arguments; render dependencies are not tracked",
        )
        .to_compile_error();
    }

    let mut func: ItemFn = match parse2(input) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    let cx_binding = cx_binding(&mut func);
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();

    let existing_stmts = &func.block.stmts;

    let new_body = quote! {
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
            __res
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
