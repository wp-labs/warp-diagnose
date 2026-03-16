use std::io::Cursor;

use objc2::rc::Allocated;
use objc2::{msg_send, ClassType, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::NSData;

#[derive(Clone, Copy)]
struct Rgba(u8, u8, u8, u8);

pub fn install_app_icon() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let png_bytes = render_icon_png(512);
    let png_data = NSData::with_bytes(&png_bytes);
    let icon_alloc: Allocated<NSImage> = unsafe { msg_send![NSImage::class(), alloc] };
    let Some(icon_image) = NSImage::initWithData(icon_alloc, &png_data) else {
        return;
    };

    unsafe {
        NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&icon_image));
    }
}

fn render_icon_png(size: u32) -> Vec<u8> {
    let mut pixels = vec![0_u8; (size * size * 4) as usize];

    fill_rounded_rect(
        &mut pixels,
        size,
        0,
        0,
        size as i32,
        size as i32,
        (size as f32 * 0.22) as i32,
        Rgba(255, 255, 255, 255),
    );
    fill_rounded_rect(
        &mut pixels,
        size,
        (size as f32 * 0.09) as i32,
        (size as f32 * 0.09) as i32,
        (size as f32 * 0.82) as i32,
        (size as f32 * 0.82) as i32,
        (size as f32 * 0.16) as i32,
        Rgba(244, 248, 252, 255),
    );

    let line_x = (size as f32 * 0.22) as i32;
    fill_rect(
        &mut pixels,
        size,
        line_x,
        (size as f32 * 0.12) as i32,
        (size as f32 * 0.04) as i32,
        (size as f32 * 0.76) as i32,
        Rgba(143, 176, 216, 255),
    );

    for y_pct in [0.25_f32, 0.5_f32, 0.75_f32] {
        fill_rect(
            &mut pixels,
            size,
            (size as f32 * 0.28) as i32,
            (size as f32 * y_pct) as i32 - (size as f32 * 0.012) as i32,
            (size as f32 * 0.5) as i32,
            (size as f32 * 0.024) as i32,
            Rgba(212, 221, 232, 255),
        );
    }

    fill_circle(
        &mut pixels,
        size,
        (size as f32 * 0.37) as i32,
        (size as f32 * 0.3) as i32,
        (size as f32 * 0.095) as i32,
        Rgba(107, 155, 74, 255),
    );
    fill_circle(
        &mut pixels,
        size,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.16) as i32,
        Rgba(255, 255, 255, 255),
    );
    stroke_circle(
        &mut pixels,
        size,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.16) as i32,
        (size as f32 * 0.032) as i32,
        Rgba(64, 85, 111, 255),
    );
    fill_circle(
        &mut pixels,
        size,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.095) as i32,
        Rgba(198, 138, 36, 255),
    );
    fill_circle(
        &mut pixels,
        size,
        (size as f32 * 0.72) as i32,
        (size as f32 * 0.68) as i32,
        (size as f32 * 0.095) as i32,
        Rgba(181, 70, 74, 255),
    );

    fill_rect(
        &mut pixels,
        size,
        (size as f32 * 0.58) as i32,
        (size as f32 * 0.56) as i32,
        (size as f32 * 0.13) as i32,
        (size as f32 * 0.03) as i32,
        Rgba(64, 85, 111, 255),
    );
    fill_rect(
        &mut pixels,
        size,
        (size as f32 * 0.67) as i32,
        (size as f32 * 0.56) as i32,
        (size as f32 * 0.03) as i32,
        (size as f32 * 0.13) as i32,
        Rgba(64, 85, 111, 255),
    );

    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut png_bytes), size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("png header");
        writer.write_image_data(&pixels).expect("png body");
    }

    png_bytes
}

fn fill_rect(
    pixels: &mut [u8],
    canvas: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: Rgba,
) {
    for py in y.max(0)..(y + height).min(canvas as i32) {
        for px in x.max(0)..(x + width).min(canvas as i32) {
            set_pixel(pixels, canvas, px as u32, py as u32, color);
        }
    }
}

fn fill_circle(
    pixels: &mut [u8],
    canvas: u32,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Rgba,
) {
    let r2 = radius * radius;
    for py in (cy - radius).max(0)..(cy + radius).min(canvas as i32) {
        for px in (cx - radius).max(0)..(cx + radius).min(canvas as i32) {
            let dx = px - cx;
            let dy = py - cy;
            if dx * dx + dy * dy <= r2 {
                set_pixel(pixels, canvas, px as u32, py as u32, color);
            }
        }
    }
}

fn stroke_circle(
    pixels: &mut [u8],
    canvas: u32,
    cx: i32,
    cy: i32,
    radius: i32,
    thickness: i32,
    color: Rgba,
) {
    let outer = radius * radius;
    let inner_radius = (radius - thickness).max(0);
    let inner = inner_radius * inner_radius;
    for py in (cy - radius).max(0)..(cy + radius).min(canvas as i32) {
        for px in (cx - radius).max(0)..(cx + radius).min(canvas as i32) {
            let dx = px - cx;
            let dy = py - cy;
            let dist = dx * dx + dy * dy;
            if dist <= outer && dist >= inner {
                set_pixel(pixels, canvas, px as u32, py as u32, color);
            }
        }
    }
}

fn fill_rounded_rect(
    pixels: &mut [u8],
    canvas: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    radius: i32,
    color: Rgba,
) {
    for py in y.max(0)..(y + height).min(canvas as i32) {
        for px in x.max(0)..(x + width).min(canvas as i32) {
            let local_x = px - x;
            let local_y = py - y;
            let nearest_x = if local_x < radius {
                radius
            } else if local_x >= width - radius {
                width - radius - 1
            } else {
                local_x
            };
            let nearest_y = if local_y < radius {
                radius
            } else if local_y >= height - radius {
                height - radius - 1
            } else {
                local_y
            };
            let dx = local_x - nearest_x;
            let dy = local_y - nearest_y;
            if dx * dx + dy * dy <= radius * radius {
                set_pixel(pixels, canvas, px as u32, py as u32, color);
            }
        }
    }
}

fn set_pixel(pixels: &mut [u8], canvas: u32, x: u32, y: u32, color: Rgba) {
    let idx = ((y * canvas + x) * 4) as usize;
    pixels[idx] = color.0;
    pixels[idx + 1] = color.1;
    pixels[idx + 2] = color.2;
    pixels[idx + 3] = color.3;
}
