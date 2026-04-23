//! Geno Dart/JSON generator.
//!
//! Generates Dart code using the `json_annotation` and `json_serializable` packages:
//! - Enums: annotated with `@JsonEnum(valueField: 'value')`, holding their integer value
//! - Classes: annotated with `@JsonSerializable()`, with `fromJson`/`toJson` methods
//!
//! After generating, run `dart run build_runner build` to produce the `.g.dart` part file.
use anyhow::Context;
use clap::Parser;
use geno::{ast, case};
use std::fmt::Write as _;
use std::io::{self, Read};

#[derive(Parser)]
#[command(
    name = "geno-dart-json",
    about = "Geno Dart/JSON code generator",
    long_about = "Generates Dart code using the json_annotation and json_serializable packages."
)]
struct Cli {
    /// The 'part' file name for build_runner (e.g. 'models.g.dart').
    /// Emitted as `part 'NAME';` in the output.
    #[arg(value_name = "PART_FILE", short = 'p', long)]
    part_name: Option<String>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }

    std::process::exit(0);
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = Vec::new();

    handle
        .read_to_end(&mut buffer)
        .context("Unable to read AST from stdin")?;

    let schema: ast::Schema =
        rmp_serde::from_slice(&buffer).context("Unable to deserialize AST from stdin")?;

    let output = generate(&schema, cli.part_name.as_deref());
    print!("{}", output);

    Ok(())
}

fn generate(schema: &ast::Schema, part_name: Option<&str>) -> String {
    let mut out = String::new();

    writeln!(
        out,
        "import 'package:json_annotation/json_annotation.dart';"
    )
    .unwrap();
    writeln!(out).unwrap();
    match part_name {
        Some(name) => writeln!(out, "part '{name}.g.dart';").unwrap(),
        None => writeln!(
            out,
            "// part 'generated.g.dart'; // Replace with your output file name or use -p argument"
        )
        .unwrap(),
    }

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

    writeln!(out, "@JsonEnum(valueField: 'value')").unwrap();
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
) {
    let dart_name = case::to_pascal(ident.as_str());

    writeln!(out, "@JsonSerializable()").unwrap();
    writeln!(out, "class {dart_name} {{").unwrap();

    // Fields
    for (_, field_ident, field_type) in fields {
        let dart_field = case::to_camel(&field_ident.as_str());
        if dart_field != *field_ident.as_str() {
            writeln!(out, "  @JsonKey(name: '{0}')", field_ident.as_str()).unwrap();
        }
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

    // fromJson / toJson
    writeln!(out).unwrap();
    writeln!(
        out,
        "  factory {dart_name}.fromJson(Map<String, dynamic> json) => _${dart_name}FromJson(json);"
    )
    .unwrap();
    writeln!(
        out,
        "  Map<String, dynamic> toJson() => _${dart_name}ToJson(this);"
    )
    .unwrap();
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
