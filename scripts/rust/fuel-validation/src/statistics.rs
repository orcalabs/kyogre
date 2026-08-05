pub fn calc_mean_and_sd(diffs: &[f64]) -> (f64, f64) {
    let n = diffs.len() as f64;
    let mean = diffs.iter().sum::<f64>() / n;

    let sd = (diffs.iter().map(|v| (v - mean).abs().powf(2.)).sum::<f64>() / n).sqrt();

    (mean, sd)
}

pub fn print_mean_and_sd(diffs: &[f64]) {
    let (mean, sd) = calc_mean_and_sd(diffs);

    println!("Mean: {mean:.0}");
    println!("SD:   {sd:.2}");
}
