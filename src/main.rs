use std::{fs, path::Path, process::Command};

const NOTEBOOK_FOLDER: &str = "/home/frobb/backups/kindle-scribe notebooks 12-05-2026";

fn main() {
    println!("Hello, world!");
    fs::create_dir(Path::new(&format!("{NOTEBOOK_FOLDER}/pdfs"))).ok();
    let mut count = 0;

    fs::read_dir(format!("{NOTEBOOK_FOLDER}/.notebooks"))
        .unwrap()
        .map(|folder| folder.unwrap())
        .filter(|folder| {
            fs::read_dir(Path::new(folder.path().to_str().unwrap()))
                .unwrap()
                .any(|f| f.unwrap().file_name() == "nbk")
        })
        .for_each(|notebook| {
            let name = notebook.file_name().into_string().unwrap();
            println!("\n\n\n{count}. name: {:?}", name);
            count += 1;

            fs::create_dir(Path::new(&format!("{NOTEBOOK_FOLDER}/pdfs/{name}"))).ok();
            Command::new("calibre-debug")
                .args([
                    "-r",
                    "KFX Input",
                    "--",
                    &format!("{NOTEBOOK_FOLDER}/.notebooks/{name}"),
                    &format!("{NOTEBOOK_FOLDER}/pdfs/{name}/notebook.epub"),
                ])
                .status()
                .expect("Couldn't convert to epub!");
            Command::new("ebook-convert")
                .args([
                    &format!("{NOTEBOOK_FOLDER}/pdfs/{name}/notebook.epub"),
                    &format!("{NOTEBOOK_FOLDER}/pdfs/{name}/notebook.pdf"),
                    "--pdf-page-margin-top",
                    "0",
                    "--pdf-page-margin-left",
                    "0",
                    "--pdf-page-margin-right",
                    "0",
                    "--pdf-page-margin-bottom",
                    "0",
                ])
                .status()
                .expect("Couldn't convert to pdf!");
        });
}
