use anyhow::Result;

#[derive(Deserialize, Debug)]
pub struct AppEnv {
    pub hcmc_host: String,
    pub alchemy_api: String,
}

#[derive(Deserialize, Debug)]
pub struct TestEnv {
    pub test_signin_url: String,
    pub test_email: String,
    pub test_pass: String,
}

pub fn get_app_env(file_name: &str) -> AppEnv {
    dotenv::from_filename(file_name).unwrap_or_else(|_| panic!("Failed to read {}", file_name));
    match envy::from_env::<AppEnv>() {
        Ok(config) => config,
        Err(e) => panic!("Couldn't read app env config ({})", e),
    }
}

pub fn get_test_env(file_name: &str) -> TestEnv {
    dotenv::from_filename(file_name).unwrap_or_else(|_| panic!("Failed to read {}", file_name));
    match envy::from_env::<TestEnv>() {
        Ok(config) => config,
        Err(e) => panic!("Couldn't read env config ({})", e),
    }
}
