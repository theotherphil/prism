
use std::path::PathBuf;
use std::rc::Rc;
use std::fs::File;
use std::io::Write;
use std::cell::Cell;
use structopt::StructOpt;
use prism::*;

#[derive(StructOpt, Debug)]
struct Opts {
    /// Example files are written to this directory.
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output_dir: PathBuf
}

/// Records the set of trace images, so that reads and writes can be ordered
/// across multiple images.
struct Tracer {
    count: Rc<Cell<usize>>
}

impl Tracer {
    fn new() -> Tracer {
        Tracer {
            count: Rc::new(Cell::new(0))
        }
    }

    fn create_new(&mut self, name: &'static str, width: usize, height: usize) -> TraceImage {
        TraceImage::new(self.count.clone(), width, height)
    }

    fn create_from_image(&mut self, name: &'static str, image: &GrayImage) -> TraceImage {
        TraceImage::from_image(self.count.clone(), image)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Action {
    Read(usize, (usize, usize)),
    Write(usize, (usize, usize), u8)
}

struct TraceImage {
    count: Rc<Cell<usize>>,
    initial_image: GrayImage,
    current_image: GrayImage,
    trace: Vec<Action>
}

impl TraceImage {
    fn new(count: Rc<Cell<usize>>, width: usize, height: usize) -> TraceImage {
        Self::from_image(count, &GrayImage::new(width, height))
    }

    fn from_image(count: Rc<Cell<usize>>, image: &GrayImage) -> TraceImage {
        TraceImage {
            count: count,
            initial_image: image.clone(),
            current_image: image.clone(),
            trace: vec![]
        }
    }

    fn get(&mut self, x: usize, y: usize) -> u8 {
        self.trace.push(Action::Read(self.incr_count(), (x, y))); // reading requires mutable access... use a RefCell?
        self.current_image[[x, y]]
    }

    fn set(&mut self, x: usize, y: usize, c: u8) {
        self.trace.push(Action::Write(self.incr_count(), (x, y), c));
        self.current_image[[x, y]] = c;
    }

    fn incr_count(&self) -> usize {
        (*self.count).set(self.count.get() + 1);
        self.count.get()
    }
}

fn upscale<T: Copy + Zero>(image: &Image<T>, factor: u8) -> Image<T> {
    let (w, h) = (factor as usize * image.width(), factor as usize * image.height());
    let mut result = Image::new(w, h);
    for y in 0..h {
        for x in 0..w {
            result[[x, y]] = image[[x / factor as usize, y / factor as usize]];
        }
    }
    result
}

// This needs to return counts in order to sync multiple images
// The flash for a read or write should occur on the count of the action
fn replay(image: &TraceImage, scale_factor: u8) -> Vec<RgbImage> {
    let mut current_image = gray_to_rgb(&image.initial_image);

    let mut frames = vec![];
    frames.push(upscale(&current_image, scale_factor));

    for action in &image.trace {
        match action {
            Action::Read(_, (x, y)) => { 
                let current = current_image[[*x, *y]];
                current_image[[*x, *y]] = [0, 255, 0];
                frames.push(upscale(&current_image, scale_factor));

                current_image[[*x, *y]] = current;
                frames.push(upscale(&current_image, scale_factor));
            },
            Action::Write(_, (x, y), c) => {
                current_image[[*x, *y]] = [255, 0, 0];
                frames.push(upscale(&current_image, scale_factor));

                current_image[[*x, *y]] = [*c, *c, *c];
                frames.push(upscale(&current_image, scale_factor));
            }
        }
    }

    frames
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

    animation_rgb(&replay(&image, 10), 100, &image_path)?;
    animation_rgb(&replay(&result, 10), 900, &result_path)?;

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

    animation_rgb(&replay(&image, 10), 200, &image_path)?;
    animation_rgb(&replay(&hblur, 10), 200, &h_path)?;
    animation_rgb(&replay(&vblur, 10), 200, &v_path)?;

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