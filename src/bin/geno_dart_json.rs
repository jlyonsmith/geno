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
use std::fmt::Write as _;
use std::io::{self, Read};

#[derive(Parser)]
#[command(
    name = "geno-dart-json",
    about = "Geno Dart/JSON code generator",
    long_about = "Generates Dart code with static encode/decode methods using dart:convert."
)]
struct Cli {
    // No CLI arguments needed since we no longer use build_runner
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
    writeln!(out).unwrap();

    let elements = schema.flatten_elements();

    for element in elements {
        writeln!(out).unwrap();
        match element {
            ast::Element::Enum {
                ident, variants, ..
            } => generate_enum(&mut out, ident, variants),
            ast::Element::Struct { ident, fields, .. } => generate_struct(&mut out, ident, fields),
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
    writeln!(out).unwrap();

    // toMap method
    writeln!(
        out,
        "  static Map<String, dynamic> toMap({dart_name} obj) {{"
    )
    .unwrap();
    writeln!(out, "    return {{'value': obj.value}};").unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out).unwrap();

    // fromMap method
    writeln!(
        out,
        "  static {dart_name} fromMap(Map<String, dynamic> json) {{"
    )
    .unwrap();
    writeln!(out, "    final value = json['value'] as int;").unwrap();
    writeln!(
        out,
        "    return {dart_name}.values.firstWhere((e) => e.value == value);"
    )
    .unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out).unwrap();

    // Static encode method
    writeln!(out, "  static Uint8List encode({dart_name} obj) {{").unwrap();
    writeln!(out, "    final json = toMap(obj);").unwrap();
    writeln!(
        out,
        "    return Uint8List.fromList(jsonEncode(json).codeUnits);"
    )
    .unwrap();
    writeln!(out, "  }}").unwrap();
    writeln!(out).unwrap();

    // Static decode method
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

fn generate_struct(
    out: &mut String,
    ident: &ast::Ident,
    fields: &[(ast::Attributes, ast::Ident, ast::NullableFieldType)],
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
        let encode_value = generate_encode_field_value(&format!("obj.{dart_field}"), field_type);
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
        let decode_value = generate_decode_field_value(&format!("json['{json_key}']"), field_type);
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

fn generate_encode_field_value(field_expr: &str, field_type: &ast::NullableFieldType) -> String {
    if field_type.nullable {
        let inner_encode = generate_encode_field_value_inner(field_expr, &field_type.field_type);
        format!("{field_expr} != null ? {inner_encode} : null")
    } else {
        generate_encode_field_value_inner(field_expr, &field_type.field_type)
    }
}

fn generate_encode_field_value_inner(field_expr: &str, field_type: &ast::FieldType) -> String {
    match field_type {
        ast::FieldType::Builtin(_) => field_expr.to_string(),
        ast::FieldType::UserDefined(ident) => {
            let dart_name = case::to_pascal(ident.as_str());
            format!("{dart_name}.toMap({field_expr})")
        }
        ast::FieldType::Array(inner, _) => {
            let inner_encode = generate_encode_field_value("e", inner);
            format!("{field_expr}.map((e) => {inner_encode}).toList()")
        }
        ast::FieldType::Map(key_type, value_type) => {
            let key_encode = match key_type {
                ast::BuiltinType::String => "entry.key".to_string(),
                _ => format!("entry.key.toString()"), // Convert other key types to string for JSON
            };
            let value_encode = generate_encode_field_value("entry.value", value_type);
            format!(
                "Map.fromEntries({field_expr}.entries.map((entry) => MapEntry({key_encode}, {value_encode})))"
            )
        }
    }
}

fn generate_decode_field_value(json_expr: &str, field_type: &ast::NullableFieldType) -> String {
    if field_type.nullable {
        let inner_decode = generate_decode_field_value_inner(json_expr, &field_type.field_type);
        format!("{json_expr} != null ? {inner_decode} : null")
    } else {
        generate_decode_field_value_inner(json_expr, &field_type.field_type)
    }
}

fn generate_decode_field_value_inner(json_expr: &str, field_type: &ast::FieldType) -> String {
    match field_type {
        ast::FieldType::Builtin(bt) => {
            let dart_type = builtin_type_str(bt);
            format!("{json_expr} as {dart_type}")
        }
        ast::FieldType::UserDefined(ident) => {
            let dart_name = case::to_pascal(ident.as_str());
            format!("{dart_name}.fromMap({json_expr} as Map<String, dynamic>)")
        }
        ast::FieldType::Array(inner, _) => {
            let inner_decode = generate_decode_field_value("e", inner);
            format!("({json_expr} as List<dynamic>).map((e) => {inner_decode}).toList()")
        }
        ast::FieldType::Map(key_type, value_type) => {
            let key_cast = match key_type {
                ast::BuiltinType::String => "entry.key".to_string(),
                ast::BuiltinType::Integer(_) => "int.parse(entry.key)".to_string(),
                ast::BuiltinType::Float(_) => "double.parse(entry.key)".to_string(),
                ast::BuiltinType::Bool => "entry.key == 'true'".to_string(),
            };
            let value_decode = generate_decode_field_value("entry.value", value_type);
            let key_type_str = builtin_type_str(key_type);
            let value_type_str = field_type_str(value_type);
            format!(
                "Map<{key_type_str}, {value_type_str}>.fromEntries(({json_expr} as Map<String, dynamic>).entries.map((entry) => MapEntry({key_cast}, {value_decode})))"
            )
        }
    }
}
