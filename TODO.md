# TODO

## README

- Document configuration format and usage

## Testing / usability

- Add support for test program that either:
  - parses config file and write details to stdout?
  - loads the library, which will behave differently in this mode

## Config

- Create a new struct for `load_libraries` field
  - Implement std::fmt::Display for new struct
  - Add param to allow LoadLibrary calls to fail
- Abstract config parser to accept io::BufReader / similar abstraction
