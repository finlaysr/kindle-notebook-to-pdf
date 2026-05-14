use indicatif::MultiProgress;

use crate::Config;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, DirEntry},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

pub(crate) fn convert_to_pdf(dir: DirEntry, config: &Config, term_out: &Arc<Mutex<MultiProgress>>) {
    const HASH_NAME: &str = ".hash.sha256";
    const EPUB_NAME: &str = "notebook.epub";
    const PDF_NAME: &str = "notebook.pdf";
    let name = dir.file_name().into_string().unwrap();

    // Create output directory if it doesn't already exist
    fs::create_dir(config.output_dir.join(&name)).ok();

    // Create hash of notebook file to check if is has been converted previously
    let hash = hex::encode(Sha256::digest(
        fs::read(config.notebook_dir.join(&name).join("nbk")).unwrap(),
    ));

    // Check if hash of the file to be converted is the same as the saved hash, if so the skip
    if let Ok(saved_hash) = fs::read_to_string(config.output_dir.join(&name).join(HASH_NAME))
        && saved_hash == hash
    {
        // Hash has not changed, no need to convert again
        term_out
            .lock()
            .unwrap()
            .println(format!("{} is already up to date, skipping", name))
            .unwrap();
        return;
    } else {
        // Remove all old files in output directory
        fs::read_dir(config.output_dir.join(&name))
            .unwrap()
            .for_each(|file| fs::remove_file(file.unwrap().path()).unwrap());
    }

    // Convert notebook to EPUB using calibre and the KFX Input plugin
    let mut attempts = 3;
    let mut success = false;
    while attempts > 0 && !success {
        let calibre_proc = match Command::new("calibre-debug")
            .env("QT_QPA_PLATFORM", "offscreen")
            .env("QT_OPENGL", "software")
            .env("LIBGL_ALWAYS_SOFTWARE", "1")
            .env("VK_ICD_FILENAMES", "")
            .env(
                "QTWEBENGINE_CHROMIUM_FLAGS",
                "--disable-gpu --headless --no-sandbox --disable-gpu-compositing",
            )
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
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
        {
            Ok(proc) => proc,
            Err(e) => {
                // If spawning fails, try again
                term_out
                    .lock()
                    .unwrap()
                    .println(format!(
                        "Error in spawning calibre process for {name}! \n Error: \n{e}"
                    ))
                    .unwrap();
                attempts -= 1;
                thread::sleep(std::time::Duration::from_secs(1)); // Sleep for a bit before retrying
                continue;
            }
        };
        let out = calibre_proc
            .wait_with_output()
            .expect("Failed to wait on calibre");

        if out.status.success() {
            success = true;
        } else {
            attempts -= 1;
            term_out
                .lock()
                .unwrap()
                .println(format!(
                    "Error in converting {name} to epub! \n Error: \n{}",
                    String::from_utf8_lossy(&out.stderr)
                ))
                .unwrap();
            thread::sleep(std::time::Duration::from_secs(1)); // Sleep for a bit before retrying
        }
    }

    // If failed to convert to EPUB, no point in trying to convert to PDF
    if !success {
        return;
    }

    // Convert EPUB to PDF
    let mut attempts = 3;
    let mut success = false;
    while attempts > 0 && !success {
        let ebook_convert_proc = match Command::new("ebook-convert")
            .env("QT_QPA_PLATFORM", "offscreen")
            .env("QT_OPENGL", "software")
            .env("LIBGL_ALWAYS_SOFTWARE", "1")
            .env("VK_ICD_FILENAMES", "")
            .env(
                "QTWEBENGINE_CHROMIUM_FLAGS",
                "--disable-gpu --headless --no-sandbox --disable-gpu-compositing",
            )
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
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
        {
            Ok(proc) => proc,
            Err(e) => {
                // If spawning fails, try again
                term_out
                    .lock()
                    .unwrap()
                    .println(format!(
                        "Error in spawning ebook-convert process for {name}! \n Error: \n{e}"
                    ))
                    .unwrap();
                attempts -= 1;
                thread::sleep(std::time::Duration::from_secs(1)); // Sleep for a bit before retrying
                continue;
            }
        };
        let out = ebook_convert_proc
            .wait_with_output()
            .expect("Failed to wait on ebook-convert");

        if out.status.success() {
            success = true;
            // Remove EPUB file since no longer needed
            fs::remove_file(config.output_dir.join(&name).join(EPUB_NAME).as_path())
                .expect("Couldn't remove the epub file!");

            // Save new hash
            if let Err(e) = fs::write(config.output_dir.join(&name).join(HASH_NAME), &hash) {
                term_out
                    .lock()
                    .unwrap()
                    .println(format!("Couldn't save hash for {name}! \nError: {e}"))
                    .unwrap();
            }
        } else {
            attempts -= 1;
            term_out
                .lock()
                .unwrap()
                .println(format!(
                    "Error in converting {name} to pdf! \n Error: \n{}",
                    String::from_utf8_lossy(&out.stderr)
                ))
                .unwrap();
            thread::sleep(std::time::Duration::from_secs(1)); // Sleep for a bit before retrying
        }
    }
}
