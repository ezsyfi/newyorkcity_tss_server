use rocket;
use rocket::{Request};
use rocksdb;


use crate::utils::settings::{get_app_env, get_hcmc_host};

use super::routes::*;
use super::storage::db;
use super::AppConfig;


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

#[launch]
pub fn get_server() -> _ {
    let env_configs = get_app_env(".env.staging");
    let hcmc_config = get_hcmc_host(&env_configs).unwrap();
    let app_config = AppConfig {
        db: get_db(),
        hcmc: hcmc_config,
        alchemy_api: env_configs.alchemy_api
    };

    rocket::build()
        .register("/", catchers![internal_error, not_found, bad_request])
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
                eth::tx_parameters,
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
