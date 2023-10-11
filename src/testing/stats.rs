use num_traits::NumCast;
use std::collections::HashSet;

pub fn rel_err<T>(b: T, p: T, abs_threshold: f64) -> f64
where
    T: NumCast,
{
    let b: f64 = NumCast::from(b).unwrap();
    let p: f64 = NumCast::from(p).unwrap();
    let diff = (b - p).abs();

    if diff > abs_threshold {
        // compute relative error
        if p == 0.0 {
            diff
        } else {
            diff / p
        }
    } else {
        0.0
    }
}

#[must_use]
pub fn dram_rel_err(
    play_stats: &playground::stats::DRAM,
    box_stats: &playground::stats::DRAM,
    abs_threshold: f64,
) -> Vec<(String, f64)> {
    vec![
        (
            "total_reads".to_string(),
            rel_err(box_stats.total_reads, play_stats.total_reads, abs_threshold),
        ),
        (
            "total_writes".to_string(),
            rel_err(
                box_stats.total_writes,
                play_stats.total_writes,
                abs_threshold,
            ),
        ),
    ]
}

#[must_use]
pub fn cache_rel_err(
    play_stats: &stats::cache::Cache,
    box_stats: &stats::cache::Cache,
    abs_threshold: f64,
) -> Vec<(String, f64)> {
    all_cache_rel_err(play_stats, box_stats, abs_threshold)
        .into_iter()
        .map(|((alloc_id, access), err)| {
            let access_name = match alloc_id {
                None => access.to_string(),
                Some(id) => format!("{id}@{access}"),
            };
            (access_name, err)
        })
        .filter(|(_, err)| *err != 0.0)
        .collect()
}

#[must_use]
pub fn all_cache_rel_err<'a>(
    play_stats: &'a stats::cache::Cache,
    box_stats: &'a stats::cache::Cache,
    abs_threshold: f64,
) -> Vec<(&'a (Option<usize>, stats::cache::AccessStatus), f64)> {
    let keys: HashSet<_> = play_stats
        .as_ref()
        .keys()
        .chain(box_stats.as_ref().keys())
        .collect();
    keys.into_iter()
        .map(|k| {
            let p = play_stats.as_ref().get(k).copied().unwrap_or_default();
            let b = box_stats.as_ref().get(k).copied().unwrap_or_default();
            let rel_err = rel_err(b, p, abs_threshold);
            (k, rel_err)
        })
        .collect()
}
