use std::{fs::File, path::PathBuf};

use clap::Parser;
use image::codecs::gif::GifEncoder;

#[derive(Debug, clap::Parser)]
struct Args {
    text: String,

    #[clap(short, long, default_value = "output.gif")]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("generating animation...");
    let frames = sparkler::render(&args.text)?;
    println!("rendering gif...");
    let mut encoder = GifEncoder::new_with_speed(File::create(args.output)?, 10);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;
    encoder.encode_frames(frames)?;
    println!("done!");
    Ok(())
}
