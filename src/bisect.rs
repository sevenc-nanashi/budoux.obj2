pub fn bisect_first_true<F>(mut low: usize, mut high: usize, mut f: F) -> Option<usize>
where
    F: FnMut(usize) -> bool,
{
    while low < high {
        let mid = low + (high - low) / 2;
        if f(mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    if low < high && f(low) {
        Some(low)
    } else {
        None
    }
}

pub fn try_bisect_first_true<F, E>(
    mut low: usize,
    mut high: usize,
    mut f: F,
) -> Result<Option<usize>, E>
where
    F: FnMut(usize) -> Result<bool, E>,
{
    while low < high {
        let mid = low + (high - low) / 2;
        if f(mid)? {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    if low < high && f(low)? {
        Ok(Some(low))
    } else {
        Ok(None)
    }
}

pub fn bisect_last_true<F>(mut low: usize, mut high: usize, mut f: F) -> Option<usize>
where
    F: FnMut(usize) -> bool,
{
    while low < high {
        let mid = low + (high - low) / 2;
        if f(mid) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    if low > 0 && f(low - 1) {
        Some(low - 1)
    } else {
        None
    }
}

pub fn try_bisect_last_true<F, E>(
    mut low: usize,
    mut high: usize,
    mut f: F,
) -> Result<Option<usize>, E>
where
    F: FnMut(usize) -> Result<bool, E>,
{
    while low < high {
        let mid = low + (high - low) / 2;
        if f(mid)? {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    if low > 0 && f(low - 1)? {
        Ok(Some(low - 1))
    } else {
        Ok(None)
    }
}
