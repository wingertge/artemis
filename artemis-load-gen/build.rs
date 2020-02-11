use artemis_build::CodegenBuilder;
use std::error::Error;

fn query(file_name: &str) -> String {
    format!("src/queries/{}", file_name)
}

fn main() -> Result<(), Box<dyn Error>> {
    generate_code()?;

    Ok(())
}

fn generate_code() -> Result<(), Box<dyn Error>> {
    CodegenBuilder::new()
        .with_out_dir("src/queries")
        .with_derives_on_variables("Debug,PartialEq,Clone")
        .with_derives_on_response("Debug,PartialEq,Clone,Serialize")
        .add_query(query("get_conference.graphql"))
        .add_query(query("add_conference.graphql"))
        .introspect_schema("http://localhost:8080/graphql", None, Vec::new())?
        .build()?;

    Ok(())
}
