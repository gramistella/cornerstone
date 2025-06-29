use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt; // for .collect()
use tower::ServiceExt; // for .oneshot()
use std::sync::{Arc, Mutex};

use common::ContactDto;
use backend::web_server::{create_router, AppState};

// A helper to create a default test contact
fn create_test_contact(id: u32, name: &str, email: &str) -> ContactDto {
    ContactDto {
        id: Some(id),
        name: name.to_string(),
        email: email.to_string(),
        age: 30,
        subscribed: true,
        contact_type: "Test".to_string(),
    }
}

#[tokio::test]
async fn test_get_all_contacts() {
    // ARRANGE
    let test_contacts = vec![create_test_contact(1, "Test User", "test@example.com")];
    // CHANGED: Correctly initialize the AppState struct
    let app_state = AppState {
        contacts: Arc::new(Mutex::new(test_contacts.clone())),
    };
    let app = create_router(app_state);

    // ACT
    let response = app
        .oneshot(Request::builder().uri("/api/contacts").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    // CHANGED: Deserialize into ContactDto
    let contacts_from_api: Vec<ContactDto> = serde_json::from_slice(&body_bytes)
        .expect("Failed to deserialize contacts from API response");
    assert_eq!(contacts_from_api, test_contacts);
}

#[tokio::test]
async fn test_get_all_contacts_empty() {
    // ARRANGE
    // CHANGED: Correctly initialize the AppState struct
    let app_state = AppState {
        contacts: Arc::new(Mutex::new(vec![])),
    };
    let app = create_router(app_state);

    // ACT
    let response = app
        .oneshot(Request::builder().uri("/api/contacts").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    // CHANGED: Deserialize into ContactDto
    let contacts_from_api: Vec<ContactDto> = serde_json::from_slice(&body_bytes).unwrap();
    assert!(contacts_from_api.is_empty());
}

#[tokio::test]
async fn test_create_contact() {
    // ARRANGE
    // CHANGED: Correctly initialize the AppState struct
    let app_state = AppState {
        contacts: Arc::new(Mutex::new(vec![])),
    };
    let app = create_router(app_state.clone());

    // CHANGED: Use the full ContactDto for the payload
    let new_contact_payload = ContactDto {
        id: None, // ID is set by the server
        name: "Bender Rodriguez".into(),
        email: "bender@planetexpress.com".into(),
        age: 99,
        subscribed: false,
        contact_type: "Robot".to_string(),
    };

    // ACT
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/contacts")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&new_contact_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // ASSERT
    // CHANGED: A successful creation should return 201 CREATED
    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let created_contact: ContactDto = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(created_contact.id, Some(1)); // Server should assign the first ID
    assert_eq!(created_contact.name, new_contact_payload.name);

    // Assert that the application state was actually modified
    let final_state = app_state.contacts.lock().unwrap();
    assert_eq!(final_state.len(), 1);
    assert_eq!(final_state[0].id, Some(1));
    assert_eq!(final_state[0].name, "Bender Rodriguez");
}

// --- NEW TESTS FOR NEW ENDPOINTS ---

#[tokio::test]
async fn test_get_single_contact() {
    // ARRANGE
    let test_contacts = vec![create_test_contact(1, "Fry", "fry@planetexpress.com")];
    let app_state = AppState {
        contacts: Arc::new(Mutex::new(test_contacts.clone())),
    };
    let app = create_router(app_state);

    // ACT
    let response = app
        .oneshot(Request::builder().uri("/api/contacts/1").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let found_contact: ContactDto = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(found_contact, test_contacts[0]);
}

#[tokio::test]
async fn test_update_contact() {
    // ARRANGE
    let original_contact = create_test_contact(1, "Leela", "leela@planetexpress.com");
    let app_state = AppState {
        contacts: Arc::new(Mutex::new(vec![original_contact])),
    };
    let app = create_router(app_state.clone());

    let updated_payload = ContactDto {
        id: Some(1),
        name: "Turanga Leela".to_string(),
        email: "leela@planetexpress.com".to_string(),
        age: 32,
        subscribed: false,
        contact_type: "Captain".to_string(),
    };

    // ACT
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/contacts/1")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&updated_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);

    // Assert the state was changed
    let final_state = app_state.contacts.lock().unwrap();
    assert_eq!(final_state.len(), 1);
    assert_eq!(final_state[0].name, "Turanga Leela");
    assert_eq!(final_state[0].subscribed, false);
}

#[tokio::test]
async fn test_delete_contact() {
    // ARRANGE
    let contact_to_delete = create_test_contact(1, "Zoidberg", "zoidberg@planetexpress.com");
    let app_state = AppState {
        contacts: Arc::new(Mutex::new(vec![contact_to_delete])),
    };
    let app = create_router(app_state.clone());

    // ACT
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/contacts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ASSERT
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Assert the state was changed
    let final_state = app_state.contacts.lock().unwrap();
    assert!(final_state.is_empty());
}