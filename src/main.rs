use std::collections::HashSet;
use std::error::Error;
use std::io::{self, Write};

use protobuf::descriptor::field_descriptor_proto::Type;
use protobuf::descriptor::{field_descriptor_proto, DescriptorProto};
use protobuf::plugin::code_generator_response::Feature::FEATURE_PROTO3_OPTIONAL;
use protobuf::plugin::{CodeGeneratorRequest, CodeGeneratorResponse};
use protobuf::Message;

fn main() -> io::Result<()> {
    println!("Starting...");
    let request = CodeGeneratorRequest::parse_from_reader(&mut io::stdin()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Could not parse codegen req: {e}"),
        )
    })?;
    println!("Request: {:?}", request);

    let response = match generate_code(request) {
        Ok(resp) => resp,
        Err(e) => {
            let mut error_response = CodeGeneratorResponse::new();
            error_response.set_error(format!("Plugin error: {e}"));
            error_response
        }
    };

    let mut output = vec![];
    response.write_to_vec(&mut output).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to serialize codegen resp: {e}"),
        )
    })?;

    io::stdout().write_all(&output)
}

fn generate_code(request: CodeGeneratorRequest) -> Result<CodeGeneratorResponse, Box<dyn Error>> {
    let mut response = CodeGeneratorResponse::new();
    if request.file_to_generate.is_empty() && request.proto_file.is_empty() {
        response.set_error("No input files to generate".to_owned());
        return Ok(response);
    }
    response.set_supported_features(FEATURE_PROTO3_OPTIONAL as u64);
    Ok(response)
}

fn process_files(request: &CodeGeneratorRequest) -> Result<(), Box<dyn Error>> {
    let files_to_gen: HashSet<&str> = request
        .file_to_generate
        .iter()
        .map(|s| s.as_str())
        .collect();

    for descriptor in request.proto_file.iter() {
        let name = descriptor.name();
        if !files_to_gen.contains(name) {
            continue; // not marked for generation
        }
        println!("Processing file: {name}");
        if descriptor.has_package() {
            println!("Package: {}", descriptor.package());
        }

        for message in descriptor.message_type.iter() {
            todo!()
        }

        for enum_ in descriptor.enum_type.iter() {
            todo!()
        }
    }

    Ok(())
}

fn generate_pydantic_model(message: &DescriptorProto) -> Result<String, std::io::Error> {
    let indent = "    ";
    let mut output = Vec::new();
    writeln!(output, "class {}(pydantic.BaseModel):", message.name())?;
    if message.field.is_empty() {
        writeln!(output, "{indent}pass")?;
    } else {
        for field in message.field.iter() {
            let type_ = map_proto_type_to_py(field.type_());
            writeln!(
                output,
                "{indent}{}: {type_} | None = Field(None, default_factory={type_})",
                field.name(),
            )?;
        }
    }
    String::from_utf8(output).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to decode generated pydantic model: {e}"),
        )
    })
}

fn map_proto_type_to_py(proto_type: Type) -> &'static str {
    match proto_type {
        Type::TYPE_STRING => "str",
        Type::TYPE_DOUBLE | Type::TYPE_FLOAT => "float",
        Type::TYPE_INT64 | Type::TYPE_UINT64 | Type::TYPE_INT32 | Type::TYPE_UINT32 => todo!(),
        Type::TYPE_FIXED64 => todo!(),
        Type::TYPE_FIXED32 => todo!(),
        Type::TYPE_BOOL => "bool",
        Type::TYPE_GROUP => todo!(),
        Type::TYPE_MESSAGE => todo!(),
        Type::TYPE_BYTES => todo!(),
        Type::TYPE_ENUM => todo!(),
        Type::TYPE_SFIXED32 => todo!(),
        Type::TYPE_SFIXED64 => todo!(),
        Type::TYPE_SINT32 => todo!(),
        Type::TYPE_SINT64 => todo!(),
        _ => "typing.Any",
    }
}

struct PythonField {
    name: String,
    type_hint: String,
    value: String,
}

struct PythonEnum {
    variants: Vec<PythonField>,
}

struct PythonClass {
    subclasses: Vec<String>,
    fields: Vec<PythonField>,
}
