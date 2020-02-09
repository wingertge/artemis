use artemis_codegen::{
    deprecation::DeprecationStrategy, generate_module_token_stream, CodegenError, CodegenMode,
    GraphQLClientCodegenOptions
};
use std::{
    env,
    error::Error,
    fmt,
    fs::File,
    io::Write,
    path::{Path, PathBuf}
};
use syn::Token;

#[cfg(feature = "introspect")]
mod introspect;
#[cfg(feature = "introspect")]
pub use introspect::IntrospectionError;

#[derive(Debug)]
pub enum BuildError {
    ArgumentError(String),
    CodegenError(CodegenError),
    IoError(std::io::Error)
}
impl Error for BuildError {}
impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BuildError::ArgumentError(msg) => write!(f, "error parsing arguments: {}", msg),
            BuildError::CodegenError(e) => write!(f, "error generating code: {}", e),
            BuildError::IoError(e) => write!(f, "io error: {}", e)
        }
    }
}

impl From<CodegenError> for BuildError {
    fn from(e: CodegenError) -> Self {
        BuildError::CodegenError(e)
    }
}

impl From<std::io::Error> for BuildError {
    fn from(e: std::io::Error) -> Self {
        BuildError::IoError(e)
    }
}

#[derive(Debug, Default)]
pub struct CodegenBuilder {
    query_paths: Vec<PathBuf>,
    variable_derives: Option<String>,
    response_derives: Option<String>,
    deprecation_strategy: Option<DeprecationStrategy>,
    output_directory: Option<PathBuf>,
    schema_path: Option<PathBuf>
}

impl CodegenBuilder {
    pub fn new() -> Self {
        Self {
            query_paths: Vec::new(),
            variable_derives: None,
            response_derives: None,
            deprecation_strategy: None,
            output_directory: None,
            schema_path: None
        }
    }

    pub fn add_query<T: AsRef<Path>>(mut self, query_path: T) -> Self {
        self.query_paths.push(query_path.as_ref().to_path_buf());
        self
    }

    pub fn with_derives_on_variables<T: Into<String>>(mut self, derives: T) -> Self {
        self.variable_derives = Some(derives.into());
        self
    }

    pub fn with_derives_on_response<T: Into<String>>(mut self, derives: T) -> Self {
        self.response_derives = Some(derives.into());
        self
    }

    pub fn with_deprecation_strategy(mut self, strategy: DeprecationStrategy) -> Self {
        self.deprecation_strategy = Some(strategy);
        self
    }

    pub fn with_out_dir<T: AsRef<Path>>(mut self, out_dir: T) -> Self {
        self.output_directory = Some(out_dir.as_ref().to_path_buf());
        self
    }

    pub fn with_schema<T: AsRef<Path>>(mut self, schema_path: T) -> Self {
        self.schema_path = Some(schema_path.as_ref().to_path_buf());
        self
    }

    #[cfg(feature = "introspect")]
    pub fn introspect_schema<T: AsRef<str>>(
        mut self,
        schema_url: T,
        authorization: Option<String>,
        extra_headers: Vec<introspect::Header>
    ) -> Result<Self, IntrospectionError> {
        let schema_path =
            introspect::introspect(schema_url.as_ref(), authorization, extra_headers)?;
        self.schema_path = Some(schema_path);
        Ok(self)
    }

    pub fn build(self) -> Result<(), BuildError> {
        if self.schema_path.is_none() {
            let msg = if cfg!(feature = "introspect") {
                "Missing schema path. Either use 'with_schema' to specify a file or 'introspect schema' to introspect a remote server."
            } else {
                r#"
                Missing schema path. Please use 'with_schema' to specify a file.
                Alternatively, enable the 'introspect' feature and use 'introspect_schema' to automatically introspect the schema from a remote server.
                "#
            };
            return Err(BuildError::ArgumentError(msg.to_string()));
        }

        let schema_path = self.schema_path.unwrap();
        let output_directory: PathBuf = self
            .output_directory
            .map(Ok)
            .unwrap_or_else(|| env::var("OUT_DIR").map(Into::into))
            .map_err(|_| {
                BuildError::ArgumentError(format!(
                    "Missing out dir. Either set 'OUT_DIR' or use 'with_out_dir'."
                ))
            })?;

        for query_path in self.query_paths {
            let schema_path = schema_path.clone();
            let params = CodegenParams {
                query_path,
                schema_path,
                selected_operation: None,
                variables_derives: self.variable_derives.clone(),
                response_derives: self.response_derives.clone(),
                deprecation_strategy: self.deprecation_strategy.clone(),
                output_directory: output_directory.clone()
            };
            println!("{:#?}", params);
            generate_code(params)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct CodegenParams {
    pub query_path: PathBuf,
    pub schema_path: PathBuf,
    pub selected_operation: Option<String>,
    pub variables_derives: Option<String>,
    pub response_derives: Option<String>,
    pub deprecation_strategy: Option<DeprecationStrategy>,
    pub output_directory: PathBuf
}

pub(crate) fn generate_code(params: CodegenParams) -> Result<(), BuildError> {
    let CodegenParams {
        variables_derives,
        response_derives,
        deprecation_strategy,
        output_directory,
        query_path,
        schema_path,
        selected_operation
    } = params;

    let mut options = GraphQLClientCodegenOptions::new(CodegenMode::Cli);

    options.set_module_visibility(
        syn::VisPublic {
            pub_token: <Token![pub]>::default()
        }
        .into()
    );

    if let Some(selected_operation) = selected_operation {
        options.set_operation_name(selected_operation);
    }

    if let Some(variables_derives) = variables_derives {
        options.set_variables_derives(variables_derives);
    }

    if let Some(response_derives) = response_derives {
        options.set_response_derives(response_derives);
    }

    if let Some(deprecation_strategy) = deprecation_strategy {
        options.set_deprecation_strategy(deprecation_strategy);
    }

    let gen = generate_module_token_stream(query_path.clone(), &schema_path, options)?;

    let generated_code = gen.to_string();
    let generated_code = generated_code;
    // TODO: Add formatting
    /*    let generated_code = if cfg!(feature = "rustfmt") && !no_formatting {
        format(&generated_code)
    } else {
        generated_code
    };*/

    let query_file_name: ::std::ffi::OsString = query_path
        .file_name()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            CodegenError::InputError(format!(
                "Failed to find a file name in the provided query path."
            ))
        })?;

    let dest_file_path: PathBuf = output_directory.join(query_file_name).with_extension("rs");

    let mut file = File::create(dest_file_path)?;
    write!(file, "{}", generated_code)?;

    Ok(())
}
