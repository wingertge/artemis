use artemis::{exchange::OperationMeta, GraphQLQuery, QueryBody};

pub mod complex_author {
    #![allow(dead_code)]
    const OPERATION_NAME: &'static str = "ComplexAuthors";
    const QUERY: &'static str = r#"
      query {
        authors {
          id
          name
          recognised
          book {
            id
            published
            name
            review {
              id
              score
              name
              reviewer {
                id
                name
                verified
              }
            }
          }
        }
      }
    "#;

    #[allow(dead_code)]
    type Boolean = bool;
    #[allow(dead_code)]
    type Float = f64;
    #[allow(dead_code)]
    type Int = i64;
    #[allow(dead_code)]
    type ID = String;

    use artemis::codegen::{FieldSelector, QueryInfo};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Variables;

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ResponseData {
        pub authors: Vec<ComplexAuthor>
    }

    impl QueryInfo<Variables> for ResponseData {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![FieldSelector::Object(
                "authors",
                String::new(),
                "Author",
                ComplexAuthor::selection(variables)
            )]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ComplexAuthor {
        pub id: ID,
        pub name: String,
        pub recognised: bool,
        pub book: ComplexBook
    }

    impl ComplexAuthor {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("name", String::new()),
                FieldSelector::Scalar("recognised", String::new()),
                FieldSelector::Object(
                    "book",
                    String::new(),
                    "Book",
                    ComplexBook::selection(variables)
                ),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ComplexBook {
        pub id: ID,
        pub published: bool,
        pub name: String,
        pub review: ComplexReview
    }

    impl ComplexBook {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("published", String::new()),
                FieldSelector::Scalar("name", String::new()),
                FieldSelector::Object(
                    "review",
                    String::new(),
                    "Review",
                    ComplexReview::selection(variables)
                ),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ComplexReview {
        pub id: ID,
        pub score: Int,
        pub name: String,
        pub reviewer: ComplexReviewer
    }

    impl ComplexReview {
        fn selection(variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("score", String::new()),
                FieldSelector::Scalar("name", String::new()),
                FieldSelector::Object(
                    "reviewer",
                    String::new(),
                    "Reviewer",
                    ComplexReviewer::selection(variables)
                ),
            ]
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct ComplexReviewer {
        pub id: ID,
        pub name: String,
        pub verified: bool
    }

    impl ComplexReviewer {
        fn selection(_variables: &Variables) -> Vec<FieldSelector> {
            vec![
                FieldSelector::Scalar("id", String::new()),
                FieldSelector::Scalar("name", String::new()),
                FieldSelector::Scalar("verified", String::new()),
            ]
        }
    }
}

pub struct ComplexAuthor;

impl GraphQLQuery for ComplexAuthor {
    type Variables = complex_author::Variables;
    type ResponseData = complex_author::ResponseData;

    fn build_query(_variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta) {
        unimplemented!()
    }
}
