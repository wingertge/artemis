use artemis_codegen::CodegenBuilder;
use std::path::PathBuf;

#[test]
fn test_code_generation() {
    let query_path: PathBuf = "tests/get_election.graphql".parse().unwrap();
    let schema_path = "tests/api-schema.json";
    let out_dir = "tests";

    let _build = CodegenBuilder::new()
        .with_out_dir(out_dir)
        .add_query(query_path)
        .build(schema_path)
        .unwrap();
}
