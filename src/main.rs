use bincode::{Decode, Encode};
use rand;
use rand::Rng;
use std::fs::File;

use serde::{Deserialize, Serialize};
use sha256::{digest, try_digest};
use std::collections::HashMap;
use std::io::{Read, Write};

#[macro_use]
extern crate log;

use redb::{Database, ReadableTable, TableDefinition};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Encode, Decode)]
enum Token {
    C(char),
    Num(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Encode, Decode)]
struct TokenEntry {
    value: Token,
    count: u64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Encode, Decode)]
struct TokenHits {
    entries: Vec<TokenEntry>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct TokenHitHash {
    hits_by_hash: HashMap<String, TokenHits>,
}

struct TokenStash {
    prefix: String,
    database: Database,
    cache: HashMap<String, TokenHitHash>,
    rng: rand::ThreadRng,
}

fn test_db() {
    const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("test_table");

    let path = "/tmp/test_redb";

    let entry = TokenEntry {
        value: Token::C('a'),
        count: 0,
    };

    // Create parent directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let db = Database::create(path).unwrap();
    
    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(TABLE).unwrap();
        let encoded: Vec<u8> = bincode::encode_to_vec(&entry, bincode::config::standard()).unwrap();
        let key: &[u8] = b"123";
        table.insert(key, encoded.as_slice()).unwrap();
    }
    write_txn.commit().unwrap();

    let read_txn = db.begin_read().unwrap();
    let table = read_txn.open_table(TABLE).unwrap();
    let key: &[u8] = b"123";
    let res = table.get(key).unwrap();

    match res {
        Some(data) => {
            let (decoded, _): (TokenEntry, usize) = bincode::decode_from_slice(data.value(), bincode::config::standard()).unwrap();
            println!("Data retrieved: {:?}", &decoded);
        }
        None => {
            panic!("No data found")
        }
    }
}

impl TokenStash {
    fn new(prefix: &str) -> Self {
        let dbname = format!("{}/db", &prefix);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&dbname).parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        
        let database = match Database::create(&dbname) {
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
        let encoded: Vec<u8> = bincode::encode_to_vec(&src, bincode::config::standard()).unwrap();
        digest(&encoded[..])
    }

    fn read_hits_from_file(self: &mut Self, hash: &str) -> TokenHits {
        const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("token_hits");
        
        let read_txn = self.database.begin_read().unwrap();
        let table = read_txn.open_table(TABLE);
        
        match table {
            Ok(table) => {
                match table.get(hash).unwrap() {
                    Some(data) => {
                        let (decoded, _): (TokenHits, usize) = bincode::decode_from_slice(data.value(), bincode::config::standard()).unwrap();
                        decoded
                    }
                    None => {
                        TokenHits { entries: vec![] }
                    }
                }
            }
            Err(_) => {
                // Table doesn't exist yet
                TokenHits { entries: vec![] }
            }
        }
    }

    fn write_hits_to_file(&mut self, hits: &TokenHits, hash: &str) {
        const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("token_hits");
        
        let write_txn = self.database.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(TABLE).unwrap();
            let encoded: Vec<u8> = bincode::encode_to_vec(&hits, bincode::config::standard()).unwrap();
            table.insert(hash, encoded.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
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
        let total = input.len();
        let mut batch: HashMap<String, TokenHits> = HashMap::new();
        
        // Collect all updates in memory
        for i in 2..input.len() {
            for j in 0..context {
                if i > 1 + j {
                    let end = i;
                    let start = i - 2 - j;
                    if start < end {
                        let substring = &input[start..end];
                        let tokenized = self.tokenize(substring);
                        if tokenized.len() >= 1 {
                            let current = &tokenized[0..tokenized.len() - 1];
                            let next = &tokenized[tokenized.len() - 1];
                            let hash = self.hash_tokens(current);
                            
                            let hits = batch.entry(hash.clone()).or_insert_with(|| {
                                self.read_hits_from_file(&hash)
                            });
                            
                            let mut found = false;
                            for e in &mut hits.entries {
                                if &e.value == next {
                                    e.count += 1;
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                let entry = TokenEntry {
                                    value: next.clone(),
                                    count: 1,
                                };
                                hits.entries.push(entry);
                            }
                        }
                    }
                }
            }
            if i % 100 == 0 {
                eprint!("\rProgress: {}/{} characters noted ({}%)", i, total, (i * 100) / total);
            }
        }
        
        // Write all updates in a single transaction
        let write_txn = self.database.begin_write().unwrap();
        {
            const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("token_hits");
            let mut table = write_txn.open_table(TABLE).unwrap();
            
            for (hash, hits) in &batch {
                let encoded: Vec<u8> = bincode::encode_to_vec(&hits, bincode::config::standard()).unwrap();
                table.insert(hash.as_str(), encoded.as_slice()).unwrap();
            }
        }
        write_txn.commit().unwrap();
        
        eprintln!(); // New line after progress completes
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
