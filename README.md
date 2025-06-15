# Rust URL Credential Extractor

This project is a Rust application that extracts lines in the format `url:credential:password` from text files located in a specified folder. It filters these lines based on a keyword provided by the user and saves the results to an output text file. The application utilizes threads to improve performance.

## Features

- Extracts lines in the format `url:credential:password`.
- Filters extracted lines based on a user-defined keyword.
- Saves the filtered results to an output text file.
- Utilizes multithreading for improved speed and efficiency.

## Getting Started

### Prerequisites

- Rust installed on your machine. You can install Rust by following the instructions at [rust-lang.org](https://www.rust-lang.org/tools/install).

### Installation

1. Clone the repository:

   ```
   git clone <repository-url>
   cd rust-extractor
   ```

2. Build the project:

   ```
   cargo build
   ```

### Usage

To run the extractor, use the following command:

```
cargo run -- <input_folder> <keyword> <output_file>
```

- `<input_folder>`: The path to the folder containing the text files to be processed.
- `<keyword>`: The keyword to filter the extracted lines.
- `<output_file>`: The path to the output text file where the results will be saved.

### Example

```
cargo run -- ./texts "my_keyword" ./output/results.txt
```

This command will process all text files in the `./texts` directory, filter lines containing "my_keyword", and save the results to `./output/results.txt`.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue for any suggestions or improvements.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.