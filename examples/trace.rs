
use std::path::PathBuf;
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
    // TODO: !
}

impl Tracer {
    fn add_trace(image: &TraceImage, name: &'static str) {
        // TODO!
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Action {
    Read(usize, usize),
    Write(usize, usize, u8)
}

struct TraceImage {
    initial_image: GrayImage,
    current_image: GrayImage,
    trace: Vec<Action>
}

impl TraceImage {
    fn new(tracer: &Tracer, width: usize, height: usize) -> TraceImage {
        Self::from_image(tracer, &GrayImage::new(width, height))
    }

    fn from_image(trace: &Tracer, image: &GrayImage) -> TraceImage {
        TraceImage {
            initial_image: image.clone(),
            current_image: image.clone(),
            trace: vec![]
        }
    }

    fn get(&mut self, x: usize, y: usize) -> u8 {
        self.trace.push(Action::Read(x, y)); // reading requires mutable access... use a RefCell?
        self.current_image[[x, y]]
    }

    fn set(&mut self, x: usize, y: usize, c: u8) {
        self.trace.push(Action::Write(x, y, c));
        self.current_image[[x, y]] = c;
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

// replay needs to be able to highlight pixels that have been changed
// we should do this with colour, but for now we'll just use a distinguishing pattern
// upscaling should also be customisable
fn replay(image: &TraceImage, scale_factor: u8) -> Vec<RgbImage> {
    let mut current_image = gray_to_rgb(&image.initial_image);

    let mut frames = vec![];
    frames.push(upscale(&current_image, scale_factor));

    for action in &image.trace {
        match action {
            Action::Read(x, y) => { 
                let current = current_image[[*x, *y]];
                current_image[[*x, *y]] = [0, 255, 0];
                frames.push(upscale(&current_image, scale_factor));

                current_image[[*x, *y]] = current;
                frames.push(upscale(&current_image, scale_factor));
            },
            Action::Write(x, y, c) => {
                current_image[[*x, *y]] = [255, 0, 0];
                frames.push(upscale(&current_image, scale_factor));

                current_image[[*x, *y]] = [*c, *c, *c];
                frames.push(upscale(&current_image, scale_factor));
            }
        }
    }

    frames
}

fn main() -> std::io::Result<()> {
    let opts = Opts::from_args();

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

    save_to_png(&i, &opts.output_dir.join("grad.png"))?;
    save_to_gif(&i, &opts.output_dir.join("grad.gif"))?;
    save_to_gif(&f, &opts.output_dir.join("grad_flip.gif"))?;
    animation(&[i, f], 300, &opts.output_dir.join("animation.gif"))?;

    let tracer = Tracer {};
    let (w, h) = (5, 5);
    let mut r = TraceImage::new(&tracer, w, h);
    for y in 0..h {
        for x in 0..w {
            r.set(x, y, 20 * (x + y) as u8);
            let _ = r.get(y, x);
        }
    }

    let frames = replay(&r, 10);
    animation_rgb(&frames, 100, &opts.output_dir.join("trace.gif"))?;

    Ok(())
}