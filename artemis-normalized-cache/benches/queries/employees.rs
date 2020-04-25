use artemis::{exchange::OperationMeta, GraphQLQuery, QueryBody};

pub mod employees {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &str = "Stores";
    pub const QUERY: &'static str = r#"
  query {
    employees {
      id
      dateOfBirth
      name
      origin
    }
  }"#;
    use artemis::codegen::{FieldSelector, QueryInfo};
    use serde::{Deserialize, Serialize};

    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;
    #[allow(dead_code)]
    type Date = String;

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Employee {
        pub id: String,
        pub name: String,
        #[serde(rename = "dateOfBirth")]
        pub date_of_birth: String,
        pub origin: String
    }

    impl Employee {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("name", String::new()),
                FieldSelector::Scalar("dateOfBirth", String::new()),
                FieldSelector::Scalar("origin", String::new()),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ResponseData {
        pub employees: Vec<Employee>
    }

    impl QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![FieldSelector::Object(
                "employees",
                String::new(),
                "Employee",
                Employee::selection(variables)
            )]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Variables;
}

pub struct Employees;

impl GraphQLQuery for Employees {
    type Variables = employees::Variables;
    type ResponseData = employees::ResponseData;

    fn build_query(_variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta) {
        unimplemented!()
    }
}
