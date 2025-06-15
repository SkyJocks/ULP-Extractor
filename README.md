# ULP Extractor

**ULP Extractor** is a fast, multithreaded Rust application with a graphical interface for extracting and filtering credentials (email:pass, user:pass, num:pass) from large text files or folders. It supports advanced filtering, deduplication, and efficient processing of both small and very large files.

## Features

- Extracts credentials in formats: `email:pass`, `user:pass`, `num:pass`
- Advanced filters: only emails, only numeric users, only alphanumeric users
- Minimum length sliders for numeric user and password
- Deduplication of results
- Multithreaded for high performance (handles hundreds of GB)
- Modern GUI with dark/red gradient background
- Preview of first results and unique result counter
- Option to append or overwrite output file

## Getting Started

### Prerequisites

- Windows 10/11 with up-to-date graphics drivers
- [Rust toolchain](https://www.rust-lang.org/tools/install)

### Build

```sh
cargo build --release
```

The executable will be at `target/release/ulp-extractor.exe`.

### Usage

1. Run the executable.
2. Select the input folder and output file.
3. Set your filters and keywords.
4. Click "Process" to extract and filter credentials.
5. Copy or use the results as needed.

### Notes

- For best performance, use the release build.
- If you see an OpenGL or WGPU error, update your graphics drivers.

## License

This project is licensed under the MIT License.