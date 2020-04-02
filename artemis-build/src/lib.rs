//! This crate allows you to statically generate all files needed to use [artemis](../artemis/index.html).
//! While you could write these by hand, they're quite complex and hand-writing can easily introduce
//! errors, so this module allows you to instead have them generated automatically from `.graphql`
//! files. All you need apart from the files is an introspected schema in JSON or `.graphql` format,
//! or alternatively a server that has introspection enabled. Note that the second option requires
//! the `introspect` feature to be enabled.
//!
//! # Usage
//!
//! ```ignore
//! use artemis_build::CodegenBuilder;
//!
//! CodegenBuilder::new()
//!     .introspect_schema("http://localhost:8080/graphql", None, Vec::new())
//!     .unwrap()
//!     .add_query("my_query.graphql")
//!     .with_out_dir("src/queries")
//!     .build();
//! ```
//!
//! The only required option is the schema - in this case we're introspecting one from
//! `http://localhost:8080/graphql`, with no authorization header and no extra headers - but if
//! you don't call `add_query` at least once the code generator won't do much. The out dir specifies
//! the directory to output your query module to. It will generate a `mod.rs` in this directory,
//! along with a file for each query and a global query enum for WASM support. **Make sure this
//! directory doesn't already have a `mod.rs` or it will be overridden.**
//!
//! The output directory defaults to `OUT_DIR`, but for good autocomplete I recommend putting the
//! files somewhere in `src` where your IDE picks them up.
//!
//! For more information see each function definition.

#![warn(missing_docs)]
#![deny(warnings)]
#![allow(unused_parens)]

use artemis_codegen::{
    deprecation::DeprecationStrategy, generate_module_token_stream, generate_root_token_stream,
    CodegenError, CodegenMode, GraphQLClientCodegenOptions
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

/// An error that occurred in the build function
#[derive(Debug)]
pub enum BuildError {
    /// The arguments passed to the code generator were invalid
    ArgumentError(String),
    /// There was an error during code generation
    CodegenError(CodegenError),
    /// A file IO error occurred
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

/// Configuration object for code generation
///
/// This is used to generate the query structs and modules that are required
/// as well as TypeScript definitions and some static analysis info.
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
    /// Create a new codegen builder with default configuration values.
    /// Note that a schema `must` be set, either by setting a file or introspecting it.
    /// All other configuration is optional, though it won't do much if you don't add any queries.
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

    /// Add a query to have the code generator generate a module for it.
    /// This is currently opt-in for each file to prevent accidentally generating unneeded code,
    /// but a directory based approach may be added later.
    pub fn add_query<T: AsRef<Path>>(mut self, query_path: T) -> Self {
        self.query_paths.push(query_path.as_ref().to_path_buf());
        self
    }

    /// A comma-separated list of derives to add to the generated `Variables` and input structs.
    /// The default derives are `Serialize` and `Clone`, with `Deserialize` added if the target
    /// arch is `wasm32`. Adding these here won't break anything, but it's redundant.
    pub fn with_derives_on_variables<T: Into<String>>(mut self, derives: T) -> Self {
        self.variable_derives = Some(derives.into());
        self
    }

    /// A comma-separated list of derives to add to the generated `ResponseData` and output structs.
    /// The default derives are `Deserialize` and `Clone`, with `Serialize` added if the target
    /// arch is `wasm32`. Adding these here won't break anything, but it's redundant.
    pub fn with_derives_on_response<T: Into<String>>(mut self, derives: T) -> Self {
        self.response_derives = Some(derives.into());
        self
    }

    /// Set the deprecation strategy used for codegen. Can be used to either warn or completely
    /// fail the build if any of your GraphQL queries contain deprecated fields.
    pub fn with_deprecation_strategy(mut self, strategy: DeprecationStrategy) -> Self {
        self.deprecation_strategy = Some(strategy);
        self
    }

    /// Set the output directory for the query module. Defaults to `OUT_DIR`, but it's recommended
    /// to put this in an otherwise empty folder in `src`.
    pub fn with_out_dir<T: AsRef<Path>>(mut self, out_dir: T) -> Self {
        self.output_directory = Some(out_dir.as_ref().to_path_buf());
        self
    }

    /// Sets the schema from a JSON or GraphQL file. The schema should be the result of an
    /// introspection query done against the API you're generating code for.
    /// If this isn't set, `introspect_schema` must be used.
    pub fn with_schema<T: AsRef<Path>>(mut self, schema_path: T) -> Self {
        self.schema_path = Some(schema_path.as_ref().to_path_buf());
        self
    }

    /// Introspect a schema from a remote server. This will download the introspection result
    /// and save it in a temporary schema file in the `OUT_DIR`.
    /// Returns an `IntrospectionError` if the request fails for any reason.
    ///
    /// # Arguments
    ///
    /// * `schema_url` - The URL of the remote server. e.g. `http://localhost:8080/graphql`
    /// * `authorization` - An optional authorization header that should be added to the request
    /// * `extra_headers` - Optional extra headers to be added to the introspection query
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

    /// Finish the configuration and generate the queries module.
    /// It will generate a `mod.rs` and a file for each query in the selected output directory.
    /// It's recommended for that to be an empty directory in `src`.
    ///
    /// This returns an error if an output directory was not set and `OUT_DIR` could not be read,
    /// a schema was not found or an error occurred during codegen or IO.
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
                BuildError::ArgumentError(
                    "Missing out dir. Either set 'OUT_DIR' or use 'with_out_dir'.".to_string()
                )
            })?;

        let params = CodegenParams {
            schema_path,
            selected_operation: None,
            variables_derives: self.variable_derives.clone(),
            response_derives: self.response_derives.clone(),
            deprecation_strategy: self.deprecation_strategy.clone(),
            output_directory
        };
        generate_code(self.query_paths, params)?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct CodegenParams {
    pub schema_path: PathBuf,
    pub selected_operation: Option<String>,
    pub variables_derives: Option<String>,
    pub response_derives: Option<String>,
    pub deprecation_strategy: Option<DeprecationStrategy>,
    pub output_directory: PathBuf
}

pub(crate) fn generate_code(
    query_paths: Vec<PathBuf>,
    params: CodegenParams
) -> Result<(), BuildError> {
    let CodegenParams {
        variables_derives,
        response_derives,
        deprecation_strategy,
        output_directory,
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

    let mut all_queries = Vec::new();
    let mut modules = Vec::new();

    for query_path in query_paths {
        let (module, variants) =
            generate_module_token_stream(query_path.clone(), &schema_path, options.clone())?;
        let module = module.to_string();

        let query_file_name: ::std::ffi::OsString = query_path
            .file_name()
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
            CodegenError::InputError(
                "Failed to find a file name in the provided query path.".to_string()
            )
        })?;
        let module_name = query_file_name.clone().into_string().unwrap();
        let module_name = module_name.splitn(2, '.').next().unwrap().to_string();
        modules.push(module_name);

        let dest_file_path: PathBuf = output_directory.join(query_file_name).with_extension("rs");

        let mut file = File::create(dest_file_path)?;
        write!(file, "{}", module)?;

        all_queries.extend(variants)
    }

    let tokens = generate_root_token_stream(modules, all_queries, options);
    let dest_file_path: PathBuf = output_directory.join("mod").with_extension("rs");
    let mut file = File::create(dest_file_path)?;
    write!(file, "{}", tokens.to_string())?;

    Ok(())
}
