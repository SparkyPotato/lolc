mod curry;
mod parse;
mod print;
mod reduce;
mod resolve;

use std::path::PathBuf;

use ariadne::{Report, ReportKind, Source};
use clap::Parser;

use crate::resolve::uncurry_expr;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Options {
	/// File to operate on.
	#[clap(value_name = "FILE")]
	file: PathBuf,
	/// Format the file.
	fmt: bool,
}

fn main() {
	let opts = Options::parse();

	let s = std::fs::read_to_string(&opts.file).unwrap();

	if !opts.fmt {
		let (file, errors) = parse::parse(&s);
		let mut cache = Source::from(s);
		handle_errors(&mut cache, errors);

		let file = curry::curry(file);
		let (mut file, errors) = resolve::resolve(file);
		handle_errors(&mut cache, errors);

		if let Some(id) = reduce::evaluate(&mut file) {
			let (file, id) = uncurry_expr(&file, id);
			let s = print::print_expr(&file, id);
			println!("{}", s);
		} else {
			handle_errors(
				&mut cache,
				vec![Report::build(ReportKind::Error, (), 0)
					.with_message("no main definition found")
					.finish()],
			);
		}
	} else {
		let s = s.replace('\\', "λ");
		std::fs::write(&opts.file, s).unwrap();
	}
}

fn handle_errors(cache: &mut Source, errors: Vec<Report>) {
	let error = errors.len() > 0;
	for error in errors {
		error.eprint(&mut *cache).unwrap();
	}
	if error {
		std::process::exit(1);
	}
}
