
use crate::{GrayImage, RgbImage, Image};
use crate::tracer::compute_tint;
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

pub fn animation_rgb<I: AsRef<Path>>(images: &[RgbImage], delay_in_ms: u16, i: I) -> Result<()> {
    use gif::SetParameter;

    // Lazily assuming all images are the same size
    assert!(!images.is_empty());

    let mut file = File::create(i.as_ref())?;
    let (w, h) = (images[0].width() as u16, images[0].height() as u16);
    let mut encoder = gif::Encoder::new(&mut file, w, h, &[])?;
    encoder.set(gif::Repeat::Infinite)?;

    for image in images {
        let mut pixels = flatten(&image.data());
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
// TODO: This and the tracing code are tightly coupled - rewrite this to just support providing a
// TODO: global palette and have the tracer code be responsible for providing the palette
pub fn write_trace_animation<I: AsRef<Path>>(images: &[RgbImage], delay_in_ms: u16, i: I) -> Result<()> {
    use gif::SetParameter;

    // Lazily assuming all images are the same size
    assert!(!images.is_empty());

    let mut global_palette = vec![];
    // Greyscale pixels where each value has an even intensity no more than 250u8
    for i in 0..126u8 {
        global_palette.extend([2 * i, 2 * i, 2 * i].iter().cloned());
    }
    // Their blue-tinted equivalents
    for i in 0..126u8 {
        let tint = compute_tint(2 * i);
        global_palette.extend([2 * i, 2 * i, 2 * i + tint].iter().cloned());
    }
    // Red, green, blue, yellow
    global_palette.extend([255, 0, 0].iter().cloned());
    global_palette.extend([0, 255, 0].iter().cloned());
    global_palette.extend([0, 255, 255].iter().cloned());
    global_palette.extend([255, 255, 0].iter().cloned());

    let mut file = File::create(i.as_ref())?;
    let (w, h) = (images[0].width() as u16, images[0].height() as u16);
    let mut encoder = gif::Encoder::new(&mut file, w, h, &global_palette)?;
    encoder.set(gif::Repeat::Infinite)?;

    let compute_palette_index = |p: [u8; 3]| {
        if p == [255u8, 0, 0] {
            252
        }
        else if p == [0, 255u8, 0] {
            253
        }
        else if p == [0, 0, 255u8] {
            254
        }
        else if p == [255u8, 255u8, 0] {
            255
        }
        else if p[0] == p[1] && p[1] == p[2] && p[0] <= 250  {
            // Round down to even values in each channel
            p[0] / 2
        }
        else if p[0] == p[1] {
            // Check if this is a blue-tinted version of an accepted greyscale value
            let t = compute_tint(p[0]);
            let b = p[0] + t;
            if b == p[2] && p[0] <= 250 {
                p[0] / 2 + 126
            } else {
                panic!("Invalid trace image RGB value {:?}", p)    
            }
        }
        else {
            panic!("Invalid trace image RGB value {:?}", p)
        }
    };

    for image in images {
        let mut pixels = Vec::with_capacity(image.width() * image.height());
        for p in image.data() {
            pixels.push(compute_palette_index(*p));
        }
        let mut frame = gif::Frame::from_indexed_pixels(w, h, &pixels, None);
        frame.delay = delay_in_ms / 10;
        encoder.write_frame(&frame)?;
    }

    Ok(())
}