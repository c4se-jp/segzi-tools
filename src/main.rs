use clap::{Parser, ValueEnum};
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
    if args.check && args.output.is_some() {
        eprintln!("--check と --output は併用できません");
        return ExitCode::from(64);
    }
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
        if let Some(path) = args.output {
            if fs::write(path, &text).is_err() {
                eprintln!("outputを書き込めません");
                return ExitCode::from(74);
            }
        } else {
            if io::stdout().write_all(text.as_bytes()).is_err() {
                eprintln!("stdoutへ書き込めません");
                return ExitCode::from(74);
            }
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
                eprintln!("reportを書き込めません");
                return ExitCode::from(74);
            }
        } else {
            if io::stderr().write_all(rendered.as_bytes()).is_err() {
                return ExitCode::from(74);
            }
        }
    }
    if args.fail_on_unresolved
        && (!report.unresolved_ambiguous_characters.is_empty()
            || !report.boundary_skipped_compound_replacements.is_empty())
    {
        ExitCode::from(2)
    } else if args.check && text != input {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
