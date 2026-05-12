use std::{
    fs::{self, DirEntry},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

const NOTEBOOK_FOLDER: &str = "/home/frobb/backups/kindle-scribe notebooks 12-05-2026";

fn main() {
    println!("Hello, world!");
    fs::create_dir(Path::new(&format!("{NOTEBOOK_FOLDER}/pdfs"))).ok();

    // iterate through each notebook

    let notebooks: Vec<DirEntry> = fs::read_dir(format!("{NOTEBOOK_FOLDER}/.notebooks"))
        .unwrap()
        .map(|dir| dir.unwrap())
        // Remove anything that doesn't contain a notebook file
        .filter(|dir| {
            !dir.path().to_str().unwrap().contains("!!")
                && fs::read_dir(Path::new(dir.path().to_str().unwrap()))
                    .unwrap()
                    .any(|f| f.unwrap().file_name() == "nbk")
        })
        .collect();

    let (sender, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));
    let num = std::thread::available_parallelism().unwrap().get();
    dbg!(num);

    let handles: Vec<JoinHandle<()>> = (1..=num)
        .map(|i| {
            let receiver = Arc::clone(&receiver);
            thread::spawn(move || {
                loop {
                    let job = {
                        let receiver = receiver.lock().unwrap();
                        receiver.recv()
                    };
                    match job {
                        Ok(dir) => {
                            println!("\n\nworker {i} working on {:?}", dir);
                            convert_to_pdf(dir);
                        }
                        Err(_) => {
                            println!("worker {i} stopping");
                            break;
                        }
                    };
                }
            })
        })
        .collect();

    notebooks.into_iter().for_each(|nb| {
        sender.send(nb).unwrap();
    });
    // Close sender
    drop(sender);

    handles.into_iter().for_each(|h| {
        h.join().expect("worker thread panicked");
    });
}

fn convert_to_pdf(dir: DirEntry) {
    let name = dir.file_name().into_string().unwrap();
    println!(" name: {:?}", name);
    // Output folder
    fs::create_dir(Path::new(&format!("{NOTEBOOK_FOLDER}/pdfs/{name}"))).ok();

    // Convert notebook to EPUB using calibre and the KFX Input plugin
    Command::new("calibre-debug")
        .stdout(Stdio::piped())
        .args([
            "-r",
            "KFX Input",
            "--",
            &format!("{NOTEBOOK_FOLDER}/.notebooks/{name}"),
            &format!("{NOTEBOOK_FOLDER}/pdfs/{name}/notebook.epub"),
        ])
        .status()
        .expect("Couldn't convert to epub!");

    // Convert EPUB to PDF
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

    // Remove EPUB file since no longer needed
    fs::remove_file(format!("{NOTEBOOK_FOLDER}/pdfs/{name}/notebook.epub"))
        .expect("Couldn't remove the epub file!");
}
