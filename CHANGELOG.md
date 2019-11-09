# Unreleased
    - Added preliminary support for reading some 'VisualFoxPro' files
    - Added support for reading dBase / FoxPro files which have 'Memo' fields. (Writing Memo fields is not supported yet)
    - Added support for reading and writing the 'Datetime' field
    - Added support for reading and writing the 'Currency' field
    - Changed how iteration on the records is made (/!\ very small breaking change)
    - Added Trait to allow users implementing it to read dBase records into their structs.

# 0.0.4
    - Added reading and writing of Float value #2
    - All dBase III types (Character, Numeric, Logical, Date, Float)
        are wrapped in Option<T> to handle empty / uninitialized values

# 0.0.3
    - Added writing of Character, Numeric, Logical, Date, Double, Integer

# 0.0.2
    - Added reading of Character, Numeric, Logical, Date, Double, Integer, types


