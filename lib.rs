/*
Copyright 2024 Benjamin Richcreek

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/
//! # The Struct-inator 3000!
//!
//! A procedural macro library for allowing conversion from iterators to user-defined
//! [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html)s.
//! 
//! This library does so by implementing a procedural macro, [`macro@iter_convertable`] for [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html) definitions that automatically implements [`structinator_traits::SpecifyCreatableStruct`]
//! for the defined [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html). 
//! 
//! For more information about how [`macro@iter_convertable`] implements [`SpecifyCreatableStruct`](structinator_traits::SpecifyCreatableStruct), visit the macro's [documentation](macro@iter_convertable)
//! 
//! For more information about how an implementation of [`SpecifyCreatableStruct`](structinator_traits::SpecifyCreatableStruct) allows for easy conversion between [`Iterator`]s and [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html)s, visit the documentation of [`structinator_traits`]
use syn;
use quote::quote;
use proc_macro::TokenStream;



///Attribute for structs that can be built from an iterator.
/// This attribute must be attached to a [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html) definition
/// 
/// # Argument
/// 
///The argument passed to the attribute must be a type, and each unique type of the fields in the target [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html) must implement 
///[`From`] or [`TryFrom`] the passed type
/// 
/// # Effects
/// This attribute implements the trait [`structinator_traits::SpecifyCreatableStruct`] with [`InnerIteratorType`](structinator_traits::SpecifyCreatableStruct::InnerIteratorType) set to
/// the argument passed to this attribute. 
/// 
/// The generated function,  [`create_struct`](structinator_traits::SpecifyCreatableStruct::create_struct), will be implemented using a [`HashMap<String,InnerIteratorType>`](std::collections::HashMap), which will store the first `N` values from the iterator, where `N` is the number of fields in the [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html) this attribute is attached to,
/// and then be assign the values in that [`HashMap`](std::collections::HashMap) to corresponding fields in the [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html), as determined by a stringification of the field's name. 
/// 
/// The passed value will then be unwrapped from the [`InnerIteratorType`](structinator_traits::SpecifyCreatableStruct::InnerIteratorType) to the type of the struct, [`panic`](https://doc.rust-lang.org/1.58.1/core/macro.panic.html)king if the conversion fails.
/// 
/// In other words, if the field definition looks like this:
/// ```no_run
/// value_name: u16,
/// ```
/// Then the corresponding field in the struct literal generated by [`create_struct`](structinator_traits::SpecifyCreatableStruct::create_struct) would look something like this (with error messages removed)
/// ```no_run
/// value_name = <u16 as TryFrom>::try_from(hash_map.remove("value_name").unwrap()).unwrap(),
/// ```
/// # Panics
/// This attribute will cause a [`panic`](https://doc.rust-lang.org/1.58.1/core/macro.panic.html) if attached to anything other than a [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html) definition
/// 
/// This attribute will implement [`SpecifyCreatableStruct`](structinator_traits::SpecifyCreatableStruct) in a manner that assumes the [`InnerIteratorType`](structinator_traits::SpecifyCreatableStruct::InnerIteratorType)
/// implements [`TryFrom`] for each unique type used in the fields of the target [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html)
/// 
/// If [`InnerIteratorType`](structinator_traits::SpecifyCreatableStruct::InnerIteratorType)'s type does not implement [`TryFrom`], or the conversion fails, this function will [`panic`](https://doc.rust-lang.org/1.58.1/core/macro.panic.html).
/// The recomended way to make sure [`TryFrom`] is always implemented, minimizing panics to only when the conversion itself fails, is to create an [`enum`](https://doc.rust-lang.org/1.58.1/std/keyword.enum.html) specifically for this purpose, with unique variants for each unique type used by the fields of the target [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html),
/// and add the attribute [`unique_try_froms()`](https://docs.rs/enum_unwrapper/0.1.2/enum_unwrapper/attr.unique_try_froms.html) to said [`enum`](https://doc.rust-lang.org/1.58.1/std/keyword.enum.html).
/// See [`enum_unwrapper`](https://docs.rs/enum_unwrapper/0.1.2/enum_unwrapper/index.html)'s documentation for detailed
/// instructions on how to do so.
/// 
/// The function will also [`panic`](https://doc.rust-lang.org/1.58.1/core/macro.panic.html) if the [`Iterator`] argument yields [`NamedField`](structinator_traits::NamedField)s with identical [`name`](structinator_traits::NamedField::name) values before providing enough values to fill the target [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html).
/// 
/// # Errors 
/// The generated implementation returns an [`Err`] containing a [`&'static str`](str) if the supplied [`Iterator`] returns [`None`] before yielding enough values to fill the target [`struct`](https://doc.rust-lang.org/1.58.1/std/keyword.struct.html).
/// # More Info
/// See [`SpecifyCreatableStruct`](structinator_traits::SpecifyCreatableStruct) documentation for more information & examples.
/// 
#[proc_macro_attribute]
pub fn iter_convertable(user_enum: TokenStream, user_structure: TokenStream) -> TokenStream {
    let inner_iterator_type = if let Ok(specific_enum) = syn::parse::<syn::Type>(user_enum) {
        specific_enum
    } else {
        panic!("Pass in the name of the enum that contains the values to be assigned to this structure.");
    };
    let base_structure: syn::ItemStruct = syn::parse(user_structure).expect("This attribute should only be attached to a struct definition");
    //to do: alter the value to ensure generics work
    let base_structure_borrow = &base_structure;
    let base_structure_name = &base_structure.ident;
    let fields = match base_structure.fields {
        syn::Fields::Named(ref field_list) => &field_list.named,
        //note to self: add this part when tuple_structinator is live
        //syn::Fields::Unnamed(_) => panic!("This library only converts from iterators to structs with named fields. consider using this library's sister library, tuple_structinator, instead"),
        _ => panic!("This library can only convert from iterator to structs with named fields"),
    };
    let fields_length = fields.len();
    let field_maker = |field: &syn::Field| -> syn::FieldValue {
        let name_copy = field.ident.as_ref().expect("All of the fields in a non-tuple struct should be named").clone();
        let name_string = name_copy.to_string();
        let field_type = field.ty.clone();
        syn::parse2(quote! {
            #name_copy: <#field_type as std::convert::TryFrom<#inner_iterator_type>>::try_from(value_storage.remove(#name_string).expect("The iterator passed to the create_struct function should yield values for every field in the base struct")).expect("The variant of InnerIteratorType passed to TryFrom should always succeed in conversion, but it failed unexpectedly")
        }).expect("An unexpected error occured. If the error persists, consider using simpler types with fewer generics")
    };
    let fields_iterator = fields.iter().map(field_maker);
    return quote! {
        #base_structure_borrow
        impl SpecifyCreatableStruct for #base_structure_name {
            type InnerIteratorType = #inner_iterator_type;
            type Error = &'static str;
            fn create_struct(seed_iterator: &mut dyn Iterator<Item = NamedField<Self::InnerIteratorType>>) -> Result<Self,&'static str> {
                let mut value_storage: std::collections::HashMap<String,Self::InnerIteratorType> = std::collections::HashMap::with_capacity(#fields_length);
                let mut looper: usize = 0;
                while looper < #fields_length {
                    let mut next_value_pair = if let Some(next) = seed_iterator.next() {
                        next
                    } else {
                        return Err("The given iterator should contain enough values to fill the implementing structure");
                    };
                    value_storage.insert(next_value_pair.name,next_value_pair.wrapped_value);
                    looper += 1;
                }
                Ok(#base_structure_name {
                    #(#fields_iterator),*
                })
            }
        }       
    }.into()
}