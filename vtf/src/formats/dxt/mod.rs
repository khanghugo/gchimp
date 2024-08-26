mod dxt1;
mod dxt5;

use super::*;

pub use dxt1::Dxt1;
pub use dxt5::Dxt5;

use utils::{pack_rgb888, unpack_rgb565};

fn dxt_color_block_to_colors(block: &[u8]) -> [[u32; 3]; 4] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);

    let (cp0, cp1) = (unpack_rgb565(c0), unpack_rgb565(c1));

    let (cp2, cp3) = if c0 > c1 {
        (
            [
                (cp0[0] * 2 + cp1[0]) / 3,
                (cp0[1] * 2 + cp1[1]) / 3,
                (cp0[2] * 2 + cp1[2]) / 3,
            ],
            [
                (cp0[0] + cp1[0] * 2) / 3,
                (cp0[1] + cp1[1] * 2) / 3,
                (cp0[2] + cp1[2] * 2) / 3,
            ],
        )
    } else {
        (
            [
                (cp0[0] + cp1[0]) / 2,
                (cp0[1] + cp1[1]) / 2,
                (cp0[2] + cp1[2]) / 2,
            ],
            [0, 0, 0],
        )
    };

    [cp0, cp1, cp2, cp3]
}

fn dxt_color_block_to_color_pixels(block: &[u8]) -> Vec<[u8; 3]> {
    let colors = dxt_color_block_to_colors(block);

    let look_up = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);

    // 4x4
    (0..16)
        .map(|idx| {
            let shift_value = 30 - idx * 2;
            let lookup_value = (look_up << shift_value) >> 30;

            pack_rgb888(colors[lookup_value as usize])
        })
        .collect::<Vec<[u8; 3]>>()
}

fn dxt_alpha_block_to_alpha(block: &[u8]) -> [u8; 8] {
    let a0 = block[0] as u32;
    let a1 = block[1] as u32;

    let [a2, a3, a4, a5, a6, a7] = if a0 > a1 {
        [
            (6 * a0 + a1) / 7,
            (5 * a0 + 2 * a1) / 7,
            (4 * a0 + 3 * a1) / 7,
            (3 * a0 + 4 * a1) / 7,
            (2 * a0 + 5 * a1) / 7,
            (a0 + 6 * a1) / 7,
        ]
    } else {
        [
            (4 * a0 + a1) / 5,
            (3 * a0 + 2 * a1) / 5,
            (2 * a0 + 3 * a1) / 5,
            (a0 + 4 * a1) / 5,
            0,
            255,
        ]
    };

    [
        a0 as u8, a1 as u8, a2 as u8, a3 as u8, a4 as u8, a5 as u8, a6 as u8, a7 as u8,
    ]
}

fn dxt_alpha_block_to_alpha_pixels(block: &[u8]) -> Vec<u8> {
    let alphas = dxt_alpha_block_to_alpha(block);

    let look_up = u64::from_le_bytes([
        block[2], block[3], block[4], block[5], block[6], block[7], 0, 0,
    ]);

    // 4x4
    (0..16)
        .map(|idx| {
            let shift_value = 61 - idx * 3;
            let lookup_value = (look_up << shift_value) >> 61;

            alphas[lookup_value as usize]
        })
        .collect::<Vec<u8>>()
}

fn dxt5_block_to_pixels(block: &[u8]) -> Vec<[u8; 4]> {
    let alphas = dxt_alpha_block_to_alpha_pixels(block);
    let colors = dxt_color_block_to_color_pixels(&block[8..]);

    colors
        .into_iter()
        .zip(alphas)
        .map(|(colors, alpha)| [colors[0], colors[1], colors[2], alpha])
        .collect()
}
