
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

fn trace_blur3_inline(tracer: &mut Tracer, image: &GrayImage) -> Vec<TraceImage> {
    let (w, h) = image.dimensions();
    let image = tracer.create_from_image("input", image);
    let mut result = tracer.create_new("result", w, h);

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let t = (image.get(x - 1, y - 1) as u16 + image.get(x, y - 1) as u16 + image.get(x + 1, y - 1) as u16) / 3;
            let m = (image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3;
            let b = (image.get(x - 1, y + 1) as u16 + image.get(x, y + 1) as u16 + image.get(x + 1, y + 1) as u16) / 3;
            let p = (t + m + b) / 3;
            result.set(x, y, p as u8);
        }
    }
    
    vec![image, result]
}

fn trace_blur3_intermediate(tracer: &mut Tracer, image: &GrayImage) -> Vec<TraceImage> {
    let (w, h) = image.dimensions();
    let image = tracer.create_from_image("input", image);
    
    let mut hblur = tracer.create_new("h", w, h);
    for y in 0..h {
        for x in 1..w - 1 {
            hblur.set(x, y, ((image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3) as u8);
        }
    }
    let mut vblur = tracer.create_new("v", w, h);
    for y in 1..h - 1 {
        for x in 0..w {
            vblur.set(x, y, ((hblur.get(x, y - 1) as u16 + hblur.get(x, y) as u16 + hblur.get(x, y + 1) as u16) / 3) as u8);
        }
    }

    vec![image, hblur, vblur]
}

fn trace_blur3_stripped(tracer: &mut Tracer, image: &GrayImage) -> Vec<TraceImage> {
    let strip_height = 2;

    assert!(image.height() % strip_height == 0);
    let buffer_height = strip_height + 2;

    let (w, h) = image.dimensions();
    let image = tracer.create_from_image("input", image);

    let mut v = tracer.create_new("v", w, h);
    let mut strip = tracer.create_new("s", w, buffer_height);

    for y_outer in 0..h / strip_height {
        let y_offset = y_outer * strip_height;

        strip.clear();

        for y_buffer in 0..buffer_height {
            if y_buffer + y_offset == 0 || y_buffer + y_offset > h {
                continue;
            }
            let y_image = y_buffer + y_offset - 1;
            for x in 1..w - 1 {
                let p = (image.get(x - 1, y_image) as u16 + image.get(x, y_image) as u16 + image.get(x + 1, y_image) as u16) / 3;
                strip.set(x, y_buffer, p as u8);
            }
        }

        for y_inner in 0..strip_height {
            if y_inner + y_offset == 0 || y_inner + y_offset == h - 1 {
                continue;
            }
            for x in 0..w {
                let y_buffer = y_inner + 1;
                let p = (strip.get(x, y_buffer - 1) as u16 + strip.get(x, y_buffer) as u16 + strip.get(x, y_buffer + 1) as u16) / 3;
                v.set(x, y_inner + y_offset, p as u8);
            }
        }
    }

    vec![image, strip, v]
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

fn create_replay_image(dir: &PathBuf, name: &str, traces: &[TraceImage]) -> std::io::Result<PathBuf> {
    let image_path = dir.join(name.to_owned() + ".gif");
    let replay = replay(traces);
    let frames: Vec<RgbImage> = replay.iter().map(|i| upscale(&i, 10)).collect();
    write_trace_animation(&frames, 80, &image_path)?;
    Ok(image_path)
}

fn create_gradient_image(width: usize, height: usize) -> GrayImage {
    let mut image = GrayImage::new(width, height);
    for y in 0..image.height() {
        for x in 0..image.width() {
            image[[x, y]] = (10 * (x % 10 + y % 10) as u8) + 50;
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
        let i = create_gradient_image(5, 6);
        let inline = trace_blur3_inline(&mut t, &i);
        replays.push(create_replay_image(dir, "inline", &inline)?);
    }
    {
        let mut t = Tracer::new();
        let i = create_gradient_image(5, 6);
        let intermediate = trace_blur3_intermediate(&mut t, &i);
        replays.push(create_replay_image(dir, "intermediate", &intermediate)?);
    }
    {
        let mut t = Tracer::new();
        let i = create_gradient_image(5, 6);
        let stripped = trace_blur3_stripped(&mut t, &i);
        replays.push(create_replay_image(dir, "stripped", &stripped)?);
    }

    write_html_page(dir, "traces.html", &replays)?;

    Ok(())
}