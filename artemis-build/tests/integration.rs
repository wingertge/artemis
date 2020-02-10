use artemis::GraphQLQuery;

#[test]
fn check_code_gen() {
    use artemis_test::get_conference::get_conference::{
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

    let (query, _) = artemis_test::get_conference::GetConference::build_query(variables);

    assert_eq!(query.variables.id, "1".to_string());
    assert_eq!(query.operation_name, "GetConference");
    // assert_eq!(meta.key, 1354603040u32); Apparently this is OS specific
}
