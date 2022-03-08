#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKey {
    pub kid: String,
    pub pem: String,
    pub der: String,
    pub alg: String,
    pub kty: String,
}

pub mod cognito;
pub mod jwt;
pub mod passthrough;
