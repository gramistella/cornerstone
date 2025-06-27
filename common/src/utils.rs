use crate::ContactDto;

/// Checks if a string might be a valid email.
pub fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

pub fn find_contact_by_id(contacts: &[ContactDto], id: u32) -> Option<&ContactDto> {
    contacts.iter().find(|contact| contact.id == Some(id))
}