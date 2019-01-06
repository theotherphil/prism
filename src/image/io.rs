
use crate::{GrayImage, RgbImage, Image};
use std::{
    fs::File,
    io::{BufWriter, Result},
    path::Path
};

pub fn load_from_png<I: AsRef<Path>>(i: I) -> Result<GrayImage> {
    let decoder = png::Decoder::new(File::open(i.as_ref())?);
    let (info, mut reader) = decoder.read_info().unwrap();
    let mut buf = vec![0; info.buffer_size()];
    reader.next_frame(&mut buf).unwrap();
    Ok(GrayImage::from_raw(info.width as usize, info.height as usize, buf))
}

pub fn save_to_png<I: AsRef<Path>>(image: &GrayImage, i: I) -> Result<()> {
    use png::HasParameters;

    let file = File::create(i.as_ref())?;
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, image.width() as u32, image.height() as u32);
    encoder.set(png::ColorType::Grayscale).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.data())?;
    Ok(())
}

pub fn save_to_gif<I: AsRef<Path>>(image: &GrayImage, i: I) -> Result<()> {
    let mut rgb = flatten(&gray_to_rgb(image).data());
    let frame = gif::Frame::from_rgb(image.width() as u16, image.height() as u16, &mut *rgb);
    let mut file = File::create(i.as_ref())?;
    let mut encoder = gif::Encoder::new(&mut file, frame.width, frame.height, &[])?;
    encoder.write_frame(&frame)?;
    Ok(())
}

pub fn gray_to_rgb(image: &GrayImage) -> RgbImage {
    RgbImage::from_raw(
        image.width(),
        image.height(),
        image.data().iter().map(|p| [*p, *p, *p]).collect()
    )
}

fn flatten(buffer: &[[u8; 3]]) -> Vec<u8> {
    let mut flat = vec![];
    for e in buffer {
        flat.push(e[0]);
        flat.push(e[1]);
        flat.push(e[2]);
    }
    flat
}

pub fn animation<I: AsRef<Path>>(images: &[GrayImage], delay_in_ms: u16, i: I) -> Result<()> {
    use gif::SetParameter;

    // Lazily assuming all images are the same size
    assert!(!images.is_empty());

    let mut file = File::create(i.as_ref())?;
    let (w, h) = (images[0].width() as u16, images[0].height() as u16);
    let mut encoder = gif::Encoder::new(&mut file, w, h, &[])?;
    encoder.set(gif::Repeat::Infinite)?;

    for image in images {
        let mut pixels = flatten(&gray_to_rgb(&image).data());
        let mut frame = gif::Frame::from_rgb(w, h, &mut *pixels);
        frame.delay = delay_in_ms / 10;
        encoder.write_frame(&frame)?;
    }

    Ok(())
}

pub struct GifPalette {
    /// 256 RGB values
    palette: Vec<u8>,
    /// Converts raw RGB values into an index into
    /// palette
    index_calculator: Box<dyn Fn([u8; 3]) -> u8>,
}

impl GifPalette {
    pub fn new(palette: &[u8], index_calculator: Box<dyn Fn([u8; 3]) -> u8>) -> GifPalette {
        GifPalette {
            palette: palette.iter().cloned().collect(),
            index_calculator: index_calculator
        }
    }
}

pub fn animation_rgb<I: AsRef<Path>>(
    images: &[RgbImage],
    delay_in_ms: u16,
    global_palette: Option<&GifPalette>,
    i: I
) -> Result<()> {
    use gif::SetParameter;

    // Lazily assuming all images are the same size
    assert!(!images.is_empty());

    let mut file = File::create(i.as_ref())?;
    let (w, h) = (images[0].width() as u16, images[0].height() as u16);

    let mut encoder = if let Some(palette) = global_palette {
        gif::Encoder::new(&mut file, w, h, &palette.palette)?
    } else {
        gif::Encoder::new(&mut file, w, h, &[])?
    };
    encoder.set(gif::Repeat::Infinite)?;

    for image in images {
        let mut frame = if let Some(ref palette) = global_palette {
            let mut pixels = Vec::with_capacity(image.width() * image.height());
            for p in image.data() {
                pixels.push((palette.index_calculator)(*p));
            }
            gif::Frame::from_indexed_pixels(w, h, &pixels, None)

        } else {
            let mut pixels = flatten(&image.data());
            // Frame::from_rgb is _extremely_ slow.
            // Use a global palette where possible
            gif::Frame::from_rgb(w, h, &mut *pixels)
        };

        frame.delay = delay_in_ms / 10;
        encoder.write_frame(&frame)?;
    }

    Ok(())
}
