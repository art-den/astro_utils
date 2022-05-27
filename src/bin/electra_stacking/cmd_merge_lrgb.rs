use itertools::izip;
use fitrs::{Fits, FitsData, Hdu};
use std::path::PathBuf;
use structopt::StructOpt;


#[derive(StructOpt, Debug)]
pub struct CmdOptions {
    /// input file for luminance channel
    #[structopt(short,long)]
    lum_file: PathBuf,

    /// input file for red channel
    #[structopt(short,long)]
    red_file: PathBuf,

    /// input file for green channel
    #[structopt(short,long)]
    green_file: PathBuf,

    /// input file for blue channel
    #[structopt(short,long)]
    blue_file: PathBuf,

    /// output file
    #[structopt(short,long)]
    out_file: PathBuf,
}

pub fn execute(opt: CmdOptions) -> anyhow::Result<()> {
    let r_fits = Fits::open(&opt.red_file)?;
    let r_data = find_mono_data(&r_fits)?;

    let g_fits = Fits::open(&opt.green_file)?;
    let g_data = find_mono_data(&g_fits)?;

    let b_fits = Fits::open(&opt.blue_file)?;
    let b_data = find_mono_data(&b_fits)?;

    let lum_fits = Fits::open(&opt.lum_file)?;
    let lum_data = find_mono_data(&lum_fits)?;

    let primary_hdu = create_rgb_hdu(lum_data, r_data, g_data, b_data)?;

    Fits::create(&opt.out_file, primary_hdu).expect("Can't save result file");

    Ok(())
}

fn get_shape(data: &FitsData) -> Option<&Vec<usize>> {
    use fitrs::FitsData::*;
    match data {
        IntegersI32(data)     => Some(&data.shape),
        IntegersU32(data)     => Some(&data.shape),
        FloatingPoint32(data) => Some(&data.shape),
        FloatingPoint64(data) => Some(&data.shape),
        _                     => None,
    }
}

fn find_mono_data(fits: &Fits) -> anyhow::Result<FitsData> {
    let mut hdu: Option<Hdu> = None;
    for h in fits.iter() {
        let data = h.read_data();
        let shape = get_shape(&data);
        if let Some(shape) = shape {
            if (shape.len() == 2) || (shape.len() == 3 && shape[2] == 1) {
                if hdu.is_none() {
                    hdu = Some(h);
                } else {
                    anyhow::bail!("Grayscale data not found in FITS file");
                }
            }
        }
    }

    Ok(hdu.unwrap().read_data())
}

fn create_fits_rgb_data_opt<T: Copy + num::Num>(
    width:    usize,
    height:   usize,
    l_arr:    &[Option<T>],
    r_arr:    &[Option<T>],
    g_arr:    &[Option<T>],
    b_arr:    &[Option<T>],
    t_to_f64: fn(T) -> f64,
    f64_to_t: fn(f64) -> T
) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    let total = width * height;
    result.resize(total*3, T::zero());
    for (i, (l, r, g, b)) in izip!(l_arr, r_arr, g_arr, b_arr).enumerate() {
        let (mut lum_r, mut lum_g, mut lum_b) = (T::zero(), T::zero(), T::zero());
        if let (&Some(l), &Some(r), &Some(g), &Some(b)) = (l, r, g, b) {
            let rgb_summ = r + g + b;
            if rgb_summ != T::zero() {
                let rgb_summ = t_to_f64(rgb_summ);
                let norm_r = t_to_f64(r) / rgb_summ;
                let norm_g = t_to_f64(g) / rgb_summ;
                let norm_b = t_to_f64(b) / rgb_summ;
                let l = t_to_f64(l);
                lum_r = f64_to_t(l * norm_r);
                lum_g = f64_to_t(l * norm_g);
                lum_b = f64_to_t(l * norm_b);
            }
        }
        result[i] = lum_r;
        result[i+total] = lum_g;
        result[i+total*2] = lum_b;
    }
    result
}

fn create_fits_rgb_data<T: Copy + num::Num>(
    width:  usize,
    height: usize,
    l_arr:  &[T],
    r_arr:  &[T],
    g_arr:  &[T],
    b_arr:  &[T]
) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    let total = width * height;
    result.resize(total*3, T::zero());
    for (i, (l, r, g, b)) in izip!(l_arr, r_arr, g_arr, b_arr).enumerate() {
        let rgb_summ = *r + *g + *b;
        let (lum_r, lum_g, lum_b) = if rgb_summ != T::zero() {
            let norm_r = *r / rgb_summ;
            let norm_g = *g / rgb_summ;
            let norm_b = *b / rgb_summ;
            (*l * norm_r, *l * norm_g, *l * norm_b)
        } else {
            (T::zero(), T::zero(), T::zero())
        };
        result[i] = lum_r;
        result[i+total] = lum_g;
        result[i+total*2] = lum_b;
    }
    result
}

fn create_rgb_hdu(
    l: FitsData,
    r: FitsData,
    g: FitsData,
    b: FitsData
) -> anyhow::Result<Hdu> {
    use fitrs::FitsData::*;

    let (width, height) = match get_shape(&l) {
        Some(shape) => (shape[0], shape[1]),
        _           => anyhow::bail!("Data type is not supported"),
    };

    let dims = [width, height, 3];

    let primary_hdu = match (l, r, g, b) {
        (IntegersI32(l), IntegersI32(r), IntegersI32(g), IntegersI32(b)) =>
            Hdu::new(
                &dims,
                create_fits_rgb_data_opt(
                    width, height,
                    &l.data, &r.data, &g.data, &b.data,
                    |v| v as f64,
                    |v| v as i32
                )
            ),

        (IntegersU32(l), IntegersU32(r), IntegersU32(g), IntegersU32(b)) =>
            Hdu::new(
                &dims,
                create_fits_rgb_data_opt(
                    width, height,
                    &l.data, &r.data, &g.data, &b.data,
                    |v| v as f64,
                    |v| v as u32
                )
            ),

        (FloatingPoint32(l), FloatingPoint32(r), FloatingPoint32(g), FloatingPoint32(b)) =>
            Hdu::new(
                &dims,
                create_fits_rgb_data(width, height, &l.data, &r.data, &g.data, &b.data)
            ),

        (FloatingPoint64(l), FloatingPoint64(r), FloatingPoint64(g), FloatingPoint64(b)) =>
            Hdu::new(
                &dims,
                create_fits_rgb_data(width, height, &l.data, &r.data, &g.data, &b.data)
            ),

        _ =>
            anyhow::bail!("types of FITS file don't match"),
    };

    Ok(primary_hdu)
}
