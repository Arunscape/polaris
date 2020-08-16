use crate::directories::{Directories, PolarisDirectories};
use anyhow::*;
use image;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, ImageOutputFormat};
use std::cmp;
use std::collections::hash_map::DefaultHasher;
use std::fs::{DirBuilder, File};
use std::hash::{Hash, Hasher};
use std::path::*;

#[derive(Debug, Hash)]
pub struct Options {
	pub max_dimension: u32,
	pub resize_if_almost_square: bool,
	pub pad_to_square: bool,
}

impl Default for Options {
	fn default() -> Options {
		Options {
			max_dimension: 400,
			resize_if_almost_square: true,
			pad_to_square: true,
		}
	}
}

fn hash(path: &Path, options: &Options) -> u64 {
	let mut hasher = DefaultHasher::new();
	path.hash(&mut hasher);
	options.hash(&mut hasher);
	hasher.finish()
}

pub fn get_thumbnail(real_path: &Path, options: &Options) -> Result<PathBuf> {
	let mut out_path = Directories::get_thumbnail_directory()?;

	let mut dir_builder = DirBuilder::new();
	dir_builder.recursive(true);
	dir_builder.create(out_path.as_path())?;

	let source_image = image::open(real_path)?;
	let (source_width, source_height) = source_image.dimensions();
	let largest_dimension = cmp::max(source_width, source_height);
	let out_dimension = cmp::min(options.max_dimension, largest_dimension);

	let hash = hash(real_path, options);
	out_path.push(format!("{}.jpg", hash.to_string()));

	if !out_path.exists() {
		let quality = 80;
		let source_aspect_ratio: f32 = source_width as f32 / source_height as f32;
		let is_almost_square = source_aspect_ratio > 0.8 && source_aspect_ratio < 1.2;

		let mut final_image;
		if is_almost_square && options.resize_if_almost_square {
			final_image =
				source_image.resize_exact(out_dimension, out_dimension, FilterType::Lanczos3);
		} else if options.pad_to_square {
			let scaled_image =
				source_image.resize(out_dimension, out_dimension, FilterType::Lanczos3);
			let (scaled_width, scaled_height) = scaled_image.dimensions();
			let background = image::Rgb([255, 255 as u8, 255 as u8]);
			final_image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
				out_dimension,
				out_dimension,
				background,
			));
			final_image.copy_from(
				&scaled_image,
				(out_dimension - scaled_width) / 2,
				(out_dimension - scaled_height) / 2,
			)?;
		} else {
			final_image = source_image.resize(out_dimension, out_dimension, FilterType::Lanczos3);
		}

		let mut out_file = File::create(&out_path)?;
		final_image.write_to(&mut out_file, ImageOutputFormat::Jpeg(quality))?;
	}

	Ok(out_path)
}
