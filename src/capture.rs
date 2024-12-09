#![allow(dead_code)]
pub mod capture {

    use egui::{pos2, ColorImage};
    use xcap::image::{RgbaImage};
    use xcap::{Window, XCapError};
    use xcap::Monitor;

    pub fn primary_monitor() -> Result<Monitor, XCapError> {
        match Monitor::all() {
            Ok(ms) => {
                for m in ms {
                    if m.is_primary() {
                        return Ok(m);
                    }
                }
                Err(XCapError::new("No primary monitor found"))
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn get_monitors() -> Vec<Monitor> {
        if let Ok(m) = Monitor::all() {
            m
        }
        else {
            Vec::new()
        }
    }

    pub fn capture(monitor: &Monitor) -> Result<RgbaImage, XCapError> {
        match monitor.capture_image() {
            Ok(img) => {
                Ok(img)
            }
            Err(e) => {
                println!("Failed to capture screenshot: {}", e);
                Err(e)
            }
        }
    }


    pub fn capture_window(window: &Window) -> Result<RgbaImage, XCapError> {
        match window.capture_image() {
            Ok(img) => {
                Ok(img)
            }
            Err(e) => {
                println!("Failed to capture screenshot of window {}: {}", window.title(), e);
                Err(e)
            }
        }
    }

    pub fn windows_list() -> Vec<String> {
        let windows = Window::all().unwrap().into_iter();
        let mut v = Vec::new();
        for w in windows {
            v.push(String::from(w.title()));
        }
        v
    }

    pub fn get_window(name: &str) -> Option<Window> {
        let list = Window::all();
        match list {
            Ok(list) => {
                list.into_iter().find(|w| {w.title() == name})
            }
            Err(e) => {
                eprintln!("{}", e);
                None
            }
        }
    }

    pub fn windows_pos() -> Option<egui::Pos2> {
        let windows = Window::all().unwrap().into_iter().nth(1);
        match windows {
            None => {
                None
            }
            Some(w) => {
                Some(pos2(w.x() as f32, w.y() as f32))
            }
        }


    }

    pub fn crop_color_image(img: &ColorImage, x: u32, y: u32, width: u32, height: u32) -> ColorImage {

        let mut cropped_pixels = Vec::with_capacity((width * height) as usize);

        for j in 0..height {
            for i in 0..width {
                let orig_x = x + i;
                let orig_y = y + j;

                let pixel_index = (orig_y * img.width() as u32 + orig_x) as usize;
                let pixel_slice = img.pixels[pixel_index];
                cropped_pixels.push(pixel_slice);
            }
        }


        ColorImage {
            size: [width as usize, height as usize],
            pixels: cropped_pixels,
        }
    }


    pub fn image_from_path(path: &str) -> Option<ColorImage>{
        if let Ok(image) = image::open(path) {
            let size = [image.width() as usize, image.height() as usize];
            let image_buffer = image.to_rgba8(); // Convert to RGBA8 format
            let pixels = image_buffer.as_flat_samples(); // Flatten the pixel data
            return Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
        }
        None
    }

}
