
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

fn write_some_example_images(dir: &PathBuf) -> std::io::Result<()> {
    let mut i = GrayImage::new(50, 50);
    for y in 0..i.height() {
        for x in 0..i.width() {
            i[[x, y]] = ((y / 10 + x / 10) * 20) as u8;
        }
    }

    let mut f = GrayImage::new(50, 50);
    for y in 0..i.height() {
        for x in 0..i.width() {
            f[[x, y]] = 255 - ((y / 10 + x / 10) * 20) as u8;
        }
    }

    save_to_png(&i, dir.join("grad.png"))?;
    save_to_gif(&i, dir.join("grad.gif"))?;
    save_to_gif(&f, dir.join("grad_flip.gif"))?;
    animation(&[i, f], 300, dir.join("animation.gif"))?;

    Ok(())
}

// Maybe we should give up on using index syntax and create an interface based on get and set
// so that we can write the algorithms once and have them work for both perf testing and tracing
fn trace_blur3_inline(dir: &PathBuf, tracer: &mut Tracer, image: &GrayImage) -> std::io::Result<()> {
    let (w, h) = image.dimensions();
    let mut image = tracer.create_from_image("input", image);
    let mut result = tracer.create_new("result", w, h);

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let t = (image.get(x - 1, y - 1) + image.get(x, y - 1) + image.get(x + 1, y - 1)) / 3;
            let m = (image.get(x - 1, y) + image.get(x, y) + image.get(x + 1, y)) / 3;
            let b = (image.get(x - 1, y + 1) + image.get(x, y + 1) + image.get(x + 1, y + 1)) / 3;
            let p = (t + m + b) / 3;
            result.set(x, y, p);
        }
    }
    
    let replay = replay(&[image, result]);
    let image_path = dir.join("image.gif");
    let frames: Vec<RgbImage> = replay.iter().map(|i| upscale(&i, 10)).collect();

    write_trace_animation(&frames, 100, &image_path)?;
    write_html_page(dir, "trace.html", &image_path)?;

    Ok(())
}

fn trace_blur3_intermediate(dir: &PathBuf, tracer: &mut Tracer, image: &GrayImage) -> std::io::Result<()> {
    let (w, h) = image.dimensions();
    let mut image = tracer.create_from_image("input", image);
    
    let mut hblur = tracer.create_new("h", w, h);
    for y in 0..h {
        for x in 1..w - 1 {
            hblur.set(x, y, (image.get(x - 1, y) + image.get(x, y) + image.get(x + 1, y)) / 3);
        }
    }
    let mut vblur = tracer.create_new("v", w, h);
    for y in 1..h - 1 {
        for x in 0..w {
            vblur.set(x, y, (hblur.get(x, y - 1) + hblur.get(x, y) + hblur.get(x, y + 1)) / 3);
        }
    }

    let image_path = dir.join("image_i.gif");
    let replay = replay(&[image, hblur, vblur]);
    let frames: Vec<RgbImage> = replay.iter().map(|i| upscale(&i, 10)).collect();
    write_trace_animation(&frames, 100, &image_path)?;
    write_html_page(dir, "trace_i.html", &image_path)?;

    Ok(())
}

fn write_html_page(dir: &PathBuf, path: &str, image: &PathBuf) -> std::io::Result<()> {
    let mut html = File::create(dir.join(path))?;
    writeln!(html, "<html>")?;
    writeln!(html, "<body>")?;
    writeln!(html, "<img src='{}'/>", image.to_string_lossy())?;
    writeln!(html, "</body>")?;
    writeln!(html, "</html>")?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let opts = Opts::from_args();

    write_some_example_images(&opts.output_dir)?;

    let mut i = GrayImage::new(4, 4);
    for y in 0..i.height() {
        for x in 0..i.width() {
            i[[x, y]] = (10 * (x % 10 + y % 10) as u8) + 50;
        }
    }

    trace_blur3_inline(&opts.output_dir, &mut Tracer::new(), &i)?;
    trace_blur3_intermediate(&opts.output_dir, &mut Tracer::new(), &i)?;

    Ok(())
}