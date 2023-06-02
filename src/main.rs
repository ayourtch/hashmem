use bincode;
use rand;
use rand::Rng;
use std::fs::File;

use serde::{Deserialize, Serialize};
use sha256::{digest, try_digest};
use std::collections::HashMap;
use std::io::{Read, Write};

use db_key;

extern crate leveldb;

#[macro_use]
extern crate log;

use leveldb::database::Database;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum Token {
    C(char),
    Num(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TokenEntry {
    value: Token,
    count: u64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct TokenHits {
    entries: Vec<TokenEntry>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct TokenHitHash {
    hits_by_hash: HashMap<String, TokenHits>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct DbKey {
    key: Vec<u8>,
}

struct TokenStash {
    prefix: String,
    database: Database<DbKey>,
    cache: HashMap<String, TokenHitHash>,
    rng: rand::ThreadRng,
}

impl db_key::Key for DbKey {
    fn from_u8(key: &[u8]) -> Self {
        DbKey { key: key.to_vec() }
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&self.key[..])
    }
}

fn test_db() {
    use leveldb::database::Database;
    use leveldb::kv::KV;
    use leveldb::options::{Options, ReadOptions, WriteOptions};

    let path = std::path::Path::new("/tmp/test");

    let entry = TokenEntry {
        value: Token::C('a'),
        count: 0,
    };

    let mut options = Options::new();
    options.create_if_missing = true;
    options.cache = Some(leveldb::database::cache::Cache::new(1024*1024));
    options.compression = leveldb_sys::Compression::Snappy;
    let mut database: Database<DbKey> = match Database::open(path, options) {
        Ok(db) => db,
        Err(e) => {
            panic!("failed to open database: {:?}", e)
        }
    };
    let key = DbKey {
        key: b"123".to_vec(),
    };

    let write_opts = WriteOptions::new();
    let encoded: Vec<u8> = bincode::serialize(&entry).unwrap();
    match database.put(write_opts, &key, &encoded) {
        Ok(_) => (),
        Err(e) => {
            panic!("failed to write to database: {:?}", e)
        }
    };

    let read_opts = ReadOptions::new();
    let res = database.get(read_opts, &key);

    match res {
        Ok(data) => {
            let data = data.unwrap();
            let decoded: TokenEntry = bincode::deserialize(&data[..]).unwrap();
            println!("Data retrieved: {:?}", &decoded);
        }
        Err(e) => {
            panic!("failed reading data: {:?}", e)
        }
    }
}

impl TokenStash {
    fn new(prefix: &str) -> Self {
        let mut options = Options::new();

        options.create_if_missing = true;
        let dbname = format!("{}/db", &prefix);

        let path = std::path::Path::new(&dbname);

        let mut database: Database<DbKey> = match Database::open(path, options) {
            Ok(db) => db,
            Err(e) => {
                panic!("failed to open database: {:?}", e)
            }
        };

        TokenStash {
            rng: rand::thread_rng(),
            prefix: prefix.to_string(),
            database,
            cache: HashMap::new(),
        }
    }

    fn tokenize(self: &Self, src: &str) -> Vec<Token> {
        src.chars().map(|x| Token::C(x)).collect()
    }

    fn hash_tokens(self: &Self, src: &[Token]) -> String {
        let encoded: Vec<u8> = bincode::serialize(&src).unwrap();
        digest(&encoded[..])
    }

    fn read_hits_from_file(self: &mut Self, hash: &str) -> TokenHits {
        use std::fs::File;

        let read_opts = ReadOptions::new();
        let key = DbKey {
            key: hash.as_bytes().to_vec(),
        };

        let res = self.database.get(read_opts, &key);

        match res {
            Ok(data) => {
                if let Some(data) = data {
                    let decoded: TokenHits = bincode::deserialize(&data[..]).unwrap();
                    decoded
                } else {
                    TokenHits { entries: vec![] }
                }
            }
            Err(e) => {
                panic!("failed reading data: {:?}", e)
            }
        }
    }

    fn write_hits_to_file(&mut self, hits: &TokenHits, hash: &str) {
        let key = DbKey {
            key: hash.as_bytes().to_vec(),
        };
        let write_opts = WriteOptions::new();
        let encoded: Vec<u8> = bincode::serialize(&hits).unwrap();
        match self.database.put(write_opts, &key, &encoded) {
            Ok(_) => (),
            Err(e) => {
                panic!("failed to write to database: {:?}", e)
            }
        };
    }

    fn note_next_token(&mut self, current: &[Token], next: &Token) {
        let hash = self.hash_tokens(current);
        let mut hits = self.read_hits_from_file(&hash);
        debug!("current: {:?} next: {:?}, hash: {}", current, next, &hash);
        debug!("Hits B4: {:?}", &hits);
        let mut found = false;
        for mut e in &mut hits.entries {
            if &e.value == next {
                e.count += 1;
                found = true;
            }
        }
        if !found {
            let entry = TokenEntry {
                value: next.clone(),
                count: 1,
            };
            hits.entries.push(entry);
        }
        debug!("Hits AF: {:?}", &hits);
        self.write_hits_to_file(&hits, &hash);
    }

    fn get_next_candidates(&mut self, current: &[Token]) -> Vec<TokenEntry> {
        let mut v: Vec<TokenEntry> = vec![];
        let hash = self.hash_tokens(current);
        debug!("input: {:?} hash: {}", &current, &hash);
        let mut hits = self.read_hits_from_file(&hash);
        debug!("hits: {:?}", &hits);
        hits.entries
    }

    fn note_string(&mut self, input: &str) {
        let input_tokenized = self.tokenize(&input);
        debug!("Tokenized: {:?}", &input_tokenized);
        self.note_next_token(
            &input_tokenized[0..input_tokenized.len() - 1],
            &input_tokenized[input_tokenized.len() - 1],
        );
    }

    fn note_all_string(&mut self, input: &str, context: usize) {
        for i in 0..context {
            if input.len() > 1 + i {
                self.note_string(&input[input.len() - 2 - i..])
            }
        }
    }

    fn note_text(&mut self, input: &str, context: usize) {
        for i in 2..input.len() {
            self.note_all_string(&input[0..i], context);
        }
    }

    fn predict_token(&mut self, input: &str) -> Vec<TokenEntry> {
        let input_tokenized = self.tokenize(&input);
        let cand = self.get_next_candidates(&input_tokenized);
        debug!("Candidates for {:?} : {:?}", &input_tokenized, &cand);
        let mut v: Vec<TokenEntry> = vec![];
        for c in cand {
            v.push(c.clone());
        }
        v
    }

    fn predict_all_string(&mut self, input: &str, context: usize) {
        for i in (0..context).rev() {
            if input.len() > i {
                let v = self.predict_token(&input[input.len() - 1 - i..]);
                if v.len() > 0 {
                    debug!("Predicted  {:?} at length {}", &v, i);
                    break;
                }
            }
        }
    }

    fn predict_all_string_return(&mut self, input: &str, context: usize) -> Option<char> {
        for i in (0..context).rev() {
            if input.len() > i {
                let v = self.predict_token(&input[input.len() - 1 - i..]);
                if v.len() > 0 {
                    debug!("Predicted  {:?} at length {}", &v, i);

                    if let Token::C(c) = v[self.rng.gen_range(0, v.len())].value {
                        return Some(c);
                        break;
                    }
                }
            }
        }
        return None;
    }
    fn generate(&mut self, input: &str, context: usize) {
        let mut content = format!("{}", input);
        print!("{}", input);
        loop {
            if let Some(c) = self.predict_all_string_return(&content, 64) {
                print!("{}", c);
                content = format!("{}{}", &content, c);
            } else {
                println!("\n\n");
                return;
            }
        }
    }
}

fn main() {
    env_logger::init();
    debug!("this is a debug {}", "message");
    let mut rng = rand::thread_rng();

    let mut stash = TokenStash::new("data");

    match std::env::args().nth(1).unwrap().as_str() {
        "note" => {
            stash.note_text(&std::env::args().nth(2).unwrap(), 32);
        }
        "note-file" => {
            let fname = std::env::args().nth(2).unwrap();
            eprintln!("Noting {}...", &fname);
            let data = std::fs::read_to_string(&fname).unwrap();
            stash.note_text(&data, 32);
        }
        "predict" => {
            stash.predict_all_string(&std::env::args().nth(2).unwrap(), 32);
        }
        "generate" => {
            stash.generate(&std::env::args().nth(2).unwrap(), 32);
        }
        "test" => {
            test_db();
        }
        x => {
            panic!("{} is not a valid operation", x);
        }
    }
}
