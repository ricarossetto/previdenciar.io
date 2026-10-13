//! Metadata, counts and static warnings only. Never print ImportResult.
use std::{env, path::PathBuf, process::ExitCode};
fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Usage: cnis_inspect <local-CNIS.pdf>"); return ExitCode::from(2);
    };
    if args.next().is_some() { eprintln!("Expected exactly one PDF path."); return ExitCode::from(2); }
    match previus_cnis::import_pdf(&PathBuf::from(path)) {
        Ok(result) => match serde_json::to_string_pretty(&result.summary()) {
            Ok(json) => { println!("{json}"); ExitCode::SUCCESS },
            Err(_) => { eprintln!("Unable to serialize safe metadata."); ExitCode::FAILURE },
        },
        Err(error) => { eprintln!("{error}"); ExitCode::FAILURE },
    }
}
