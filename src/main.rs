use clap::{Parser, ValueEnum};
use segzify::Converter;
use std::{
    fs,
    io::{self, Read},
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
#[command(name = "segzify")]
struct Args {
    input: Option<PathBuf>,
    #[arg(short, long)]
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
    let args = Args::parse();
    let input = match args.input {
        Some(path) => fs::read_to_string(path),
        None => {
            let mut s = String::new();
            io::stdin().read_to_string(&mut s).map(|_| s)
        }
    };
    let Ok(input) = input else {
        return ExitCode::from(66);
    };
    let Ok(converter) = Converter::embedded() else {
        return ExitCode::from(70);
    };
    let (text, report) = converter.convert(&input);
    if args.check && text != input {
        return ExitCode::from(1);
    }
    if !args.check {
        if let Some(path) = args.output {
            if fs::write(path, text).is_err() {
                return ExitCode::from(66);
            }
        } else {
            print!("{text}");
        }
    }
    if !matches!(args.report, ReportFormat::None) {
        let rendered = match args.report {
            ReportFormat::Json => serde_json::to_string_pretty(&report).unwrap() + "\n",
            ReportFormat::Text => format!("{report:#?}\n"),
            ReportFormat::None => String::new(),
        };
        if let Some(path) = args.report_output {
            if fs::write(path, rendered).is_err() {
                return ExitCode::from(66);
            }
        } else {
            eprint!("{rendered}");
        }
    }
    if args.fail_on_unresolved
        && (!report.unresolved_ambiguous_characters.is_empty()
            || !report.boundary_skipped_compound_replacements.is_empty())
    {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}
