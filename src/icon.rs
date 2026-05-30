use windows_sys::Win32::{
    Foundation::HINSTANCE,
    UI::WindowsAndMessaging::{CreateIcon, HICON},
};

const SIZE: usize = 32;

pub fn create_app_icon() -> HICON {
    let and_mask = [0u8; SIZE * SIZE / 8];
    let xor_bits = icon_pixels_bgra_bottom_up();
    unsafe {
        CreateIcon(
            std::ptr::null_mut::<std::ffi::c_void>() as HINSTANCE,
            SIZE as i32,
            SIZE as i32,
            1,
            32,
            and_mask.as_ptr(),
            xor_bits.as_ptr(),
        )
    }
}

fn icon_pixels_bgra_bottom_up() -> Vec<u8> {
    let mut pixels = vec![0u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let top_y = SIZE - 1 - y;
            let offset = (y * SIZE + x) * 4;
            let in_margin = (3..=28).contains(&x) && (3..=28).contains(&top_y);
            let in_h_left = (8..=11).contains(&x) && (8..=23).contains(&top_y);
            let in_h_right = (20..=23).contains(&x) && (8..=23).contains(&top_y);
            let in_h_bar = (8..=23).contains(&x) && (14..=17).contains(&top_y);

            let (b, g, r, a) = if in_h_left || in_h_right || in_h_bar {
                (255, 255, 255, 255)
            } else if in_margin {
                (24, 24, 220, 255)
            } else {
                (0, 0, 0, 0)
            };

            pixels[offset] = b;
            pixels[offset + 1] = g;
            pixels[offset + 2] = r;
            pixels[offset + 3] = a;
        }
    }
    pixels
}
