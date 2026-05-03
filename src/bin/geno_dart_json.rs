//! Geno Dart/JSON generator.
//!
//! Generates Dart code using the `dart:convert` package:
//! - Enums: enhanced enums with static `encode` and `decode` methods for JSON serialization
//! - Classes: with static `encode` and `decode` methods that convert to/from UTF-8 encoded JSON
//!
//! Uses `jsonEncode` and `jsonDecode` from `dart:convert` for JSON processing.
use anyhow::Context;
use clap::Parser;
use geno::{ast, case};
use std::collections::HashSet;
use std::fmt::Write as _;
use std::io::{self, Read};

#[derive(Parser)]
#[command(
    name = "geno-dart-json",
    about = "Geno Dart/JSON code generator",
    long_about = "Generates Dart code with static encode/decode methods using dart:convert."
)]
struct Cli {
    // No CLI arguments at the moment
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }

    std::process::exit(0);
}

fn run() -> anyhow::Result<()> {
    let _cli = Cli::parse();

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = Vec::new();

    handle
        .read_to_end(&mut buffer)
        .context("Unable to read AST from stdin")?;

    let schema: ast::Schema =
        rmp_serde::from_slice(&buffer).context("Unable to deserialize AST from stdin")?;

    let output = generate(&schema);
    print!("{}", output);

    Ok(())
}

fn generate(schema: &ast::Schema) -> String {
    let mut out = String::new();

    writeln!(out, "import 'dart:convert';").unwrap();
    writeln!(out, "import 'dart:typed_data';").unwrap();

    let elements = schema.flatten_elements();
    let enum_names: HashSet<&str> = elements
        .iter()
        .filter_map(|e| match e {
            ast::Element::Enum { ident, .. } => Some(ident.as_str()),
            _ => None,
        })
        .collect();

    for element in elements {
        writeln!(out).unwrap();
        match element {
            ast::Element::Enum {
                ident, variants, ..
            } => generate_enum(&mut out, ident, variants),
            ast::Element::Struct { ident, fields, .. } => {
                generate_struct(&mut out, ident, fields, &enum_names)
            }
            ast::Element::Include { .. } => {}
        }
    }

    out
}

fn generate_enum(
    out: &mut String,
    ident: &ast::Ident,
    variants: &[(ast::Attributes, ast::Ident, ast::IntegerValue)],
) {
    let dart_name = case::to_pascal(ident.as_str());

    writeln!(out, "enum {dart_name} {{").unwrap();

    for (i, (_, variant_ident, value)) in variants.iter().enumerate() {
        let dart_variant = case::to_camel(variant_ident.as_str());
        let actual_value = integer_value_str(value);
        let trailing = if i < variants.len() - 1 { "," } else { ";" };
        writeln!(out, "  {dart_variant}({actual_value}){trailing}").unwrap();
    }

    writeln!(out).unwrap();
    writeln!(out, "  const {dart_name}(this.value);").unwrap();
    writeln!(out, "  final int value;").unwrap();
    writeln!(out, "}}").unwrap();
}

fn generate_struct(
    out: &mut String,
    ident: &ast::Ident,
    fields: &[(ast::Attributes, ast::Ident, ast::NullableFieldType)],
    enum_names: &HashSet<&str>,
) {
    let dart_name = case::to_pascal(ident.as_str());

    writeln!(out, "class {dart_name} {{").unwrap();

    // Fields (without annotations)
    for (_, field_ident, field_type) in fields {
        let dart_field = case::to_camel(&field_ident.as_str());
        writeln!(out, "  final {} {dart_field};", field_type_str(field_type)).unwrap();
    }

    // Constructor
    writeln!(out).unwrap();
    writeln!(out, "  {dart_name}({{").unwrap();
    for (_, field_ident, field_type) in fields {
        let dart_field = case::to_camel(&field_ident.as_str());
        if field_type.nullable {
            writeln!(out, "    this.{dart_field},").unwrap();
        } else {
            writeln!(out, "    required this.{dart_field},").unwrap();
        }
    }
    writeln!(out, "  }});").unwrap();

    // default method
    writeln!(out).unwrap();
    writeln!(out, "  static {dart_name} defaultValue() {{").unwrap();
    writeln!(out, "    return {dart_name}(").unwrap();
    for (_, field_ident, field_type) in fields {
        let dart_field = case::to_camel(&field_ident.as_str());
        let default_value = field_default_value(field_type, enum_names);
        writeln!(out, "      {dart_field}: {default_value},").unwrap();
    }
    writeln!(out, "    );").unwrap();
    writeln!(out, "  }}").unwrap();

    // toMap method
    writeln!(out).unwrap();
    writeln!(
        out,
        "  static Map<String, dynamic> toMap({dart_name} obj) {{"
    )
    .unwrap();
    writeln!(out, "    return <String, dynamic>{{").unwrap();
    for (_, field_ident, field_type) in fields {
        let dart_field = case::to_camel(&field_ident.as_str());
        let json_key = field_ident.as_str();
        let encode_value = field_value_to_map(&format!("obj.{dart_field}"), field_type, enum_names);
        writeln!(out, "      '{json_key}': {encode_value},").unwrap();
    }
    writeln!(out, "    }};").unwrap();
    writeln!(out, "  }}").unwrap();

    // fromMap method
    writeln!(out).unwrap();
    writeln!(
        out,
        "  static {dart_name} fromMap(Map<String, dynamic> json) {{"
    )
    .unwrap();
    writeln!(out, "    return {dart_name}(").unwrap();
    for (_, field_ident, field_type) in fields {
        let dart_field = case::to_camel(&field_ident.as_str());
        let json_key = field_ident.as_str();
        let decode_value =
            field_value_from_map(&format!("json['{json_key}']"), field_type, enum_names);
        writeln!(out, "      {dart_field}: {decode_value},").unwrap();
    }
    writeln!(out, "    );").unwrap();
    writeln!(out, "  }}").unwrap();

    // Static encode method
    writeln!(out).unwrap();
    writeln!(out, "  static Uint8List encode({dart_name} obj) {{").unwrap();
    writeln!(out, "    final json = toMap(obj);").unwrap();
    writeln!(
        out,
        "    return Uint8List.fromList(jsonEncode(json).codeUnits);"
    )
    .unwrap();
    writeln!(out, "  }}").unwrap();

    // Static decode method
    writeln!(out).unwrap();
    writeln!(out, "  static {dart_name} decode(Uint8List data) {{").unwrap();
    writeln!(out, "    final jsonStr = String.fromCharCodes(data);").unwrap();
    writeln!(
        out,
        "    final json = jsonDecode(jsonStr) as Map<String, dynamic>;"
    )
    .unwrap();
    writeln!(out, "    return fromMap(json);").unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out, "}}").unwrap();
}

fn field_type_str(ft: &ast::NullableFieldType) -> String {
    match ft {
        ast::NullableFieldType {
            field_type: ast::FieldType::Builtin(bt),
            nullable,
        } => {
            let base = builtin_type_str(bt);
            if *nullable { format!("{base}?") } else { base }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::UserDefined(ident),
            nullable,
        } => {
            let dart_name = case::to_pascal(ident.as_str());
            if *nullable {
                format!("{dart_name}?")
            } else {
                dart_name
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Array(inner, _length),
            nullable,
        } => {
            let inner_str = field_type_str(inner);
            let base = format!("List<{inner_str}>");
            if *nullable { format!("{base}?") } else { base }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Map(key_type, value_type),
            nullable,
        } => {
            let key_str = builtin_type_str(key_type);
            let value_str = field_type_str(value_type);
            let base = format!("Map<{key_str}, {value_str}>");
            if *nullable { format!("{base}?") } else { base }
        }
    }
}

fn builtin_type_str(bt: &ast::BuiltinType) -> String {
    match bt {
        ast::BuiltinType::Integer(_) => "int".to_string(),
        ast::BuiltinType::Float(_) => "double".to_string(),
        ast::BuiltinType::String => "String".to_string(),
        ast::BuiltinType::Bool => "bool".to_string(),
    }
}

fn integer_value_str(v: &ast::IntegerValue) -> String {
    match v {
        ast::IntegerValue::I8(n) => n.to_string(),
        ast::IntegerValue::I16(n) => n.to_string(),
        ast::IntegerValue::I32(n) => n.to_string(),
        ast::IntegerValue::I64(n) => n.to_string(),
        ast::IntegerValue::U8(n) => n.to_string(),
        ast::IntegerValue::U16(n) => n.to_string(),
        ast::IntegerValue::U32(n) => n.to_string(),
        ast::IntegerValue::U64(n) => n.to_string(),
    }
}

fn field_value_to_map(
    field_expr: &str,
    field_type: &ast::NullableFieldType,
    enum_names: &HashSet<&str>,
) -> String {
    match field_type {
        ast::NullableFieldType {
            field_type: ast::FieldType::Builtin(_),
            ..
        } => format!("{field_expr}"),
        ast::NullableFieldType {
            field_type: ast::FieldType::UserDefined(ident),
            nullable,
        } => {
            let dart_name = case::to_pascal(ident.as_str());

            if enum_names.contains(ident.as_str()) {
                format!("{field_expr}{}.value", if *nullable { "?" } else { "" })
            } else {
                if *nullable {
                    format!("{field_expr} != null ? {dart_name}.toMap({field_expr}!) : null")
                } else {
                    format!("{dart_name}.toMap({field_expr})")
                }
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Array(inner, _),
            nullable,
        } => {
            let inner_encode = field_value_to_map("e", inner, enum_names);

            format!(
                "{field_expr}{}.map((e) => {inner_encode}).toList()",
                if *nullable { "?" } else { "" }
            )
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Map(key_type, value_type),
            nullable,
        } => {
            let key_encode = match key_type {
                ast::BuiltinType::String => "entry.key".to_string(),
                _ => format!("entry.key.toString()"), // Convert other key types to string for JSON
            };
            let value_encode = field_value_to_map("entry.value", value_type, enum_names);

            if *nullable {
                format!(
                    "{field_expr} != null ? Map.fromEntries({field_expr}!.entries.map((entry) => MapEntry({key_encode}, {value_encode}))) : null"
                )
            } else {
                format!(
                    "Map.fromEntries({field_expr}.entries.map((entry) => MapEntry({key_encode}, {value_encode})))"
                )
            }
        }
    }
}

fn field_value_from_map(
    json_key: &str,
    field_type: &ast::NullableFieldType,
    enum_names: &HashSet<&str>,
) -> String {
    match field_type {
        ast::NullableFieldType {
            field_type: ast::FieldType::Builtin(bt),
            nullable,
        } => {
            let dart_type = builtin_type_str(bt);

            if *nullable {
                format!("{json_key} != null ? {json_key}! as {dart_type} : null")
            } else {
                format!("{json_key} as {dart_type}")
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::UserDefined(ident),
            nullable,
        } => {
            let dart_name = case::to_pascal(ident.as_str());

            if enum_names.contains(ident.as_str()) {
                if *nullable {
                    format!(
                        "{json_key} != null ? {dart_name}.values.firstWhere((e) => e.value == ({json_key}! as int)) : null"
                    )
                } else {
                    format!("{dart_name}.values.firstWhere((e) => e.value == ({json_key} as int))")
                }
            } else {
                if *nullable {
                    format!(
                        "{json_key} != null ? {dart_name}.fromMap({json_key}! as Map<String, dynamic>) : null"
                    )
                } else {
                    format!("{dart_name}.fromMap({json_key} as Map<String, dynamic>)")
                }
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Array(inner, _),
            nullable,
        } => {
            let inner_decode = field_value_from_map("e", inner, enum_names);

            if *nullable {
                format!(
                    "{json_key} != null ? ({json_key} as List<dynamic>?)?.map((e) => {inner_decode}).toList() : null"
                )
            } else {
                format!("({json_key} as List<dynamic>).map((e) => {inner_decode}).toList()")
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Map(key_type, value_type),
            nullable,
        } => {
            let key_cast = match key_type {
                ast::BuiltinType::String => "entry.key".to_string(),
                ast::BuiltinType::Integer(_) => "int.parse(entry.key)".to_string(),
                ast::BuiltinType::Float(_) => "double.parse(entry.key)".to_string(),
                ast::BuiltinType::Bool => "entry.key == 'true'".to_string(),
            };
            let value_decode = field_value_from_map("entry.value", value_type, enum_names);
            let key_type_str = builtin_type_str(key_type);
            let value_type_str = field_type_str(value_type);

            if *nullable {
                format!(
                    "{json_key} != null ? Map<{key_type_str}, {value_type_str}>.fromEntries(({json_key}! as Map<String, dynamic>).entries.map((entry) => MapEntry({key_cast}, {value_decode}))) : null"
                )
            } else {
                format!(
                    "Map<{key_type_str}, {value_type_str}>.fromEntries(({json_key} as Map<String, dynamic>).entries.map((entry) => MapEntry({key_cast}, {value_decode})))"
                )
            }
        }
    }
}

fn field_default_value(field_type: &ast::NullableFieldType, enum_names: &HashSet<&str>) -> String {
    match field_type {
        ast::NullableFieldType {
            field_type: ast::FieldType::Builtin(bt),
            nullable,
        } => {
            if *nullable {
                "null".to_string()
            } else {
                match bt {
                    ast::BuiltinType::Integer(_) => "0",
                    ast::BuiltinType::Float(_) => "0.0",
                    ast::BuiltinType::String => "''",
                    ast::BuiltinType::Bool => "false",
                }
                .to_string()
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::UserDefined(ident),
            nullable,
        } => {
            let dart_name = case::to_pascal(ident.as_str());
            if enum_names.contains(ident.as_str()) {
                if *nullable {
                    "null".to_string()
                } else {
                    format!("{dart_name}.values.first")
                }
            } else {
                if *nullable {
                    "null".to_string()
                } else {
                    format!("{dart_name}.default()")
                }
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Array(array_item_type, fixed_length),
            nullable,
        } => {
            if *nullable {
                "null".to_string()
            } else {
                if let Some(fixed_length) = fixed_length {
                    format!(
                        "List.filled({}, {})",
                        integer_value_str(fixed_length),
                        field_default_value(array_item_type, enum_names)
                    )
                } else {
                    format!("[]")
                }
            }
        }
        ast::NullableFieldType {
            field_type: ast::FieldType::Map(..),
            nullable,
        } => {
            if *nullable {
                "null".to_string()
            } else {
                format!("{{}}")
            }
        }
    }
}
