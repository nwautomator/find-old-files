use clap::{Arg, Command};
use old_files::get_access_times;

fn main() -> anyhow::Result<()> {
    let args = Command::new("cmd")
        .arg(
            Arg::new("recursive")
                .long("recursive")
                .short('r')
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("directory")
                .long("directory")
                .short('d')
                .default_value("."),
        )
        .get_matches();

    get_access_times(
        std::path::Path::new(args.get_one::<String>("directory").unwrap().as_str()),
        args.get_flag("recursive"),
    )?;

    Ok(())
}
