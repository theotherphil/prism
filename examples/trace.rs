
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

fn write_html_page(dir: &PathBuf, path: &str, images: &[PathBuf]) -> std::io::Result<()> {
    let mut html = File::create(dir.join(path))?;
    writeln!(html, "<html>")?;
    writeln!(html, "<body>")?;
    for image in images {
        writeln!(html, "<img src='{}'/>", image.to_string_lossy())?;
        writeln!(html, "<br><br>")?;
    }
    writeln!(html, "</body>")?;
    writeln!(html, "</html>")?;
    Ok(())
}

fn create_replay_image(dir: &PathBuf, name: &str, trace: &Trace) -> std::io::Result<PathBuf> {
    let image_path = dir.join(name.to_owned() + ".gif");
    let replay = replay(trace);
    let frames: Vec<RgbImage> = replay.iter().map(|i| upscale(&i, 10)).collect();
    write_trace_animation(&frames, 60, &image_path)?;
    Ok(image_path)
}

fn create_gradient_image(width: usize, height: usize) -> GrayImage {
    let mut image = GrayImage::new(width, height);
    for y in 0..image.height() {
        for x in 0..image.width() {
            image.set(x, y, (10 * (x % 10 + y % 10) as u8) + 50);
        }
    }
    image
}

fn main() -> std::io::Result<()> {
    let opts = Opts::from_args();
    let dir = &opts.output_dir;

    let mut replays = vec![];
    {
        let mut t = Tracer::new();
        {
            let i = create_gradient_image(5, 6);
            let i = t.create_from_image(&i);
            let _ = blur3_inline(&mut t, &i);
        }
        replays.push(create_replay_image(dir, "inline", &t.trace)?);
    }
    {
        let mut t = Tracer::new();
        {
            let i = create_gradient_image(5, 6);
            let i = t.create_from_image(&i);
            let _ = blur3_intermediate(&mut t, &i);
        }
        replays.push(create_replay_image(dir, "intermediate", &t.trace)?);
    }
    {
        let mut t = Tracer::new();
        {
            let i = create_gradient_image(5, 6);
            let i = t.create_from_image(&i);
            let _ = blur3_split_y(&mut t, &i, 2);
        }
        replays.push(create_replay_image(dir, "stripped", &t.trace)?);
    }
    {
        let mut t = Tracer::new();
        {
            let i = create_gradient_image(9, 6);
            let i = t.create_from_image(&i);
            let _ = blur3_tiled(&mut t, &i, 3, 3);
        }
        replays.push(create_replay_image(dir, "tiled", &t.trace)?);
    }

    write_html_page(dir, "traces.html", &replays)?;

    Ok(())
}