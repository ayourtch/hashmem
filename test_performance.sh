#!/bin/bash

# Performance test for note operation optimization

echo "Testing hashmem note operation performance..."
echo "=============================================="
echo ""

# Clean up any existing database
rm -rf data

# Create test file of different sizes
echo "Creating test files..."

# Small test (~500 chars)
echo "Creating small_test.txt (500 chars)..."
head -c 500 /dev/urandom | tr -dc 'a-zA-Z0-9\n' > small_test.txt

# Medium test (~5,000 chars)
echo "Creating medium_test.txt (5,000 chars)..."
head -c 5000 /dev/urandom | tr -dc 'a-zA-Z0-9\n' > medium_test.txt

# Large test (~50,000 chars)
echo "Creating large_test.txt (50,000 chars)..."
head -c 50000 /dev/urandom | tr -dc 'a-zA-Z0-9\n' > large_test.txt

echo ""
echo "Test files created:"
echo "  - small_test.txt:  $(wc -c < small_test.txt) bytes"
echo "  - medium_test.txt: $(wc -c < medium_test.txt) bytes"
echo "  - large_test.txt:  $(wc -c < large_test.txt) bytes"
echo ""

# Test each file size
for size in small medium large; do
    file="${size}_test.txt"
    chars=$(wc -c < "$file")
    
    echo "Testing $file ($chars bytes)..."
    echo "-------------------------------"
    
    # Clean database between tests
    rm -rf data
    
    # Time the operation
    time ./target/release/hashmem note-file "$file"
    
    # Check database size
    if [ -d "data" ]; then
        db_size=$(du -sh data | cut -f1)
        echo "Database size: $db_size"
    fi
    
    echo ""
done

echo "=============================================="
echo "Performance test complete!"
echo ""
echo "Expected results with optimization:"
echo "  - Small (500 chars):   < 1 second"
echo "  - Medium (5,000 chars): < 5 seconds"
echo "  - Large (50,000 chars): < 30 seconds"
echo ""
echo "Without optimization, the large test would take hours!"