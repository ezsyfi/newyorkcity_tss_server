use rocket;
use rocket::{Request, Rocket};
use rocksdb;

use crate::utils::settings::get_hcmc_host;

use super::routes::*;
use super::storage::db;
use super::AppConfig;

use std::collections::HashMap;

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
    let settings = get_settings_as_map("env.staging.toml");
    let hcmc_config = get_hcmc_host(settings).unwrap();
    let app_config = AppConfig {
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

pub fn get_settings_as_map(file_path: &str) -> HashMap<String, String> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name(file_path))
        .build()
        .unwrap();

    settings
        .try_deserialize::<HashMap<String, String>>()
        .unwrap()
}

fn get_db() -> db::DB {
    match rocksdb::DB::open_default("./db") {
        Ok(db) => {
            print!("Init RocksDB connection successfully");
            db::DB::Local(db)
        }
        Err(e) => {
            error!("{:#?}", e);
            db::DB::ConnError(
                "Failed to connect RocksDB, please check your configuration".to_string(),
            )
        }
    }
}
