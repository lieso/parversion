pub struct ColorPalette {
    primary_hex: String,
    secondary_hex: String,
    background_hex: String,
}

pub struct Observation {
    color_palette: ColorPalette,
}

pub fn html_to_color_palette(html: String) -> ColorPalette {
    log::trace!("In html_to_color_palette");
    unimplemented!();
}
