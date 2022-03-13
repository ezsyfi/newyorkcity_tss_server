use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::Outcome;


use super::passthrough;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

impl<'a, 'r> FromRequest<'a, 'r> for Claims {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Claims, ()> {
        let auths: Vec<_> = request.headers().get("Authorization").collect();

        if auths.is_empty() {
            return Outcome::Failure((Status::Unauthorized, ()));
        }

        Outcome::Success(passthrough::get_empty_claim())
    }
}
