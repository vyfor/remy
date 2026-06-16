use proc_macro::TokenStream;

mod component;
mod intent;
mod store;

#[proc_macro_attribute]
pub fn store(_attr: TokenStream, input: TokenStream) -> TokenStream {
    store::expand_store(input.into()).into()
}

#[proc_macro_attribute]
pub fn component(attr: TokenStream, input: TokenStream) -> TokenStream {
    component::expand_component(attr.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn intent(_attr: TokenStream, input: TokenStream) -> TokenStream {
    intent::expand_intent(input.into()).into()
}
