use std::cmp::Ordering;

pub(crate) fn cmp_f64_asc(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.total_cmp(&b),
    }
}

pub(crate) fn cmp_f64_desc(a: f64, b: f64) -> Ordering {
    cmp_f64_asc(b, a)
}

#[cfg(test)]
mod tests {
    use super::{cmp_f64_asc, cmp_f64_desc};

    #[test]
    fn test_cmp_f64_desc_places_finite_scores_before_nan() {
        let mut scores = vec![1.0, f64::NAN, 2.0, -1.0, f64::INFINITY];
        scores.sort_by(|a, b| cmp_f64_desc(*a, *b));

        assert_eq!(scores[0], f64::INFINITY);
        assert_eq!(scores[1], 2.0);
        assert_eq!(scores[2], 1.0);
        assert_eq!(scores[3], -1.0);
        assert!(scores[4].is_nan());
    }

    #[test]
    fn test_cmp_f64_asc_treats_nan_as_lowest_for_max_by() {
        let scores = [f64::NAN, 0.5, 2.0, -1.0];
        let best = scores
            .iter()
            .max_by(|a, b| cmp_f64_asc(**a, **b))
            .copied();

        assert_eq!(best, Some(2.0));
    }
}
