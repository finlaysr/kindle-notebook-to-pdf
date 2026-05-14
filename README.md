# Kindle Notebook To PDF

Convert kindle scribe notebook `nbk` files to PDFs.

Uses the Calibre KFX plugin to convert to each notebook to an EPUB, then converts each EPUB to a PDF

## Requirements

* [Rust compiler](https://rustup.rs/)
* [Calibre](https://calibre-ebook.com/)
* [Calibre KFX Input Plugin](https://www.mobileread.com/forums/showthread.php?t=291290)

## Usage

* Connect the kindle scribe to a computer using a USB Cable
* Extract the `.notebooks` folder from the kindle scribe, and save to your computer
* Given the following file structure:
  ```
  example_folder
  └── .notebooks
      ├── 0a9722ce-9a4a-5786-1e4c-435f50640efd
      ├── 0aa30f71-67fb-f88e-6324-13a9273197a2
      ...
  ```
* To convert all the notebooks to pdfs, run:
  ```
  cargo run -- path/to/example_folder/.notebooks
  ```
* This will convert all the notebook files to pdf, storing the ouptut in a newly create `output` folder
  ```
  example_folder
  ├── .notebooks
  │   ├── 0a9722ce-9a4a-5786-1e4c-435f50640efd
  │   ├── 0aa30f71-67fb-f88e-6324-13a9273197a2
  │   ...
  │
  └── output
      ├── 0a9722ce-9a4a-5786-1e4c-435f50640efd
      │   └── notebook.pdf
      ├── 0aa30f71-67fb-f88e-6324-13a9273197a2
      │   └── notebook.pdf
      ...
  ```
* You can also specify an output directory, for example
  ```
  cargo run -- path/to/example_folder/.notebooks  path/to/output
  ```
