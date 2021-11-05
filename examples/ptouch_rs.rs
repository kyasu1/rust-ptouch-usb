use std::ffi::OsString;

use anyhow::{Context, Result};
use clap::{App, Arg};
use ptouch::{Media, Model, Printer, PrinterProfile};

#[derive(Debug)]
struct Args {
    model: Model,
    serial: String,
    command: Command,
}

#[derive(Debug)]
enum Command {
    Print(PrintOptions),
    Status,
    Help,
}
#[derive(Debug)]
struct PrintOptions {
    label: Media,
    dither: bool,
    compress: bool,
    dpi_600: bool,
    low_quality: bool,
    no_cut: bool,
    filename: String,
}

impl Args {
    fn new() -> Self {
        Self::new_from(std::env::args_os().into_iter()).unwrap_or_else(|e| e.exit())
    }
    fn new_from<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: Iterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let app = App::new("ptouch-rs")
            .version("1.0")
            .about("P-Touch Printing Utility")
            .author("Yasuyuki Komatsubara");

        let model_option = Arg::new("model")
            .short('m')
            .long("model")
            .takes_value(true)
            .about("Please specify printer model")
            .required(true);

        let serial_option = Arg::new("serial")
            .short('s')
            .long("serial")
            .takes_value(true)
            .about("Please specify the serial number")
            .required(true);

        let print_command = App::new("print")
            .about("Print a label of the provided IMAGE.")
            .arg(
                Arg::new("label")
                    .short('l')
                    .long("label")
                    .takes_value(true)
                    .required(true),
            )
            .arg(Arg::new("dither").short('d').long("dither"))
            .arg(Arg::new("compress").short('c').long("compress"))
            .arg(Arg::new("600dpi").long("600dpi"))
            .arg(Arg::new("lq").long("lq"))
            .arg(Arg::new("no-cut").long("no-cut"))
            .arg(Arg::new("filename").required(true));

        let mut app = app
            .arg(model_option)
            .arg(serial_option)
            .subcommand(print_command)
            .subcommand(App::new("status"));

        let mut buffer: Vec<u8> = Vec::new();
        app.write_help(&mut buffer).unwrap();

        let matches = app.try_get_matches_from(args)?;

        let model = matches
            .value_of("model")
            .and_then(|model| Model::from_str(&model.to_uppercase()))
            .expect("Please specify a printer model");

        let serial = matches
            .value_of("serial")
            .and_then(|serial| {
                if serial.len() != 12 || !serial.chars().all(char::is_alphanumeric) {
                    None
                } else {
                    Some(serial)
                }
            })
            .expect("Please specify a valid printer serial");

        let command = if let Some(matches) = matches.subcommand_matches("print") {
            let label = matches
                .value_of("label")
                .and_then(|label| Media::from_str(label))
                .expect("Please specify a valid label name");

            let dither = if matches.is_present("dither") {
                true
            } else {
                false
            };

            let compress = if matches.is_present("compress") {
                true
            } else {
                false
            };

            let dpi_600 = if matches.is_present("600dpi") {
                true
            } else {
                false
            };

            let low_quality = if matches.is_present("lq") {
                true
            } else {
                false
            };

            let no_cut = if matches.is_present("no-cut") {
                true
            } else {
                false
            };

            let filename = matches
                .value_of("filename")
                .expect("Please specify a file name");

            let print_options = PrintOptions {
                label,
                dither,
                compress,
                dpi_600,
                low_quality,
                no_cut,
                filename: filename.to_string(),
            };

            Command::Print(print_options)
        } else if let Some(_) = matches.subcommand_matches("status") {
            Command::Status
        } else {
            Command::Help
        };

        Ok(Args {
            model,
            serial: serial.to_string(),
            command,
        })
    }
}
fn main() -> Result<()> {
    env_logger::init();

    let args = Args::new();

    println!("{:?}", args);

    match args.command {
        Command::Print(options) => {
            let profile = PrinterProfile::build_usb_profile(args.model, args.serial.clone())
                .context(format!(
                    "could not open printer {} with serial {}",
                    args.model, args.serial
                ))?;

            let label: image::DynamicImage = image::open(options.filename.clone())
                .context(format!("could not open file {}", options.filename))?;

            let matrix =
                ptouch::convert_fit(label, options.dpi_600, args.model.pins(), options.label);

            let printer = Printer::new(profile, options.label)
                .high_resolution(options.dpi_600)
                .cut_at_end(!options.no_cut)
                .two_colors(false)
                .enable_auto_cut(1);

            match printer.print(vec![matrix].into_iter()) {
                Ok(_) => {
                    println!("Print success!");
                }
                Err(err) => {
                    println!("Print error {:?}", err)
                }
            }
            Ok(())
        }
        Command::Status => {
            let profile = PrinterProfile::build_usb_profile(args.model, args.serial).unwrap();

            let status = profile
                .read_status()
                .with_context(|| format!("could not read printer status"))?;

            println!("{:?}", status);

            Ok(())
        }
        Command::Help => {
            anyhow::bail!("THIS SHOULD NOT HAPPEN");
        }
    }
}
