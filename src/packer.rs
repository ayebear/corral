use std::error::Error;
use std::fs;

use crate::tree2d::{DataSize, Tree2d};
use image::{DynamicImage, ImageEncoder};

pub struct Config {
    padding: u8,
    input_dir: String,
    output_file: String,
}

impl Config {
    pub fn parse(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("Too few arguments, call like: `corral input_dir output_sheet.png`");
        }

        let input_dir = args[1].clone();
        let output_file = args[2].clone();

        Ok(Config {
            padding: 2u8,
            input_dir,
            output_file,
        })
    }
}

struct NamedDynamicImage {
    name: String,
    img: DynamicImage,
}

struct ImageCollection {
    named_images: Vec<NamedDynamicImage>,
    max_width: u32,
    max_height: u32,
    num_images: u32,
}

impl ImageCollection {
    fn new(mut named_images: Vec<NamedDynamicImage>) -> ImageCollection {
        let mut max_width = 0u32;
        let mut max_height = 0u32;
        for NamedDynamicImage { name: _, img } in &named_images {
            max_width = max_width.max(img.width());
            max_height = max_height.max(img.height());
        }
        let num_images = (&named_images).len() as u32;

        named_images.sort_by(|a, b| {
            (b.img.width() * b.img.height()).cmp(&(a.img.width() * a.img.height()))
        });

        ImageCollection {
            named_images,
            max_width,
            max_height,
            num_images,
        }
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let img_collection = load_all(&config.input_dir)?;
    let img_packed = pack(config.padding, img_collection)?;
    write_img(&config.output_file, &img_packed)?;
    Ok(())
}

fn load_all(input_dir: &str) -> Result<ImageCollection, Box<dyn Error>> {
    let mut images = Vec::new();

    let paths = fs::read_dir(input_dir)?;

    for path in paths {
        let path = path?.path();
        if let (Some(path_str), Some(fname)) = (path.to_str(), path.file_name()) {
            images.push(NamedDynamicImage {
                name: fname.to_string_lossy().to_string(),
                img: image::io::Reader::open(path_str)?.decode()?,
            });
        }
    }

    Ok(ImageCollection::new(images))
}

fn pack(padding: u8, img_collection: ImageCollection) -> Result<DynamicImage, Box<dyn Error>> {
    // let height =
    //     (img_collection.max_height + padding as u32) * img_collection.num_images + padding as u32;
    // let width =
    //     (img_collection.max_width + padding as u32) * img_collection.num_images + padding as u32;

    let mut data = vec![];
    for NamedDynamicImage { img, .. } in img_collection.named_images.iter() {
        data.push((
            DataSize {
                width: img.width() + padding as u32,
                height: img.height() + padding as u32,
            },
            img,
        ));
    }
    let mut tree = Tree2d::<&DynamicImage>::new();
    tree.insert_all(data)?;
    let flattened = tree.flatten();
    let bb = tree.get_total_bounding_box();
    let mut img_packed =
        image::RgbaImage::new(bb.width + padding as u32, bb.height + padding as u32);
    for (img, bb) in flattened {
        image::imageops::replace(
            &mut img_packed,
            *img,
            bb.x as i64 + padding as i64,
            bb.y as i64 + padding as i64,
        );
    }

    Ok(DynamicImage::ImageRgba8(img_packed))
}

fn write_img(output_file: &str, img_packed: &DynamicImage) -> Result<(), Box<dyn Error>> {
    let buf = fs::File::create(&output_file)?;
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        buf,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::Adaptive,
    );

    encoder.write_image(
        img_packed.as_bytes(),
        img_packed.width(),
        img_packed.height(),
        img_packed.color(),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rect(w: u32, h: u32) -> image::DynamicImage {
        let mut img = image::RgbaImage::new(w, h);
        for i in 0..w {
            for j in 0..h {
                img.put_pixel(i, j, image::Rgba([255, 0, 0, 255]));
            }
        }
        image::DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn pack_one() -> Result<(), Box<dyn Error>> {
        let (w, h) = (1, 1);
        let padding = 1;
        let mut expected_output_img = image::RgbaImage::new(w + padding * 2, h + padding * 2);
        for i in 0..w + padding * 2 {
            for j in 0..h + padding * 2 {
                let color = if i < padding || j < padding {
                    image::Rgba([0, 0, 0, 0])
                } else if i >= padding + w || j >= padding + h {
                    image::Rgba([0, 0, 0, 0])
                } else {
                    image::Rgba([255, 0, 0, 255])
                };
                expected_output_img.put_pixel(i, j, color);
            }
        }

        let img_collection = ImageCollection::new(vec![NamedDynamicImage {
            name: "red_pixel".to_owned(),
            img: make_rect(w, h),
        }]);

        if let Some(img) = pack(padding as u8, img_collection)?.as_rgba8() {
            let p: Vec<&image::Rgba<u8>> = img.pixels().collect();
            let q: Vec<&image::Rgba<u8>> = expected_output_img.pixels().collect();
            assert_eq!(q, p);
        }
        Ok(())
    }

    #[test]
    fn pack_many() -> Result<(), Box<dyn Error>> {
        let dims = vec![
            (128, 96),
            (96, 128),
            (64, 96),
            (96, 64),
            (64, 64),
            (96, 96),
            (256, 64),
            (32, 32),
            (32, 32),
            (32, 32),
            (32, 32),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (128, 96),
            (96, 128),
            (64, 96),
            (96, 64),
            (64, 64),
            (96, 96),
            (256, 64),
            (32, 32),
            (32, 32),
            (32, 32),
            (32, 32),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (128, 96),
            (96, 128),
            (64, 96),
            (96, 64),
            (64, 64),
            (96, 96),
            (256, 64),
            (32, 32),
            (32, 32),
            (32, 32),
            (32, 32),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (42, 42),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (16, 16),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
            (8, 8),
        ];
        let mut imgs = vec![];
        for (i, (w, h)) in (dims).iter().enumerate() {
            imgs.push(NamedDynamicImage {
                name: i.to_string(),
                img: make_rect(*w, *h),
            })
        }
        let img_collection = ImageCollection::new(imgs);
        let img_packed = pack(2, img_collection)?;
        let _ = write_img("many.png", &img_packed);
        Ok(())
    }
}
