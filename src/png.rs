
use crate::GrayImage;
use png::HasParameters;
use std::{
    fs::File,
    io::{BufWriter, Result},
    path::Path
};

pub fn save_to_png<I: AsRef<Path>>(image: &GrayImage, i: I) -> Result<()> {
    let path = i.as_ref();
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, image.width() as u32, image.height() as u32);
    encoder.set(png::ColorType::Grayscale).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    writer.write_image_data(&image.buffer)?;

    Ok(())
}