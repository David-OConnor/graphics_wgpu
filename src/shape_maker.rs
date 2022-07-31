use std::collections::HashMap;
use std::f32::consts::TAU;

use super::types::{Mesh, Normal, Brush, Vertex};

const τ: f32 = TAU;

pub fn value_from_grid(i: u32, res: u32, val_range: (f32, f32)) -> f32 {
    // Used for iterating over grids; correlate an index to a value.
    (i as f32 / res as f32) * (val_range.1 - val_range.0) - (val_range.1 - val_range.0) / 2.
}

pub fn make_normals(vertices: &HashMap<u32, Vertex>, faces: &Vec<Vec<u32>>) -> Vec<Normal> {
    // Only take into account x, y, and z when computing normals.
    let mut normals = Vec::new();
    for face in faces {
        // todo make sure these aren't reversed!
        let line1 = vertices[&face[1]].subtract(&vertices[&face[0]]);
        let line2 = vertices[&face[2]].subtract(&vertices[&face[0]]);
        normals.push(line1.cross(&line2));
    }

    normals
}

pub fn combine_meshes(mut base: Mesh, meshes: Vec<(Mesh, [f32; 3])>) -> Mesh {
    // The array in the meshes tuple is position offset for that shape.
    let mut id_addition = base.vertices.len() as u32;
    for (mesh, offset) in &meshes {
        for (id, vertex) in &mesh.vertices {
            // For the roof, modify the ids to be unique.
            base.vertices.insert(
                id + id_addition,
                Vertex::new(
                    vertex.position[0] + offset[0],
                    vertex.position[1] + offset[1],
                    vertex.position[2] + offset[2],
                ),
            );
        }

        for face in &mesh.faces_vert {
            base.faces_vert.push(face + id_addition);
        }

        for normal in &mesh.normals {
            // todo rotate normals!
            base.normals.push(normal.clone());
        }

        id_addition += mesh.vertices.len() as u32;
    }

    base.make_tris();
    base
}

pub fn box_(lens: (f32, f32, f32)) -> Mesh {
    // Make a rectangular prism.  Use negative lengths to draw in the opposite
    // direction.

    let coords = [
        // Front
        [-1., -1., -1.],
        [1., -1., -1.],
        [-1., 1., -1.],
        [1., 1., -1.],
        // Back
        [-1., -1., 1.],
        [1., -1., 1.],
        [-1., 1., 1.],
        [1., 1., 1.],
    ];

    let mut vertices = HashMap::new();
    for (id, coord) in coords.iter().enumerate() {
        vertices.insert(
            id as u32,
            Vertex::new(
                coord[0] * lens.0 / 2.,
                coord[1] * lens.1 / 2.,
                coord[2] * lens.2 / 2.,
            ),
        );
    }

    let faces_vert = vec![
        // Vertex indices for each face.
        vec![0, 1, 2, 3], // Front
        vec![4, 5, 6, 7], // Back
        vec![3, 2, 6, 7], // Top
        vec![0, 1, 5, 4], // Bottom
        vec![0, 4, 7, 3], // Left
        vec![1, 5, 6, 2], // Right
    ];

    //  Normals correspond to faces.
    let normals = vec![
        Normal::new(0., 0., -1.),
        Normal::new(0., 0., 1.),
        Normal::new(0., 1., 0.),
        Normal::new(0., -1., 0.),
        Normal::new(-1., 0., 0.),
        Normal::new(1., 0., 0.),
    ];

    Mesh::new(vertices, faces_vert, normals)
}

pub fn cube(side_len: f32) -> Mesh {
    // Convenience function.
    // We'll still treat the center as the center of the base portion.
    box_((side_len, side_len, side_len))
}

fn avg_normals(normals: Vec<Normal>) -> Normal {
    let x = normals.iter().fold(0., |acc, norm| acc + norm.normal.0);
    let y = normals.iter().fold(0., |acc, norm| acc + norm.normal.1);
    let z = normals.iter().fold(0., |acc, norm| acc + norm.normal.2);

    let len = normals.len() as f32;
    Normal::new(x / len, y / len, z / len)
}

pub fn terrain(dims: (f32, f32), res: u32, height_map: Vec<Vec<f32>>) -> Mesh {
    // Make a triangle-based terrain mesh.  dims is an [x, z] tuple.
    // We could make a 4d terrain too... id a volume of u-mappings... or have
    // w and y mappings for each x/z point...
    // dims refers to the size of the terrain. res is the number of cells
    // dividing our terrain in each direction. Perhaps replace this argument with
    // something more along the traditional def of resolution?

    // todo include some of your code streamlining from make_spherinder;
    // todo better yet: Combine these two with a helper func.

    let mut vertices = HashMap::new();
    let mut normals = Vec::new();

    let mut id = 0;

    let mut active_ind = 0;
    // Faces for this terrain are triangles. Don't try to make square faces;
    // they'd really have creases down a diagonal.
    let mut faces_vert = Vec::new();

    for i in 0..res {
        // x
        let x = value_from_grid(i, res, (0., dims.0));
        for j in 0..res {
            // z
            let z = value_from_grid(j, res, (0., dims.1));
            let height = height_map[[i as usize, j as usize]];
            // You could change which planes this is over by rearranging
            // these node points.
            vertices.insert(id, Vertex::new(x, height, z));
            id += 1;
        }
    }

    for i in 0..res - 1 {
        for j in 0..res - 1 {
            // The order we build these triangles and normals is subject to trial+error.
            // two face triangles per grid square. There are two ways to split
            // up the squares into triangles; picking one arbitrarily.
            faces_vert.push(vec![
                // shows front right
                active_ind + j,           // back left
                active_ind + j + 1,       // back right
                active_ind + j + res + 1, // front left
                active_ind + j + res
            ]);

            let current_ind = active_ind + j;
            let current_vert = &vertices[&(current_ind)];

            // Compute normal as the avg of the norm of all 4 neighboring faces.
            // We are ignoring w, for now.
            let mut edge_pairs = Vec::new();
            // If logic is to prevent index mistakes on edge cases.
            // Start at North; go around clockwise.
            if i != res - 2 && j != res - 2 {
                // not at ne corner
                edge_pairs.push((
                    vertices[&(current_ind + 1)].subtract(current_vert), // n
                    vertices[&(current_ind + res + 1)].subtract(current_vert), // ne
                ));
                edge_pairs.push((
                    vertices[&(current_ind + res + 1)].subtract(current_vert), // ne
                    vertices[&(current_ind + res)].subtract(current_vert),     // e
                ));
            }
            if i != res - 2 && j != 0 {
                // not at se corner
                edge_pairs.push((
                    vertices[&(current_ind + res)].subtract(current_vert), // e
                    vertices[&(current_ind + res - 1)].subtract(current_vert), // se
                ));
                edge_pairs.push((
                    vertices[&(current_ind + res - 1)].subtract(current_vert), // se
                    vertices[&(current_ind - 1)].subtract(current_vert),       // s
                ));
            }
            if i != 0 && j != 0 {
                // not at sw corner
                edge_pairs.push((
                    vertices[&(current_ind - 1)].subtract(current_vert), // s
                    vertices[&(current_ind - res - 1)].subtract(current_vert), // sw
                ));
                edge_pairs.push((
                    vertices[&(current_ind - res - 1)].subtract(current_vert), // sw
                    vertices[&(current_ind - res)].subtract(current_vert),     // w
                ));
            }
            if i != 0 && j != res - 2 {
                // not at nw corner
                edge_pairs.push((
                    vertices[&(current_ind - res)].subtract(current_vert), // w
                    vertices[&(current_ind - res + 1)].subtract(current_vert), // nw
                ));
                edge_pairs.push((
                    // nw
                    vertices[&(current_ind - res + 1)].subtract(current_vert), // nw
                    vertices[&(current_ind + 1)].subtract(current_vert),       // n
                ));
            }

            // Note: This isn't normalized; we handle that in the shader, for now.
            let mut surrounding_norms = Vec::new();
            for (edge0, edge1) in &edge_pairs {
                surrounding_norms.push(edge0.cross(edge1));
            }

            normals.push(avg_normals(surrounding_norms));
        }
        active_ind += res;
    }

    Mesh::new(vertices, faces_vert, normals)
}

//pub fn grid(n_dims: u32, dims: Vec<f32>, res: u32) -> HashMap<u32, Shape> {
//    // An evenly-spaced grid of n-dimensions.
//    let mut result = HashMap::new();
//    if n_dims == 0 {
//        return result
//    }
//
//    for i in 0..res {
//
//    }
//
//    grid(active_dim, dims, res)
//}

//pub fn make_sphere(radius: f32, res: u32) -> Mesh {
//    assert_eq!(res % 2, 0);
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube() {}
}
