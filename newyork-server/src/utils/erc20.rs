use std::{collections::HashMap, fs, str::FromStr};

use anyhow::{anyhow, Result};
use web3::{contract::Contract, transports::WebSocket, types::Address, Web3};

pub const ERC20_ADDRESSES_JSON: &str = r#"{
    "rinkeby": {
      "usdt": "0x3B00Ef435fA4FcFF5C209a37d1f3dcff37c705aD"
    }
  }  
  "#;

pub fn get_contract_abi(
    token_network: &str,
    token_name: &str,
    web3: Web3<WebSocket>,
) -> Result<web3::ethabi::Contract> {
    let token_path = format!("{}/{}", token_network, token_name);

    let token_address_map: HashMap<&str, &str> =
        HashMap::from([("rinkeby/usdt", "0x3B00Ef435fA4FcFF5C209a37d1f3dcff37c705aD")]);

    let token_address = match token_address_map.get(token_path.as_str()) {
        Some(map) => map,
        None => {
            return Err(anyhow!("Can't find token that matches provided name"));
        }
    };
    let contract_address = match Address::from_str(token_address) {
        Ok(s) => s,
        Err(err) => {
            return Err(anyhow!("Failed to parse ERC20 address: {}", err));
        }
    };
    let abi_path = format!("eth_chain/{}.json", token_path);
    let contents = fs::read(abi_path).expect("Something went wrong reading the file");
    let contract = match Contract::from_json(web3.eth(), contract_address, &contents) {
        Ok(c) => c,
        Err(err) => {
            return Err(anyhow!("Failed to parse ERC20 contract: {}", err));
        }
    };
    Ok(contract.abi().clone())
}
