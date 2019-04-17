use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    #[structopt(short = "b", long = "base-size", default_value = "3.0")]
    base_size: f32,
    #[structopt(short = "p", long = "pixel-size", default_value = "1.0")]
    pixel_size: f32,
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}
#[derive(Debug)]
struct Slope {
    x: u32,
    y: u32,
    z: i32,
    slope: i32
}

fn quad_tris((x0, y0, z0): (f32, f32, f32), (x1, y1, z1): (f32, f32, f32), (x2, y2, z2): (f32, f32, f32), (x3, y3, z3): (f32, f32, f32)) -> impl Iterator<Item=stl_io::Triangle> {
    vec![stl_io::Triangle { normal: [0.0, 0.0, 0.0], vertices: [[x0,y0,z0], [x2,y2,z2], [x3,y3,z3]] },
     stl_io::Triangle { normal: [0.0, 0.0, 0.0], vertices: [[x0,y0,z0], [x1,y1,z1], [x2,y2,z2]] }
    ].into_iter()

}
fn process_image(input: image::DynamicImage, pixel_size: f32, base_size: f32) -> Vec<stl_io::Triangle> {
    let gray_image = input.rotate270().to_luma();
    let slopes: Vec<_> = gray_image.enumerate_pixels().scan((0, 0), |(z, zx), (y, x, pixel)| {
        if x != *zx {
            *z = 0;
            *zx = x;
        }
        let slope = Slope { x, y, z: *z, slope: pixel.data[0] as i32 - 127 };
        *z += slope.slope as i32;
        Some(slope)
    }).collect();

    let avg_zs: Vec<_> = (0..gray_image.height()).map(|x| {
        let sum: i32 = slopes.iter().filter(|s| s.x ==x).map(|s| s.z).sum();
        sum / gray_image.width() as i32
    }).collect();

    let slopes: Vec<_> = slopes.into_iter().map(|s| Slope {
        z: s.z - avg_zs[s.x as usize], ..s
    }).collect();

    let min_z = slopes.iter().map(|s| s.z + s.slope.min(0)).min().unwrap_or(0);
    let max_x = slopes.iter().map(|s| s.x).max().unwrap_or(0);
    let max_y = slopes.iter().map(|s| s.y).max().unwrap_or(0);
    let z_base = -base_size + min_z.min(0) as f32/128.0 * pixel_size;
    let tris: Vec<_> = slopes.iter().flat_map(|s| {
        let x0 = (s.x as f32 - max_x as f32 / 2.0) * pixel_size;
        let y0 = (s.y as f32 - max_y as f32 / 2.0) * pixel_size;
        let x1 = x0 + pixel_size;
        let y1 = y0 + pixel_size;
        let z0 = s.z as f32 / 128.0 * pixel_size;
        let z1 = (s.z as f32 + s.slope as f32) / 128.0 * pixel_size;

        quad_tris((x0, y0, z0), (x1, y0, z0), (x1, y1, z1), (x0, y1, z1))
            .chain(quad_tris((x0, y0, z_base), (x1, y0, z_base), (x1, y1, z_base), (x0, y1, z_base)))
            .chain(quad_tris((x0, y0, z_base), (x1, y0, z_base), (x1, y0, z0), (x0, y0, z0)))
            .chain(quad_tris((x0, y1, z_base), (x1, y1, z_base), (x1, y1, z1), (x0, y1, z1)))
            .chain(quad_tris((x0, y0, z_base), (x0, y1, z_base), (x0, y1, z1), (x0, y0, z0)))
            .chain(quad_tris((x1, y0, z_base), (x1, y1, z_base), (x1, y1, z1), (x1, y0, z0)))
    }).collect();
    tris
}

fn main() -> Result<(), Box<std::error::Error>>{
    let opt = Opt::from_args();
    let output_path = opt.input.with_extension("stl");
    let input_image = image::open(opt.input)?;
    let tris = process_image(input_image, opt.pixel_size, opt.base_size);
    let mut file = std::fs::OpenOptions::new().write(true).create(true).open(output_path).unwrap();
    stl_io::write_stl(&mut file, tris.iter()).unwrap();
    Ok(())
}
