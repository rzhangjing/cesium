//! Ported from `packages/engine/Source/Core/Tipsify.js`.
//!
//! Optimizes triangles for the post-vertex-shader cache.

/// Options for tipsify and calculate_acmr.
pub struct TipsifyOptions<'a> {
    pub indices: &'a [u32],
    pub maximum_index: Option<u32>,
    pub cache_size: u32,
}

impl<'a> Default for TipsifyOptions<'a> {
    fn default() -> Self {
        Self {
            indices: &[],
            maximum_index: None,
            cache_size: 24,
        }
    }
}

/// Calculates the average cache miss ratio (ACMR) for given indices.
pub fn calculate_acmr(options: &TipsifyOptions) -> f64 {
    let indices = options.indices;
    let cache_size = options.cache_size;
    let num_indices = indices.len();

    if num_indices < 3 || num_indices % 3 != 0 {
        return 0.0;
    }

    let maximum_index = options.maximum_index.unwrap_or_else(|| {
        indices.iter().copied().max().unwrap_or(0)
    });

    let mut vertex_time_stamps = vec![0u32; maximum_index as usize + 1];

    let mut s = cache_size + 1;
    for j in 0..num_indices {
        let idx = indices[j] as usize;
        if s - vertex_time_stamps[idx] > cache_size {
            vertex_time_stamps[idx] = s;
            s += 1;
        }
    }

    (s - cache_size + 1) as f64 / (num_indices as f64 / 3.0)
}

/// Optimizes triangles for the post-vertex shader cache.
pub fn tipsify(options: &TipsifyOptions) -> Vec<u32> {
    let indices = options.indices;
    let cache_size = options.cache_size;
    let num_indices = indices.len();

    if num_indices < 3 || num_indices % 3 != 0 {
        return Vec::new();
    }

    let maximum_index_plus_one = match options.maximum_index {
        Some(mi) => mi as usize + 1,
        None => indices.iter().copied().max().unwrap_or(0) as usize + 1,
    };

    struct Vertex {
        num_live_triangles: u32,
        time_stamp: u32,
        vertex_triangles: Vec<u32>,
    }

    let mut vertices: Vec<Vertex> = (0..maximum_index_plus_one)
        .map(|_| Vertex {
            num_live_triangles: 0,
            time_stamp: 0,
            vertex_triangles: Vec::new(),
        })
        .collect();

    let mut current_index = 0;
    let mut triangle = 0u32;
    let end_index = num_indices;
    while current_index < end_index {
        for offset in 0..3 {
            let idx = indices[current_index + offset] as usize;
            vertices[idx].vertex_triangles.push(triangle);
            vertices[idx].num_live_triangles += 1;
        }
        triangle += 1;
        current_index += 3;
    }

    let mut f: isize = 0;
    let mut s = cache_size + 1;
    let mut cursor: usize = 1;

    let mut dead_end: Vec<u32> = Vec::new();
    let mut output_indices: Vec<u32> = Vec::new();
    let num_triangles = num_indices / 3;
    let mut triangle_emitted = vec![false; num_triangles];

    let skip_dead_end = |dead_end: &mut Vec<u32>,
                         vertices: &[Vertex],
                         cursor: &mut usize,
                         max_plus_one: usize|
     -> isize {
        while !dead_end.is_empty() {
            let d = *dead_end.last().unwrap();
            dead_end.pop();
            if vertices[d as usize].num_live_triangles > 0 {
                return d as isize;
            }
        }
        while *cursor < max_plus_one {
            if vertices[*cursor].num_live_triangles > 0 {
                *cursor += 1;
                return *cursor as isize - 1;
            }
            *cursor += 1;
        }
        -1
    };

    while f != -1 {
        let mut one_ring: Vec<u32> = Vec::new();
        // Clone the triangle list to release the immutable borrow
        let triangles: Vec<u32> = vertices[f as usize].vertex_triangles.clone();
        for &tri in &triangles {
            if !triangle_emitted[tri as usize] {
                triangle_emitted[tri as usize] = true;
                let ci = tri as usize * 3;
                for j in 0..3 {
                    let index = indices[ci + j];
                    one_ring.push(index);
                    dead_end.push(index);
                    output_indices.push(index);
                    let vertex = &mut vertices[index as usize];
                    vertex.num_live_triangles -= 1;
                    if s - vertex.time_stamp > cache_size {
                        vertex.time_stamp = s;
                        s += 1;
                    }
                }
            }
        }

        // getNextVertex
        let mut n: isize = -1;
        let mut m: i32 = -1;
        for &index in &one_ring {
            let v = &vertices[index as usize];
            if v.num_live_triangles > 0 {
                let mut p: i32 = 0;
                if (s - v.time_stamp) as i32 + 2 * v.num_live_triangles as i32
                    <= cache_size as i32
                {
                    p = (s - v.time_stamp) as i32;
                }
                if p > m || m == -1 {
                    m = p;
                    n = index as isize;
                }
            }
        }
        if n == -1 {
            f = skip_dead_end(&mut dead_end, &vertices, &mut cursor, maximum_index_plus_one);
        } else {
            f = n;
        }
    }

    output_indices
}
