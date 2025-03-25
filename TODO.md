# TODO

## README

- Document configuration format and usage

## Testing / usability

- Add support for test function using rundll32 entrypoint
  - Parse config file and write details to stdout?

## Config

- Create a new struct for `load_libraries` field
  - Implement std::fmt::Display for new struct
  - Add param to allow LoadLibrary calls to fail
- Abstract config parser to accept io::BufReader / similar abstraction
