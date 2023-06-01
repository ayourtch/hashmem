use bincode;
use serde::{Deserialize, Serialize};
use sha256::{digest, try_digest};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenHits {
    entries: Vec<TokenEntry>,
}

fn tokenize(src: &str) -> Vec<Token> {
    src.chars().map(|x| Token::C(x)).collect()
}

fn hash_tokens(src: &[Token]) -> String {
    let encoded: Vec<u8> = bincode::serialize(&src).unwrap();
    digest(&encoded[..])
}

fn get_file_name(tokens: &[Token]) -> String {
    let v = hash_tokens(tokens);
    let fname = format!(
        "data/{}/{}",
        &v[0..3],
        &v[3..]
    );
    fname
}

fn read_hits_from_file(filename: &str) -> TokenHits {
    use std::fs::File;

    if let Ok(mut f) = File::open(&filename) {
        let metadata = std::fs::metadata(&filename).expect("unable to read metadata");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).expect("buffer overflow");
        let decoded: TokenHits = bincode::deserialize(&buffer[..]).unwrap();
        decoded
    } else {
        TokenHits { entries: vec![] }
    }
}

fn write_hits_to_file(hits: &TokenHits, fname: &str) {
    let encoded: Vec<u8> = bincode::serialize(&hits).unwrap();

    // Create all the directories in the path if they don't exist
    if let Some(parent_dir) = std::path::Path::new(fname).parent() {
        if !parent_dir.exists() {
            if let Err(err) = std::fs::create_dir_all(parent_dir) {
                eprintln!("Failed to create directories: {}", err);
                return;
            }
        }
    }

    let mut file = match std::fs::File::create(fname) {
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

fn note_next_token(current: &[Token], next: &Token) {
    let fname = get_file_name(current);
    let mut hits = read_hits_from_file(&fname);
    debug!("current: {:?} next: {:?}, fname: {}", current, next, &fname);
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
    write_hits_to_file(&hits, &fname);
}

fn get_next_candidates(current: &[Token]) -> Vec<TokenEntry> {
    let mut v: Vec<TokenEntry> = vec![];
    let fname = get_file_name(current);
    debug!("input: {:?} fname: {}", &current, &fname);
    let mut hits = read_hits_from_file(&fname);
    debug!("hits: {:?}", &hits);
    hits.entries
}

fn note_string(input: &str) {
    let input_tokenized = tokenize(&input);
    debug!("Tokenized: {:?}", &input_tokenized);
    note_next_token(
        &input_tokenized[0..input_tokenized.len() - 1],
        &input_tokenized[input_tokenized.len() - 1],
    );
}

fn note_all_string(input: &str, context: usize) {
    for i in 0..context {
        if input.len() > 1 + i {
            note_string(&input[input.len() - 2 - i..])
        }
    }
}

fn note_text(input: &str, context: usize) {
    for i in 2..input.len() {
        note_all_string(&input[0..i], context);
    }
}

fn predict_token(input: &str) -> Vec<TokenEntry> {
    let input_tokenized = tokenize(&input);
    let cand = get_next_candidates(&input_tokenized);
    debug!("Candidates for {:?} : {:?}", &input_tokenized, &cand);
    let mut v: Vec<TokenEntry> = vec![];
    for c in cand {
        v.push(c.clone());
    }
    v
}

fn predict_all_string(input: &str, context: usize) {
    for i in (0..context).rev() {
        if input.len() > i {
            let v = predict_token(&input[input.len() - 1 - i..]);
            if v.len() > 0 {
                debug!("Predicted  {:?} at length {}", &v, i);
                break;
            }
        }
    }
}

fn main() {
    env_logger::init();
    debug!("this is a debug {}", "message");
    match std::env::args().nth(1).unwrap().as_str() {
        "note" => {
            note_text(&std::env::args().nth(2).unwrap(), 16);
        }
        "note-file" => {
            let fname = std::env::args().nth(2).unwrap();
            eprintln!("Noting {}...", &fname);
            let data = std::fs::read_to_string(&fname).unwrap();
            note_text(&data, 16);
        }
        "predict" => {
            predict_all_string(&std::env::args().nth(2).unwrap(), 16);
        }
        x => {
            panic!("{} is not a valid operation", x);
        }
    }
}
