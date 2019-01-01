//!
//! Runs some handwritten blur functions using the TraceImage type to generate replay
//! visualisations.
//!
//! Example command line:
//!
//! $ cargo run --release --example trace -- -o /some/directory
//!

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use structopt::StructOpt;
use prism::*;

#[derive(StructOpt, Debug)]
struct Opts {
    /// Example files are written to this directory.
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_dir: PathBuf
}

fn write_html_page(dir: &PathBuf, path: &str, images: &[(&str, PathBuf)]) -> std::io::Result<()> {
    let mut html = File::create(dir.join(path))?;
    writeln!(html, "<html>")?;
    writeln!(html, "<body>")?;
    for image in images {
        writeln!(html, "<p>{}<p/>", image.0)?;
        writeln!(html, "<img src='{}'/>", image.1.to_string_lossy())?;
        writeln!(html, "<br><br>")?;
    }
    writeln!(html, "</body>")?;
    writeln!(html, "</html>")?;
    Ok(())
}

fn gradient_image(width: usize, height: usize) -> GrayImage {
    let mut image = GrayImage::new(width, height);
    for y in 0..image.height() {
        for x in 0..image.width() {
            image.set(x, y, (10 * (x % 10 + y % 10) as u8) + 50);
        }
    }
    image
}

// Appplies function to input image, creates visualistion from the trace, writes replay
// to disk and returns the path to this file
fn visualise<F>(dir: &PathBuf, name: &str, image: &GrayImage, f: F, delay_in_ms: u16) -> std::io::Result<PathBuf>
where F: Fn(&mut Tracer, &TraceImage) -> TraceImage
{
    let mut t = Tracer::new();
    let image = t.create_from_image(&image);
    let _ = f(&mut t, &image);

    let replay = replay(&t.trace);
    let frames = replay.iter().map(|i| upscale(&i, 10)).collect::<Vec<_>>();

    let palette = create_gif_palette();
    let image_path = dir.join(name.to_owned() + ".gif");
    animation_rgb(&frames, delay_in_ms, Some(&palette), &image_path)?;

    Ok(image_path)
}

fn main() -> std::io::Result<()> {
    let opts = Opts::from_args();
    let dir = &opts.output_dir;

    let replays = vec![
        (
            "Dimensions: y, x. Compute at blur_v.x, store at blur_v.x",
            visualise(dir, "inline", &gradient_image(5, 6), |t, i| blur3_inline(t, i), 60)?
        ),
        (
            "Dimensions: y, x. Compute at root, store at root",
            visualise(dir, "intermediate", &gradient_image(5, 6), |t, i| blur3_intermediate(t, i), 60)?
        ),
        (
            "Dimensions: y, x. Compute at blur_v.x, store at root",
            visualise(dir, "local_intermediate", &gradient_image(5, 6), |t, i| blur3_local_intermediate(t, i), 60)?
        ),
        (
            "Dimensions: yo, y, x. Compute at blur_v.yo, store at blur_v.yo",
            visualise(dir, "stripped", &gradient_image(5, 6), |t, i| blur3_split_y(t, i, 2), 60)?
        ),
        (
            "Dimension: yo, xo, y, x. Compute at blur_v.xo, store at blur_v.xo",
            visualise(dir, "tiled", &gradient_image(9, 6), |t, i| blur3_tiled(t, i, 3, 3), 20)?
        ),
    ];

    write_html_page(dir, "traces.html", &replays)?;

    Ok(())
}