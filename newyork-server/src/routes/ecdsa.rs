#![allow(non_snake_case)]

use std::fmt::Debug;

use super::super::Result;
use curv::cryptographic_primitives::proofs::sigma_dlog::*;
use curv::cryptographic_primitives::twoparty::coin_flip_optimal_rounds;
use curv::cryptographic_primitives::twoparty::dh_key_exchange_variant_with_pok_comm::{
    CommWitness, EcKeyPair, Party1FirstMessage, Party1SecondMessage,
};
use curv::elliptic::curves::secp256_k1::Secp256k1Scalar;
use curv::elliptic::curves::secp256_k1::GE;
use curv::BigInt;
use kms::chain_code::two_party as chain_code;
use kms::ecdsa::two_party::*;
use kms::rotation::two_party::party1::Rotation1;
use multi_party_ecdsa::protocols::two_party_ecdsa::lindell_2017::*;
use rocket::State;
use rocket_contrib::json::Json;
use uuid::Uuid;

use super::super::auth::jwt::AuthPayload;
use super::super::storage::db;
use super::super::Config;
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct HDPos {
    pos: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Alpha {
    value: BigInt,
}

#[derive(Debug)]
pub enum EcdsaStruct {
    KeyGenFirstMsg,
    CommWitness,
    EcKeyPair,
    PaillierKeyPair,
    Party1Private,
    Party2Public,

    PDLProver,
    PDLDecommit,
    Alpha,
    Party2PDLFirstMsg,

    CCKeyGenFirstMsg,
    CCCommWitness,
    CCEcKeyPair,
    CC,

    Party1MasterKey,

    EphEcKeyPair,
    EphKeyGenFirstMsg,

    RotateCommitMessage1M,
    RotateCommitMessage1R,
    RotateRandom1,
    RotateFirstMsg,
    RotatePrivateNew,
    RotatePdlDecom,
    RotateParty2First,
    RotateParty1Second,

    POS,
}

impl db::MPCStruct for EcdsaStruct {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }

    fn require_customer_id(&self) -> bool {
        self.to_string() == "Party1MasterKey"
    }
}

#[post("/ecdsa/keygen/first", format = "json")]
pub fn first_message(
    state: State<Config>,
    auth_payload: AuthPayload,
) -> Result<Json<(String, party_one::KeyGenFirstMsg)>> {
    let id = Uuid::new_v4().to_string();

    let (key_gen_first_msg, comm_witness, ec_key_pair) = MasterKey1::key_gen_first_message();

    //save pos 0
    db::insert(
        &state.db, // current DB connection state
        &auth_payload.user_id,
        &id, // uuid to unify DB column key
        &EcdsaStruct::POS,
        &HDPos { pos: 0u32 }, // Initial HD position
    )?;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::KeyGenFirstMsg,
        &key_gen_first_msg,
    )?;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CommWitness,
        &comm_witness,
    )?;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::EcKeyPair,
        &ec_key_pair,
    )?;

    Ok(Json((id, key_gen_first_msg)))
}

#[post("/ecdsa/keygen/<id>/second", format = "json", data = "<dlog_proof>")]
pub fn second_message(
    state: State<Config>,
    auth_payload: AuthPayload,
    id: String,
    dlog_proof: Json<DLogProof<GE>>,
) -> Result<Json<party1::KeyGenParty1Message2>> {
    let party2_public: GE = dlog_proof.0.pk;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::Party2Public,
        &party2_public,
    )?;

    let comm_witness: party_one::CommWitness = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CommWitness,
    )?
    .ok_or_else(|| format_err!("No CommWitness for such identifier {}", id))?;

    let ec_key_pair: party_one::EcKeyPair = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::EcKeyPair,
    )?
    .ok_or_else(|| format_err!("No EcKeyPair for such identifier {}", id))?;

    let (kg_party_one_second_message, paillier_key_pair, party_one_private) =
        MasterKey1::key_gen_second_message(comm_witness, &ec_key_pair, &dlog_proof.0);

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::PaillierKeyPair,
        &paillier_key_pair,
    )?;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::Party1Private,
        &party_one_private,
    )?;

    Ok(Json(kg_party_one_second_message))
}

#[post("/ecdsa/keygen/<id>/chaincode/first", format = "json")]
pub fn chain_code_first_message(
    state: State<Config>,
    auth_payload: AuthPayload,
    id: String,
) -> Result<Json<Party1FirstMessage>> {
    let (cc_party_one_first_message, cc_comm_witness, cc_ec_key_pair1) =
        chain_code::party1::ChainCode1::chain_code_first_message();

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CCKeyGenFirstMsg,
        &cc_party_one_first_message,
    )?;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CCCommWitness,
        &cc_comm_witness,
    )?;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CCEcKeyPair,
        &cc_ec_key_pair1,
    )?;

    Ok(Json(cc_party_one_first_message))
}

#[post(
    "/ecdsa/keygen/<id>/chaincode/second",
    format = "json",
    data = "<cc_party_two_first_message_d_log_proof>"
)]
pub fn chain_code_second_message(
    state: State<Config>,
    auth_payload: AuthPayload,
    id: String,
    cc_party_two_first_message_d_log_proof: Json<DLogProof<GE>>,
) -> Result<Json<Party1SecondMessage<GE>>> {
    let cc_comm_witness: CommWitness<GE> = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CCCommWitness,
    )?
    .ok_or_else(|| format_err!("No CCCommWitness for such identifier {}", id))?;

    let party1_cc = chain_code::party1::ChainCode1::chain_code_second_message(
        cc_comm_witness,
        &cc_party_two_first_message_d_log_proof.0,
    );

    let party2_pub = &cc_party_two_first_message_d_log_proof.pk;

    chain_code_compute_message(state, auth_payload, id, party2_pub)?;

    Ok(Json(party1_cc))
}

pub fn chain_code_compute_message(
    state: State<Config>,
    auth_payload: AuthPayload,
    id: String,
    cc_party2_public: &GE,
) -> Result<Json<()>> {
    let cc_ec_key_pair_party1: EcKeyPair<GE> = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CCEcKeyPair,
    )?
    .ok_or_else(|| format_err!("No CCEcKeyPair for such identifier {}", id))?;
    let party1_cc = chain_code::party1::ChainCode1::compute_chain_code(
        &cc_ec_key_pair_party1,
        cc_party2_public,
    );

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CC,
        &party1_cc,
    )?;
    master_key(state, auth_payload, id)?;
    Ok(Json(()))
}

pub fn master_key(state: State<Config>, auth_payload: AuthPayload, id: String) -> Result<()> {
    let party2_public: GE = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::Party2Public,
    )?
    .ok_or_else(|| format_err!("No Party2Public for such identifier {}", id))?;

    let paillier_key_pair: party_one::PaillierKeyPair = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::PaillierKeyPair,
    )?
    .ok_or_else(|| format_err!("No PaillierKeyPair for such identifier {}", id))?;

    let party1_cc: chain_code::party1::ChainCode1 =
        db::get(&state.db, &auth_payload.user_id, &id, &EcdsaStruct::CC)?
            .ok_or_else(|| format_err!("No CC for such identifier {}", id))?;

    let party_one_private: party_one::Party1Private = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::Party1Private,
    )?
    .ok_or_else(|| format_err!("No Party1Private for such identifier {}", id))?;

    let comm_witness: party_one::CommWitness = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::CommWitness,
    )?
    .ok_or_else(|| format_err!("No CommWitness for such identifier {}", id))?;

    let masterKey = MasterKey1::set_master_key(
        &party1_cc.chain_code,
        party_one_private,
        &comm_witness.public_share,
        &party2_public,
        paillier_key_pair,
    );

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::Party1MasterKey,
        &masterKey,
    )
}

#[post(
    "/ecdsa/sign/<id>/first",
    format = "json",
    data = "<eph_key_gen_first_message_party_two>"
)]
pub fn sign_first(
    state: State<Config>,
    auth_payload: AuthPayload,
    id: String,
    eph_key_gen_first_message_party_two: Json<party_two::EphKeyGenFirstMsg>,
) -> Result<Json<party_one::EphKeyGenFirstMsg>> {
    let (sign_party_one_first_message, eph_ec_key_pair_party1) = MasterKey1::sign_first_message();

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::EphKeyGenFirstMsg,
        &eph_key_gen_first_message_party_two.0,
    )?;

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::EphEcKeyPair,
        &eph_ec_key_pair_party1,
    )?;

    Ok(Json(sign_party_one_first_message))
}

// Added here because the attribute data takes only a single struct
#[derive(Serialize, Deserialize)]
pub struct SignSecondMsgRequest {
    pub message: BigInt,
    pub party_two_sign_message: party2::SignMessage,
    pub x_pos_child_key: BigInt,
    pub y_pos_child_key: BigInt,
}
#[post("/ecdsa/sign/<id>/second", format = "json", data = "<request>")]
pub fn sign_second(
    state: State<Config>,
    auth_payload: AuthPayload,
    id: String,
    request: Json<SignSecondMsgRequest>,
) -> Result<Json<party_one::SignatureRecid>> {
    let master_key: MasterKey1 = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::Party1MasterKey,
    )?
    .ok_or_else(|| format_err!("No Party1MasterKey for such identifier {}", id))?;

    let x: BigInt = request.x_pos_child_key.clone();
    let y: BigInt = request.y_pos_child_key.clone();

    let child_master_key = master_key.get_child(vec![x, y]);

    let eph_ec_key_pair_party1: party_one::EphEcKeyPair = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::EphEcKeyPair,
    )?
    .ok_or_else(|| format_err!("No EphEcKeyPair for such identifier {}", id))?;

    let eph_key_gen_first_message_party_two: party_two::EphKeyGenFirstMsg = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::EphKeyGenFirstMsg,
    )?
    .ok_or_else(|| format_err!("No EphKeyGenFirstMsg for such identifier {}", id))?;

    let signature_with_recid = child_master_key.sign_second_message(
        &request.party_two_sign_message,
        &eph_key_gen_first_message_party_two,
        &eph_ec_key_pair_party1,
        &request.message,
    );

    if signature_with_recid.is_err() {
        panic!("validation failed")
    };

    Ok(Json(signature_with_recid.unwrap()))
}

pub fn get_mk(state: &State<Config>, auth_payload: AuthPayload, id: &String) -> Result<MasterKey1> {
    db::get(
        &state.db,
        &auth_payload.user_id,
        id,
        &EcdsaStruct::Party1MasterKey,
    )?
    .ok_or_else(|| format_err!("No Party1MasterKey for such identifier {}", id))
}

#[post("/ecdsa/rotate/<id>/first", format = "json")]
pub fn rotate_first(
    state: State<Config>,
    auth_payload: AuthPayload,
    id: String,
) -> Result<Json<coin_flip_optimal_rounds::Party1FirstMessage<GE>>> {
    let (party1_coin_flip_first_message, m1, r1) = Rotation1::key_rotate_first_message();
    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::RotateCommitMessage1M,
        &m1,
    )?;
    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::RotateCommitMessage1R,
        &r1,
    )?;
    Ok(Json(party1_coin_flip_first_message))
}

#[post(
    "/ecdsa/rotate/<id>/second",
    format = "json",
    data = "<party2_first_message>"
)]
pub fn rotate_second(
    state: State<Config>,
    id: String,
    auth_payload: AuthPayload,
    party2_first_message: Json<coin_flip_optimal_rounds::Party2FirstMessage<GE>>,
) -> Result<
    Json<(
        coin_flip_optimal_rounds::Party1SecondMessage<GE>,
        party1::RotationParty1Message1,
    )>,
> {
    let party_one_master_key = get_mk(&state, auth_payload.clone(), &id)?;

    let m1: Secp256k1Scalar = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::RotateCommitMessage1M,
    )?
    .ok_or_else(|| format_err!("No RotateCommitMessage1M for such identifier {}", id))?;

    let r1: Secp256k1Scalar = db::get(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::RotateCommitMessage1R,
    )?
    .ok_or_else(|| format_err!("No RotateCommitMessage1R for such identifier {}", id))?;

    let (party1_second_message, random1) =
        Rotation1::key_rotate_second_message(&party2_first_message.0, &m1, &r1);
    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::RotateRandom1,
        &random1,
    )?;

    let (rotation_party_one_first_message, party_one_master_key_rotated) =
        party_one_master_key.rotation_first_message(&random1);

    db::insert(
        &state.db,
        &auth_payload.user_id,
        &id,
        &EcdsaStruct::Party1MasterKey,
        &party_one_master_key_rotated,
    )?;

    Ok(Json((
        party1_second_message,
        rotation_party_one_first_message,
    )))
}

#[post("/ecdsa/<id>/recover", format = "json")]
pub fn recover(state: State<Config>, auth_payload: AuthPayload, id: String) -> Result<Json<u32>> {
    let pos_old: u32 = db::get(&state.db, &auth_payload.user_id, &id, &EcdsaStruct::POS)?
        .ok_or_else(|| format_err!("No POS for such identifier {}", id))?;
    Ok(Json(pos_old))
}
