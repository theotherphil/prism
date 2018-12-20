
use crate::GrayImage;
use std::{
    fs::File,
    io::{BufWriter, Result},
    path::Path
};

pub fn save_to_png<I: AsRef<Path>>(image: &GrayImage, i: I) -> Result<()> {
    use png::HasParameters;

    let file = File::create(i.as_ref())?;
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, image.width() as u32, image.height() as u32);
    encoder.set(png::ColorType::Grayscale).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.buffer)?;
    Ok(())
}

pub fn save_to_gif<I: AsRef<Path>>(image: &GrayImage, i: I) -> Result<()> {
    let mut pixels = gray_to_rgb(&image.buffer);
    let frame = gif::Frame::from_rgb(image.width() as u16, image.height() as u16, &mut *pixels);
    let mut file = File::create(i.as_ref())?;
    let mut encoder = gif::Encoder::new(&mut file, frame.width, frame.height, &[])?;
    encoder.write_frame(&frame)?;
    Ok(())
}

fn gray_to_rgb(image: &[u8]) -> Vec<u8> {
    let mut pixels = vec![];
    for p in image {
        for _ in 0..3 {
            pixels.push(*p);
        }
    }
    pixels
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
        let mut pixels = gray_to_rgb(&image.buffer);
        let mut frame = gif::Frame::from_rgb(w, h, &mut *pixels);
        frame.delay = delay_in_ms / 10;
        encoder.write_frame(&frame)?;
    }

    Ok(())
}