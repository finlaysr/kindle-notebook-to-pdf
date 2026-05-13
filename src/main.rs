use std::{
    env,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use indicatif::{MultiProgress, ProgressBar};

#[derive(Clone)]
struct Config {
    notebook_dir: PathBuf,
    output_dir: PathBuf,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(&args);
    let config: Config = match args.len() {
        ..=1 | 4.. => {
            panic!(
                "Arguments Required:
<.notebooks folder> [output folder]
ouptut folder is optional, if not set then a new folder will be created next to .notebook",
            );
        }
        2 => {
            let config = Config {
                notebook_dir: PathBuf::from(&args[1]),
                output_dir: PathBuf::from(&args[1]).parent().unwrap().join("output"),
            };
            if fs::exists(&config.output_dir).unwrap() {
                fs::remove_dir_all(&config.output_dir)
                    .expect("Couldn't remove exisitng output directory");
            }
            fs::create_dir(&config.output_dir).expect("Couldn't create output directory");
            config
        }
        3 => Config {
            notebook_dir: PathBuf::from(&args[1]),
            output_dir: PathBuf::from(&args[2]),
        },
    };

    // Array of all notebooks to be converted
    let notebooks: Vec<DirEntry> = fs::read_dir(&config.notebook_dir)
        .unwrap()
        .map(|dir| dir.unwrap())
        // Remove anything that doesn't contain a notebook file or is an annotation file
        .filter(|dir| {
            !dir.path().to_str().unwrap().contains("!!") // annotation file
                && fs::read_dir(Path::new(dir.path().to_str().unwrap()))
                    .unwrap()
                    .any(|f| f.unwrap().file_name() == "nbk")
        })
        .collect();
    let notebook_count = notebooks.len();

    let bars = Arc::new(Mutex::new(MultiProgress::new()));
    let bar = Arc::new(Mutex::new(
        bars.lock()
            .unwrap()
            .add(ProgressBar::new(notebook_count as u64)),
    ));

    // Receivers and senders for sharing jobs between threads
    let (sender, receiver) = mpsc::channel::<DirEntry>();
    let receiver = Arc::new(Mutex::new(receiver));
    let threads = std::thread::available_parallelism().unwrap().get();
    dbg!(threads);

    // Set up threads

    let handles: Vec<JoinHandle<()>> = (1..=threads)
        .map(|i| {
            let config = config.clone();
            let receiver = Arc::clone(&receiver);
            let bars = Arc::clone(&bars);
            let bar = Arc::clone(&bar);
            thread::spawn(move || {
                loop {
                    let job = receiver.lock().unwrap().recv();
                    match job {
                        Ok(dir) => {
                            bars.lock()
                                .unwrap()
                                .println(format!(
                                    "worker {i} working on {}",
                                    dir.file_name().to_str().unwrap()
                                ))
                                .unwrap();

                            convert_to_pdf(dir, &config);
                            bar.lock().unwrap().inc(1);
                        }
                        Err(_) => {
                            println!("worker {i} stopping");
                            break;
                        }
                    }
                }
            })
        })
        .collect();

    // Send jobs to all threads
    notebooks.into_iter().for_each(|nb| {
        sender.send(nb).unwrap();
    });

    // ensure all threads have finished
    handles.into_iter().for_each(|h| {
        h.join().expect("worker thread panicked");
    });

    // Close sender
    drop(sender);
}

fn convert_to_pdf(dir: DirEntry, config: &Config) {
    const EPUB_NAME: &str = "notebook.epub";
    const PDF_NAME: &str = "notebook.pdf";

    let name = dir.file_name().into_string().unwrap();
    // Output folder
    fs::create_dir(config.output_dir.join(&name)).expect("Couldn't create notebook output folder!");

    // Convert notebook to EPUB using calibre and the KFX Input plugin
    Command::new("calibre-debug")
        .args([
            "-r",
            "KFX Input",
            "--",
            config.notebook_dir.join(&name).to_str().unwrap(),
            config
                .output_dir
                .join(&name)
                .join(EPUB_NAME)
                .to_str()
                .unwrap(),
        ])
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("Couldn't convert to epub!");

    // Convert EPUB to PDF
    Command::new("ebook-convert")
        .args([
            config
                .output_dir
                .join(&name)
                .join(EPUB_NAME)
                .to_str()
                .unwrap(),
            config
                .output_dir
                .join(&name)
                .join(PDF_NAME)
                .to_str()
                .unwrap(),
            "--pdf-page-margin-top",
            "0",
            "--pdf-page-margin-left",
            "0",
            "--pdf-page-margin-right",
            "0",
            "--pdf-page-margin-bottom",
            "0",
        ])
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("Couldn't convert to pdf!");

    // Remove EPUB file since no longer needed
    fs::remove_file(config.output_dir.join(&name).join(EPUB_NAME).as_path())
        .expect("Couldn't remove the epub file!");
}
