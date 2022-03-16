use super::super::Result;
use rocksdb;
use serde;

pub enum DB {
    Local(rocksdb::DB),
    ConnError(String),
}

pub trait MPCStruct {
    fn to_string(&self) -> String;

    fn require_customer_id(&self) -> bool {
        true
    }
}

fn idify(user_id: &str, id: &str, name: &dyn MPCStruct) -> String {
    format!("{}_{}_{}", user_id, id, name.to_string())
}

pub fn insert<T>(db: &DB, user_id: &str, id: &str, name: &dyn MPCStruct, v: T) -> Result<()>
where
    T: serde::ser::Serialize,
{
    match db {
        DB::Local(rocksdb_client) => {
            let identifier = idify(user_id, id, name);
            let v_string = serde_json::to_string(&v).unwrap();
            rocksdb_client.put(identifier.as_bytes(), v_string.as_bytes())?;
            Ok(())
        }
        DB::ConnError(msg) => {
            panic!("{}", msg);
        }
    }
}

pub fn get<T>(db: &DB, user_id: &str, id: &str, name: &dyn MPCStruct) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match db {
        DB::Local(rocksdb_client) => {
            let identifier = idify(user_id, id, name);
            debug!("Getting from db ({})", identifier);

            let db_option = rocksdb_client.get(identifier.as_bytes())?;
            let vec_option: Option<Vec<u8>> = db_option.map(|v| v.to_vec());
            match vec_option {
                Some(vec) => Ok(serde_json::from_slice(&vec).unwrap()),
                None => Ok(None),
            }
        }
        DB::ConnError(msg) => {
            panic!("{}", msg);
        }
    }
}
