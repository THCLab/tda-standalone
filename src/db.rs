use crate::models::Token;
use lmdb::*;
use std::path::Path;

pub struct TDADatabase {
    env: Option<Environment>,
}

impl TDADatabase {
    pub fn new() -> Self {
        // Start enviroment.
        let env = match Environment::new()
            .set_max_dbs(1)
            .open(Path::new("./src/db"))
        {
            Ok(env) => Some(env),
            Err(e) => None,
        };
        TDADatabase { env }
    }

    pub fn add_message(&self) {
        todo!();
    }

    pub fn add_token(&self, t: Token) {
        match &self.env {
            Some(env) => {
                let mut trans = env.begin_rw_txn().expect("Can't begin transiction.");
                let db = unsafe {
                    trans
                        .create_db(Some("tokens"), DatabaseFlags::empty())
                        .expect("Can't open database.")
                };
                // Encode Token and put key-value pair to db.
                let encoded_token = bincode::serialize(&t).expect("Can't encode token.");

                println!("Encoded vector: {:?}", encoded_token);

                trans.put(db, &t.uuid, &encoded_token, WriteFlags::empty());
                trans.commit();
            }
            None => {}
        }
    }

    pub fn get_token(&self, uuid: String) -> Option<Token> {
        match &self.env {
            Some(env) => {
                let trans = env.begin_ro_txn().expect("Can't begin transiction.");
                let db = unsafe { trans.open_db(Some("tokens")).expect("Can't open database.") };

                let out = match trans.get(db, &uuid) {
                    Ok(encoded_token) => {
                        println!("Decoded vec: {:?}", encoded_token);
                        let token: Token =
                            bincode::deserialize(&encoded_token).expect("Can't deserialize token.");

                        println!("{:?}", token.uuid);

                        Some(token)
                    }
                    Err(e) => None,
                };

                trans.commit();
                out
            }
            None => None,
        }
    }

    pub fn check_token(&self, t: Token) -> bool {
        match &self.env {
            Some(env) => {
                let trans = env.begin_ro_txn().expect("Can't begin transiction.");
                let db = unsafe { trans.open_db(Some("tokens")).expect("Can't open database.") };
                let encoded_uid = bincode::serialize(&t).expect("Can't encode token.");
                let out = match trans.open_ro_cursor(db) {
                    Ok(mut cursor) => cursor.iter().any(|x| x.1 == encoded_uid),
                    Err(e) => false,
                };

                trans.commit();
                out
            }
            None => false,
        }
    }

    pub fn get(&self) {
        todo!();
    }
}
