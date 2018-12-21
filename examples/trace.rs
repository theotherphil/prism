
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
    
    let image_path = dir.join("image.gif");
    let result_path = dir.join("result.gif");

    write_trace_animation(&replay(&image, 10), 100, &image_path)?;
    write_trace_animation(&replay(&result, 10), 900, &result_path)?;

    let mut html = File::create(dir.join("trace.html"))?;
    writeln!(html, "<html>")?;
    writeln!(html, "<body>")?;
    writeln!(html, "<img src='{}'/>", image_path.to_string_lossy())?;
    writeln!(html, "<img src='{}'/>", result_path.to_string_lossy())?;
    writeln!(html, "</body>")?;
    writeln!(html, "</html>")?;

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
    let h_path = dir.join("h.gif");
    let v_path = dir.join("v.gif");

    write_trace_animation(&replay(&image, 10), 200, &image_path)?;
    write_trace_animation(&replay(&hblur, 10), 200, &h_path)?;
    write_trace_animation(&replay(&vblur, 10), 200, &v_path)?;

    let mut html = File::create(dir.join("trace_i.html"))?;
    writeln!(html, "<html>")?;
    writeln!(html, "<body>")?;
    writeln!(html, "<img src='{}'/>", image_path.to_string_lossy())?;
    writeln!(html, "<img src='{}'/>", h_path.to_string_lossy())?;
    writeln!(html, "<img src='{}'/>", v_path.to_string_lossy())?;
    writeln!(html, "</body>")?;
    writeln!(html, "</html>")?;

    Ok(())
}

fn basic_trace(dir: &PathBuf) -> std::io::Result<()> {
    let mut tracer = Tracer::new();
    let (w, h) = (5, 5);
    let mut r = tracer.create_new("in", w, h);
    for y in 0..h {
        for x in 0..w {
            r.set(x, y, 20 * (x + y) as u8);
            let _ = r.get(y, x);
        }
    }

    let frames = replay(&r, 10);
    animation_rgb(&frames, 100, dir.join("trace.gif"))?;

    for a in &r.trace {
        println!("ACTION {:?}", a);
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let opts = Opts::from_args();

    //write_some_example_images(&opts.output_dir)?;
    //basic_trace(&opts.output_dir)?;

    let mut i = GrayImage::new(5, 5);
    for y in 0..i.height() {
        for x in 0..i.width() {
            i[[x, y]] = 10 * (x % 10 + y % 10) as u8;
        }
    }

    trace_blur3_inline(&opts.output_dir, &mut Tracer::new(), &i)?;
    trace_blur3_intermediate(&opts.output_dir, &mut Tracer::new(), &i)?;

    Ok(())
}