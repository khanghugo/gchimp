use image::{DynamicImage, GenericImage, ImageBuffer, Rgba};

#[inline]
/// Unpacks 565 from a u16 into scaled 888 where each channel is u32
pub fn unpack_rgb565(c: u16) -> [u32; 3] {
    [
        ((c >> 11) as u32 * 256 / 32),
        (((c << 5) >> 10) as u32 * 256 / 64),
        (((c << 11) >> 11) as u32 * 256 / 32),
    ]
}

#[inline]
pub fn pack_rgb888(c: [u32; 3]) -> [u8; 3] {
    [
        c[0].clamp(0, 255) as u8,
        c[1].clamp(0, 255) as u8,
        c[2].clamp(0, 255) as u8,
    ]
}

pub fn rgb8_buffer_to_image(buf: &[[u8; 3]], width: u32, height: u32) -> DynamicImage {
    let mut res = DynamicImage::new_rgb8(width, height);

    for x in 0..width {
        for y in 0..height {
            let curr_pixel = buf[(y * width + x) as usize];
            let converted = [curr_pixel[0], curr_pixel[1], curr_pixel[2], 255];

            res.put_pixel(x, y, Rgba(converted));
        }
    }

    res
}

pub fn rgba8_buffer_to_image(buf: &[[u8; 4]], width: u32, height: u32) -> DynamicImage {
    let mut res = DynamicImage::new_rgba8(width, height);

    for x in 0..width {
        for y in 0..height {
            let curr_pixel = buf[(y * width + x) as usize];
            let converted = [curr_pixel[0], curr_pixel[1], curr_pixel[2], curr_pixel[3]];

            res.put_pixel(x, y, Rgba(converted));
        }
    }

    res
}
