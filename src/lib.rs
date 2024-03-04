use image::{
    codecs::gif::GifDecoder,
    imageops::{blur, invert, overlay},
    AnimationDecoder, Delay, Frame, Rgba, RgbaImage,
};
use imageproc::drawing::{draw_text, text_size};
use rusttype::{Font, Scale};

const PADDING: u16 = 24;
const FONT_SIZE: f32 = 64.0;
const WRAP_COL: usize = 15;
const NUM_SPARKLES: usize = 3;
const JELLEE_DATA: &[u8] = include_bytes!("../res/Jellee-Bold.otf");
pub const SPARKLES_DATA: &[u8] = include_bytes!("../res/sparkles.gif");

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to load font data.")]
    LoadFont,

    #[error("Not enough text.")]
    NotEnoughText,

    #[error(transparent)]
    Image(#[from] image::ImageError),
}

fn text_image<S>(lines: &[S], font: &Font) -> Result<RgbaImage, Error>
where
    S: AsRef<str>,
{
    let sizes = lines
        .iter()
        .map(|line| {
            let (w, y) = text_size(Scale::uniform(FONT_SIZE), font, line.as_ref());
            (w as u32, y as u32)
        })
        .collect::<Vec<_>>();
    let w = sizes
        .iter()
        .map(|(w, _)| *w)
        .max()
        .ok_or(Error::NotEnoughText)?;
    let h = sizes.iter().map(|(_, h)| *h).sum::<u32>();
    let mut img = RgbaImage::new(w + u32::from(PADDING) * 2, h + u32::from(PADDING) * 2);
    let mut row = i32::from(PADDING);
    for (line, (_, h)) in lines.iter().zip(sizes) {
        img = draw_text(
            &img,
            Rgba([0, 0, 0, 255]),
            i32::from(PADDING),
            row,
            Scale::uniform(FONT_SIZE),
            font,
            line.as_ref(),
        );
        row += h as i32;
    }
    Ok(img)
}

fn outline(img: &RgbaImage) -> RgbaImage {
    let mut bg = blur(img, 3.0);
    invert(&mut bg);
    overlay(&mut bg, img, 0, 0);
    bg
}

fn sparkle_frames() -> Result<Vec<Frame>, Error> {
    let decoder = GifDecoder::new(SPARKLES_DATA)?;
    let frames = decoder.into_frames().collect_frames()?;
    Ok(frames)
}

fn sparkle_positions(w: u32, h: u32) -> [(i64, i64); NUM_SPARKLES] {
    [
        (10, 10),
        (i64::from(w) - 42, i64::from(h) - 42),
        ((w / 3).into(), (2 * h / 3).into()),
    ]
}

const fn sparkle_phases(num_frames: usize) -> [usize; NUM_SPARKLES] {
    [0, num_frames / 3, num_frames - num_frames / 11]
}

/// Render a text string into a series of frames with sparkles.
///
/// # Errors
///
/// Returns an error if the font data cannot be loaded or if the text is empty.
pub fn render(text: &str) -> Result<Vec<Frame>, Error> {
    let font = Font::try_from_bytes(JELLEE_DATA).ok_or(Error::LoadFont)?;
    let lines = textwrap::wrap(text, WRAP_COL);
    let img = text_image(&lines, &font)?;
    let img = outline(&img);
    let sparkle_frames = sparkle_frames()?;
    let positions = sparkle_positions(img.width() - 32, img.height() - 32);
    let phases = sparkle_phases(sparkle_frames.len());
    let num_sparkles = if img.width() < 200 { 1 } else { NUM_SPARKLES };
    let frames = (0..sparkle_frames.len())
        .map(|i| {
            let mut frame = img.clone();
            for sparkle_idx in 0..num_sparkles {
                let (x, y) = positions[sparkle_idx];
                let phase = phases[sparkle_idx];
                let sparkle_frame = &sparkle_frames[(i + phase) % sparkle_frames.len()];
                overlay(&mut frame, sparkle_frame.buffer(), x, y);
            }
            Frame::from_parts(frame, 0, 0, Delay::from_numer_denom_ms(30, 1))
        })
        .collect();
    Ok(frames)
}
