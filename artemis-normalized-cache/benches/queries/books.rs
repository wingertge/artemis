use artemis::{exchange::OperationMeta, GraphQLQuery, QueryBody};

pub mod books {
    #![allow(dead_code)]
    pub const OPERATION_NAME: &str = "Books";
    pub const QUERY: &'static str = r#"
  query {
    books {
      id
      title
      genre
      published
      rating
      release
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
    pub struct Book {
        pub id: String,
        pub title: String,
        pub published: bool,
        pub genre: String,
        pub rating: Int,
        pub release: String
    }

    impl Book {
        #[allow(unused_variables)]
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("title", String::new()),
                FieldSelector::Scalar("published", String::new()),
                FieldSelector::Scalar("genre", String::new()),
                FieldSelector::Scalar("rating", String::new()),
                FieldSelector::Scalar("release", String::new()),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ResponseData {
        pub books: Vec<Book>
    }

    impl QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![FieldSelector::Object(
                "books",
                String::new(),
                "Book",
                Book::selection(variables)
            )]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Variables;
}

pub struct Books;

impl GraphQLQuery for Books {
    type Variables = books::Variables;
    type ResponseData = books::ResponseData;

    fn build_query(_variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta) {
        unimplemented!()
    }
}
