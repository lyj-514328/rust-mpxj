//! Batch-parse every real sample in the repository and report the success rate.
//!
//! Usage: `cargo run --release --example sample_survey`
//!
//! Walk every `.mpp`, `.mpx` and `.xml` sample under `mpxj/junit/data/`, pass each one to
//! [`MPPReader`] / [`MPXReader`] / [`MSPDIReader`] for parsing, then print:
//! - total / succeeded / failed / success rate
//! - failure reason grouping (aggregated by error text with paths and digits removed)
//! - list of failing samples
//!
//! Note: the sample directory contains a few **intentionally corrupt** files (used by MPXJ's own tests,
//! see directory names such as `corrupt`); their parse failures are expected. The report flags such samples separately.
//!
//! The XML samples are dispatched by content: the directory also holds XML samples belonging to
//! readers this crate does not port (Phoenix, GanttProject, P6, ...), so a file is only handed to
//! [`MSPDIReader`] when it carries the MSPDI root-element fingerprint — the same check Java's
//! `UniversalProjectReader` uses (see [`is_mspdi`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;

use mpxj::mpp::mpp_reader::MPPReader;
use mpxj::mpx::locale_utility::MpxLocale;
use mpxj::mpx::mpx_reader::MPXReader;
use mpxj::mspdi::mspdi_reader::MSPDIReader;
use mpxj::reader::project_reader::ProjectReader;
use mpxj::ProjectFile;

/// Reason for the most recent panic (retrieved after `catch_unwind`).
static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

/// Parse result for a single sample.
enum Outcome {
    Ok,
    /// Parsing returned an error
    Error(String),
}

/// Normalise error text so it can be grouped by reason:
/// strip absolute paths and concrete digits, keeping only the error's structure.
fn normalise(message: &str) -> String {
    let mut result = String::with_capacity(message.len());
    let mut last_was_digit = false;

    for ch in message.chars() {
        if ch.is_ascii_digit() {
            if !last_was_digit {
                result.push('#');
            }
            last_was_digit = true;
        } else {
            result.push(ch);
            last_was_digit = false;
        }
    }

    // Content inside single quotes is usually a file or entry name; erase it so the same reason is not split apart
    let mut collapsed = String::with_capacity(result.len());
    let mut in_quote = false;
    for ch in result.chars() {
        if ch == '\'' || ch == '"' {
            in_quote = !in_quote;
            collapsed.push(ch);
        } else if in_quote {
            continue;
        } else {
            collapsed.push(ch);
        }
    }

    collapsed
}

/// Return whether the path is under an "intentionally corrupt" sample directory.
fn is_intentionally_corrupt(path: &Path) -> bool {
    let text = path.to_string_lossy().to_ascii_lowercase();
    text.contains("corrupt")
        || text.contains("password")
        || text.contains("encrypted")
        || text.contains("invalid")
}

/// Recursively collect sample files with the given extension.
fn collect(root: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, extension, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            out.push(path);
        }
    }
}

/// Parse a single MPP sample.
fn read_mpp(path: &Path) -> Option<Outcome> {
    let mut reader = MPPReader::new();
    Some(match reader.read_file(path) {
        Ok(_) => Outcome::Ok,
        Err(error) => Outcome::Error(error.message().to_owned()),
    })
}

/// Infer the locale used by an MPX file from the sample file name.
///
/// MPX text content is localized to the language version of Microsoft Project, so reading a
/// non-English version requires specifying the locale first (see the "Locale" section of
/// `mpxj/mkdocs/docs/howto-read-mpx.md`). Sample file names look like `sample.de.mpx`, where the
/// language suffix is the locale; when there is no language suffix, English is used (`MPXReader`'s default).
fn locale_for(path: &Path) -> MpxLocale {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");

    stem.rsplit('.')
        .next()
        .and_then(MpxLocale::get_instance)
        .unwrap_or(MpxLocale::En)
}

/// Parse a single MPX sample.
fn read_mpx(path: &Path) -> Option<Outcome> {
    let mut reader = MPXReader::new();
    // Corresponds to Java `LocaleTest.testReadGerman`: `reader.setLocale(Locale.GERMAN)`
    reader.set_locale(locale_for(path));
    Some(match reader.read_file(path) {
        Ok(_) => Outcome::Ok,
        Err(error) => Outcome::Error(error.message().to_owned()),
    })
}

/// Return whether an XML sample is an MSPDI document.
///
/// Corresponds to Java: the MSPDI fingerprint of `UniversalProjectReader`
/// (`Pattern.compile(".*xmlns=\"http://schemas\\.microsoft\\.com/project.*", Pattern.DOTALL)`),
/// applied to the first 512 bytes of the file (`UniversalProjectReader.BUFFER_SIZE`).
///
/// The reverse-comparison case is not reproduced: this sample tree also holds XML files for readers
/// this crate does not port, and those are skipped rather than reported as failures.
fn is_mspdi(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };

    let limit = bytes.len().min(512);
    String::from_utf8_lossy(&bytes[..limit]).contains("http://schemas.microsoft.com/project")
}

/// Parse a single MSPDI sample (non-MSPDI XML files are skipped — see [`is_mspdi`]).
fn read_mspdi(path: &Path) -> Option<Outcome> {
    if !is_mspdi(path) {
        return None;
    }

    let mut reader = MSPDIReader::new();
    Some(match reader.read_file(path) {
        Ok(_) => Outcome::Ok,
        Err(error) => Outcome::Error(error.message().to_owned()),
    })
}

/// Survey a set of samples.
///
/// A reader returns `None` for a sample it does not claim; such samples are counted as skipped
/// rather than as failures.
fn survey(label: &str, root: &Path, extension: &str, reader: fn(&Path) -> Option<Outcome>) {
    let mut files = Vec::new();
    collect(root, extension, &mut files);
    files.sort();

    let (mut ok, mut corrupt_ok, mut skipped) = (0usize, 0usize, 0usize);
    let mut reasons: HashMap<String, usize> = HashMap::new();
    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    let mut panics = 0usize;

    for path in &files {
        let outcome = catch_unwind(AssertUnwindSafe(|| reader(path)));

        match outcome {
            Ok(Some(Outcome::Ok)) => {
                ok += 1;
                if is_intentionally_corrupt(path) {
                    corrupt_ok += 1;
                }
            }
            Ok(Some(Outcome::Error(message))) => {
                *reasons.entry(normalise(&message)).or_default() += 1;
                failures.push((path.clone(), message));
            }
            Ok(None) => {
                skipped += 1;
            }
            Err(_) => {
                panics += 1;
                let message = LAST_PANIC
                    .lock()
                    .ok()
                    .and_then(|mut slot| slot.take())
                    .unwrap_or_else(|| "<panic>".to_owned());
                *reasons.entry(message.clone()).or_default() += 1;
                failures.push((path.clone(), message));
            }
        }
    }

    let total = files.len();
    let failed = total - ok - skipped;
    println!();
    println!("=========== {label} ===========");
    println!("total samples : {total}");
    if skipped > 0 {
        println!("skipped (not this format) : {skipped}");
    }
    println!("parsed ok : {ok}");
    println!("parse failed : {failed}  (of which panic {panics})");
    if total > skipped {
        println!(
            "success rate : {:.2}%",
            (ok as f64) * 100.0 / ((total - skipped) as f64)
        );
    }
    if corrupt_ok > 0 {
        println!("(of the successful samples, {corrupt_ok} are in corrupt/password/encrypted/invalid directories)");
    }

    if !reasons.is_empty() {
        println!();
        println!("--- failure reason categories (descending by count) ---");
        let mut grouped: Vec<(String, usize)> = reasons.into_iter().collect();
        grouped.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (reason, count) in &grouped {
            println!("{count:>5}  {reason}");
        }

        println!();
        println!("--- failed sample list (up to 40) ---");
        let base = root.parent().unwrap_or(root);
        for (path, message) in failures.iter().take(40) {
            let relative = path.strip_prefix(base).unwrap_or(path);
            println!("  {} :: {}", relative.display(), message);
        }
        if failures.len() > 40 {
            println!("  ... {} more omitted", failures.len() - 40);
        }
    }
}

fn main() {
    // When running samples in bulk, record only the panic reason and do not print backtraces, to avoid flooding the output
    std::panic::set_hook(Box::new(|info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_owned());

        let location = info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "<unknown location>".to_owned());

        if let Ok(mut slot) = LAST_PANIC.lock() {
            *slot = Some(format!("{payload}  @ {location}"));
        }
    }));

    let data = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("mpxj")
        .join("junit")
        .join("data");

    survey("MPP samples", &data, "mpp", read_mpp);
    survey("MPX samples", &data, "mpx", read_mpx);
    survey("MSPDI samples", &data, "xml", read_mspdi);

    // Restore the panic hook so behaviour on exit is normal
    let _ = std::panic::take_hook();

    println!();
    println!("survey complete.");
    let _: Option<Rc<RefCell<ProjectFile>>> = None;
}
