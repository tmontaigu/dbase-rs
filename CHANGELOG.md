# 0.1.1
    - Added RecordIterator to the lib.rs exports
    - Added derive `Clone` to the `Reader`
    - Removed `pub` attribute from `FieldInfo`'s `name` struct member.

# 0.1.0
    - Added preliminary support for reading some 'VisualFoxPro' files
    - Added support for reading dBase / FoxPro files which have 'Memo' fields. (Writing Memo fields is not supported yet)
    - Added support for reading and writing the 'Datetime' field
    - Added support for reading and writing the 'Currency' field
    - Added Trait to allow users implementing it to read dBase records into their structs.
    - Added Trait to allow users implementing it to writre structs as dBase record.
    - Added optional feature "serde", to automatically impl the 2 trait described above
    - Added dependency to chrono
    
    - Changed how iteration on the records is made (/!\ very small breaking change)
    - Changed how dbase Writer are created, users now have to use the TableWriterBuilder
      to specify fields that constitute a record before being able to write records.
    - Changed the Error type.
      
    - Bumped byteorder dependency to 1.3.0

# 0.0.4
    - Added reading and writing of Float value #2
    - All dBase III types (Character, Numeric, Logical, Date, Float)
        are wrapped in Option<T> to handle empty / uninitialized values

# 0.0.3
    - Added writing of Character, Numeric, Logical, Date, Double, Integer

# 0.0.2
    - Added reading of Character, Numeric, Logical, Date, Double, Integer, types


