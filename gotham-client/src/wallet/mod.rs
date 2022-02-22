// Gotham-city
//
// Copyright 2018 by Kzen Networks (kzencorp.com)
// Gotham city is free software: you can redistribute
// it and/or modify it under the terms of the GNU General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//

use bitcoin;
use bitcoin::consensus::encode::serialize;
use bitcoin::hashes::{hex::FromHex, sha256d};
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::Signature;
use bitcoin::util::bip143::SighashComponents;
use bitcoin::{TxIn, TxOut, Txid};
use curv::elliptic::curves::secp256_k1::{GE, PK};
use curv::elliptic::curves::traits::ECPoint;
use curv::BigInt;
use kms::ecdsa::two_party::MasterKey2;
use kms::ecdsa::two_party::*;
use serde_json;
use std::fs;
use uuid::Uuid;

use centipede::juggling::proof_system::{Helgamalsegmented, Proof};
use centipede::juggling::segmentation::Msegmentation;
use kms::chain_code::two_party::party2::ChainCode2;

use super::ecdsa;
use super::ecdsa::types::PrivateShare;
use super::escrow;
use super::utilities::requests;
use super::ClientShim;
use curv::arithmetic::traits::Converter;
use hex;
use itertools::Itertools;
use std::collections::HashMap;
use std::str::FromStr;

// TODO: move that to a config file and double check electrum server addresses
const WALLET_FILENAME: &str = "wallet/wallet.data";
const BACKUP_FILENAME: &str = "wallet/backup.data";
const BLOCK_CYPHER_HOST: &str = "https://api.blockcypher.com/v1/btc/test3";

#[derive(Serialize, Deserialize)]
pub struct SignSecondMsgRequest {
    pub message: BigInt,
    pub party_two_sign_message: party2::SignMessage,
    pub pos_child_key: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetBalanceResponse {
    pub address: String,
    pub confirmed: u64,
    pub unconfirmed: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GetListUnspentResponse {
    pub height: isize,
    pub tx_hash: String,
    pub tx_pos: usize,
    pub value: usize,
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct GetWalletBalanceResponse {
    pub confirmed: u64,
    pub unconfirmed: i64,
}

#[derive(Serialize, Deserialize)]
pub struct AddressDerivation {
    pub pos: u32,
    pub mk: MasterKey2,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BlockCypherAddress {
    address: String,
    total_received: u64,
    total_sent: u64,
    balance: u64,
    unconfirmed_balance: i64,
    final_balance: u64,
    n_tx: u64,
    unconfirmed_n_tx: u64,
    final_n_tx: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    txrefs: Option<Vec<BlockCypherTxRef>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct BlockCypherTxRef {
    pub tx_hash: String,
    pub block_height: isize,
    pub tx_input_n: isize,
    pub tx_output_n: usize,
    pub value: usize,
    pub ref_balance: u64,
    pub spent: bool,
    pub confirmations: u64,
    pub confirmed: String,
}

#[derive(Serialize, Deserialize)]
struct BlockCypherRawTx {
    pub tx: String,
}

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    pub id: String,
    pub network: String,
    pub private_share: PrivateShare,
    pub last_derived_pos: u32,
    pub addresses_derivation_map: HashMap<String, AddressDerivation>,
}

impl Wallet {
    pub fn new(client_shim: &ClientShim, net: &String) -> Wallet {
        let id = Uuid::new_v4().to_string();
        let private_share = ecdsa::get_master_key(client_shim);
        let last_derived_pos = 0;
        let addresses_derivation_map = HashMap::new();
        let network = net.clone();

        Wallet {
            id,
            network,
            private_share,
            last_derived_pos,
            addresses_derivation_map,
        }
    }

    pub fn rotate(self, client_shim: &ClientShim) -> Self {
        ecdsa::rotate_master_key(self, client_shim)
    }

    pub fn backup(&self, escrow_service: escrow::Escrow) {
        let g: GE = ECPoint::generator();
        let y = escrow_service.get_public_key();
        let (segments, encryptions) = self.private_share.master_key.private.to_encrypted_segment(
            escrow::SEGMENT_SIZE,
            escrow::NUM_SEGMENTS,
            &y,
            &g,
        );

        let proof = Proof::prove(&segments, &encryptions, &g, &y, &escrow::SEGMENT_SIZE);

        let client_backup_json = serde_json::to_string(&(
            encryptions,
            proof,
            self.private_share.master_key.public.clone(),
            self.private_share.master_key.chain_code.clone(),
            self.private_share.id.clone(),
        ))
        .unwrap();

        fs::write(BACKUP_FILENAME, client_backup_json).expect("Unable to save client backup!");

        debug!("(wallet id: {}) Backup wallet with escrow", self.id);
    }

    pub fn verify_backup(&self, escrow_service: escrow::Escrow) {
        let g: GE = ECPoint::generator();
        let y = escrow_service.get_public_key();

        let data = fs::read_to_string(BACKUP_FILENAME).expect("Unable to load client backup!");
        let (encryptions, proof, client_public, _, _): (
            Helgamalsegmented,
            Proof,
            Party2Public,
            ChainCode2,
            String,
        ) = serde_json::from_str(&data).unwrap();
        let verify = proof.verify(
            &encryptions,
            &g,
            &y,
            &client_public.p2,
            &escrow::SEGMENT_SIZE,
        );
        match verify {
            Ok(_x) => println!("backup verified 🍻"),
            Err(_e) => println!("Backup was not verified correctly 😲"),
        }
    }

    pub fn recover_and_save_share(
        escrow_service: escrow::Escrow,
        net: &String,
        client_shim: &ClientShim,
    ) -> Wallet {
        let g: GE = ECPoint::generator();
        let y_priv = escrow_service.get_private_key();

        let data = fs::read_to_string(BACKUP_FILENAME).expect("Unable to load client backup!");

        let (encryptions, _proof, public_data, chain_code2, key_id): (
            Helgamalsegmented,
            Proof,
            Party2Public,
            BigInt,
            String,
        ) = serde_json::from_str(&data).unwrap();

        let sk = Msegmentation::decrypt(&encryptions, &g, &y_priv, &escrow::SEGMENT_SIZE);

        let client_master_key_recovered =
            MasterKey2::recover_master_key(sk.unwrap(), public_data, chain_code2);
        let pos_old: u32 =
            requests::post(client_shim, &format!("ecdsa/{}/recover", key_id)).unwrap();

        let pos_old = if pos_old < 10 { 10 } else { pos_old };
        //TODO: temporary, server will keep updated pos, to do so we need to send update to server for every get_new_address

        let id = Uuid::new_v4().to_string();
        let addresses_derivation_map = HashMap::new(); //TODO: add a fucntion to recreate
        let network = net.clone();

        let new_wallet = Wallet {
            id,
            network,
            private_share: PrivateShare {
                master_key: client_master_key_recovered,
                id: key_id,
            },
            last_derived_pos: pos_old,
            addresses_derivation_map,
        };

        new_wallet.save();
        println!("Recovery Completed Successfully ❤️");

        new_wallet
    }

    pub fn save_to(&self, filepath: &str) {
        let wallet_json = serde_json::to_string(self).unwrap();

        fs::write(filepath, wallet_json).expect("Unable to save wallet!");

        debug!("(wallet id: {}) Saved wallet to disk", self.id);
    }

    pub fn save(&self) {
        self.save_to(WALLET_FILENAME)
    }

    pub fn load_from(filepath: &str) -> Wallet {
        let data = fs::read_to_string(filepath).expect("Unable to load wallet!");

        let wallet: Wallet = serde_json::from_str(&data).unwrap();

        debug!("(wallet id: {}) Loaded wallet to memory", wallet.id);

        wallet
    }

    pub fn load() -> Wallet {
        Wallet::load_from(WALLET_FILENAME)
    }

    pub fn send(
        &mut self,
        to_address: String,
        amount_btc: f32,
        client_shim: &ClientShim,
    ) -> String {
        let selected = self.select_tx_in(amount_btc);
        if selected.is_empty() {
            panic!("Not enough fund");
        }

        let to_btc_adress = bitcoin::Address::from_str(&to_address).unwrap();

        let txs_in: Vec<TxIn> = selected
            .clone()
            .into_iter()
            .map(|s| bitcoin::TxIn {
                previous_output: bitcoin::OutPoint {
                    txid: Txid::from_hash(sha256d::Hash::from_hex(&s.tx_hash).unwrap()),
                    vout: s.tx_pos as u32,
                },
                script_sig: bitcoin::Script::default(),
                sequence: 0xFFFFFFFF,
                witness: Vec::default(),
            })
            .collect();

        // let fees = 10_000;

        let amount_satoshi = (amount_btc * 100_000_000 as f32) as u64;

        let change_address = self.get_new_bitcoin_address();

        let total_selected = selected
            .clone()
            .into_iter()
            .fold(0, |sum, val| sum + val.value) as u64;

        println!(
            "amount_satoshi: {} - total_selected: {}  ",
            amount_satoshi, total_selected
        );
        println!("{} - back", total_selected - amount_satoshi);

        let txs_out = vec![
            TxOut {
                value: amount_satoshi,
                script_pubkey: to_btc_adress.script_pubkey(),
            },
            TxOut {
                value: total_selected - amount_satoshi,
                script_pubkey: change_address.script_pubkey(),
            },
        ];

        let transaction = bitcoin::Transaction {
            version: 0,
            lock_time: 0,
            input: txs_in,
            output: txs_out,
        };

        let mut signed_transaction = transaction.clone();

        for i in 0..transaction.input.len() {
            let address_derivation = self
                .addresses_derivation_map
                .get(&selected[i].address)
                .unwrap();

            let mk = &address_derivation.mk;
            let pk = mk.public.q.get_element();

            let comp = SighashComponents::new(&transaction);
            let sig_hash = comp.sighash_all(
                &transaction.input[i],
                &bitcoin::Address::p2pkh(&to_bitcoin_public_key(pk), self.get_bitcoin_network())
                    .script_pubkey(),
                (selected[i].value as u32).into(),
            );

            let signature = ecdsa::sign(
                client_shim,
                BigInt::from_hex(&hex::encode(&sig_hash[..])).unwrap(),
                &mk,
                BigInt::from(0),
                BigInt::from(address_derivation.pos),
                &self.private_share.id,
            )
            .unwrap();

            let mut v = BigInt::to_bytes(&signature.r);
            v.extend(BigInt::to_bytes(&signature.s));

            let mut sig_vec = Signature::from_compact(&v[..])
                .unwrap()
                .serialize_der()
                .to_vec();
            sig_vec.push(01);

            let pk_vec = pk.serialize().to_vec();

            signed_transaction.input[i].witness = vec![sig_vec, pk_vec];
        }

        let raw_tx_hex = hex::encode(serialize(&signed_transaction));

        let raw_tx_url = BLOCK_CYPHER_HOST.to_owned() + "/txs/push";
        let raw_tx = BlockCypherRawTx {
            tx: raw_tx_hex.clone(),
        };
        let res = reqwest::blocking::Client::new()
            .post(raw_tx_url)
            .json(&raw_tx)
            .send()
            .unwrap()
            .text()
            .unwrap();
            
        print!("{}", res);

        res
    }

    pub fn get_new_bitcoin_address(&mut self) -> bitcoin::Address {
        let (pos, mk) = Self::derive_new_key(&self.private_share, self.last_derived_pos);
        let pk = mk.public.q.get_element();
        let address =
            bitcoin::Address::p2wpkh(&to_bitcoin_public_key(pk), self.get_bitcoin_network())
                .expect(
                    "Cannot panic because `to_bitcoin_public_key` creates a compressed address",
                );

        self.addresses_derivation_map
            .insert(address.to_string(), AddressDerivation { mk, pos });

        self.last_derived_pos = pos;

        address
    }

    pub fn derived(&mut self) {
        for i in 0..self.last_derived_pos {
            let (pos, mk) = Self::derive_new_key(&self.private_share, i);

            let address = bitcoin::Address::p2wpkh(
                &to_bitcoin_public_key(mk.public.q.get_element()),
                self.get_bitcoin_network(),
            )
            .expect("Cannot panic because `to_bitcoin_public_key` creates a compressed address");

            self.addresses_derivation_map
                .insert(address.to_string(), AddressDerivation { mk, pos });
        }
    }

    pub fn get_balance(&mut self) -> GetWalletBalanceResponse {
        let mut aggregated_balance = GetWalletBalanceResponse {
            confirmed: 0,
            unconfirmed: 0,
        };

        for b in self.get_all_addresses_balance() {
            aggregated_balance.unconfirmed += b.unconfirmed;
            aggregated_balance.confirmed += b.confirmed;
        }

        aggregated_balance
    }

    // TODO: handle fees
    // Select all txin enough to pay the amount
    fn select_tx_in(&self, amount_btc: f32) -> Vec<GetListUnspentResponse> {
        // greedy selection
        let list_unspent: Vec<GetListUnspentResponse> = self
            .get_all_addresses_balance()
            .into_iter()
            //            .filter(|b| b.confirmed > 0)
            .map(|a| self.list_unspent_for_addresss(a.address.to_string()))
            .flatten()
            .sorted_by(|a, b| a.value.partial_cmp(&b.value).unwrap())
            .into_iter()
            .collect();

        let mut remaining: i64 = (amount_btc * 100_000_000.0) as i64;
        let mut selected: Vec<GetListUnspentResponse> = Vec::new();
        for unspent in list_unspent {
            selected.push(unspent.clone());
            remaining -= unspent.value as i64;
            if remaining < 0 {
                break;
            }
        }
        selected
    }

    pub fn list_unspent(&self) -> Vec<GetListUnspentResponse> {
        let response: Vec<GetListUnspentResponse> = self
            .get_all_addresses()
            .into_iter()
            .map(|a| self.list_unspent_for_addresss(a.to_string()))
            .flatten()
            .collect();
        
        response
    }

    /* PRIVATE */
    fn list_unspent_for_addresss(&self, address: String) -> Vec<GetListUnspentResponse> {
        let unspent_tx_url =
            BLOCK_CYPHER_HOST.to_owned() + "/addrs/" + &address.to_string() + "?unspentOnly=true";
        let res = reqwest::blocking::get(unspent_tx_url)
            .unwrap()
            .text()
            .unwrap();
        let address_balance_with_tx_refs: BlockCypherAddress =
            serde_json::from_str(res.as_str()).unwrap();
        if let Some(tx_refs) = address_balance_with_tx_refs.txrefs {
            tx_refs
                .iter()
                .map(|u| GetListUnspentResponse {
                    value: u.value,
                    height: u.block_height,
                    tx_hash: u.tx_hash.clone(),
                    tx_pos: u.tx_output_n,
                    address: address.clone(),
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn get_address_balance(address: &bitcoin::Address) -> GetBalanceResponse {
        let balance_url =
            BLOCK_CYPHER_HOST.to_owned() + "/addrs/" + &address.to_string() + "/balance";
        let res = reqwest::blocking::get(balance_url).unwrap().text().unwrap();
        let address_balance: BlockCypherAddress = serde_json::from_str(res.as_str()).unwrap();
        println!("{:#?}", address_balance);

        GetBalanceResponse {
            confirmed: address_balance.balance,
            unconfirmed: address_balance.unconfirmed_balance,
            address: address.to_string(),
        }
    }

    fn get_all_addresses_balance(&self) -> Vec<GetBalanceResponse> {
        let response: Vec<GetBalanceResponse> = self
            .get_all_addresses()
            .into_iter()
            .map(|a| Self::get_address_balance(&a))
            .collect();

        response
    }

    fn get_all_addresses(&self) -> Vec<bitcoin::Address> {
        let init = 0;
        let last_pos = self.last_derived_pos;

        let mut response: Vec<bitcoin::Address> = Vec::new();

        for n in init..=last_pos {
            let mk = self
                .private_share
                .master_key
                .get_child(vec![BigInt::from(0), BigInt::from(n)]);
            let bitcoin_address = Self::to_bitcoin_address(&mk, self.get_bitcoin_network());

            response.push(bitcoin_address);
        }

        response
    }

    fn derive_new_key(private_share: &PrivateShare, pos: u32) -> (u32, MasterKey2) {
        let last_pos: u32 = pos + 1;

        let last_child_master_key = private_share
            .master_key
            .get_child(vec![BigInt::from(0), BigInt::from(last_pos)]);

        (last_pos, last_child_master_key)
    }

    fn get_bitcoin_network(&self) -> Network {
        self.network.parse::<Network>().unwrap()
    }

    fn to_bitcoin_address(mk: &MasterKey2, network: Network) -> bitcoin::Address {
        bitcoin::Address::p2wpkh(&to_bitcoin_public_key(mk.public.q.get_element()), network)
            .expect("Cannot panic because `to_bitcoin_public_key` creates a compressed address")
    }
}

// type conversion
fn to_bitcoin_public_key(pk: PK) -> bitcoin::util::key::PublicKey {
    bitcoin::util::key::PublicKey {
        compressed: true,
        key: pk,
    }
}

#[cfg(test)]
mod tests {
    use bitcoin::hashes::hex::ToHex;
    use bitcoin::hashes::sha256d;
    use bitcoin::hashes::Hash;
    use curv::arithmetic::traits::Converter;
    use curv::BigInt;

    #[test]
    fn test_message_conv() {
        let message: [u8; 32] = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        // 14abf5ed107ff58bf844ee7f447bec317c276b00905c09a45434f8848599597e
        let hash = sha256d::Hash::from_slice(&message).unwrap();

        // 7e59998584f83454a4095c90006b277c31ec7b447fee44f88bf57f10edf5ab14
        let ser = hash.to_hex();

        // 57149727877124134702546803488322951680010683936655914236113461592936003513108
        let b: BigInt = BigInt::from_hex(&ser).unwrap();

        println!("({},{},{})", hash, ser, b.to_hex());
    }
}
