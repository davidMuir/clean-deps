# Clean-Deps

Basic dependency cleanup tool for dev dependencies (`node_modules/`, `target/`, `bin/`, `obj/`):

Works for the following languages:

- Dotnet
- Javascript (and TypeScript)
- Rust

## Usage

```sh
Usage: clean-deps [OPTIONS] [PATH]

Arguments:
  [PATH]  

Options:
  -d, --delete               
  -l, --language <LANGUAGE>  [possible values: dotnet, rust, javascript]
  -h, --help                 Print help
```

