use std::collections::HashMap;

pub trait FastSearch<S, P> {
    fn search(&self, pattern: &P) -> Option<usize>;
    fn rsearch(&self, pattern: &P) -> Option<usize>;
    fn search_all(&self, pattern: &P) -> Vec<usize>;
}

impl<T, O> FastSearch<T, O> for T
where
    T: AsRef<[u8]>,
    O: AsRef<[u8]>,
{
    fn search(&self, pattern: &O) -> Option<usize> {
        let space = self.as_ref();
        let pattern = pattern.as_ref();
        let (pat_len, space_len) = (pattern.len(), space.len());
        let offset = pat_len - 1;

        let table: HashMap<u8, usize> = pattern
            .iter()
            .enumerate()
            .map(|(idx, b)| {
                if idx == offset {
                    (*b, pat_len)
                } else {
                    (*b, pat_len - idx - 1)
                }
            })
            .collect();

        let mut idx = offset;
        space
            .iter()
            .find_map(|_| {
                if idx >= space_len {
                    idx = space_len - 1;
                };
                if let Some(hit) = table.get(&space[idx]) {
                    let catch = pattern
                        .iter()
                        .enumerate()
                        .rev()
                        .find_map(|(sg_idx, sg_byte)| {
                            let nxt = idx - (offset - sg_idx);
                            if &space[nxt] == sg_byte {
                                if sg_idx == 0 {
                                    Some(Some(idx - offset))
                                } else {
                                    None
                                }
                            } else {
                                Some(None)
                            }
                        });
                    match catch {
                        Some(Some(value)) => Some(Some(value)),
                        _ => {
                            if idx + hit >= space_len {
                                Some(None)
                            } else {
                                idx += hit;
                                None
                            }
                        }
                    }
                } else {
                    if idx + pat_len >= space_len {
                        Some(None)
                    } else {
                        idx += pat_len;
                        None
                    }
                }
            })
            .flatten()
    }

    fn rsearch(&self, pattern: &O) -> Option<usize> {
        let space = self.as_ref();
        let pattern = pattern.as_ref();
        let (pat_len, space_len) = (pattern.len(), space.len());

        if space_len == 0 {
            return None;
        }

        let table: HashMap<u8, usize> = pattern
            .iter()
            .enumerate()
            .map(|(idx, b)| if idx == 0 { (*b, pat_len) } else { (*b, idx) })
            .collect();

        let mut idx = space_len - pat_len;
        space
            .iter()
            .find_map(|_| {
                if let Some(hit) = table.get(&space[idx]) {
                    let catch = pattern.iter().enumerate().find_map(|(sg_idx, sg_byte)| {
                        let nxt = idx + sg_idx;
                        if &space[nxt] == sg_byte {
                            if sg_idx == 0 {
                                Some(Some(idx))
                            } else {
                                None
                            }
                        } else {
                            Some(None)
                        }
                    });
                    match catch {
                        Some(Some(value)) => Some(Some(value)),
                        _ => {
                            if let Some(sub) = idx.checked_sub(*hit) {
                                idx = sub;
                                None
                            } else {
                                Some(None)
                            }
                        }
                    }
                } else {
                    if let Some(sub) = idx.checked_sub(pat_len) {
                        idx = sub;
                        None
                    } else {
                        Some(None)
                    }
                }
            })
            .flatten()
    }

    fn search_all(&self, pattern: &O) -> Vec<usize> {
        let mut buffer = Vec::new();
        let space = self.as_ref();
        let pattern = pattern.as_ref();
        let (pat_len, space_len) = (pattern.len(), space.len());
        let offset = pat_len - 1;

        if space_len == 0 {
            return buffer;
        }

        let table: HashMap<u8, usize> = pattern
            .iter()
            .enumerate()
            .map(|(idx, b)| {
                if idx == offset {
                    (*b, pat_len)
                } else {
                    (*b, pat_len - idx - 1)
                }
            })
            .collect();

        let mut idx = offset;
        while space_len > idx {
            if let Some(hit) = table.get(&space[idx]) {
                let miss = pattern
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(pat_idx, byte)| {
                        let nxt = idx - (offset - pat_idx);
                        if &space[nxt] == byte {
                            if pat_idx == 0 {
                                Some(Some(nxt))
                            } else {
                                None
                            }
                        } else {
                            Some(None)
                        }
                    });

                match miss {
                    Some(Some(not_miss)) => {
                        buffer.push(not_miss);
                        idx += pat_len
                    }
                    _ => idx += hit,
                };
            } else {
                idx += pat_len
            }
        }
        buffer
    }
}

#[test]
fn reverse_search() {
    let space = [0, 1, 2, 3, 4, 5, 6, 7, 6, 9, 5, 6, 7, 8];
    let pat = [5];

    assert_eq!(Some(10), space.rsearch(&pat))
}

#[test]
fn search() {
    let space = [0, 1, 2, 3, 4, 5, 6, 7, 6, 9, 5, 6, 7, 8];
    let pat = [5];

    assert_eq!(Some(5), space.search(&pat))
}

#[test]
fn search_multiple() {
    let space = [0, 8, 2, 3, 4, 0, 6, 0, 8, 9, 10, 0, 12, 0, 8, 15];
    let pat = [0, 8];
    let v1 = space.search_all(&pat);
    let v2 = vec![0, 7, 13];

    assert_eq!(v1, v2);
}
