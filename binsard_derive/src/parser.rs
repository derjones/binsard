use std::collections::HashMap;

use syn::{spanned::Spanned, DeriveInput, Expr, ExprLit, Lit, PathArguments, Type, TypeArray, TypePath};

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(binsard))]
pub struct BinSardStructAttribute {
    #[deluxe(default = 8)]
    pub bits: usize,
    #[deluxe(default = 0)]
    pub reserve_bits: usize,
}

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(binsard))]
pub struct BinSardFieldAttribute {
    pub bits: Option<usize>,
    pub len_bytes: Option<usize>,
}

pub struct BinSardAttributes {
    pub bits: usize,
    pub reserve_bits: usize,
    pub field_attrs: HashMap<String, BinSardFieldAttribute>,
}

pub fn get_bits_attr(
    name: Option<&syn::Ident>,
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
    variant_prefix: Option<&syn::Ident>,
) -> Option<usize> {
    name.and_then(|n| {
            let key = match variant_prefix {
                Some(v) => format!("{v}::{n}"),
                None => n.to_string(),
            };
            field_attrs.get(&key)
        })
        .and_then(|a| a.bits)
}

pub fn get_len_bytes_attr(
    name: Option<&syn::Ident>,
    field_attrs: &HashMap<String, BinSardFieldAttribute>,
    variant_prefix: Option<&syn::Ident>,
) -> Option<usize> {
    name.and_then(|n| {
            let key = match variant_prefix {
                Some(v) => format!("{v}::{n}"),
                None => n.to_string(),
            };
            field_attrs.get(&key)
        })
        .and_then(|a| a.len_bytes)
}

pub fn validate_len_bytes(len_bytes: usize, field_name: &str) -> Result<(), syn::Error> {
    if !matches!(len_bytes, 1 | 2 | 4) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "field `{field_name}`: #[binsard(len_bytes = {len_bytes})] must be 1, 2, or 4"
            ),
        ));
    }
    Ok(())
}

pub fn make_mod_name(name: &syn::Ident, suffix: &str) -> syn::Ident {
    let lower = name.to_string().to_lowercase();
    quote::format_ident!("__binsard_{suffix}_{lower}")
}

pub fn parse_attributes(ast: &mut DeriveInput) -> Result<BinSardAttributes, syn::Error> {
    let BinSardStructAttribute { bits, reserve_bits } = deluxe::extract_attributes(ast)?;
    let mut field_attrs: HashMap<String, BinSardFieldAttribute> = HashMap::new();

    match &mut ast.data {
        syn::Data::Struct(data_struct) => {
            for field in &mut data_struct.fields {
                if let Some(field_name) = field.ident.clone() {
                    let attrs: BinSardFieldAttribute = deluxe::extract_attributes(field)?;
                    if let Some(lb) = attrs.len_bytes {
                        validate_len_bytes(lb, &field_name.to_string())?;
                    }
                    field_attrs.insert(field_name.to_string(), attrs);
                }
            }
        }
        syn::Data::Enum(data_enum) => {
            for variant in &mut data_enum.variants {
                for field in &mut variant.fields {
                    if let Some(field_name) = field.ident.clone() {
                        let attrs: BinSardFieldAttribute = deluxe::extract_attributes(field)?;
                        let key = format!("{}::{}", variant.ident, field_name);
                        if let Some(lb) = attrs.len_bytes {
                            validate_len_bytes(lb, &key)?;
                        }
                        field_attrs.insert(key, attrs);
                    }
                }
            }
        }
        syn::Data::Union(u) => {
            return Err(syn::Error::new(
                u.union_token.span(),
                "binsard: unions are not supported",
            ));
        }
    }

    Ok(BinSardAttributes {
        bits,
        reserve_bits,
        field_attrs,
    })
}

fn get_inner_type_path(type_path: &TypePath) -> Option<syn::Type> {
    if let Some(seg) = type_path.path.segments.last() {
        if let PathArguments::AngleBracketed(args) = &seg.arguments {
            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                return Some(inner_ty.clone());
            }
        }
    }
    None
}

fn get_ident(type_path: &TypePath) -> Option<syn::Ident> {
    type_path.path.segments.last().map(|e| e.ident.clone())
}

#[derive(Debug)]
pub enum StructOrEnum {
    Struct {
        name: syn::Ident,
        fields: Fields,
    },
    Enum {
        name: syn::Ident,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug)]
pub struct EnumVariant {
    pub variant: syn::Ident,
    pub fields: Fields,
}

#[derive(Debug)]
pub enum Fields {
    Named { fields: Vec<FieldInfo> },
    Unnamed { fields: Vec<FieldInfo> },
    Unit,
}

#[derive(Debug)]
pub struct FieldInfo {
    pub name: Option<syn::Ident>,
    pub optional: bool,
    pub kind: FieldKind,
}

#[derive(Debug)]
pub enum FieldKind {
    Bool,
    String,
    Primitive { type_ident: syn::Ident },
    Vec { inner_type: syn::Ident },
    Array { size: usize },
    StructOrEnum { type_ident: syn::Ident },
}

fn classify_type(ty: &syn::Type) -> Result<(bool, FieldKind), syn::Error> {
    match ty {
        syn::Type::Path(type_path) => {
            let Some(ident) = get_ident(type_path) else {
                return Err(syn::Error::new(
                    ty.span(),
                    "binsard: could not resolve type identifier",
                ));
            };
            match ident.to_string().as_str() {
                "bool" => Ok((false, FieldKind::Bool)),
                "String" => Ok((false, FieldKind::String)),
                "Vec" => {
                    let Some(inner) = get_inner_type_path(type_path) else {
                        return Err(syn::Error::new(
                            type_path.span(),
                            "binsard: Vec must have a type parameter",
                        ));
                    };
                    let Type::Path(inner_path) = inner else {
                        return Err(syn::Error::new(
                            inner.span(),
                            "binsard: Vec inner type must be a simple type path",
                        ));
                    };
                    let Some(inner_ident) = get_ident(&inner_path) else {
                        return Err(syn::Error::new(
                            inner_path.span(),
                            "binsard: could not resolve Vec inner type",
                        ));
                    };
                    Ok((false, FieldKind::Vec { inner_type: inner_ident }))
                }
                "i128" | "i64" | "i32" | "i16" | "i8"
                | "u128" | "u64" | "u32" | "u16" | "u8"
                | "f32" | "f64" => {
                    Ok((false, FieldKind::Primitive { type_ident: ident }))
                }
                "Option" => {
                    let Some(inner) = get_inner_type_path(type_path) else {
                        return Err(syn::Error::new(
                            type_path.span(),
                            "binsard: Option must have a type parameter",
                        ));
                    };
                    match inner {
                        Type::Path(_) => {
                            let (optional, kind) = classify_type(&inner)?;
                            // Wrapping the inner type in Option is always considered optional here.
                            let _ = optional;
                            Ok((true, kind))
                        }
                        Type::Array(TypeArray { len, .. }) => {
                            let size = parse_array_len(&len)?;
                            Ok((true, FieldKind::Array { size }))
                        }
                        _ => Err(syn::Error::new(
                            inner.span(),
                            "binsard: unsupported Option inner type",
                        )),
                    }
                }
                _ => Ok((false, FieldKind::StructOrEnum { type_ident: ident })),
            }
        }
        syn::Type::Array(TypeArray { len, .. }) => {
            let size = parse_array_len(&len)?;
            Ok((false, FieldKind::Array { size }))
        }
        _ => Err(syn::Error::new(
            ty.span(),
            "binsard: unsupported type; expected a path, Option, Vec, or array",
        )),
    }
}

fn parse_array_len(len: &Expr) -> Result<usize, syn::Error> {
    if let Expr::Lit(ExprLit { lit: Lit::Int(litint), .. }) = len {
        litint
            .base10_parse::<usize>()
            .map_err(|_| syn::Error::new(len.span(), "binsard: array length is not a valid integer"))
    } else {
        Err(syn::Error::new(
            len.span(),
            "binsard: array length is not a literal",
        ))
    }
}

fn parse_syn_fields(syn_fields: &syn::Fields) -> Result<Fields, syn::Error> {
    match syn_fields {
        syn::Fields::Named(named) => {
            let mut fields = Vec::new();
            for f in &named.named {
                let name = f.ident.clone();
                let (optional, kind) = classify_type(&f.ty)?;
                fields.push(FieldInfo { name, optional, kind });
            }
            Ok(Fields::Named { fields })
        }
        syn::Fields::Unnamed(unnamed) => {
            let mut fields = Vec::new();
            for f in &unnamed.unnamed {
                let (optional, kind) = classify_type(&f.ty)?;
                fields.push(FieldInfo { name: None, optional, kind });
            }
            Ok(Fields::Unnamed { fields })
        }
        syn::Fields::Unit => Ok(Fields::Unit),
    }
}

impl StructOrEnum {
    pub fn parse(ast: &DeriveInput) -> Result<Self, syn::Error> {
        let name = &ast.ident;
        match &ast.data {
            syn::Data::Struct(data_struct) => {
                let fields = parse_syn_fields(&data_struct.fields)?;
                Ok(Self::Struct {
                    name: name.clone(),
                    fields,
                })
            }
            syn::Data::Enum(data_enum) => {
                let mut variants = Vec::new();
                for v in &data_enum.variants {
                    let fields = parse_syn_fields(&v.fields)?;
                    variants.push(EnumVariant {
                        variant: v.ident.clone(),
                        fields,
                    });
                }
                Ok(Self::Enum {
                    name: name.clone(),
                    variants,
                })
            }
            syn::Data::Union(u) => Err(syn::Error::new(
                u.union_token.span(),
                "binsard: unions are not supported",
            )),
        }
    }
}
