


use anyhow::Result;
use rocket::State;
use rocket::serde::json::Json;
use web3::{Web3, transports};
use web3::types::{TransactionParameters, Address, U256, AccessList, U64, SignedTransaction};


use super::super::auth::guards::AuthPayload;
use super::super::AppConfig;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EthTxParamsResp {
    pub to: Option<Address>,
    pub nonce: U256,
    pub gas: U256,
    pub gas_price: U256,
    pub value: U256,
    pub data: Vec<u8>,
    pub transaction_type: Option<U64>,
    pub access_list: AccessList,
    pub max_priority_fee_per_gas: U256,
    pub chain_id: u64
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EthTxParamsReqBody {
    pub from_address: Address,
    pub to_address: Address,
    pub eth_value: f64
}

// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
// pub struct EthSignedTx {
//     pub signed_tx: SignedTransaction,
// }

const EIP1559_TX_ID: u64 = 2;

#[post("/eth/tx/params", format = "json", data = "<tx_info>")]
pub async fn tx_parameters(
    state: &State<AppConfig>,
    auth_payload: AuthPayload,
    tx_info: Json<EthTxParamsReqBody>
) -> Result<Json<EthTxParamsResp>, rocket::response::Debug<anyhow::Error>> {
    print!("tx_info {:?}", tx_info);

    let tx_params = create_eth_transaction(tx_info.to_address, tx_info.eth_value)?;

    println!("tx_params {:?}", tx_params);
    println!("alchemy_api {:?}", &state.alchemy_api);

    let web3 = establish_web3_connection(&state.alchemy_api).await?;
    println!("web3 {:?}", web3);

    let (nonce, gas_price, chain_id) = get_chain_required_params(tx_info.from_address, tx_params.clone(), web3).await?;

    let max_priority_fee_per_gas = match tx_params.transaction_type {
        Some(tx_type) if tx_type == U64::from(EIP1559_TX_ID) => {
            tx_params.max_priority_fee_per_gas.unwrap_or(gas_price)
        }
        _ => gas_price,
    };

    let resp = EthTxParamsResp {
        to: tx_params.to,
        nonce,
        gas: tx_params.gas,
        gas_price,
        value: tx_params.value,
        data: tx_params.data.0,
        transaction_type: tx_params.transaction_type,
        access_list: tx_params.access_list.unwrap_or_default(),
        max_priority_fee_per_gas,
        chain_id
    };

    println!("{:?}", resp);

    Ok(Json(resp))
}

// #[post("/eth/tx/sign", format = "json", data = "<tx_info>")]
// pub fn tx_params(
//     state: State<AppConfig>,
//     auth_payload: AuthPayload,
//     tx_info: Json<EthTxParamsReqBody>
// ) -> Result<Json<EthTxParamsResp>> {

//     let tx_params = create_eth_transaction(tx_info.to_address, tx_info.eth_value)?;
//     let web3 = establish_web3_connection(&state.alchemy_api)?;
//     let (nonce, gas_price, chain_id) = get_chain_required_params(tx_info.from_address, &tx_params, &web3)?;

//     let max_priority_fee_per_gas = match tx_params.transaction_type {
//         Some(tx_type) if tx_type == U64::from(EIP1559_TX_ID) => {
//             tx_params.max_priority_fee_per_gas.unwrap_or(gas_price)
//         }
//         _ => gas_price,
//     };
//     Ok(Json(EthTxParamsResp {
//         to: tx_params.to,
//         nonce,
//         gas: tx_params.gas,
//         gas_price,
//         value: tx_params.value,
//         data: tx_params.data.0,
//         transaction_type: tx_params.transaction_type,
//         access_list: tx_params.access_list.unwrap_or_default(),
//         max_priority_fee_per_gas,
//         chain_id
//     }))
// }

fn create_eth_transaction(to: Address, eth_value: f64) -> Result<TransactionParameters> {
    Ok(TransactionParameters {
        to: Some(to),
        value: eth_to_wei(eth_value),
        ..Default::default()
    })
}

// #[tokio::main]
pub async fn establish_web3_connection(url: &str) -> Result<Web3<transports::WebSocket>> {
    let transport = transports::WebSocket::new(url).await?;
    Ok(Web3::new(transport))
}

// #[tokio::main]
pub async fn get_chain_required_params(from_address: Address, tx_params: TransactionParameters, web3: Web3<transports::WebSocket>) -> Result<(U256, U256, u64)> {
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
    println!("a {:?} {:?} {:?}",gas_price, from_address, web3);

    // let (nonce, gas_price, chain_id) = futures::future::try_join3(
    //     maybe!(
    //         tx_params.nonce,
    //         web3.eth().transaction_count(from_address, None)
    //     ),
    //     maybe!(gas_price, web3.eth().gas_price()),
    //     maybe!(tx_params.chain_id.map(U256::from), web3.eth().chain_id()),
    // )
    // .await?;
    let chain_id = web3.eth().chain_id().await?.as_u64();
    let nonce = tx_params.nonce.unwrap();
    println!("b {:?} {:?} {:?}", tx_params.nonce, gas_price, chain_id);
    Ok((nonce, gas_price.unwrap(), chain_id))
}

pub fn eth_to_wei(eth_value: f64) -> U256 {
    let result = eth_value * 1_000_000_000_000_000_000.0;
    let result = result as u128;

    U256::from(result)
}