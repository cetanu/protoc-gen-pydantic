use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use protobuf::descriptor::field_descriptor_proto::{
    Label,
    Type::{self, *},
};
use protobuf::descriptor::{
    field_descriptor_proto, DescriptorProto, EnumDescriptorProto, FieldDescriptorProto,
    FileDescriptorProto,
};
use protobuf::plugin::code_generator_response::Feature::FEATURE_PROTO3_OPTIONAL;
use protobuf::plugin::{CodeGeneratorRequest, CodeGeneratorResponse};
use protobuf::Message;

#[derive(Default)]
struct PythonModule {}

#[derive(Default, Debug, Eq, PartialEq, Hash)]
struct PythonArgument {
    type_: String,
    default: String,
}

#[derive(Default)]
struct GeneratorContext<'ctx> {
    package_name: &'ctx str,
    processed: HashSet<PathBuf, PythonModule>,
    type_refs: Mutex<HashMap<&'ctx str, PythonArgument>>,
}

struct MessageContext<'ctx> {
    message_name: &'ctx str,
    location: &'ctx str,
}

impl<'ctx> MessageContext<'ctx> {
    fn new(message_name: &'ctx str, location: &'ctx str) -> Self {
        Self {
            message_name,
            location,
        }
    }
}

impl<'ctx> GeneratorContext<'ctx> {
    fn new(package_name: &'ctx str) -> Self {
        Self {
            package_name,
            ..Default::default()
        }
    }

    fn process_maps(&self, messages: &'ctx [DescriptorProto]) {
        for msg in messages.iter().filter(|p| p.options.map_entry()) {
            if let Ok(mut refs) = self.type_refs.lock() {
                let key = map_proto_type_to_py_type(&msg.field[0]).unwrap();
                let value = map_proto_type_to_py_type(&msg.field[1]).unwrap();
                refs.entry(msg.name()).or_insert(PythonArgument {
                    type_: format!("{}, {}", key, value),
                    default: String::from("Field(default_factory=dict)"),
                });
            }
        }
        for msg in messages.iter() {
            self.process_maps(&msg.nested_type);
        }
    }

    fn process_messages(&self, messages: &'ctx [DescriptorProto]) {
        for msg in messages.iter().filter(|p| !p.options.map_entry()) {
            let ctx = MessageContext::new(msg.name(), self.package_name);
            // eprintln!("\n# File: {}/__init__.py", self.package_name);
            eprintln!("\nclass {}(pydantic.BaseModel)", msg.name());
            self.process_fields(&msg.field, ctx);
            self.process_messages(&msg.nested_type);
            self.process_enums(&msg.enum_type);
        }
    }
    fn process_enums(&self, enums: &[EnumDescriptorProto]) {
        for enum_ in enums {
            eprintln!("\nclass {}(enum.Enum)", enum_.name());
            for variant in &enum_.value {
                eprintln!("    {} = {}", variant.name(), variant.number() + 1);
            }
        }
    }

    fn process_fields(&self, fields: &[FieldDescriptorProto], ctx: MessageContext) {
        for field in fields {
            if field.type_name().ends_with("Entry") {
                if let Some((_path, obj)) = field.type_name().rsplit_once(".") {
                    if let Ok(refs) = self.type_refs.lock() {
                        if let Some(type_ref) = refs.get(obj) {
                            eprintln!(
                                "    {}: dict[{}] = {}",
                                field.name(),
                                type_ref.type_,
                                type_ref.default
                            );
                            continue;
                        } else {
                            eprintln!("# Key {} not in type_refs: {:?}", obj, refs);
                        }
                    }
                }
            }
            eprintln!(
                "    {}: {}",
                field.name(),
                map_proto_type_to_py_type(field).unwrap()
            );
        }
    }
}

fn map_proto_type_to_py_type(f: &FieldDescriptorProto) -> Option<String> {
    Some(
        match f.type_() {
            TYPE_MESSAGE | TYPE_ENUM => return f.type_name().strip_prefix(".").map(String::from),
            TYPE_BOOL => "bool",
            TYPE_STRING => "str",
            TYPE_BYTES => "bytes",
            TYPE_GROUP => panic!(),
            TYPE_DOUBLE | TYPE_FLOAT => "float",
            TYPE_SINT32 | TYPE_SINT64 | TYPE_INT64 | TYPE_UINT64 | TYPE_INT32 | TYPE_FIXED32
            | TYPE_FIXED64 | TYPE_SFIXED32 | TYPE_SFIXED64 | TYPE_UINT32 => "int",
        }
        .to_string(),
    )
}

fn main() -> io::Result<()> {
    let request = CodeGeneratorRequest::parse_from_reader(&mut io::stdin()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Could not parse codegen req: {e}"),
        )
    })?;

    for proto in request.proto_file.iter() {
        // dbg!(proto);
        let package = proto.package().replace(".", "/");
        let ctx = GeneratorContext::new(package.as_str());
        eprintln!("--------------\n# Module: {}", package);
        if proto.dependency.len() > 0 {
            eprintln!("# TODO: import stuff from {}", proto.dependency.join("\n"));
        }
        // Process maps first to avoid creating classes for them
        ctx.process_maps(&proto.message_type);
        ctx.process_messages(&proto.message_type);
        ctx.process_enums(&proto.enum_type);
    }

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
