use std::path::PathBuf;
use structopt::StructOpt;
use image::GenericImageView;

const Z_BASE: f32 = -3.0;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
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
fn process_image(input: image::DynamicImage) -> Vec<stl_io::Triangle> {
    let gray_image = input.rotate270().to_luma();
    //println!("gray: {:?}", gray_image);
    let slopes: Vec<_> = gray_image.enumerate_pixels().scan((0, 0), |(z, zx), (y, x, pixel)| {
        if x != *zx {
            *z = 0;
            *zx = x;
        }
        let slope = Slope { x, y, z: *z, slope: pixel.data[0] as i32 - 127 };
        *z += slope.slope as i32;
        Some(slope)
    }).collect();
    //println!("Slopes: {:?}", slopes);
    let min_z = slopes.iter().map(|s| s.z + s.slope.min(0)).min().unwrap_or(0);
    let z_base = Z_BASE + min_z.min(0) as f32/128.0;
    let tris: Vec<_> = slopes.iter().flat_map(|s| {
        let x = s.x as f32;
        let y = s.y as f32;
        let z0 = s.z as f32 / 128.0;
        let z1 = (s.z as f32 + s.slope as f32) / 128.0;
        quad_tris((x, y, z0), (x + 1.0, y, z0), (x + 1.0, y + 1.0, z1), (x, y + 1.0, z1))
            .chain(quad_tris((x, y, z_base), (x + 1.0, y, z_base), (x + 1.0, y + 1.0, z_base), (x, y + 1.0, z_base)))
            .chain(quad_tris((x, y, z_base), (x + 1.0, y, z_base), (x + 1.0, y, z0), (x, y, z0)))
            .chain(quad_tris((x, y + 1.0, z_base), (x + 1.0, y + 1.0, z_base), (x + 1.0, y + 1.0, z1), (x, y + 1.0, z1)))
            .chain(quad_tris((x, y, z_base), (x, y + 1.0, z_base), (x, y + 1.0, z1), (x, y, z0)))
            .chain(quad_tris((x + 1.0, y, z_base), (x + 1.0, y + 1.0, z_base), (x + 1.0, y + 1.0, z1), (x + 1.0, y, z0)))
    }).collect();
    //println!("Tris: {:?}", tris);
    tris
}

fn main() -> Result<(), Box<std::error::Error>>{
    let opt = Opt::from_args();
    println!("Opts: {:?}", opt);
    let output_path = opt.input.with_extension("stl");
    let input_image = image::open(opt.input)?;
    println!("image size: {}, {}", input_image.width(), input_image.height());
    let tris = process_image(input_image);
    let max_x = tris.iter().flat_map(|t| t.vertices.iter().map(|v| v[0] as i64)).max().unwrap_or(0);
    let min_x = tris.iter().flat_map(|t| t.vertices.iter().map(|v| v[0] as i64)).min().unwrap_or(0);
    let max_y = tris.iter().flat_map(|t| t.vertices.iter().map(|v| v[1] as i64)).max().unwrap_or(0);
    let min_y = tris.iter().flat_map(|t| t.vertices.iter().map(|v| v[1] as i64)).min().unwrap_or(0);
    let max_z = tris.iter().flat_map(|t| t.vertices.iter().map(|v| v[2] as i64)).max().unwrap_or(0);
    let min_z = tris.iter().flat_map(|t| t.vertices.iter().map(|v| v[2] as i64)).min().unwrap_or(0);
    println!("Model dimensions: ({}, {}, {})", max_x - min_x, max_y - min_y, max_z - min_z);
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(output_path).unwrap();
    stl_io::write_stl(&mut file, tris.iter()).unwrap();
    Ok(())
}
