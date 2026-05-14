use indicatif::MultiProgress;

use crate::Config;
use std::{
    fs::{self, DirEntry},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

pub(crate) fn convert_to_pdf(dir: DirEntry, config: &Config, bars: &Arc<Mutex<MultiProgress>>) {
    const EPUB_NAME: &str = "notebook.epub";
    const PDF_NAME: &str = "notebook.pdf";

    let name = dir.file_name().into_string().unwrap();
    // Output folder
    fs::create_dir(config.output_dir.join(&name)).expect("Couldn't create notebook output folder!");

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
                bars.lock()
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
            bars.lock()
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
                bars.lock()
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
        } else {
            attempts -= 1;
            bars.lock()
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
