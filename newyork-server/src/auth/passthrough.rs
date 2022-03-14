use super::jwt::AuthPayload;

pub fn get_empty_claim() -> AuthPayload {
    AuthPayload {
        token: "pass_through_guest_user".to_string(),
        user_id: "guest_user".to_string(),
    }
}
