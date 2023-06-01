use bincode;
use std::fs::File;

use serde::{Deserialize, Serialize};
use sha256::{digest, try_digest};
use std::collections::HashMap;
use std::io::{Read, Write};

#[macro_use]
extern crate log;

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
struct TokenStash {
    prefix: String,
    cache: HashMap<String, TokenHitHash>,
}

impl TokenStash {
    fn new(prefix: &str) -> Self {
        let prefix = prefix.to_string();
        TokenStash {
            prefix,
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

    fn get_hash_file_name(self: &Self, v: &str) -> String {
        let fname = format!("{}/{}/hash-{}.bin", &self.prefix, &v[0..3], &v[0..3]);
        fname
    }

    fn read_hits_from_file(self: &Self, hash: &str) -> TokenHits {
        use std::fs::File;

        let filename = self.get_hash_file_name(&hash);

        if let Ok(mut f) = File::open(&filename) {
            let metadata = std::fs::metadata(&filename).expect("unable to read metadata");
            let mut buffer = vec![0; metadata.len() as usize];
            f.read(&mut buffer).expect("buffer overflow");
            let decoded: TokenHitHash = bincode::deserialize(&buffer[..]).unwrap();
            if let Some(decoded) = decoded.hits_by_hash.get(hash) {
                decoded.clone()
            } else {
                TokenHits { entries: vec![] }
            }
        } else {
            TokenHits { entries: vec![] }
        }
    }

    fn flush_cache(&mut self) {
        for (name, ht) in &self.cache {
            self.save_hash_on_disk(&ht, &name);
        }
    }

    fn save_hash(&mut self, hashtable: &TokenHitHash, filename: &str) {
        self.cache.insert(filename.to_string(), hashtable.clone());
    }
    fn save_hash_on_disk(self: &Self, hashtable: &TokenHitHash, filename: &str) {
        let encoded: Vec<u8> = bincode::serialize(&hashtable).unwrap();

        // Create all the directories in the path if they don't exist
        if let Some(parent_dir) = std::path::Path::new(&filename).parent() {
            if !parent_dir.exists() {
                if let Err(err) = std::fs::create_dir_all(parent_dir) {
                    eprintln!("Failed to create directories: {}", err);
                    return;
                }
            }
        }

        let mut file = match std::fs::File::create(filename) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Failed to open file: {}", err);
                return;
            }
        };

        // Write to the file
        if let Err(err) = file.write_all(&encoded) {
            eprintln!("Failed to write to file: {}", err);
            return;
        }
    }

    fn read_hash(&mut self, filename: &str) -> TokenHitHash {
        if let Some(hashtable) = self.cache.get(filename) {
            return hashtable.clone();
        }
        let mut hashtable: TokenHitHash = if let Ok(mut f) = File::open(&filename) {
            let metadata = std::fs::metadata(&filename).expect("unable to read metadata");
            let mut buffer = vec![0; metadata.len() as usize];
            f.read(&mut buffer).expect("buffer overflow");
            let decoded: TokenHitHash = bincode::deserialize(&buffer[..]).unwrap();
            decoded
        } else {
            Default::default()
        };
        self.cache.insert(filename.to_string(), hashtable.clone());
        hashtable
    }

    fn write_hits_to_file(&mut self, hits: &TokenHits, hash: &str) {
        let filename = self.get_hash_file_name(&hash);

        let mut hashtable = self.read_hash(&filename);

        hashtable
            .hits_by_hash
            .insert(hash.to_string(), hits.clone());

        self.save_hash(&hashtable, &filename);
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

    fn get_next_candidates(&self, current: &[Token]) -> Vec<TokenEntry> {
        let mut v: Vec<TokenEntry> = vec![];
        let hash = self.hash_tokens(current);
        debug!("input: {:?} hash: {}", &current, &hash);
        let mut hits = self.read_hits_from_file(&hash);
        debug!("hits: {:?}", &hits);
        hits.entries
    }

    fn note_string(&mut self, input: &str) {
        let input_tokenized = self.tokenize(&input);
        info!("NOTE: '{}'", input);
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

    fn predict_token(&self, input: &str) -> Vec<TokenEntry> {
        let input_tokenized = self.tokenize(&input);
        let cand = self.get_next_candidates(&input_tokenized);
        debug!("Candidates for {:?} : {:?}", &input_tokenized, &cand);
        let mut v: Vec<TokenEntry> = vec![];
        for c in cand {
            v.push(c.clone());
        }
        v
    }

    fn predict_all_string(&self, input: &str, context: usize) {
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
}

fn main() {
    env_logger::init();
    debug!("this is a debug {}", "message");
    let mut stash = TokenStash::new("data");

    match std::env::args().nth(1).unwrap().as_str() {
        "note" => {
            stash.note_text(&std::env::args().nth(2).unwrap(), 16);
        }
        "note-file" => {
            let fname = std::env::args().nth(2).unwrap();
            eprintln!("Noting {}...", &fname);
            let data = std::fs::read_to_string(&fname).unwrap();
            stash.note_text(&data, 16);
        }
        "predict" => {
            stash.predict_all_string(&std::env::args().nth(2).unwrap(), 16);
        }
        x => {
            panic!("{} is not a valid operation", x);
        }
    }
    stash.flush_cache();
}
