use crate::types::{MipMap, MipTex, Palette, TextureName};

// gz deepseek
pub fn create_blue_miptex(width: u32, height: u32, name: &str) -> MipTex {
    // Create palette with blue at index 255 (last entry)
    let mut palette = vec![[0u8; 3]; 256];
    palette[255] = [0, 0, 255]; // RGB for blue

    // Create mip level 0 (full size) with all pixels pointing to blue
    let mip0_size = width * height;
    let mip0_data = vec![255u8; mip0_size as usize]; // All pixels use palette index 255

    // Create dummy mip levels (will be quarter sizes)
    let mip1_size = width * height / 4;
    let mip2_size = width * height / 16;
    let mip3_size = width * height / 64;

    // Calculate offsets (WAD3 header is 40 bytes)
    let mip_offsets: Vec<u32> = vec![
        40,                                     // Mip0 starts right after header
        40 + mip0_size,                         // Mip1 after mip0
        40 + mip0_size + mip1_size,             // Mip2 after mip1
        40 + mip0_size + mip1_size + mip2_size, // Mip3 after mip2
    ];

    // // Palette starts after all mip levels
    // let palette_offset = mip_offsets[3] + mip3_size;

    // Create texture name (16 bytes, null-padded)
    let mut texture_name = [0u8; 16];
    texture_name[..name.len().min(16)].copy_from_slice(&name.as_bytes()[..name.len().min(16)]);

    MipTex {
        texture_name: TextureName(texture_name.to_vec()),
        width,
        height,
        mip_offsets,
        mip_images: vec![
            MipMap::new(mip0_data),
            MipMap::new(vec![255; mip1_size as usize]),
            MipMap::new(vec![255; mip2_size as usize]),
            MipMap::new(vec![255; mip3_size as usize]),
        ],
        colors_used: 256,
        palette: Palette::new(palette),
    }
}
