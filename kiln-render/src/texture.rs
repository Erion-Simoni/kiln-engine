//! GPU textures and atlas UV helpers for 2D sprites.
//!
//! Textures are uploaded as raw RGBA8 (no `image` crate yet — PNG lands with
//! the asset→GPU step). [`TextureId::WHITE`] is a 1×1 white texel for
//! colour-only quads.

use wgpu::{Device, Queue};

/// Handle into a [`crate::SpriteRenderer`]'s texture list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);

impl TextureId {
    /// Built-in 1×1 white texture (tint with sprite colour).
    pub const WHITE: TextureId = TextureId(0);
}

/// Full texture UV rect (u0, v0, u1, v1).
pub const FULL_UV: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// Convert a pixel rect inside an atlas to normalised UVs.
pub fn uv_rect_pixels(atlas_w: u32, atlas_h: u32, x: u32, y: u32, w: u32, h: u32) -> [f32; 4] {
    let aw = atlas_w.max(1) as f32;
    let ah = atlas_h.max(1) as f32;
    [
        x as f32 / aw,
        y as f32 / ah,
        (x + w) as f32 / aw,
        (y + h) as f32 / ah,
    ]
}

/// Build a row-major grid atlas as RGBA8 bytes plus per-cell UVs.
///
/// `fill_cell(col, row, pixels)` writes `cell_w * cell_h * 4` bytes.
pub fn build_grid_atlas_rgba(
    cell_w: u32,
    cell_h: u32,
    cols: u32,
    rows: u32,
    mut fill_cell: impl FnMut(u32, u32, &mut [u8]),
) -> (Vec<u8>, u32, u32, Vec<[f32; 4]>) {
    let atlas_w = cell_w * cols;
    let atlas_h = cell_h * rows;
    let mut pixels = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    let mut uvs = Vec::with_capacity((cols * rows) as usize);

    for row in 0..rows {
        for col in 0..cols {
            let mut cell = vec![0u8; (cell_w * cell_h * 4) as usize];
            fill_cell(col, row, &mut cell);
            let ox = col * cell_w;
            let oy = row * cell_h;
            for cy in 0..cell_h {
                let src = ((cy * cell_w) * 4) as usize;
                let dst = (((oy + cy) * atlas_w + ox) * 4) as usize;
                let len = (cell_w * 4) as usize;
                pixels[dst..dst + len].copy_from_slice(&cell[src..src + len]);
            }
            uvs.push(uv_rect_pixels(atlas_w, atlas_h, ox, oy, cell_w, cell_h));
        }
    }

    (pixels, atlas_w, atlas_h, uvs)
}

/// Upload RGBA8 bytes as an RGBA8Unorm texture (sampled + copy-dst).
pub fn create_rgba8_texture(
    device: &Device,
    queue: &Queue,
    label: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> (wgpu::Texture, wgpu::TextureView) {
    create_rgba8_texture_with_options(device, queue, label, [width, height], rgba, false)
}

/// Box-filtered RGBA levels, including the base, with complete odd-size coverage.
pub fn rgba_mip_chain(size: [u32; 2], rgba: &[u8], enabled: bool) -> Vec<([u32; 2], Vec<u8>)> {
    let [mut width, mut height] = size;
    assert!(width > 0 && height > 0);
    assert_eq!(rgba.len(), width as usize * height as usize * 4);
    let mut levels = vec![(size, rgba.to_vec())];
    while enabled && (width > 1 || height > 1) {
        let w = (width / 2).max(1);
        let h = (height / 2).max(1);
        let source = &levels.last().unwrap().1;
        let mut pixels = vec![0; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let mut sums = [0u32; 4];
                let mut count = 0;
                for sy in y * height / h..(y + 1) * height / h {
                    for sx in x * width / w..(x + 1) * width / w {
                        for channel in 0..4 {
                            sums[channel] +=
                                source[((sy * width + sx) * 4) as usize + channel] as u32;
                        }
                        count += 1;
                    }
                }
                for channel in 0..4 {
                    pixels[((y * w + x) * 4) as usize + channel] = (sums[channel] / count) as u8;
                }
            }
        }
        levels.push(([w, h], pixels));
        width = w;
        height = h;
    }
    levels
}

/// Upload all generated mip levels; sampling choice belongs to the bind group.
pub fn create_rgba8_texture_with_options(
    device: &Device,
    queue: &Queue,
    label: &str,
    size: [u32; 2],
    rgba: &[u8],
    mipmaps: bool,
) -> (wgpu::Texture, wgpu::TextureView) {
    let levels = rgba_mip_chain(size, rgba, mipmaps);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, ([width, height], pixels)) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(*height),
            },
            wgpu::Extent3d {
                width: *width,
                height: *height,
                depth_or_array_layers: 1,
            },
        );
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_mips_include_odd_edge_texels_and_preserve_constant_alpha() {
        let pixels: Vec<_> = (0..9u8).flat_map(|value| [value, 20, 80, 255]).collect();
        let levels = rgba_mip_chain([3, 3], &pixels, true);
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[1], ([1, 1], vec![4, 20, 80, 255]));
        assert_eq!(rgba_mip_chain([3, 3], &pixels, false).len(), 1);
    }

    #[test]
    fn uv_rect_full_texture() {
        assert_eq!(uv_rect_pixels(64, 32, 0, 0, 64, 32), FULL_UV);
    }

    #[test]
    fn uv_rect_quadrant() {
        let uv = uv_rect_pixels(2, 2, 1, 0, 1, 1);
        assert_eq!(uv, [0.5, 0.0, 1.0, 0.5]);
    }

    #[test]
    fn grid_atlas_two_by_one() {
        let (px, w, h, uvs) = build_grid_atlas_rgba(2, 2, 2, 1, |col, _row, cell| {
            let c = if col == 0 { 255 } else { 0 };
            for chunk in cell.chunks_exact_mut(4) {
                chunk.copy_from_slice(&[c, 0, 0, 255]);
            }
        });
        assert_eq!((w, h), (4, 2));
        assert_eq!(px.len(), 32);
        assert_eq!(uvs.len(), 2);
        assert_eq!(uvs[0], [0.0, 0.0, 0.5, 1.0]);
        assert_eq!(uvs[1], [0.5, 0.0, 1.0, 1.0]);
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);
        assert_eq!(&px[8..12], &[0, 0, 0, 255]);
    }
}
