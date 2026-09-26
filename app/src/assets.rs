//! Embedded brand assets for GPUI `img("…")` and window chrome.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let data: Option<&'static [u8]> = match path {
            "brand/tinkerway-icon.jpg" | "brand/tinkerway-icon.jpeg" => {
                Some(include_bytes!("../assets/brand/tinkerway-icon.jpg"))
            }
            "brand/tinkerway-icon.png" => {
                Some(include_bytes!("../assets/brand/tinkerway-icon.png"))
            }
            "brand/tinkerway-header.jpg" | "brand/tinkerway-header.jpeg" => {
                Some(include_bytes!("../assets/brand/tinkerway-header.jpg"))
            }
            _ => None,
        };
        Ok(data.map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let path = path.trim_matches('/');
        if path.is_empty() || path == "brand" {
            Ok(vec![
                "tinkerway-icon.jpg".into(),
                "tinkerway-icon.png".into(),
                "tinkerway-header.jpg".into(),
            ])
        } else {
            Ok(Vec::new())
        }
    }
}
