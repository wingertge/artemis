use artemis_build::{BuildError, CodegenBuilder};

fn query(file_name: &str) -> String {
    format!("src/queries/{}", file_name)
}

#[rustversion::nightly]
fn main() -> Result<(), BuildError> {
    generate_code()?;

    let cmd = std::process::Command::new("cargo").arg("fmt").spawn();
    cmd.expect("Failed to format generated code");

    Ok(())
}

#[rustversion::not(nightly)]
fn main() -> Result<(), BuildError> {
    generate_code()?;

    Ok(())
}

fn generate_code() -> Result<(), BuildError> {
    CodegenBuilder::new()
        .with_out_dir("src/queries")
        .with_derives_on_variables("Debug,PartialEq")
        .with_derives_on_response("Debug,PartialEq")
        .add_query(query("get_conference.graphql"))
        .build("src/api-schema.json")?;

    Ok(())
}
