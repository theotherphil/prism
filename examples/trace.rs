
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use structopt::StructOpt;
use std::rc::Rc;
use std::cell::RefCell;
use prism::*;

#[derive(StructOpt, Debug)]
struct Opts {
    /// Example files are written to this directory.
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_dir: PathBuf
}

fn trace_blur3_inline(storage: &mut Tracer, image: Rc<RefCell<TraceImage>>) -> Rc<RefCell<TraceImage>> {
    let image = image.borrow_mut();
    let (w, h) = image.dimensions();
    let result_ref = storage.create_image(w, h);

    {
        let mut result = result_ref.borrow_mut();

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let t = (image.get(x - 1, y - 1) as u16 + image.get(x, y - 1) as u16 + image.get(x + 1, y - 1) as u16) / 3;
                let m = (image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3;
                let b = (image.get(x - 1, y + 1) as u16 + image.get(x, y + 1) as u16 + image.get(x + 1, y + 1) as u16) / 3;
                let p = (t + m + b) / 3;
                result.set(x, y, p as u8);
            }
        }
    }

    result_ref
}

fn trace_blur3_intermediate(storage: &mut Tracer, image: Rc<RefCell<TraceImage>>) -> Rc<RefCell<TraceImage>> {
    let image = image.borrow_mut();
    let (w, h) = image.dimensions();

    let hblur_ref = storage.create_image(w, h);
    let vblur_ref = storage.create_image(w, h);

    {
        let mut hblur = hblur_ref.borrow_mut();
        let mut vblur = vblur_ref.borrow_mut();

        for y in 0..h {
            for x in 1..w - 1 {
                hblur.set(x, y, ((image.get(x - 1, y) as u16 + image.get(x, y) as u16 + image.get(x + 1, y) as u16) / 3) as u8);
            }
        }
        
        for y in 1..h - 1 {
            for x in 0..w {
                vblur.set(x, y, ((hblur.get(x, y - 1) as u16 + hblur.get(x, y) as u16 + hblur.get(x, y + 1) as u16) / 3) as u8);
            }
        }
    }

    vblur_ref
}

fn trace_blur3_stripped(storage: &mut Tracer, image: Rc<RefCell<TraceImage>>) -> Rc<RefCell<TraceImage>> {
    let image = image.borrow_mut();
    let strip_height = 2;

    assert!(image.height() % strip_height == 0);
    let buffer_height = strip_height + 2;

    let (w, h) = image.dimensions();

    let strip_ref = storage.create_image(w, buffer_height);
    let v_ref = storage.create_image(w, h);

    {
        let mut v = v_ref.borrow_mut();
        let mut strip = strip_ref.borrow_mut();

        for y_outer in 0..h / strip_height {
            let y_offset = y_outer * strip_height;

            strip.clear();

            for y_buffer in 0..buffer_height {
                if y_buffer + y_offset == 0 || y_buffer + y_offset > h {
                    continue;
                }
                let y_image = y_buffer + y_offset - 1;
                for x in 1..w - 1 {
                    let p = (
                        image.get(x - 1, y_image) as u16
                        + image.get(x, y_image) as u16
                        + image.get(x + 1, y_image) as u16
                        ) / 3;
                    strip.set(x, y_buffer, p as u8);
                }
            }

            for y_inner in 0..strip_height {
                if y_inner + y_offset == 0 || y_inner + y_offset == h - 1 {
                    continue;
                }
                for x in 0..w {
                    let y_buffer = y_inner + 1;
                    let p = (
                        strip.get(x, y_buffer - 1) as u16
                        + strip.get(x, y_buffer) as u16
                        + strip.get(x, y_buffer + 1) as u16
                        ) / 3;
                    v.set(x, y_inner + y_offset, p as u8);
                }
            }
        }
    }

    v_ref
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
            let _ = trace_blur3_inline(&mut t, i);
        }
        replays.push(create_replay_image(dir, "inline", &t.images())?);
    }
    {
        let mut t = Tracer::new();
        {
            let i = create_gradient_image(5, 6);
            let i = t.create_from_image(&i);
            let _ = trace_blur3_intermediate(&mut t, i);
        }
        replays.push(create_replay_image(dir, "intermediate", &t.images())?);
    }
    {
        let mut t = Tracer::new();
        {
            let i = create_gradient_image(5, 6);
            let i = t.create_from_image(&i);
            let _ = trace_blur3_stripped(&mut t, i);
        }
        replays.push(create_replay_image(dir, "stripped", &t.images())?);
    }

    write_html_page(dir, "traces.html", &replays)?;

    Ok(())
}