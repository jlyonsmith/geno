# Geno

[![coverage](https://shields.io/endpoint?url=https://raw.githubusercontent.com/jlyonsmith/geno/main/coverage.json)](https://github.com/jlyonsmith/geno/blob/main/coverage.json)
[![Crates.io](https://img.shields.io/crates/v/geno.svg)](https://crates.io/crates/geno)
[![Docs.rs](https://docs.rs/geno/badge.svg)](https://docs.rs/geno)

A cross-language schema compiler that generates type definitions and serialization code from a simple, declarative schema language.

Define your data types once in a `.geno` file, then generate idiomatic code for multiple target languages.

The name **geno** comes from the word **genome**, the set of genetic instructions containing all information needed for an organism to develop, function, and reproduce.

> This project is still in development. In particular, the schema language is not yet stable. Please feel free to contribute!

## Architecture

Geno uses a multi-process pipeline. The main `geno` binary parses the schema and serializes the AST to MessagePack. It then pipes those bytes to a code generator binary (`geno-<format>`) via stdin, which writes generated source code to stdout.

```
`.geno` file ──► geno (parser + validator) ──► AST (serialized with MessagePack) ──► `geno-<format>` ──► source code
```

## Schema Language

The recommended extension for Geno files is `.geno`.  Geno schemas consist of a single `meta` section followed by any number of `enum` and `struct` declarations.  Schemas can be nested using the `include` statement.  For example, you could have a file called `common.geno`:

```geno
#![format = 1]

enum fruit: i16 {
    apple = 1,
    orange = 2,
    kiwiFruit = 3,
    pear, // auto-incremented to 4
}
```

And another file in the same directory called `order.geno`:

```geno
#![format = 1]

include "./common.geno"

struct order {
    id: u64,
    name: string,
    quantity: i32,
    price: f64,
    fruit: fruit,
    tags: [string],
    metadata: {string: string},
    notes: string?,       // nullable
    items: [order; 10],   // fixed-length array
}
```

Whether nesting is preserved in the generated code is dependent on the generator implementation; the AST structures track the nesting.

### Attributes

Geno supports adding attributes to language elements.  The syntax is loosely modelled after the Rust programming language.

There are two attribute levels in Geno metadata:

- A file level attribute block starts with `#![` and ends with `]`
- Language element level attributes start with `#[` and ends with `]`

Attribute values come in three types:

- **Boolean** This attribute is `true` if present, otherwise `false`, e.g. `#[boolAttr]`
- **Integer** This is a signed integer value, e.g. `#[version = 10]`
- **String** This is a string surrounded by double quotes, e.g. `#[name = "blah"]`

Language elements are:

- `enum`
- `struct`
- `include`
- enum variants
- structure fields

A file level attribute block can only appear once in each `.geno` file. Element level attributes and can appear once before each specific language element. 

One file level atttribute value is required to define the schema format being used:

| Key      | Values | Description |
|----------|--------|-------------|
| `format` | `1`    | This is the only supported format value at present |

Use the `Attributes` member of the different elements in the AST to access values in code generators.

> Right now there are no pre-defined attributes other than `format`. We may need to add a prefix system to avoid conflicts, or create a table of common ones. 

### Types

| Category | Types |
|----------|-------|
| Integers | `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64` |
| Floats | `f32`, `f64` |
| Other | `string`, `bool` |
| Arrays | `[T]` variable-length, `[T; N]` fixed-length |
| Maps | `{K: V}` where `K` is a builtin type |
| Nullable | Append `?` to any type |
| User-defined | Reference any declared enum or struct by name |

### Enums

Enums have an optional integer base type (which defaults to `i32`). Variant values must be given explicitly and there cannot be variants with the same  value.

```
enum color: u8 {
    red = 1,
    green = 2,
    blue = 3,
}
```

Integer literals support decimal, hex (`0xFF`), and binary (`0b1010`) notation.

### Comments

Single-line comments with `//` are supported.

## Code Generators

Geno comes with some built-in generators for several language/encoding formats and serve as examples of how to write your own generator:

| Format | Binary | Description |
|--------|--------|-------------|
| `rust-serde` | `geno-rust-serde` | Rust structs/enums with `Serialize`/`Deserialize` derives |
| `dart-mp` | `geno-dart-mp` | Dart classes/enums with MessagePack `toBytes`/`fromBytes` serialization |
| `dart-json` | `geno-dart-json` | Dart classes/enums with `json_annotation` and `json_serializable` support |

### Rust and Serde

The binary `geno-rust-serde` generates code that:

- Derives `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`
- Converts type names to `PascalCase` and field names to `snake_case`
- Adds `#[serde(rename = "...")]` when names are converted
- Maps arrays to `Vec<T>` or `[T; N]`, maps to `HashMap<K, V>`, nullable to `Option<T>`
- Supports supressing `include` code with `#[noCodeAttr]` to avoid duplicate type definitions

### Dart and MessagePack

The binary `geno-dart-mp` generates code that:

- Generates classes with `final` fields and constructors with `required` named arguments
- Converts type names to `PascalCase` and field/variant names to `lowerCamelCase`
- Generates `toBytes()` and `static fromBytes()` methods using the [`messagepack`](https://pub.dev/packages/messagepack) package
- Maps arrays to `List<T>` and maps to `Map<K, V>`, nullable to a Dart nullable
- All Dart integer types map to `int`, floats to `double`

### Dart and JSON

The binary `geno-dart-json` generates code that:

- Generates classes with `json_annotation`
- Converts type names to `PascalCase` and field/variant names to `lowerCamelCase`
- Generates classes with `fromJson` and `toJson` methods to support `json_serializable` codegen
- Maps arrays to `List<T>` and maps to `Map<K, V>`, nullable to a Dart nullable
- All Dart integer types map to `int`, floats to `double`

### Command Line

```
Geno is a schema compiler for generating source code from a schema definition.

Usage: geno [OPTIONS] <INPUT_FILE> [EXTRA_ARGS]...

Arguments:
  <INPUT_FILE>
          Input .geno file
  [EXTRA_ARGS]...

Options:
  -o, --output-path <OUTPUT_FILE>
          Output file path for the generated source code, or STDOUT if not provided
  -t, --ast-path <AST_FILE>
          Intermediate AST file path for debugging. Program will write the AST to this file in MessagePack format then exit
  -f, --format <FORMAT>
          Output source code format (e.g. -f dart-json or -f rust-rmp)
  -h, --help
          Print help (see a summary with '-h')
  -V, --version
          Print version```

Note that can pass arguments to the generators by adding `--` at the end of the command line.

Set `GENO_DEBUG=1` to invoke code generators via `cargo run` instead of looking for installed binaries on `PATH`.

```bash
GENO_DEBUG=1 geno schema.geno -f rust-serde
```

See the `GenoError` enumeration in the documentation for the list of errors that the parser/validator looks for.

Code generators are standalone binaries that read a MessagePack-encoded `Schema` from stdin. This makes it straightforward to add new target languages without modifying the core parser.

### Example Usage

```bash
# Generate Rust code to stdout
geno schema.geno -f rust-serde

# Generate Dart code to a file
geno schema.geno -f dart-mp -o lib/generated.dart

# Dump the intermediate AST for debugging
geno schema.geno -t schema.ast
```

## Building

Building the `geno` core requires the Rust toolchain.  Generators can be written in any language, and just need to conform to the `geno-` prefix naming convention and be in the path to be used.

```bash
# Build all binaries
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .
```
