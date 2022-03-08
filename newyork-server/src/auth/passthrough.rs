use super::jwt::Claims;

pub fn get_empty_claim() -> Claims {
    Claims {
        sub: "pass_through_guest_user".to_string(),
        exp: 0,
    }
}
