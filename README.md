# dbase-rs

A Rust library to read and write .dbf (dBase / FoxPro) files with Python bindings.

## Features

- Support for dBase III and FoxPro file formats
- High-performance read and write operations
- Multiple character encodings support:
  - ASCII
  - UTF-8/Unicode
  - GBK/CP936 (Chinese)
- Comprehensive field type support:
  - Character (C)
  - Numeric (N)
  - Logical (L)
  - Date (D)
  - Float (F)
  - Currency (Y)
  - DateTime (T)
  - Integer (I)
  - Double (B)
  - Memo (read-only)

## Python API

### Creating a DBF File

```python
from dbase import DBFFile

# Initialize with optional encoding
dbf = DBFFile("example.dbf", encoding="utf-8")  # or "ascii", "gbk"

# Define fields: (name, type, length, decimal_places)
fields = [
    ("NAME", "C", 50, None),      # Character field, length 50
    ("EMAIL", "C", 50, None),     # Character field, length 50
    ("AGE", "N", 3, 0),          # Numeric field, length 3, no decimals
    ("SALARY", "N", 10, 2),      # Numeric field, length 10, 2 decimal places
    ("BIRTH", "D", 8, None),     # Date field
    ("ACTIVE", "L", 1, None),    # Logical field
]

# Create the file structure
dbf.create(fields)
```

### Writing Records

```python
# Records can be dictionaries or tuples
records = [
    {
        "NAME": "John Doe",
        "EMAIL": "john@example.com",
        "AGE": 30,
        "SALARY": 50000.00,
        "BIRTH": "20000101",    # Date format: YYYYMMDD
        "ACTIVE": True
    }
]

# Append multiple records
dbf.append_records(records)
```

### Reading Records

```python
# Read all records
records = dbf.read_records()
# Returns a list of dictionaries with field names as keys
```

### Updating Records

```python
# Update specific fields in a record by index
dbf.update_record(0, {
    "SALARY": 55000.00,
    "ACTIVE": False
})

# If index >= num_records, a new record will be appended
```

## Performance

The library is designed for high performance:
- Efficient memory usage
- Optimized record reading and writing
- Support for both single and batch operations

### Benchmark Results
(Test environment: 10,000 records with 14 fields, file size 2.40MB)

| Operation | dbase-rs | Python dbf |
|-----------|----------|------------|
| Create    | 0.002s   | 0.003s     |
| Write     | 0.091s   | 2.373s     |
| Write Speed| 109,718 records/s | 4,214 records/s |
| Read      | 0.082s   | 0.273s     |
| Read Speed | 121,804 records/s | 36,613 records/s |

Key performance advantages:
- ~26x faster writing speed compared to Python dbf
- ~3.3x faster reading speed compared to Python dbf
- Minimal memory footprint
- Optimized for both single and batch operations

## Error Handling

The library provides comprehensive error handling for:
- Invalid field types
- Encoding issues
- File I/O errors
- Record validation
- Field value conversion

If dbase-rs fails to read or write or does something incorrectly, don't hesitate to open an issue.



