# HashMem

A character-level language model using RedDB for storage, SHA-256 hashing for context, and bincode 2.0 for serialization.

## Overview

HashMem is a lightweight character-based language model implementation in Rust. It learns patterns from text input and can make predictions or generate new text based on learned contexts.

## Features

- **Character-level tokenization**: Breaks input text into individual characters
- **SHA-256 hashing**: Uses SHA-256 to create unique hashes for token sequences
- **RedDB storage**: Efficient, pure-Rust key-value storage for learned token patterns
- **Context-aware predictions**: Makes predictions based on variable-length context (up to 64 characters)
- **Text generation**: Can generate new text based on learned patterns with random sampling
- **Transactional database operations**: ACID-compliant transactions for data integrity

## Installation

### Prerequisites

- Rust (1.60 or later)
- Cargo

### Building

```bash
cargo build --release
```

The binary will be available at `target/release/hashmem`.

## Usage

HashMem provides several commands for interacting with the model:

### Note Text

Learn from text input:

```bash
./target/release/hashmem note "your text here"
```

Or learn from a file:

```bash
./target/release/hashmem note-file input.txt
```

The model will tokenize the input and learn character transition patterns for various context lengths.

### Make Predictions

Predict the next character based on context:

```bash
./target/release/hashmem predict "your text"
```

This will show debug information about potential next characters based on learned patterns.

### Generate Text

Generate new text based on a seed:

```bash
./target/release/hashmem generate "seed text"
```

The model will continuously generate characters until it can't find a matching pattern, using weighted random selection based on learned frequencies.

### Test Database

Run a simple database test to verify installation:

```bash
./target/release/hashmem test
```

## How It Works

1. **Tokenization**: Input text is tokenized into individual characters (`Token::C(char)`)
2. **Hashing**: Sequences of tokens are hashed using SHA-256 to create unique keys
3. **Storage**: Token transition statistics are stored in RedDB with the following structure:
   - Key: SHA-256 hash of token sequence
   - Value: `TokenHits` containing `TokenEntry` records (token + count)
4. **Prediction**: When predicting, the model:
   - Hashes the current context (token sequence)
   - Looks up the hash in the database
   - Retrieves all learned next tokens with their frequencies
   - Uses the context window (default: 32 characters) to fall back to shorter contexts if needed
5. **Generation**: For text generation, the model:
   - Starts with seed text
   - Predicts the next character using weighted random sampling
   - Appends the character and repeats until no predictions are available

## Architecture

### Core Components

- **Token**: Enum representing either a character (`C(char)`) or number (`Num(u64)`)
- **TokenEntry**: Stores a token value and its occurrence count
- **TokenHits**: Collection of TokenEntry records for a given context
- **TokenStash**: Main structure managing the database and model operations

### Database Schema

RedDB table: `token_hits`
- **Key type**: `&str` (SHA-256 hash as string)
- **Value type**: `&[u8]` (serialized `TokenHits` using bincode)

The database uses explicit transactions:
- Read transactions: `begin_read()` → `open_table()` → `get()`
- Write transactions: `begin_write()` → `open_table()` → `insert()` → `commit()`

## Configuration

### Context Window

The model uses a default context window of 32 characters when learning from text. This means it learns patterns for sequences up to 32 characters long, allowing it to capture both short and long-range dependencies.

### Database Location

The model stores its database in the `data/db` directory by default. The database directory will be created automatically if it doesn't exist.

**Important**: The database format is incompatible with LevelDB. If you're migrating from an older version using LevelDB, you'll need to retrain your model from scratch.

## Dependencies

- `serde`: Serialization/deserialization support with derive macros
- `sha256`: SHA-256 hashing for token sequences
- `bincode`: 2.0 - Modern binary serialization for efficient storage with explicit configuration
- `log`/`env_logger`: Logging support (set `RUST_LOG=debug` for debug output)
- `rand`: Random number generation for sampling during text generation
- `redb`: Pure-Rust key-value storage with ACID transactions

## Example Workflow

```bash
# 1. Train the model with some text
cargo run -- note-file sample.txt

# 2. Test predictions
cargo run -- predict "The quick"

# 3. Generate text
cargo run -- generate "Once upon a"
```

## Performance Considerations

- **Database transactions**: Each read/write operation uses transactions for data integrity
- **Hash lookups**: SHA-256 hashes provide uniform key distribution
- **Context fallback**: The model tries shorter contexts if longer ones aren't found
- **Memory usage**: The model keeps minimal in-memory state; most data is stored in RedDB

## Troubleshooting

### Build Issues

If you encounter build issues:
```bash
cargo clean
cargo build
```

### Debug Logging

To see detailed debug information:
```bash
RUST_LOG=debug cargo run -- note "test"
```

### Database Issues

If you need to start fresh:
```bash
rm -rf data/db
```

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Report.

## Migration Notes

This version has been migrated from LevelDB to RedDB and upgraded from bincode 1.0 to 2.0.

### Database Migration (LevelDB → RedDB)
- Pure Rust implementation (no C++ dependencies)
- Explicit transaction model
- Better type safety with compile-time table definitions
- Improved performance and reliability

### Serialization Migration (bincode 1.0 → 2.0)
- Explicit configuration with `bincode::config::standard()`
- Better compile-time guarantees with `Encode`/`Decode` derives
- Improved error messages and performance
- `decode_from_slice` returns `(T, usize)` tuple

**Important:** Both the database format and serialization format are incompatible with previous versions. Existing databases and data must be rebuilt from scratch.

See `MIGRATION_SUMMARY.md` for detailed migration information.
