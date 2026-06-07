use std::collections::HashMap;

use quote::format_ident;
use syn::DeriveInput;

use crate::parser::{
    self, get_bits_attr, get_len_bytes_attr, make_mod_name, BinSardAttributes,
    BinSardFieldAttribute, EnumVariant, FieldInfo, FieldKind, Fields, StructOrEnum,
};

fn get_byte_size(ident: &syn::Ident) -> usize {
    match ident.to_string().as_str() {
        "u8" | "i8" => 1,
        "u16" | "i16" => 2,
        "u32" | "i32" | "f32" => 4,
        "u64" | "i64" | "f64" => 8,
        "u128" | "i128" => 16,
        other => unreachable!("binsard: unknown primitive size for `{other}`"),
    }
}

fn gen_len_prefix_read(len_bytes: usize) -> proc_macro2::TokenStream {
    // Hard upper bound on decoded lengths to avoid excessive allocations.
    // This is a conservative default and can be adjusted in the future if needed.
    let max_len = quote::quote! { 16 * 1024 * 1024usize };

    match len_bytes {
        1 => quote::quote! {
            if data_idx >= data.len() {
                return Err(binsard::DecodeError::UnexpectedEof);
            }
            let len = data[data_idx] as usize;
            data_idx += 1;
            if len > #max_len {
                return Err(binsard::DecodeError::LengthTooLarge);
            }
        },
        2 => quote::quote! {
            if data_idx + 2 > data.len() {
                return Err(binsard::DecodeError::UnexpectedEof);
            }
            let len = u16::from_be_bytes(data[data_idx..data_idx + 2].try_into().unwrap()) as usize;
            data_idx += 2;
            if len as usize > #max_len {
                return Err(binsard::DecodeError::LengthTooLarge);
            }
        },
        4 => quote::quote! {
            if data_idx + 4 > data.len() {
                return Err(binsard::DecodeError::UnexpectedEof);
            }
            let len = u32::from_be_bytes(data[data_idx..data_idx + 4].try_into().unwrap()) as usize;
            data_idx += 4;
            if len as usize > #max_len {
                return Err(binsard::DecodeError::LengthTooLarge);
            }
        },
        _ => unreachable!(),
    }
}

fn gen_decode_expr(
    kind: &FieldKind,
    bits_attr: Option<usize>,
    len_bytes_attr: Option<usize>,
) -> proc_macro2::TokenStream {
    match kind {
        FieldKind::Bool => {
            quote::quote! { helper.read_bit(data)? }
        }
        FieldKind::String => {
            let lb = len_bytes_attr.unwrap_or(1);
            let prefix_read = gen_len_prefix_read(lb);
            quote::quote! {{
                #prefix_read
                if data_idx + len > data.len() {
                    return Err(binsard::DecodeError::UnexpectedEof);
                }
                let res = String::from_utf8(data[data_idx..data_idx + len].to_vec())
                    .map_err(|_| binsard::DecodeError::InvalidUtf8)?;
                data_idx += len;
                res
            }}
        }
        FieldKind::Primitive { type_ident } => {
            if let Some(bits) = bits_attr {
                quote::quote! { helper.read_partly(data, #bits)? as #type_ident }
            } else {
                let size = get_byte_size(type_ident);
                quote::quote! {{
                    if data_idx + #size > data.len() {
                        return Err(binsard::DecodeError::UnexpectedEof);
                    }
                    let res = #type_ident::from_be_bytes(data[data_idx..data_idx + #size].try_into().unwrap());
                    data_idx += #size;
                    res
                }}
            }
        }
        FieldKind::Vec { inner_type } => {
            if inner_type == "u8" {
                let lb = len_bytes_attr.unwrap_or(2);
                let prefix_read = gen_len_prefix_read(lb);
                quote::quote! {{
                    #prefix_read
                    if data_idx + len > data.len() {
                        return Err(binsard::DecodeError::UnexpectedEof);
                    }
                    let vec = data[data_idx..data_idx + len].to_vec();
                    data_idx += len;
                    vec
                }}
            } else {
                let lb = len_bytes_attr.unwrap_or(1);
                let prefix_read = gen_len_prefix_read(lb);
                quote::quote! {{
                    #prefix_read
                    let mut vec = Vec::new();
                    for _ in 0..len {
                        let (len, item) = #inner_type::decode_internal(&data[data_idx..], helper)?;
                        vec.push(item);
                        data_idx += len;
                    }
                    vec
                }}
            }
        }
        FieldKind::Array { size, .. } => {
            quote::quote! {{
                let mut idx = data_idx;
                let mut items = Vec::with_capacity(#size);
                for _ in 0..#size {
                    let (consumed, item) = <_>::decode_internal(&data[idx..], helper)?;
                    idx += consumed;
                    items.push(item);
                }
                data_idx = idx;
                items.try_into().expect("binsard: failed to convert Vec<T> into array")
            }}
        }
        FieldKind::StructOrEnum { type_ident } => {
            quote::quote! {{
                let (len, item) = #type_ident::decode_internal(&data[data_idx..], helper)?;
                data_idx += len;
                item
            }}
        }
    }
}

fn gen_decode_field_expr(
    field: &FieldInfo,
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
    variant_prefix: Option<&syn::Ident>,
) -> proc_macro2::TokenStream {
    let bits = get_bits_attr(field.name.as_ref(), field_attrs, variant_prefix);
    let len_bytes = get_len_bytes_attr(field.name.as_ref(), field_attrs, variant_prefix);
    let inner_expr = gen_decode_expr(&field.kind, bits, len_bytes);

    if field.optional {
        quote::quote! {
            if helper.read_bit(data)? {
                Some(#inner_expr)
            } else {
                None
            }
        }
    } else {
        inner_expr
    }
}

fn gen_struct_decode(
    name: &syn::Ident,
    fields: &Fields,
    reserve_bits: usize,
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
) -> proc_macro::TokenStream {
    let mod_name = make_mod_name(name, "decode");

    match fields {
        Fields::Named { fields } => {
            let setters: Vec<_> = fields.iter().map(|f| {
                let fname = f.name.as_ref().unwrap();
                let expr = gen_decode_field_expr(f, field_attrs, None);
                quote::quote! { #fname: #expr, }
            }).collect();

            quote::quote! {
                mod #mod_name {
                    use super::*;
                    use binsard::DecodeHelper;
                    impl Decode for #name {
                        fn decode(data: &[u8]) -> Result<Self, binsard::DecodeError> {
                            let mut data_idx = 0;
                            let mut helper_init = DecodeHelper::default();
                            let helper = &mut helper_init;
                            helper.reserve_bits(#reserve_bits);
                            Ok(Self { #(#setters)* })
                        }
                    }
                    impl #name {
                        pub(crate) fn decode_internal(data: &[u8], helper: &mut DecodeHelper) -> Result<(usize, Self), binsard::DecodeError> {
                            let mut data_idx = 0;
                            let res = Self { #(#setters)* };
                            Ok((data_idx, res))
                        }
                    }
                }
            }
            .into()
        }
        Fields::Unnamed { fields } => {
            let bindings: Vec<_> = (0..fields.len())
                .map(|i| format_ident!("_{}", i))
                .collect();
            let decoders: Vec<_> = fields.iter().enumerate().map(|(i, f)| {
                let binding = &bindings[i];
                let expr = gen_decode_field_expr(f, field_attrs, None);
                quote::quote! { let #binding = #expr; }
            }).collect();

            quote::quote! {
                mod #mod_name {
                    use super::*;
                    use binsard::DecodeHelper;
                    impl Decode for #name {
                        fn decode(data: &[u8]) -> Result<Self, binsard::DecodeError> {
                            let mut data_idx = 0;
                            let mut helper_init = DecodeHelper::default();
                            let helper = &mut helper_init;
                            helper.reserve_bits(#reserve_bits);
                            #(#decoders)*
                            Ok(Self(#(#bindings),*))
                        }
                    }
                    impl #name {
                        pub(crate) fn decode_internal(data: &[u8], helper: &mut DecodeHelper) -> Result<(usize, Self), binsard::DecodeError> {
                            let mut data_idx = 0;
                            #(#decoders)*
                            Ok((data_idx, Self(#(#bindings),*)))
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
                    use binsard::DecodeHelper;
                    impl Decode for #name {
                        fn decode(_data: &[u8]) -> Result<Self, binsard::DecodeError> { Ok(Self) }
                    }
                    impl #name {
                        pub(crate) fn decode_internal(_data: &[u8], _helper: &mut DecodeHelper) -> Result<(usize, Self), binsard::DecodeError> {
                            Ok((0, Self))
                        }
                    }
                }
            }
            .into()
        }
    }
}

fn gen_enum_variant_decode(
    enum_name: &syn::Ident,
    idx: u8,
    variant: &EnumVariant,
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
) -> proc_macro2::TokenStream {
    let vname = &variant.variant;
    match &variant.fields {
        Fields::Named { fields } => {
            let setters: Vec<_> = fields.iter().map(|f| {
                let fname = f.name.as_ref().unwrap();
                let expr = gen_decode_field_expr(f, field_attrs, Some(vname));
                quote::quote! { #fname: #expr, }
            }).collect();
            quote::quote! {
                #idx => #enum_name::#vname { #(#setters)* },
            }
        }
        Fields::Unnamed { fields } => {
            let bindings: Vec<_> = (0..fields.len())
                .map(|i| format_ident!("_{}", i))
                .collect();
            let decoders: Vec<_> = fields.iter().enumerate().map(|(i, f)| {
                let binding = &bindings[i];
                let expr = gen_decode_field_expr(f, field_attrs, Some(vname));
                quote::quote! { let #binding = #expr; }
            }).collect();
            quote::quote! {
                #idx => {
                    #(#decoders)*
                    #enum_name::#vname(#(#bindings),*)
                },
            }
        }
        Fields::Unit => {
            quote::quote! { #idx => #enum_name::#vname, }
        }
    }
}

pub fn impl_decode_trait(mut ast: DeriveInput) -> proc_macro::TokenStream {
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
            gen_struct_decode(&name, &fields, reserve_bits, &field_attrs)
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

            let setters: Vec<_> = variants.iter().enumerate().map(|(idx, v)| {
                gen_enum_variant_decode(&name, idx as u8, v, &field_attrs)
            }).collect();

            let mod_name = make_mod_name(&name, "decode");
            quote::quote! {
                mod #mod_name {
                    use super::*;
                    use binsard::DecodeHelper;
                    impl Decode for #name {
                        fn decode(data: &[u8]) -> Result<Self, binsard::DecodeError> {
                            let mut helper_init = DecodeHelper::default();
                            let helper = &mut helper_init;
                            helper.reserve_bits(#reserve_bits);
                            let tag = helper.read_partly(data, #bits)? as u8;
                            let mut data_idx = 0;
                            Ok(match tag {
                                #(#setters)*
                                _ => return Err(binsard::DecodeError::UnknownTag(tag)),
                            })
                        }
                    }
                    impl #name {
                        pub(crate) fn decode_internal(data: &[u8], helper: &mut DecodeHelper) -> Result<(usize, Self), binsard::DecodeError> {
                            let tag = helper.read_partly(data, #bits)? as u8;
                            let mut data_idx = 0;
                            let res = match tag {
                                #(#setters)*
                                _ => return Err(binsard::DecodeError::UnknownTag(tag)),
                            };
                            Ok((data_idx, res))
                        }
                    }
                }
            }
            .into()
        }
    }
}
