use std::env;
use std::process::ExitCode;

const HELP: &str = "\
segzify — 正字正かなづかひ變換CLI

Usage:
  segzify [OPTIONS]

Options:
  -h, --help     このhelpを表示する
  -V, --version  versionを表示する
";

fn run(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = args.into_iter();
    let _program = args.next();

    match args.next().as_deref() {
        None | Some("-h" | "--help") => {
            if let Some(argument) = args.next() {
                return Err(format!("unknown argument: {argument}\n\n{HELP}"));
            }
            print!("{HELP}");
            Ok(())
        }
        Some("-V" | "--version") => {
            if let Some(argument) = args.next() {
                return Err(format!("unknown argument: {argument}\n\n{HELP}"));
            }
            println!("segzify {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(argument) => Err(format!("unknown argument: {argument}\n\n{HELP}")),
    }
}

fn main() -> ExitCode {
    match run(env::args()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprint!("{message}");
            ExitCode::from(64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn accepts_help() {
        assert!(run(["segzify".into(), "--help".into()]).is_ok());
    }

    #[test]
    fn accepts_version() {
        assert!(run(["segzify".into(), "--version".into()]).is_ok());
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(run(["segzify".into(), "--unknown".into()]).is_err());
    }

    #[test]
    fn rejects_unknown_trailing_arguments() {
        assert!(run(["segzify".into(), "--version".into(), "--typo".into()]).is_err());
    }
}
