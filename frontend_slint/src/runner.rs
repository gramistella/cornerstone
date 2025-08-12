// --- File: frontend/src/runner.rs ---

slint::include_modules!();

use common::ContactDto; // Use the DTO for backend communication
use common::Credentials;
use common::LoginResponse;
use slint::VecModel;
use std::rc::Rc;
use std::sync::Arc;

use tracing;

// Helper to spawn async tasks differently for native and wasm
fn spawn_local<F: std::future::Future<Output = ()> + 'static>(fut: F) {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(fut);
    #[cfg(not(target_arch = "wasm32"))]
    tokio::task::spawn_local(fut);
}

impl Contact {
    /// Converts the local Slint `Contact` struct to the foreign `ContactDto`.
    pub fn to_dto(&self) -> ContactDto {
        ContactDto {
            // Note: We assume an existing UI contact has a valid ID.
            id: Some(self.id.into()),
            name: self.name.to_string(),
            email: self.email.to_string(),
            age: self.age.into(),
            subscribed: self.subscribed,
            contact_type: self.contact_type.to_string(),
        }
    }
}

// Optional: If you need to convert from ContactDto back to slint::Contact for UI updates
impl From<ContactDto> for Contact {
    fn from(dto_contact: ContactDto) -> Self {
        Contact {
            id: dto_contact.id.unwrap_or_default() as i32,
            name: dto_contact.name.into(),
            email: dto_contact.email.into(),
            age: dto_contact.age as i32,
            subscribed: dto_contact.subscribed,
            contact_type: dto_contact.contact_type.into(),
        }
    }
}

pub fn run() {
    // For native builds, we need a tokio runtime.
    #[cfg(not(target_arch = "wasm32"))]
    let _tokio_runtime = tokio::runtime::Runtime::new().unwrap();

    let app = App::new().unwrap();

    // We'll use a single reqwest client for all requests.
    let client: Arc<reqwest::Client> = Arc::new(reqwest::Client::new());
    let base_url = "http://127.0.0.1:8080/api/v1";

    let app_weak = app.as_weak();
    let client_clone = client.clone();
    let base_url_clone = base_url.to_string();
    app.on_login(move |email, password| {
        let app_weak = app_weak.clone();
        let client = client_clone.clone();
        let url = format!("{base_url_clone}/login");
        let credentials = Credentials {
            email: email.to_string(),
            password: password.to_string(),
        };

        spawn_local(async move {
            match client.post(&url).json(&credentials).send().await {
                Ok(response) => {
                    tracing::info!("Login response: {:?}", response);
                    if response.status().is_success() {
                        match response.json::<LoginResponse>().await {
                            Ok(login_response) => {
                                let token = login_response.access_token;
                                slint::invoke_from_event_loop(move || {
                                    app_weak.unwrap().set_auth_token(token.into());
                                    // Fetch contacts after successful login
                                    //app_weak.unwrap().invoke_fetch_contacts();
                                })
                                .unwrap();
                            }
                            _ => {
                                tracing::error!("Failed to parse login response");
                            }
                        }
                    } else {
                        let error_msg = response.text().await.unwrap_or_default();
                        tracing::error!("Login failed: {}", error_msg);
                        // Here you could show an error message in the UI
                    }
                }
                Err(e) => {
                    tracing::error!("Error during login request: {}", e);
                }
            }
        });
    });

    let client_clone = client.clone();
    let base_url_clone = base_url.to_string();
    app.on_register(move |email, password| {
        let client = client_clone.clone();
        let url = format!("{base_url_clone}/register");
        let credentials = Credentials {
            email: email.to_string(),
            password: password.to_string(),
        };

        spawn_local(async move {
            match client.post(&url).json(&credentials).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        println!("Registration successful! Please log in.");
                        // Here you could show a success message in the UI
                    } else {
                        let error_msg = response.text().await.unwrap_or_default();
                        println!("Registration failed: {error_msg}");
                    }
                }
                Err(e) => println!("Error during registration request: {e}"),
            }
        });
    });

    let app_weak = app.as_weak();
    app.on_logout(move || {
        let app_weak = app_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            app_weak.unwrap().set_auth_token("".into());
        });
    });

    // --- Callback for fetching contacts ---
    let app_weak = app.as_weak();
    let client_clone = client.clone();
    let base_url_clone = base_url.to_string();
    app.on_fetch_contacts(move || {
        let app_weak = app_weak.clone();
        let client = client_clone.clone();
        let url = format!("{base_url_clone}/contacts");
        let token = app_weak.unwrap().get_auth_token().to_string();
        spawn_local(async move {
            println!("Fetching contacts from backend...");
            match client.get(&url).bearer_auth(token).send().await {
                Ok(response) => {
                    match response.json::<Vec<ContactDto>>().await {
                        Ok(contacts_dto) => {
                            // This data is `Send` and can be moved across threads.
                            let ui_contacts: Vec<Contact> =
                                contacts_dto.into_iter().map(Into::into).collect();

                            // Post a task to the Slint event loop to update the UI.
                            // The `move` captures `ui_contacts` and `app_weak`.
                            let _ = slint::invoke_from_event_loop(move || {
                                // This closure runs on the main UI thread.
                                // It's now safe to create the Rc-based Slint model.
                                let contacts_model = Rc::new(VecModel::from(ui_contacts));

                                // Set the model on the App component.
                                // .into() is fine here, or you can pass it directly.
                                app_weak.unwrap().set_contacts(contacts_model.into());
                            });
                            println!("Successfully fetched and updated contacts.");
                        }
                        _ => {
                            println!("Failed to parse contacts from response.");
                        }
                    }
                }
                Err(e) => {
                    println!("Error fetching contacts: {e}");
                }
            }
        });
    });

    // --- Callback for adding a new contact ---
    let app_weak = app.as_weak();
    let base_url_clone = base_url.to_string();
    let client_clone = client.clone();
    app.on_add_contact(move |name, email, age, subscribed, contact_type| {
        let app_weak = app_weak.clone();
        let client = client_clone.clone();
        let url = format!("{base_url_clone}/contacts");

        // Create the DTO to send to the backend
        let new_contact = ContactDto {
            id: None, // The backend will assign the ID
            name: name.to_string(),
            email: email.to_string(),
            age: age.into(),
            subscribed,
            contact_type: contact_type.to_string(),
        };

        let token = app_weak.unwrap().get_auth_token().to_string();

        spawn_local(async move {
            println!("Sending new contact to backend...");
            match client
                .clone()
                .post(&url)
                .bearer_auth(token)
                .json(&new_contact)
                .send()
                .await
            {
                Ok(_) => {
                    println!("Successfully added contact. Refreshing list...");
                    // After adding, trigger a fetch to refresh the list
                    let _ = slint::invoke_from_event_loop(move || {
                        app_weak.unwrap().invoke_fetch_contacts();
                    });
                }
                Err(e) => {
                    println!("Error adding contact: {e}");
                }
            }
        });
    });

    // --- NEW: Callback for updating an existing contact ---
    let app_weak = app.as_weak();
    let client_clone = client.clone();
    let base_url_clone = base_url.to_string();
    app.on_update_contact(move |contact_to_update| {
        let app_weak = app_weak.clone();
        let client = client_clone.clone();
        let url = format!("{}/contacts/{}", base_url_clone, contact_to_update.id);
        let contact_dto: ContactDto = contact_to_update.to_dto();
        let token = app_weak.unwrap().get_auth_token().to_string();
        spawn_local(async move {
            match client
                .put(&url)
                .bearer_auth(token)
                .json(&contact_dto)
                .send()
                .await
            {
                Ok(_) => {
                    println!("Successfully updated contact. Refreshing list...");
                    let _ = slint::invoke_from_event_loop(move || {
                        app_weak.unwrap().invoke_fetch_contacts();
                    });
                }
                Err(e) => println!("Error updating contact: {e}"),
            }
        });
    });

    // --- NEW: Callback for deleting a contact ---
    let app_weak = app.as_weak();
    let base_url_clone = base_url.to_string();
    let client_clone = client.clone();
    app.on_delete_contact(move |id| {
        let app_weak = app_weak.clone();
        let client = client_clone.clone();
        let url = format!("{base_url_clone}/contacts/{id}");
        let token = app_weak.unwrap().get_auth_token().to_string();
        spawn_local(async move {
            match client.delete(&url).bearer_auth(token).send().await {
                Ok(_) => {
                    println!("Successfully deleted contact. Refreshing list...");
                    let _ = slint::invoke_from_event_loop(move || {
                        app_weak.unwrap().invoke_fetch_contacts();
                    });
                }
                Err(e) => println!("Error deleting contact: {e}"),
            }
        });
    });

    let app_weak = app.as_weak();
    let client_clone = client.clone();
    let base_url_clone = base_url.to_string();
    app.on_get_contact_for_edit(move |id| {
        let app_weak = app_weak.clone();
        let client = client_clone.clone();
        let url = format!("{base_url_clone}/contacts/{id}");
        let token = app_weak.unwrap().get_auth_token().to_string();
        spawn_local(async move {
            println!("Fetching contact {id} for edit...");
            match client.get(&url).bearer_auth(token).send().await {
                Ok(response) => {
                    match response.json::<ContactDto>().await {
                        Ok(contact_dto) => {
                            // Convert DTO to a slint::Contact struct
                            let ui_contact: Contact = contact_dto.into();

                            // Update the UI on the main thread
                            let _ = slint::invoke_from_event_loop(move || {
                                app_weak.unwrap().set_contact_to_edit(ui_contact);
                            });
                        }
                        _ => {
                            println!("Failed to parse single contact from response.");
                        }
                    }
                }
                Err(e) => {
                    println!("Error fetching single contact: {e}");
                }
            }
        });
    });

    // Initial fetch of contacts
    //app.invoke_fetch_contacts();

    app.run().unwrap();
}
