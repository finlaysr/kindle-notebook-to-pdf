use std::{
    env,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

mod converter;

use indicatif::{MultiProgress, ProgressBar};

#[derive(Clone)]
struct Config {
    notebook_dir: PathBuf,
    output_dir: PathBuf,
}

fn main() {
    let config = get_config();

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

    let term_out = Arc::new(Mutex::new(MultiProgress::new()));
    let bar = Arc::new(Mutex::new(
        term_out
            .lock()
            .unwrap()
            .add(ProgressBar::new(notebook_count as u64)),
    ));

    // Receivers and senders for sharing jobs between threads
    let (sender, receiver) = mpsc::channel::<DirEntry>();
    let receiver = Arc::new(Mutex::new(receiver));
    let threads = std::thread::available_parallelism().unwrap().get() / 2; // Don't kill it with too many threads
    println!("Using {} threads", threads);

    // Set up threads
    bar.lock().unwrap().inc(0);
    let handles: Vec<JoinHandle<()>> = (1..=threads)
        .map(|i| {
            let config = config.clone();
            let receiver = Arc::clone(&receiver);
            let term_out = Arc::clone(&term_out);
            let bar = Arc::clone(&bar);
            thread::spawn(move || {
                loop {
                    let job = receiver.lock().unwrap().recv();
                    match job {
                        Ok(dir) => {
                            term_out
                                .lock()
                                .unwrap()
                                .println(format!(
                                    "worker {i:<2} working on {}",
                                    dir.file_name().to_str().unwrap()
                                ))
                                .unwrap();

                            converter::convert_to_pdf(dir, &config, &term_out);
                            bar.lock().unwrap().inc(1);
                        }
                        Err(_) => {
                            term_out
                                .lock()
                                .unwrap()
                                .println(format!("worker {i} stopping"))
                                .unwrap();
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

    // Close sender
    drop(sender);

    // ensure all threads have finished
    handles.into_iter().for_each(|h| {
        h.join().expect("worker thread panicked");
    });
}

fn get_config() -> Config {
    let args: Vec<String> = env::args().collect();
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

            fs::create_dir(&config.output_dir).ok(); // Create output_dir if doesn't exist
            config
        }
        3 => Config {
            notebook_dir: PathBuf::from(&args[1]),
            output_dir: PathBuf::from(&args[2]),
        },
    };
    config
}
