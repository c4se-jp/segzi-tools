use clap::{Parser, ValueEnum, error::ErrorKind};
use segzify::Converter;
use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Clone, ValueEnum)]
enum ReportFormat {
    Text,
    Json,
    None,
}
#[derive(Parser)]
#[command(name = "segzify", version)]
struct Args {
    input: Option<PathBuf>,
    #[arg(short, long, conflicts_with = "check")]
    output: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "text")]
    report: ReportFormat,
    #[arg(long)]
    report_output: Option<PathBuf>,
    #[arg(long)]
    check: bool,
    #[arg(long)]
    fail_on_unresolved: bool,
}
fn main() -> ExitCode {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let success = matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            );
            let _ = error.print();
            return ExitCode::from(if success { 0 } else { 64 });
        }
    };
    let input = match args.input {
        Some(path) => fs::read_to_string(path),
        None => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s).map(|_| s)
        }
    };
    let Ok(input) = input else {
        eprintln!("inputを讀み込めません");
        return ExitCode::from(66);
    };
    let Ok(converter) = Converter::embedded() else {
        eprintln!("變換dataを初期化できません");
        return ExitCode::from(70);
    };
    let (text, report) = converter.convert(&input);
    if !args.check {
        if write_output(args.output.as_deref(), text.as_bytes(), false).is_err() {
            eprintln!("outputを書き込めません");
            return ExitCode::from(74);
        }
    }
    let rendered = match args.report {
        ReportFormat::Json => Some(serde_json::to_string_pretty(&report).unwrap() + "\n"),
        ReportFormat::Text => Some(format!("{report:#?}\n")),
        ReportFormat::None => None,
    };
    if let Some(rendered) = rendered {
        if write_output(args.report_output.as_deref(), rendered.as_bytes(), true).is_err() {
            eprintln!("reportを書き込めません");
            return ExitCode::from(74);
        }
    }
    let mut status = 0;
    if args.check && text != input {
        status |= 1;
    }
    if args.fail_on_unresolved
        && (!report.unresolved_ambiguous_characters.is_empty()
            || !report.boundary_skipped_compound_replacements.is_empty())
    {
        status |= 2;
    }
    ExitCode::from(status)
}

fn write_output(path: Option<&std::path::Path>, text: &[u8], stderr: bool) -> io::Result<()> {
    if let Some(path) = path {
        fs::write(path, text)
    } else if stderr {
        io::stderr().write_all(text)
    } else {
        io::stdout().write_all(text)
    }
}
