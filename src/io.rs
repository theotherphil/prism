
use crate::{GrayImage, RgbImage};
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
    let mut rgb = flatten(&gray_to_rgb(image).buffer);
    let frame = gif::Frame::from_rgb(image.width() as u16, image.height() as u16, &mut *rgb);
    let mut file = File::create(i.as_ref())?;
    let mut encoder = gif::Encoder::new(&mut file, frame.width, frame.height, &[])?;
    encoder.write_frame(&frame)?;
    Ok(())
}

pub fn gray_to_rgb(image: &GrayImage) -> RgbImage {
    RgbImage {
        width: image.width(),
        height: image.height(),
        buffer: image.buffer.iter().map(|p| [*p, *p, *p]).collect()
    }
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
        let mut pixels = flatten(&gray_to_rgb(&image).buffer);
        let mut frame = gif::Frame::from_rgb(w, h, &mut *pixels);
        frame.delay = delay_in_ms / 10;
        encoder.write_frame(&frame)?;
    }

    Ok(())
}

pub fn animation_rgb<I: AsRef<Path>>(images: &[RgbImage], delay_in_ms: u16, i: I) -> Result<()> {
    use gif::SetParameter;

    // Lazily assuming all images are the same size
    assert!(!images.is_empty());

    let mut file = File::create(i.as_ref())?;
    let (w, h) = (images[0].width as u16, images[0].height as u16);
    let mut encoder = gif::Encoder::new(&mut file, w, h, &[])?;
    encoder.set(gif::Repeat::Infinite)?;

    for image in images {
        let mut pixels = flatten(&image.buffer);
        // This call accounts for nearly all the time when creating tracing examples,
        // and the rest of the code is already extremely slow.
        // write_trace_animation avoids this by assuming a restricted range of input
        // values and using a global palette 
        let mut frame = gif::Frame::from_rgb(w, h, &mut *pixels);
        frame.delay = delay_in_ms / 10;
        encoder.write_frame(&frame)?;
    }

    Ok(())
}

// Like animation_rgb, except that we assume the images are the outputs from tracing and so
// only use the grayscale values 0 to 253, pure red and pure green
pub fn write_trace_animation<I: AsRef<Path>>(images: &[RgbImage], delay_in_ms: u16, i: I) -> Result<()> {
    use gif::SetParameter;

    // Lazily assuming all images are the same size
    assert!(!images.is_empty());

    let mut global_palette = vec![];
    for i in 0..254u8 {
        global_palette.extend([i, i, i].iter().cloned());
    }
    global_palette.extend([255, 0, 0].iter().cloned());
    global_palette.extend([0, 255, 0].iter().cloned());

    let mut file = File::create(i.as_ref())?;
    let (w, h) = (images[0].width as u16, images[0].height as u16);
    let mut encoder = gif::Encoder::new(&mut file, w, h, &global_palette)?;
    encoder.set(gif::Repeat::Infinite)?;

    for image in images {
        let mut pixels = Vec::with_capacity(image.width() * image.height());
        for p in &image.buffer {
            if p[0] == p[1] && p[1] == p[2] && p[0] < 254 {
                pixels.push(p[0]);
            }
            else if *p == [255u8, 0, 0] {
                pixels.push(254);
            }
            else if *p == [0, 255u8, 0] {
                pixels.push(255);
            }
            else {
                panic!("Invalid trace image RGB value {:?}", p);
            }
        }
        let mut frame = gif::Frame::from_indexed_pixels(w, h, &pixels, None);
        frame.delay = delay_in_ms / 10;
        encoder.write_frame(&frame)?;
    }

    Ok(())
}