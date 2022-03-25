use rocket;
use rocket::{Request, Rocket};
use rocksdb;

use crate::utils::settings::get_hcmc_host;

use super::routes::*;
use super::storage::db;
use super::Config;

use std::collections::HashMap;

#[derive(Deserialize)]
pub struct AuthConfig {
    pub issuer: String,
    pub audience: String,
    pub region: String,
    pub pool_id: String,
}

impl AuthConfig {
    pub fn load(settings: HashMap<String, String>) -> AuthConfig {
        let issuer = settings.get("issuer").unwrap_or(&"".to_string()).to_owned();
        let audience = settings
            .get("audience")
            .unwrap_or(&"".to_string())
            .to_owned();
        let region = settings.get("region").unwrap_or(&"".to_string()).to_owned();
        let pool_id = settings
            .get("pool_id")
            .unwrap_or(&"".to_string())
            .to_owned();

        AuthConfig {
            issuer,
            audience,
            region,
            pool_id,
        }
    }
}

#[catch(500)]
fn internal_error() -> &'static str {
    "Internal server error"
}

#[catch(400)]
fn bad_request() -> &'static str {
    "Bad request"
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Unknown route '{}'.", req.uri())
}

pub fn get_server() -> Rocket {
    let settings = get_settings_as_map();
    let hcmc_config = get_hcmc_host(settings).unwrap();
    let app_config = Config {
        db: get_db(),
        hcmc: hcmc_config,
    };

    rocket::ignite()
        .register(catchers![internal_error, not_found, bad_request])
        .mount(
            "/",
            routes![
                ping::ping,
                ecdsa::first_message,
                ecdsa::second_message,
                ecdsa::chain_code_first_message,
                ecdsa::chain_code_second_message,
                ecdsa::sign_first,
                ecdsa::sign_second,
                ecdsa::rotate_first,
                ecdsa::rotate_second,
                ecdsa::recover,
                // schnorr::keygen_first,
                // schnorr::keygen_second,
                // schnorr::keygen_third,
                // schnorr::sign,
                // eddsa::keygen,
                // eddsa::sign_first,
                // eddsa::sign_second,
            ],
        )
        .manage(app_config)
}

fn get_settings_as_map() -> HashMap<String, String> {
    let config_file = include_str!("../Local.toml");
    let mut settings = config::Config::default();
    settings
        .merge(config::File::from_str(
            config_file,
            config::FileFormat::Toml,
        ))
        .unwrap()
        .merge(config::Environment::new())
        .unwrap();

    settings.try_into::<HashMap<String, String>>().unwrap()
}

fn get_db() -> db::DB {
    print!("Init RocksDB connection");
    match rocksdb::DB::open_default("./db") {
        Ok(db) => db::DB::Local(db),
        Err(e) => {
            println!("Error: {}", e);
            db::DB::ConnError("Failed to open rocksdb, please check your configuration".to_string())
        }
    }
}
