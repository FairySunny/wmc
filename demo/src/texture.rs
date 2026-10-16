pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>
}

impl Image {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
        let img = image::load_from_memory(bytes)?;
        let rgba = img.to_rgba8();
        let dimensions = image::GenericImageView::dimensions(&img);
        Ok(Self {
            width: dimensions.0,
            height: dimensions.1,
            data: rgba.into_vec()
        })
    }
}
