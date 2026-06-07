use std::collections::HashMap;

use quote::format_ident;
use syn::DeriveInput;

use crate::parser::{
    self, get_bits_attr, get_len_bytes_attr, make_mod_name, BinSardAttributes,
    BinSardFieldAttribute, EnumVariant, FieldInfo, FieldKind, Fields, StructOrEnum,
};

fn gen_len_prefix_write(
    accessor: &proc_macro2::TokenStream,
    len_bytes: usize,
) -> proc_macro2::TokenStream {
    match len_bytes {
        1 => quote::quote! {
            let len = #accessor.len();
            if len > u8::MAX as usize {
                panic!("binsard: length prefix (len_bytes = 1) too large: {len}");
            }
            data.push(len as u8);
        },
        2 => quote::quote! {
            let len = #accessor.len();
            if len > u16::MAX as usize {
                panic!("binsard: length prefix (len_bytes = 2) too large: {len}");
            }
            data.extend_from_slice(&(len as u16).to_be_bytes());
        },
        4 => quote::quote! {
            let len = #accessor.len();
            if len > u32::MAX as usize {
                panic!("binsard: length prefix (len_bytes = 4) too large: {len}");
            }
            data.extend_from_slice(&(len as u32).to_be_bytes());
        },
        _ => unreachable!(),
    }
}

fn gen_encode_stmt(
    kind: &FieldKind,
    accessor: &proc_macro2::TokenStream,
    optional: bool,
    needs_deref: bool,
    bits_attr: Option<usize>,
    len_bytes_attr: Option<usize>,
) -> Vec<proc_macro2::TokenStream> {
    if optional {
        let inner_stmts = gen_encode_inner(kind, &quote::quote! { val }, true, bits_attr, len_bytes_attr);
        return vec![quote::quote! {
            if let Some(val) = &#accessor {
                helper.write_bit(true);
                #(#inner_stmts)*
            } else {
                helper.write_bit(false);
            }
        }];
    }
    gen_encode_inner(kind, accessor, needs_deref, bits_attr, len_bytes_attr)
}

fn gen_encode_inner(
    kind: &FieldKind,
    accessor: &proc_macro2::TokenStream,
    needs_deref: bool,
    bits_attr: Option<usize>,
    len_bytes_attr: Option<usize>,
) -> Vec<proc_macro2::TokenStream> {
    match kind {
        FieldKind::Bool => {
            if needs_deref {
                vec![quote::quote! { helper.write_bit(*#accessor); }]
            } else {
                vec![quote::quote! { helper.write_bit(#accessor); }]
            }
        }
        FieldKind::String => {
            let lb = len_bytes_attr.unwrap_or(1);
            let prefix = gen_len_prefix_write(accessor, lb);
            vec![quote::quote! {
                #prefix
                data.extend_from_slice(#accessor.as_bytes());
            }]
        }
        FieldKind::Primitive { .. } => {
            if let Some(bits) = bits_attr {
                if needs_deref {
                    vec![quote::quote! { helper.write_partly(*#accessor as u64, #bits); }]
                } else {
                    vec![quote::quote! { helper.write_partly(#accessor as u64, #bits); }]
                }
            } else {
                vec![quote::quote! {
                    data.extend_from_slice(&#accessor.to_be_bytes());
                }]
            }
        }
        FieldKind::Vec { inner_type } => {
            if inner_type == "u8" {
                let lb = len_bytes_attr.unwrap_or(2);
                let prefix = gen_len_prefix_write(accessor, lb);
                vec![quote::quote! {
                    #prefix
                    data.extend_from_slice(&#accessor);
                }]
            } else {
                let lb = len_bytes_attr.unwrap_or(1);
                let prefix = gen_len_prefix_write(accessor, lb);
                vec![quote::quote! {
                    #prefix
                    for item in #accessor.iter() {
                        data.append(&mut item.encode_internal(helper));
                    }
                }]
            }
        }
        FieldKind::Array { .. } => {
            // Arrays are encoded elementweise über Encode/EncodeInternal.
            vec![quote::quote! {
                for item in #accessor.iter() {
                    data.append(&mut item.encode_internal(helper));
                }
            }]
        }
        FieldKind::StructOrEnum { .. } => {
            vec![quote::quote! {
                data.append(&mut #accessor.encode_internal(helper));
            }]
        }
    }
}

fn gen_fields_encode(
    fields: &[FieldInfo],
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
    accessor_fn: impl Fn(usize, &FieldInfo) -> (proc_macro2::TokenStream, bool),
    variant_prefix: Option<&syn::Ident>,
) -> Vec<proc_macro2::TokenStream> {
    let mut stmts = vec![];
    for (i, field) in fields.iter().enumerate() {
        let bits = get_bits_attr(field.name.as_ref(), field_attrs, variant_prefix);
        let len_bytes = get_len_bytes_attr(field.name.as_ref(), field_attrs, variant_prefix);
        let (accessor, needs_deref) = accessor_fn(i, field);
        stmts.extend(gen_encode_stmt(&field.kind, &accessor, field.optional, needs_deref, bits, len_bytes));
    }
    stmts
}

fn struct_accessor(i: usize, field: &FieldInfo) -> (proc_macro2::TokenStream, bool) {
    field.name.as_ref().map_or_else(
        || {
            let idx = syn::Index::from(i);
            (quote::quote! { self.#idx }, false)
        },
        |name| (quote::quote! { self.#name }, false),
    )
}

fn gen_struct_encode(
    name: &syn::Ident,
    fields: &Fields,
    reserve_bits: usize,
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
) -> proc_macro::TokenStream {
    let mod_name = make_mod_name(name, "encode");

    match fields {
        Fields::Named { fields } | Fields::Unnamed { fields } => {
            let setters = gen_fields_encode(fields, field_attrs, struct_accessor, None);
            quote::quote! {
                mod #mod_name {
                    use super::*;
                    use binsard::EncodeHelper;
                    impl Encode for #name {
                        fn encode(&self) -> Vec<u8> {
                            let mut helper_init = EncodeHelper::default();
                            let mut helper = &mut helper_init;
                            helper.reserve_bits(#reserve_bits);
                            let mut data = vec![];
                            #(#setters)*
                            data.append(&mut helper_init.finish());
                            data
                        }
                    }
                    impl #name {
                        pub(crate) fn encode_internal(&self, helper: &mut EncodeHelper) -> Vec<u8> {
                            let mut data = vec![];
                            #(#setters)*
                            data
                        }
                    }
                }
            }
            .into()
        }
        Fields::Unit => {
            quote::quote! {
                mod #mod_name {
                    use super::*;
                    use binsard::EncodeHelper;
                    impl Encode for #name {
                        fn encode(&self) -> Vec<u8> { vec![] }
                    }
                    impl #name {
                        pub(crate) fn encode_internal(&self, _helper: &mut EncodeHelper) -> Vec<u8> { vec![] }
                    }
                }
            }
            .into()
        }
    }
}

fn gen_enum_variant_encode(
    enum_name: &syn::Ident,
    variant: &EnumVariant,
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let vname = &variant.variant;
    match &variant.fields {
        Fields::Named { fields } => {
            let field_names: Vec<_> = fields.iter().map(|f| f.name.as_ref().unwrap()).collect();
            let enum_accessor = |_i: usize, field: &FieldInfo| -> (proc_macro2::TokenStream, bool) {
                let name = field.name.as_ref().unwrap();
                (quote::quote! { #name }, true)
            };
            let stmts = gen_fields_encode(fields, field_attrs, enum_accessor, Some(vname));
            let setter = quote::quote! {
                #enum_name::#vname { #(#field_names),* } => {
                    #(#stmts)*
                },
            };
            let tag = quote::quote! { #enum_name::#vname { .. } };
            (setter, tag)
        }
        Fields::Unnamed { fields } => {
            let bindings: Vec<_> = (0..fields.len())
                .map(|i| format_ident!("_{}", i))
                .collect();
            let enum_accessor = |i: usize, _field: &FieldInfo| -> (proc_macro2::TokenStream, bool) {
                let binding = format_ident!("_{}", i);
                (quote::quote! { #binding }, true)
            };
            let stmts = gen_fields_encode(fields, field_attrs, enum_accessor, Some(vname));
            let setter = quote::quote! {
                #enum_name::#vname(#(#bindings),*) => {
                    #(#stmts)*
                },
            };
            let tag = quote::quote! { #enum_name::#vname(..) };
            (setter, tag)
        }
        Fields::Unit => {
            let setter = quote::quote! {
                #enum_name::#vname => {},
            };
            let tag = quote::quote! { #enum_name::#vname };
            (setter, tag)
        }
    }
}

pub fn impl_encode_trait(mut ast: DeriveInput) -> proc_macro::TokenStream {
    let parsed = match StructOrEnum::parse(&ast) {
        Ok(parsed) => parsed,
        Err(err) => return err.to_compile_error().into(),
    };
    let BinSardAttributes { bits, reserve_bits, field_attrs } = match parser::parse_attributes(&mut ast) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };

    match parsed {
        StructOrEnum::Struct { name, fields } => {
            gen_struct_encode(&name, &fields, reserve_bits, &field_attrs)
        }
        StructOrEnum::Enum { name, variants } => {
            let max_variants = 1usize.checked_shl(bits as u32).unwrap_or(usize::MAX);
            if variants.len() > max_variants {
                let count = variants.len();
                return syn::Error::new(
                    name.span(),
                    format!(
                        "enum `{name}` has {count} variants but #[binsard(bits = {bits})] supports at most {max_variants}"
                    ),
                ).to_compile_error().into();
            }

            let mut setters = Vec::new();
            let mut tags = Vec::new();
            for (idx, variant) in variants.iter().enumerate() {
                let (setter, tag_pattern) = gen_enum_variant_encode(&name, variant, &field_attrs);
                setters.push(setter);
                tags.push(quote::quote! { #tag_pattern => #idx, });
            }

            let mod_name = make_mod_name(&name, "encode");
            quote::quote! {
                mod #mod_name {
                    use super::*;
                    use binsard::EncodeHelper;
                    impl Encode for #name {
                        fn encode(&self) -> Vec<u8> {
                            let mut helper_init = EncodeHelper::default();
                            let mut helper = &mut helper_init;
                            helper.reserve_bits(#reserve_bits);
                            let mut data = vec![];
                            let tag = match self { #(#tags)* };
                            helper.write_partly(tag as u64, #bits);
                            match self { #(#setters)* };
                            data.append(&mut helper_init.finish());
                            data
                        }
                    }
                    impl #name {
                        pub(crate) fn encode_internal(&self, helper: &mut EncodeHelper) -> Vec<u8> {
                            let mut data = vec![];
                            let tag = match self { #(#tags)* };
                            helper.write_partly(tag as u64, #bits);
                            match self { #(#setters)* };
                            data
                        }
                    }
                }
            }
            .into()
        }
    }
}
