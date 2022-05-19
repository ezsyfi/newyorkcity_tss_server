use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use rocket::serde::json::Json;
use rocket::State;
use web3::contract::Contract;
use web3::types::{Address, Bytes, TransactionParameters, H256, U256, U64};
use web3::{transports, Web3};

use crate::utils::requests::validate_auth_token;
use crate::AnyhowError;

use super::super::auth::guards::AuthPayload;
use super::super::AppConfig;
use super::ERC20_ADDRESSES_JSON;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct EthTxParamsResp {
    pub nonce: U256,
    pub gas: U256,
    pub gas_price: U256,
    pub transaction_type: Option<U64>,
    pub max_priority_fee_per_gas: U256,
    pub chain_id: u64,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct EthTxParamsReqBody {
    pub from_address: Address,
    pub to_address: Address,
    pub eth_value: f64,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct EthSendTxResp {
    pub tx_hash: H256,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct EthSendTxReqBody {
    pub raw_tx: Bytes,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Erc20ReqBody {
    pub name: String,
    pub network: String,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Erc20Resp {
    pub contract: web3::ethabi::Contract,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Erc20 {
    pub address: String,
    pub abi: String,
}

const EIP1559_TX_ID: u64 = 2;

#[post("/eth/tx/params", format = "json", data = "<tx_info>")]
pub async fn tx_parameters(
    state: &State<AppConfig>,
    auth_payload: AuthPayload,
    tx_info: Json<EthTxParamsReqBody>,
) -> Result<Json<EthTxParamsResp>, AnyhowError> {
    validate_auth_token(state, &auth_payload).await?;
    let web3 = establish_web3_connection(&state.alchemy_api).await?;

    let tx_params = TransactionParameters {
        ..Default::default()
    };
    let (nonce, gas_price, chain_id) =
        get_chain_required_params(tx_info.from_address, tx_params.clone(), web3).await?;

    let max_priority_fee_per_gas = match tx_params.transaction_type {
        Some(tx_type) if tx_type == U64::from(EIP1559_TX_ID) => {
            tx_params.max_priority_fee_per_gas.unwrap_or(gas_price)
        }
        _ => gas_price,
    };

    let resp = EthTxParamsResp {
        nonce,
        gas: tx_params.gas,
        gas_price,
        transaction_type: tx_params.transaction_type,
        max_priority_fee_per_gas,
        chain_id,
    };

    Ok(Json(resp))
}

#[post("/eth/contract", format = "json", data = "<token_info>")]
pub async fn contract_data(
    state: &State<AppConfig>,
    auth_payload: AuthPayload,
    token_info: Json<Erc20ReqBody>,
) -> Result<Json<Erc20Resp>, AnyhowError> {
    validate_auth_token(state, &auth_payload).await?;
    let web3 = establish_web3_connection(&state.alchemy_api).await?;

    let network_token_map: HashMap<String, HashMap<String, Erc20>> =
        match serde_json::from_str(ERC20_ADDRESSES_JSON) {
            Ok(map) => map,
            Err(err) => {
                return Err(AnyhowError::from(anyhow!(
                    "Failed to parse ERC20 network map: {}",
                    err
                )));
            }
        };

    let name_token_map = match network_token_map.get(&token_info.network) {
        Some(map) => map,
        None => {
            return Err(AnyhowError::from(anyhow!(
                "Can't find token collection of provided network"
            )));
        }
    };

    let token_obj = match name_token_map.get(&token_info.name) {
        Some(map) => map,
        None => {
            return Err(AnyhowError::from(anyhow!(
                "Can't find token that matches provided name"
            )));
        }
    };
    let contract_address = match Address::from_str(&token_obj.address) {
        Ok(s) => s,
        Err(err) => {
            return Err(AnyhowError::from(anyhow!(
                "Failed to parse ERC20 address: {}",
                err
            )));
        }
    };

    let contract = match Contract::from_json(web3.eth(), contract_address, token_obj.abi.as_bytes())
    {
        Ok(c) => c,
        Err(err) => {
            return Err(AnyhowError::from(anyhow!(
                "Failed to parse ERC20 contract: {}",
                err
            )));
        }
    };

    Ok(Json(Erc20Resp {
        contract: contract.abi().clone(),
    }))
}

#[post("/eth/tx/send", format = "json", data = "<signed>")]
pub async fn tx_send(
    state: &State<AppConfig>,
    auth_payload: AuthPayload,
    signed: Json<EthSendTxReqBody>,
) -> Result<Json<EthSendTxResp>, AnyhowError> {
    validate_auth_token(state, &auth_payload).await?;
    let web3 = establish_web3_connection(&state.alchemy_api).await?;
    let tx_hash = send_tx(web3, signed.raw_tx.clone()).await?;

    Ok(Json(EthSendTxResp { tx_hash }))
}

pub async fn establish_web3_connection(url: &str) -> Result<Web3<transports::WebSocket>> {
    let transport = transports::WebSocket::new(url).await?;
    Ok(Web3::new(transport))
}

pub async fn get_chain_required_params(
    from_address: Address,
    tx_params: TransactionParameters,
    web3: Web3<transports::WebSocket>,
) -> Result<(U256, U256, u64)> {
    macro_rules! maybe {
        ($o: expr, $f: expr) => {
            async {
                match $o {
                    Some(value) => Ok(value),
                    None => $f.await,
                }
            }
        };
    }

    let gas_price = match tx_params.transaction_type {
        Some(tx_type)
            if tx_type == U64::from(EIP1559_TX_ID) && tx_params.max_fee_per_gas.is_some() =>
        {
            tx_params.max_fee_per_gas
        }
        _ => tx_params.gas_price,
    };

    let (nonce, gas_price, chain_id) = futures::future::try_join3(
        maybe!(
            tx_params.nonce,
            web3.eth().transaction_count(from_address, None)
        ),
        maybe!(gas_price, web3.eth().gas_price()),
        maybe!(tx_params.chain_id.map(U256::from), web3.eth().chain_id()),
    )
    .await?;

    Ok((nonce, gas_price, chain_id.as_u64()))
}

pub async fn send_tx(web3: Web3<transports::WebSocket>, raw_tx: Bytes) -> Result<H256> {
    let tx_hash = web3.eth().send_raw_transaction(raw_tx).await?;
    Ok(tx_hash)
}
