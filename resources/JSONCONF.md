# Configuration

With this we can configure the parser itself, as well as some essential transformizations/sanitizations.

On the upper most level, a configuration consists of these objects:

```jsonc
{
    "comment": "Some explanation",  // 1) (optional)
    "parserOpts": {},               // 2) (mandatory)
    "sanitizeColumns": [],          // 3) (optional)
    "typeColumns": []               // 4) (optional)
}

```
1. An optional comment / description of this configuration.
2. The **parser options**.
3. The **column sanitization configuration**, i.e. clean up the file to be usable
4. The **column typing configuration** / setup, i.e. we type the columns


## `parserOpts` - Parser Options
This is how the parser can be configured.

```jsonc
"parserOpts": {                                     // 0) 
    "comment": "Some explanation/documentation",    // 1) (optional)
    "separatorChar": ",",                           // 2) (mandatory)
    "enclosureChar": "\"",                          // 3) (optional)
    "lines": {                                      //    (optional)
        "comment": "Some optional explanation",     // 4) (optional)
        "skipLinesFromStart": 3,                    // 5) (optional)
        "skipLinesFromEnd": 1,                      // 6) (optional)
        "skipLinesByStartswith": ["#", "-"],        // 7) (optional)
        "takeLinesByStartswith": ["\""],            // 8) (optional)
        "skipEmptyLines": true                      // 9) (optional)
    },
    "firstLineIsHeader": true                       // 10) (mandatory)
},
```
0. The `parserOpts` json object.
1. A comment describing why it's configured the way it is.
2. The separator character used.
3. TODO: The enclosure character used (any defaults here?)
4. A comment describing why it's configured the way it is.
5. How many lines should be skipped from start of the file (1-indexed)
6. How many lines should be skipped from end of the file (1-indexed). **CAUTION: This does _only_ work for Files and is a rather expensive operation. If you can get the same result by using `skipLinesByStartswith` or `takeLinesByStartswith` you should!.**
7. Skip lines that start with these strings (or characters). **NOTE: only one of either(`skipLinesByStartswith`|`takeLinesByStartswith`) makes sense to use.**
8. Only take lines that start with these strings (or characters). **NOTE: only one of either(`skipLinesByStartswith`|`takeLinesByStartswith`) makes sense to use.**
9. Skip empty lines
10. Is the first line we read (**after** skipping/taking) a header line?

## `sanitizeColumns` - Column Sanitization Configuration
Some arguments can be found for and against having some transformation capabilities **in** the parser.

On the plus side, some sanitization is often required, in order to correctly type everything in the **column typing"** step. In other words, there is just enough sanitization (or transformation, really) provided to clean up the token strings, so that they can be typed.

Also, because of this, a downstream transformation step might not be needed.

On the down side, arguably, things can be done with this that are not traditionally the job of a parser, but rather the downstream processing.

Column sanitization, from a json standpoint, is an array of "sanitizers" for **every** column. **Column indexing is 0-based!**

**CAUTION**: The correcteness of the config is **not** checked at this point! It is totally possible to build faulty configurations!

```jsonc
"sanitizeColumns": [{               // 0)
    "comment": "Some explanation",  // 1) (optional)
    "sanitizers": [{                // 2) (mandatory)
        "type": "trim",             // 3) (mandatory)
        "spec": "left"              // 4) (mandatory)
    }]
}, {
    "comment": "Some explanation",
    "idx": 0,                       // 5) (optional)
    "sanitizers": [{
        "type": "casing",
        "spec": "toLower"
    }]
}
```
0. The `sanitizeColumns` array, holding the sanitizer configs as json objects. Each sanitizer config can have many sanitizers.
1. An optional comment on "column" basis.
2. The array holding the actual, individual, sanitizer configuration. **NOTE**. We do _not_ have column index configured here. Meaning it is a global configuration that will be applied to _all_ columns.
3. The sanitization type. In this example a _trim_ operation.
4. The specification for this type. In this example _left_. Meaning a left trim operation.

### `trim` sanitizer

```jsonc
{
    "type": "trim", // 1) (mandatory)
    "spec": "left"  // 2) (mandatory) (example)
}
```
1. The type (name) of sanitizer to use. `trim` in this case.
2. The specification. I.e. which trim mode to use. Available are:
   1. `all` Removes leading and trailing whitespace
   2. `leading` Removes leading whitespace
   3. `trailing` Removes trailing whitespace


### `casing` sanitizer
Changes the casing for the complete string, depending on the `spec` attribute.
```jsonc
{   
    "type": "casing",   // 1) (mandatory)
    "spec": "toUpper"   // 2) (mandatory)  (example)
}
```
1. The type (name) of sanitizer to use. `casing` in this case.
2. The specification. I.e. which casing mode to use. Available are:
   1. `toLower` Lowercases the complete string.
   2. `toUpper` Uppercases the complete string.

### `eradicate` sanitizer
Eradicates a certain sub-string or sub-strings.
```jsonc
{   
    "type": "eradicate",    // 1) (mandatory)
    "spec": [" USD", ","]   // 2) (mandatory)  (example)
}
```
1. The type (name) of sanitizer to use. `eradicate` in this case.
2. The specification. In this case an array of strings to be eradicated. E.g a value of "1,000.00 USD" would become "1000.00".

### `regexTake` sanitizer
Takes a value, specified by the regular expression capture. **The first capture is what will be taken as the value.**
```jsonc
{   
    "type": "regexTake",        // 1) (mandatory)
    "spec": "(\\d+\\.\\d+)\\D*" // 2) (mandatory)
}
```
1. The type (name) of sanitizer to use. `regexTake` in this case.
2. The specification. In this the regex pattern to use. E.g. a value of "1000.00 (USD)" would become "1000.00".

### `replace` sanitizer
Replace a string with another string.

The spec allows for an array of replacements.
```jsonc
{   
    "type": "replace",  // 1) (mandatory)
    "spec": [{          // 2) (mandatory) (example, see all of the lines below)
        "from": "USD",  
        "to": "$"       
    }, {                
        "from": "_",    
        "to": " ",      
    }]
}
```
1. The type (name) of sanitizer to use. `regexTake` in this case.
2. The specification. In this example, a value of "USD_1,000.00" would become "$ 1,000.00".


## `typeColumns` - Column Typing Configuration
After all the sanitization we can finally type our columns!
Note that we do not need to specify column indices here. This configuration relies on the order of the elements in the array, since we have to type all columns anyway! (**NOTE**: if `typeColumns` is omitted, everything is implicitly typed as `String`.)

In essence every line of the config performs a String->Type transformation.
```jsonc
{   
    "typeColumns": [{               // 1) 
        "comment": "column-0",      // 2) (optional)
        "header": "Header-1",       // 3) (optional)
        "targetType": "Char"        // 4) (mandatory)
    },{
        "comment": "column-1",      // 2) (optional)
        "header": "Header-2",       // 3) (optional)
        "targetType": "DateTime",   // 4) (mandatory)
        "srcPattern": "%FT%T%:z"    // 5) (optional)
    }]
}
```
1. The array that holds the config. Every entry is the type configuration for one column.
2. A comment
3. The (new / final) header name.
4. The target type. (see: xxx)
5. An optional (chrono based, see: yyy) pattern used for String->DateType parsing. If no `srcPattern` is provided, the following applies:
    1. For `NaiveDate` we expect a format like `2022-12-31`, i.e. ISO8601 format (i.e. the chrono pattern _`%Y-%m-%d`_)
    2. For `NaiveDateTime` we expect a format like `2022-12-31T10:20:30` or `2022-12-31T10:20:30.500`, i.e. the chrono pattern _`%Y-%m-%dT%H:%M:%S`_  _`%Y-%m-%dT%H:%M:%S%.3f`_, respectivly.
    2. For `DateTime` we expect a format like `2022-12-31T10:20:30.500+02:00`, i.e. RFC3339 format

### Data Types
The following data types are supported.

* `Char`, `String`
* `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
* `UInt8`, `UInt16`, `UInt32`, `UInt64`, `UInt128`
* `Float32`, `Float64`
* `Bool`
* `Decimal` (*)
* `NaiveDate`, `NaiveDateTime`, `DateTime` (**)

(see also: https://doc.rust-lang.org/book/ch03-02-data-types.html)

(*) = through the `rust_decimal` crate. See: https://docs.rs/rust_decimal/latest/rust_decimal/

(**) = through the `chrono` crate. See: https://docs.rs/chrono/latest/chrono/
