#![recursion_limit = "128"]
#![feature(proc_macro_hygiene)]
#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate time_test;
extern crate jsonwebtoken as jwt;

pub mod auth;
pub mod routes;
pub mod server;
pub mod storage;
pub mod tests;
pub mod utils;

type Result<T> = std::result::Result<T, failure::Error>;

pub struct Config {
    pub db: storage::db::DB,
    pub hcmc: utils::settings::HcmcConfig,
}
