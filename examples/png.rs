
use std::path::PathBuf;
use structopt::StructOpt;
use prism::*;

#[derive(StructOpt, Debug)]
struct Opts {
    /// An example file is written to this location.
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output: PathBuf
}

fn main() -> std::io::Result<()> {
    let opts = Opts::from_args();

    let mut i = GrayImage::new(50, 50);
    for y in 0..i.height() {
        for x in 0..i.width() {
            i[[x, y]] = ((y / 10 + x / 10) * 20) as u8;
        }
    }
    save_to_png(&i, &opts.output)?;

    Ok(())
}