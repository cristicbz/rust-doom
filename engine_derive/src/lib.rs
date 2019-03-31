extern crate proc_macro;

use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, parse_quote, Data, DataStruct, DeriveInput, ExprPath, Fields};

#[proc_macro_derive(DependenciesFrom)]
pub fn dependencies_from_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    do_impl(parse_quote!(::engine), input)
}

#[proc_macro_derive(InternalDependenciesFrom)]
pub fn internal_dependencies_from_derive(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    do_impl(parse_quote!(crate), input)
}

fn do_impl(engine: ExprPath, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ast: DeriveInput = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let context_t = Ident::new("ContextT", Span::call_site());
    let indices_t = Ident::new("IndicesT", Span::call_site());
    let rest = Ident::new("rest", Span::call_site());

    let generics = &mut ast.generics;
    let input_generics = generics.clone();
    let (_, ty_generics, _) = input_generics.split_for_impl();

    let mut type_list_type = quote!(#engine::type_list::Nil);
    let mut deconstruct = quote! { let _ = #rest; };
    let mut construct = quote!();

    match ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            for field in &fields.named {
                let name = field.ident.as_ref().unwrap();
                let ty = &field.ty;
                construct = parse_quote!( #name, #construct );
                deconstruct = parse_quote! {
                    let #engine::type_list::Cons { head: #name, tail: rest } = rest;
                    #deconstruct
                };
                type_list_type = parse_quote!(#engine::type_list::Cons<#ty, #type_list_type>);
            }
        }
        _ => panic!("Unsupported data type for DependenciesFrom."),
    }

    generics.params.push(parse_quote!(#indices_t));
    generics.params.push(parse_quote!(
        #context_t : #engine::type_list::PluckList<#type_list_type, #indices_t>
    ));
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let generated = quote! {
        impl #impl_generics #engine::DependenciesFrom<#context_t, #indices_t> for #name #ty_generics #where_clause {
            fn dependencies_from(context: #context_t) -> Self {
                let (rest, _) = context.pluck_list();
                #deconstruct
                Self { #construct }
            }
        }
    };

    generated.into()
}
