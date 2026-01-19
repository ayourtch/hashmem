# HashMem

A character-level language model using LevelDB for storage and SHA-256 hashing for context.

## Overview

HashMem is a lightweight character-based language model implementation in Rust. It learns patterns from text input and can make predictions or generate new text based on learned contexts.

## Features

- **Character-level tokenization**: Breaks input text into individual characters
- **SHA-256 hashing**: Uses SHA-256 to create unique hashes for token sequences
- **LevelDB storage**: Efficient key-value storage for learned token patterns
- **Context-aware predictions**: Makes predictions based on variable-length context
- **Text generation**: Can generate new text based on learned patterns

## Installation

### Prerequisites

- Rust (1.60 or later)
- Cargo
- LevelDB development libraries

### Building

```bash
cargo build --release
```

## Usage

HashMem provides several commands for interacting with the model:

### Note Text

Learn from text input:

```bash
./target/release/hashmem note "your text here"
```

Or from a file:

```bash
./target/release/hashmem note-file input.txt
```

### Make Predictions

Predict the next character based on context:

```bash
./target/release/hashmem predict "your text"
```

### Generate Text

Generate new text based on a seed:

```bash
./target/release/hashmem generate "seed text"
```

### Test Database

Run a simple database test:

```bash
./target/release/hashmem test
```

## How It Works

1. **Tokenization**: Input text is tokenized into individual characters
2. **Hashing**: Sequences of tokens are hashed using SHA-256 to create unique keys
3. **Storage**: Token transition statistics are stored in LevelDB
4. **Prediction**: When predicting, the model looks up the hash of the current context and retrieves learned transition probabilities
5. **Generation**: For text generation, the model uses learned probabilities to sample the next character

## Configuration

The model stores its database in the `data/db` directory by default. You can change this by modifying the `prefix` parameter when creating a `TokenStash`.

## Dependencies

- `serde`: Serialization/deserialization support
- `sha256`: SHA-256 hashing
- `bincode`: Binary serialization
- `log`/`env_logger`: Logging support
- `rand`: Random number generation
- `leveldb`: Key-value storage backend
- `db-key`: Database key abstraction
- `leveldb-sys`: LevelDB bindings

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
