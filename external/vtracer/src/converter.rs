use std::collections::HashMap;
use std::path::Path;
use std::{fs::File, io::Write};

use super::config::{ColorMode, Config, ConverterConfig, Hierarchical};
use super::svg::SvgFile;
// use fastrand::Rng;
use visioncortex::color_clusters::{Cluster, ClusterIndex, Clusters, KeyingAction, HIERARCHICAL_MAX};
use visioncortex::{Color, ColorImage, ColorName, PathSimplifyMode};

use crate::runner::{Runner, RunnerConfig};
const NUM_UNUSED_COLOR_ITERATIONS: usize = 6;

/// Convert an in-memory image into an in-memory SVG
pub fn convert(img: ColorImage, config: Config) -> Result<SvgFile, String> {
    let config = config.into_converter_config();
    match config.color_mode {
        ColorMode::Color => color_image_to_svg(img, config),
        ColorMode::Binary => binary_image_to_svg(img, config),
    }
}

/// Convert an image file into svg file
pub fn convert_image_to_svg(
    input_path: &Path,
    output_path: &Path,
    config: Config,
) -> Result<(), String> {
    let img = read_image(input_path)?;
    convert_image_to_svg_from_memory(img, output_path, config)
}

pub fn convert_image_to_svg_from_memory(
    img: ColorImage,
    output_path: &Path,
    config: Config,
) -> Result<(), String> {
    let svg = convert(img, config)?;
    write_svg(svg, output_path)
}

fn color_exists_in_image(img: &ColorImage, color: Color) -> bool {
    for y in 0..img.height {
        for x in 0..img.width {
            let pixel_color = img.get_pixel(x, y);
            if pixel_color.r == color.r && pixel_color.g == color.g && pixel_color.b == color.b {
                return true;
            }
        }
    }
    false
}

fn find_unused_color_in_image(img: &ColorImage) -> Result<Color, String> {
    let special_colors = IntoIterator::into_iter([
        Color::new(255, 0, 0),
        Color::new(0, 255, 0),
        Color::new(0, 0, 255),
        Color::new(255, 255, 0),
        Color::new(0, 255, 255),
        Color::new(255, 0, 255),
    ]);
    // let rng = rand::thread_rng();
    // let random_colors =
    //     (0..NUM_UNUSED_COLOR_ITERATIONS).map(|_| Color::new(rng.u8(..), rng.u8(..), rng.u8(..)));
	// just use pre-generated random numbers to avoid pulling in the rand crate

	let random_colors = [
		Color::new(253, 123, 123),
		Color::new(250, 162, 215),
		Color::new(186, 136, 54),
		Color::new(211, 111, 40),
		Color::new(149, 152, 46),
		Color::new(157, 125, 127),
		Color::new(15, 10, 188),
	];
    for color in special_colors.chain(random_colors) {
        if !color_exists_in_image(img, color) {
            return Ok(color);
        }
    }
    Err(String::from(
        "unable to find unused color in image to use as key",
    ))
}

fn should_key_image(img: &ColorImage, config: &ConverterConfig) -> bool {
    if img.width == 0 || img.height == 0 {
        return false;
    }

    // Check for transparency at several scanlines
    let threshold = ((img.width * 2) as f32 * config.keying_threshold) as usize;
    let mut num_transparent_pixels = 0;
    let y_positions = [
        0,
        img.height / 4,
        img.height / 2,
        3 * img.height / 4,
        img.height - 1,
    ];
    for y in y_positions {
        for x in 0..img.width {
            if img.get_pixel(x, y).a == 0 {
                num_transparent_pixels += 1;
            }
            if num_transparent_pixels >= threshold {
                return true;
            }
        }
    }

    false
}


fn remove_pixels_from_lower_layers(clusters: &Clusters) -> Vec<Cluster>{
    let view = clusters.view();
    let mut pixel_alpha_map = HashMap::new();
    let mut clusters = Vec::new();
    let clusters_output = view.clusters_output.to_vec();
    for &i in clusters_output.iter() {
        let mut cluster = view.get_cluster(i).clone();
        let indices = cluster.indices.clone();
        let indices_size = indices.len();
        if cluster.residue_sum.average().a == 254 {
            cluster.residue_sum.a = 255 * cluster.residue_sum.counter;
        }
        let color = cluster.residue_color();
        for (i, index) in indices.iter().rev().enumerate() {
            let pix_idx = *index;
            let real_i = indices_size - i - 1;
            if !pixel_alpha_map.contains_key(&pix_idx) {
                pixel_alpha_map.insert(pix_idx, color.a);
            } else {
                // Unless the pixel above this one is completely opaque, remove it
                // (This check prevents tiny cracks in the svg between pixels)
                if pixel_alpha_map.get(&pix_idx).unwrap() != &255 {
                    cluster.indices.remove(real_i);
                }
            }
        }
        clusters.push(cluster);
    }
    clusters
}

fn color_image_to_svg(mut img: ColorImage, config: ConverterConfig) -> Result<SvgFile, String> {
    let width = img.width;
    let height = img.height;

    let key_color = if should_key_image(&img, &config) {
        let key_color = find_unused_color_in_image(&img)?;
        for y in 0..height {
            for x in 0..width {
                if img.get_pixel(x, y).a == 0 {
                    img.set_pixel(x, y, &key_color);
                }
            }
        }
        key_color
    } else {
        // The default color is all zeroes, which is treated by visioncortex as a special value meaning no keying will be applied.
        Color::default()
    };
    let same_color_b = if matches!(config.mode, PathSimplifyMode::None) { 0 } else { 1 };

    let runner = Runner::new(
        RunnerConfig {
            diagonal: config.layer_difference == 0,
            hierarchical: HIERARCHICAL_MAX,
            batch_size: 25600,
            good_min_area: config.filter_speckle_area,
            good_max_area: (width * height),
            is_same_color_a: config.color_precision_loss,
            is_same_color_b: same_color_b,
            deepen_diff: config.layer_difference,
            hollow_neighbours: 1,
            key_color,
            keying_action: if matches!(config.hierarchical, Hierarchical::Cutout) {
                KeyingAction::Keep
            } else {
                KeyingAction::Discard
            },
        },
        img,
    );

    let mut clusters: Clusters = runner.run();
    // tell it to ignore the warning about not being used

    let mut new_cluster_vec = Vec::new();
    let _ = &new_cluster_vec;
    let mut new_cluster_output = Vec::new();
    let mut new_cluster_indices = Vec::new();

    let view = match config.hierarchical {
        Hierarchical::Stacked => {
            if matches!(config.mode, PathSimplifyMode::None) {
                let mut view = clusters.view();
                new_cluster_vec = remove_pixels_from_lower_layers(&clusters);
                for i in 0..new_cluster_vec.len() {
                    let idx = ClusterIndex(i as u32);
                    new_cluster_output.push(idx);
                    new_cluster_indices.push(idx);
                }
                view.clusters_output = &new_cluster_output;
                view.cluster_indices = &new_cluster_indices;
                view.clusters = &new_cluster_vec;
                view
            } else {
                clusters.view()
            }
        }
        Hierarchical::Cutout => {
            let view = clusters.view();
            let image = view.to_color_image();
            let runner = Runner::new(
                RunnerConfig {
                    diagonal: false,
                    hierarchical: 64,
                    batch_size: 25600,
                    good_min_area: 0,
                    good_max_area: (image.width * image.height) as usize,
                    is_same_color_a: 0,
                    is_same_color_b: same_color_b,
                    deepen_diff: 0,
                    hollow_neighbours: 0,
                    key_color,
                    keying_action: KeyingAction::Discard,
                },
                image,
            );
            clusters = runner.run();
            clusters.view()
        }
    };

    let mut svg = SvgFile::new(width, height, config.path_precision);
    for &cluster_index in view.clusters_output.iter().rev() {
        let cluster = view.get_cluster(cluster_index);
        let paths = cluster.to_compound_path(
            &view,
            false,
            config.mode,
            config.corner_threshold,
            config.length_threshold,
            config.max_iterations,
            config.splice_threshold,
        );
        let mut residue_color = cluster.residue_color();
        if !(matches!(config.mode, PathSimplifyMode::None)) {
            residue_color.a = 255;
        }
        svg.add_path(paths, residue_color);
    }

    Ok(svg)
}

fn binary_image_to_svg(img: ColorImage, config: ConverterConfig) -> Result<SvgFile, String> {
    let img = img.to_binary_image(|x| x.r < 128);
    let width = img.width;
    let height = img.height;

    let clusters = img.to_clusters(false);

    let mut svg = SvgFile::new(width, height, config.path_precision);
    for i in 0..clusters.len() {
        let cluster = clusters.get_cluster(i);
        if cluster.size() >= config.filter_speckle_area {
            let paths = cluster.to_compound_path(
                config.mode,
                config.corner_threshold,
                config.length_threshold,
                config.max_iterations,
                config.splice_threshold,
            );
            svg.add_path(paths, Color::color(&ColorName::Black));
        }
    }

    Ok(svg)
}

fn read_image(input_path: &Path) -> Result<ColorImage, String> {
    let img = image::open(input_path);
    let img = match img {
        Ok(file) => file.to_rgba8(),
        Err(_) => return Err(String::from("No image file found at specified input path")),
    };

    let (width, height) = (img.width() as usize, img.height() as usize);
    let img = ColorImage {
        pixels: img.as_raw().to_vec(),
        width,
        height,
    };

    Ok(img)
}

fn write_svg(svg: SvgFile, output_path: &Path) -> Result<(), String> {
    let out_file = File::create(output_path);
    let mut out_file = match out_file {
        Ok(file) => file,
        Err(_) => return Err(String::from("Cannot create output file.")),
    };

    write!(&mut out_file, "{}", svg).expect("failed to write file.");

    Ok(())
}
