use artemis::{Client, ClientBuilder, GraphQLQuery, Middleware};
use std::sync::Arc;

mod queries;

const URL: &str = "http://localhost:8080/graphql";

pub(crate) type Long = String;

pub async fn test_artemis() {
    print!("-- check code generated properly    ");
    check_code_gen();
    print!("✔️\n");
    print!("-- build client                     ");
    let client = build_client();
    print!("✔️\n");
    print!("-- test query                       ");
    test_query(client).await;
    print!("✔️\n")
}

fn check_code_gen() {
    use queries::get_conference::get_conference::{
        GetConferenceConference, GetConferenceConferenceTalks,
        GetConferenceConferenceTalksSpeakers, ResponseData, Variables
    };
    let variables = Variables {
        id: "1".to_string()
    };
    let speakers = GetConferenceConferenceTalksSpeakers {
        name: "test_name".to_string()
    };
    let talks = GetConferenceConferenceTalks {
        id: "2".to_string(),
        title: "test_title".to_string(),
        speakers: Some(vec![speakers])
    };
    let conference = GetConferenceConference {
        id: "3".to_string(),
        name: "test_conf_name".to_string(),
        city: Some("test_city".to_string()),
        talks: Some(vec![talks])
    };
    let _response_data = ResponseData {
        conference: Some(conference)
    };

    let (query, _) = queries::get_conference::GetConference::build_query(variables);

    assert_eq!(query.variables.id, "1".to_string());
    assert_eq!(query.operation_name, "GetConference");
    // assert_eq!(meta.key, 1354603040u32); Apparently this is OS specific
}

fn build_client() -> Arc<Client<impl Middleware>> {
    let builder = ClientBuilder::new(URL).with_default_middleware();

    Arc::new(builder.build())
}

async fn test_query<M: Middleware + Sync + Send>(client: Arc<Client<M>>) {
    use queries::get_conference::{get_conference::*, GetConference};
    let variables = Variables {
        id: "1".to_string()
    };
    let result = client.query(GetConference, variables).await;

    assert!(result.is_ok(), "Query returned an error");

    let response = result.unwrap();
    assert!(response.errors.is_none(), "Query returned errors");
    assert!(response.data.is_some(), "Query didn't return any data");
    let data = response.data.unwrap().conference;
    assert!(data.is_some(), "Conference was not set");
    let conf = data.unwrap();
    assert_eq!(conf.id, "1", "Returned the wrong conference");
    assert_eq!(
        conf.name, "Nextbuild 2018",
        "Returned the wrong conference name"
    );
    assert!(conf.city.is_some(), "Missing city from conference");
    assert_eq!(conf.city.unwrap(), "Eindhoven", "Returned wrong city");
    assert!(conf.talks.is_some(), "Missing talks from conference");
    let mut talks = conf.talks.unwrap();
    assert_eq!(talks.len(), 1, "Length of talks isn't 1");
    let talk = talks.pop().unwrap();
    assert_eq!(talk.id, "22", "Returned wrong talk ID");
    assert_eq!(
        talk.title, "Software Architecture for Developers",
        "Returned wrong talk title"
    );
    assert!(talk.speakers.is_some(), "Speakers missing from talk");
    let mut speakers = talk.speakers.unwrap();
    assert_eq!(speakers.len(), 1, "Speakers list isn't of length 1");
    let speaker = speakers.pop().unwrap();
    assert_eq!(speaker.name, "Simon", "Returned wrong speaker name");
}
