use crate::types::Username;

pub fn valid_username(raw: &str) -> Option<Username> {
    Username::new(raw)
}
