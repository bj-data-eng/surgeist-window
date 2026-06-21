use super::Result;

pub trait Clipboard {
    fn read_text(&mut self) -> Result<Option<String>>;
    fn write_text(&mut self, text: &str) -> Result<()>;
    fn read_image(&mut self) -> Result<Option<ClipboardImage>>;
    fn write_image(&mut self, image: ClipboardImageRef<'_>) -> Result<()>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClipboardImageRef<'a> {
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}

#[derive(Clone, Debug, Default)]
pub struct MemoryClipboard {
    text: Option<String>,
    image: Option<ClipboardImage>,
}

impl MemoryClipboard {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Clipboard for MemoryClipboard {
    fn read_text(&mut self) -> Result<Option<String>> {
        Ok(self.text.clone())
    }

    fn write_text(&mut self, text: &str) -> Result<()> {
        self.text = Some(text.to_owned());
        Ok(())
    }

    fn read_image(&mut self) -> Result<Option<ClipboardImage>> {
        Ok(self.image.clone())
    }

    fn write_image(&mut self, image: ClipboardImageRef<'_>) -> Result<()> {
        self.image = Some(ClipboardImage {
            width: image.width,
            height: image.height,
            rgba: image.rgba.to_vec(),
        });
        Ok(())
    }
}
