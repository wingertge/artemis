#[cfg(not(target_arch = "wasm32"))]
use artemis_build::CodegenBuilder;
#[cfg(not(target_arch = "wasm32"))]
use std::error::Error;

#[cfg(not(target_arch = "wasm32"))]
fn query(file_name: &str) -> String {
    format!("src/queries/{}", file_name)
}

#[rustversion::nightly]
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn Error>> {
    generate_code()?;

    let cmd = std::process::Command::new("cargo").arg("fmt").spawn();
    cmd.expect("Failed to format generated code");

    Ok(())
}

#[rustversion::not(nightly)]
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn Error>> {
    generate_code()?;

    Ok(())
}

// Skip if wasm
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn generate_code() -> Result<(), Box<dyn Error>> {
    CodegenBuilder::new()
        .with_out_dir("src/queries")
        .with_derives_on_variables("Debug,PartialEq")
        .with_derives_on_response("Debug,PartialEq,Serialize")
        .add_query(query("get_conference.graphql"))
        .add_query(query("get_conferences.graphql"))
        .add_query(query("add_conference.graphql"))
        .introspect_schema("http://localhost:8080/graphql", None, Vec::new())?
        .build()?;

    Ok(())
}
