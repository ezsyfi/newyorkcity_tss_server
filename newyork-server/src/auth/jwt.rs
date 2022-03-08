use jsonwebtoken::DecodingKey;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::{Outcome, State};
use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::auth::PublicKey;

use super::super::jwt::{decode, decode_header, Algorithm, Header, Validation};
use super::super::server::AuthConfig;
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
        let config = request.guard::<State<AuthConfig>>()?;

        if config.issuer.is_empty()
            && config.audience.is_empty()
            && config.region.is_empty()
            && config.pool_id.is_empty()
        {
            debug!("!!! Auth config empty, request in PASSTHROUGH mode !!! ");
            if auths.is_empty() {
                // No Authorization header
                debug!("!!! No Authorization header, request accepted !!! ");
                return Outcome::Success(passthrough::get_empty_claim());
            } else {
                error!("!!! Auth config empty but authorization header, rejecting requests !!!");
                return Outcome::Failure((Status::Unauthorized, ()));
            }
        }

        if auths.is_empty() {
            return Outcome::Failure((Status::Unauthorized, ()));
        }

        let claim = match verify(
            &config.issuer,
            &config.audience,
            &config.region,
            &config.pool_id,
            &auths[0].to_string(),
        ) {
            Ok(claim) => claim,
            Err(_) => {
                error!("!!! Auth error: Unauthorized (401) !!!");
                return Outcome::Failure((Status::Unauthorized, ()));
            }
        };

        Outcome::Success(claim)
    }
}

const ALGORITHM: Algorithm = Algorithm::RS256;
const TOKEN_TYPE: &str = "Bearer";
pub fn verify(
    issuer: &String,
    audience: &String,
    region: &String,
    pool_id: &String,
    authorization_header: &String,
) -> Result<Claims, ()> {
    let mut header_parts = authorization_header.split_whitespace();
    let token_type = header_parts.next();
    assert_eq!(token_type, Some(TOKEN_TYPE));

    let token = header_parts.next().unwrap();

    let header = match decode_header_from_token(token.to_string()) {
        Ok(h) => h,
        Err(_) => return Err(()),
    };

    let key_set_str: String = match get_jwt_to_pems(region, pool_id) {
        Ok(k) => k,
        Err(_) => return Err(()),
    };

    let key_set: HashMap<String, PublicKey> = match serde_json::from_str(&key_set_str) {
        Ok(k) => k,
        Err(_) => return Err(()),
    };

    let header_kid = header.kid.unwrap();

    if !key_set.contains_key(&header_kid) {
        return Err(());
    }

    let key = key_set.get(&header_kid).unwrap();

    let secret = hex::decode(&key.der).unwrap();
    let algorithms: Vec<Algorithm> = vec![ALGORITHM];

    get_claims(issuer, audience, &token.to_string(), &secret, algorithms)
}

fn get_jwt_to_pems(region: &String, pool_id: &String) -> Result<String, ()> {
    match Command::new("node")
        .arg("jwt-to-pems.js")
        .arg(format!("--region={}", region))
        .arg(format!("--poolid={}", pool_id))
        .current_dir("../newyork-utilities/server/cognito")
        .output()
    {
        Ok(o) => return Ok(String::from_utf8_lossy(&o.stdout).to_string()),
        Err(_) => return Err(()),
    };
}

#[allow(clippy::result_unit_err)]
pub fn get_claims(
    issuer: &str,
    audience: &str,
    token: &str,
    secret: &[u8],
    algorithms: Vec<Algorithm>,
) -> Result<Claims, ()> {
    let mut validation = Validation::new(algorithms[0]);

    validation.iss = Some(HashSet::from([issuer.to_owned()]));
    // Setting audience
    validation.set_audience(&[audience]);
    let secret = &DecodingKey::from_secret(secret.as_ref());
    let token_data = match decode::<Claims>(token, secret, &validation) {
        Ok(c) => c,
        Err(_) => return Err(()),
    };

    Ok(token_data.claims)
}

pub fn decode_header_from_token(token: String) -> Result<Header, ()> {
    let header = match decode_header(&token) {
        Ok(h) => h,
        Err(_) => return Err(()),
    };

    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::{decode_header_from_token, get_claims};
    use hex;
    use jwt::{Algorithm, Header};
    use std::str;

    #[test]
    #[should_panic] // Obviously hardcoded authorization_header become invalid
    fn get_claims_test() {
        let der_hex : &str = "30820122300d06092a864886f70d01010105000382010f003082010a0282010100dd5a02e27e4d48e77fa7fba44de5963a4952850df2d89750408665ac9e814ca58d961348693c424cf884a5f44c377fced421ef3070eb974e7ec76fed861d9c4ff777aefcbb4a1c7396d35dde8feba2476dd42d3a38f73f2f4547d1b35e1cd9d3da9bf7341dc00bd543a97c890f5dfa2f2d800b5ecb44a2a679e8a5848123dd7a8087ec094c503b92dddd027e609e4f61caf5452344be6a401bf1b01a198967b526b13d9e9c2c0e6712e5b3359348135a48a936027d4c2a5c54b4d31eca1f94c00c02be82a91b4ef01498b58b652508110d06105c986750502e1b243fb69b05ad34e3eb1a86cc7cdaf69c4b29d3c00aa97a6055b293797017f1b59a998d2ade970203010001";

        let token: String = "eyJraWQiOiJZeEdoUlhsTytZSWpjU2xWZFdVUFA1dHhWd\
                             FRSTTNmTndNZTN4QzVnXC9YZz0iLCJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJjNDAz\
                             ZTBlNy1jM2QwLTRhNDUtODI2Mi01MTM5OTIyZjc5NTgiLCJhdWQiOiI0cG1jaXUx\
                             YWhyZjVzdm1nbTFobTVlbGJ1cCIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJjdXN0\
                             b206ZGV2aWNlUEsiOiJbXCItLS0tLUJFR0lOIFBVQkxJQyBLRVktLS0tLVxcbk1G\
                             a3dFd1lIS29aSXpqMENBUVlJS29aSXpqMERBUWNEUWdBRUdDNmQ1SnV6OUNPUVVZ\
                             K08rUUV5Z0xGaGxSOHpcXHJsVjRRTTV1ZUhsQjVOTVQ2dm04c1dFMWtpak5udnpP\
                             WDl0cFRZUEVpTEIzbHZORWNuUmszTXRRZVNRPT1cXG4tLS0tLUVORCBQVUJMSUMg\
                             S0VZLS0tLS1cIl0iLCJ0b2tlbl91c2UiOiJpZCIsImF1dGhfdGltZSI6MTU0NjUz\
                             MzM2NywiaXNzIjoiaHR0cHM6XC9cL2NvZ25pdG8taWRwLnVzLXdlc3QtMi5hbWF6\
                             b25hd3MuY29tXC91cy13ZXN0LTJfZzlqU2xFYUNHIiwiY29nbml0bzp1c2VybmFt\
                             ZSI6ImM0MDNlMGU3LWMzZDAtNGE0NS04MjYyLTUxMzk5MjJmNzk1OCIsImV4cCI6\
                             MTU0NzEwNzI0OSwiaWF0IjoxNTQ3MTAzNjQ5LCJlbWFpbCI6ImdhcnkrNzgyODJA\
                             a3plbmNvcnAuY29tIn0.WLo9fiDiovRqC1RjR959aD8O1E3lqi5Iwnsq4zobqPU5\
                             yZHW2FFIDwnEGf3UmQWMLgscKcuy0-NoupMUCbTvG52n5sPvOrCyeIpY5RkOk3mH\
                             enH3H6jcNRA7UhDQwhMu_95du3I1YHOA173sPqQQvmWwYbA8TtyNAKOq9k0QEOuq\
                             PWRBXldmmp9pxivbEYixWaIRtsJxpK02ODtOUR67o4RVeVLfthQMR4wiANO_hKLH\
                             rt76DEkAntM0KIFODS6o6PBZw2IP4P7x21IgcDrTO3yotcc-RVEq0X1N3wI8clr8\
                             DaVVZgolenGlERVMfD5i0YWIM1j7GgQ1fuQ8J_LYiQ"
            .to_string();

        let der = hex::decode(der_hex).unwrap();
        let issuer: String = "issuer".to_string();
        let audience: String = "audience".to_string();
        let algorithms = vec![Algorithm::RS256];
        assert!(get_claims(&issuer, &audience, &token, der.as_ref(), algorithms).is_ok());
    }

    #[test]
    fn decode_token_test() {
        let token: String = "eyJraWQiOiJZeEdoUlhsTytZSWpjU2xWZFdVUFA1dHhWd\
                             FRSTTNmTndNZTN4QzVnXC9YZz0iLCJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJjNDAz\
                             ZTBlNy1jM2QwLTRhNDUtODI2Mi01MTM5OTIyZjc5NTgiLCJhdWQiOiI0cG1jaXUx\
                             YWhyZjVzdm1nbTFobTVlbGJ1cCIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJjdXN0\
                             b206ZGV2aWNlUEsiOiJbXCItLS0tLUJFR0lOIFBVQkxJQyBLRVktLS0tLVxcbk1G\
                             a3dFd1lIS29aSXpqMENBUVlJS29aSXpqMERBUWNEUWdBRUdDNmQ1SnV6OUNPUVVZ\
                             K08rUUV5Z0xGaGxSOHpcXHJsVjRRTTV1ZUhsQjVOTVQ2dm04c1dFMWtpak5udnpP\
                             WDl0cFRZUEVpTEIzbHZORWNuUmszTXRRZVNRPT1cXG4tLS0tLUVORCBQVUJMSUMg\
                             S0VZLS0tLS1cIl0iLCJ0b2tlbl91c2UiOiJpZCIsImF1dGhfdGltZSI6MTU0NjUz\
                             MzM2NywiaXNzIjoiaHR0cHM6XC9cL2NvZ25pdG8taWRwLnVzLXdlc3QtMi5hbWF6\
                             b25hd3MuY29tXC91cy13ZXN0LTJfZzlqU2xFYUNHIiwiY29nbml0bzp1c2VybmFt\
                             ZSI6ImM0MDNlMGU3LWMzZDAtNGE0NS04MjYyLTUxMzk5MjJmNzk1OCIsImV4cCI6\
                             MTU0NzEwNzI0OSwiaWF0IjoxNTQ3MTAzNjQ5LCJlbWFpbCI6ImdhcnkrNzgyODJA\
                             a3plbmNvcnAuY29tIn0.WLo9fiDiovRqC1RjR959aD8O1E3lqi5Iwnsq4zobqPU5\
                             yZHW2FFIDwnEGf3UmQWMLgscKcuy0-NoupMUCbTvG52n5sPvOrCyeIpY5RkOk3mH\
                             enH3H6jcNRA7UhDQwhMu_95du3I1YHOA173sPqQQvmWwYbA8TtyNAKOq9k0QEOuq\
                             PWRBXldmmp9pxivbEYixWaIRtsJxpK02ODtOUR67o4RVeVLfthQMR4wiANO_hKLH\
                             rt76DEkAntM0KIFODS6o6PBZw2IP4P7x21IgcDrTO3yotcc-RVEq0X1N3wI8clr8\
                             DaVVZgolenGlERVMfD5i0YWIM1j7GgQ1fuQ8J_LYiQ"
            .to_string();

        let header: Header = decode_header_from_token(token).unwrap();
        assert_eq!(
            header.kid.unwrap(),
            "YxGhRXlO+YIjcSlVdWUPP5txVtTRM3fNwMe3xC5g/Xg="
        );
    }
}
