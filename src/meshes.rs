//! This module generates meshes

use core::f32::consts::TAU;

use crate::{
    init_graphics::UP_VEC,
    types::{Mesh, Vertex},
};

use lin_alg2::f32::Vec3;

/// Rotate a 2d vector counter-clockwise a given angle.
fn rotate_vec_2d(vec: [f32; 2], θ: f32) -> [f32; 2] {
    // Self-contained 2d rotation matrix (col-maj)
    let (sin_θ, cos_θ) = θ.sin_cos();
    let mat = [cos_θ, sin_θ, -sin_θ, cos_θ];

    [
        vec[0] * mat[0] + vec[1] * mat[2],
        vec[0] * mat[1] + vec[1] * mat[3],
    ]
}

impl Mesh {
    /// Create a (normalized cube) sphere mesh. A higher div count results in a smoother sphere.
    /// https://medium.com/@oscarsc/four-ways-to-create-a-mesh-for-a-sphere-d7956b825db4
    /// todo: Temporarily, a uv_sphere while we figure out how to make better ones.
    pub fn new_sphere(radius: f32) -> Self {
        // todo: These params are fixed due to the temporary nature of this mesh
        let num_lats = 10;
        let num_lons = 12;

        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        // We use faces to construct indices (triangles)
        let mut indices = Vec::new();

        // In radians
        let lat_size = TAU / (2. * num_lats as f32);
        let lon_size = TAU / num_lons as f32;

        let mut current_i = 0;

        for i in 0..num_lats {
            // todo: Faces for top and bottom sections
            if i == 0 {
                vertices.push(Vertex::new(
                    [0., -radius, 0.],
                    Vec3::new(0., -1., 0.) .to_normalized(),
                ));
            } else if i == num_lats - 1 {
                vertices.push(Vertex::new(
                    [0., radius, 0.],
                    Vec3::new(0., 1., 0.) .to_normalized(),
                ));
            }

            let lat = i as f32 * lat_size;

            for j in 0..num_lons {
                let lon = j as f32 * lon_size;

                let x = (lat).cos() * (lon).cos() * 1. * radius;
                let y = (lat).cos() * (lon).sin() * 1. * radius;
                let z = (lat).sin() * 1. * radius;

                vertices.push(Vertex::new(
                    [x, y, z],
                    Vec3::new(
                        radius * radius * lon.cos() * lat.sin() * lat.sin(),
                        radius * radius * lon.sin() * lat.sin() * lat.sin(),
                        -radius * radius * lat.sin() * lat.cos(),
                    )
                    .to_normalized(),
                ));
                current_i += 1;

                if i != num_lats && j != num_lons {
                    // In CCW order
                    faces.push([
                        current_i,  current_i + 1, current_i + num_lons + 1,
                        current_i + num_lons
                    ]);
                    // face.push(current_i);
                    // face.push(current_i + 1);
                    // face.push();
                    // face.push();
                }

            }
        }

        for f in faces {
            indices.append(&mut vec![f[0], f[1], f[2], f[0], f[2], f[3]]);
        }

        Mesh {
            vertices,
            indices,
            // vertex_buffer: Vec<usize>,
            // index_buffer: Vec<usize>,
            // num_elements: u32,
            material: 0,
        }
    }

    /// Create a tetrahedron mesh
    pub fn new_tetrahedron(side_len: f32) -> Self {
        let v_0 = [side_len, side_len, side_len];
        let v_1 = [side_len, -side_len, -side_len];
        let v_2 = [-side_len, side_len, -side_len];
        let v_3 = [-side_len, -side_len, side_len];

        // Note: For tetrahedrons, the normals are the corners of the cube we
        // didn't use for vertices.
        let n_0 = Vec3::new(1., 1., -1.).to_normalized();
        let n_1 = Vec3::new(1., -1., 1.).to_normalized();
        let n_2 = Vec3::new(-1., 1., 1.).to_normalized();
        let n_3 = Vec3::new(-1., -1., -1.).to_normalized();

        let mut vertices = vec![
            // Face 0
            Vertex::new(v_0, n_0),
            Vertex::new(v_2, n_0),
            Vertex::new(v_1, n_0),
            // Face 1
            Vertex::new(v_0, n_1),
            Vertex::new(v_1, n_1),
            Vertex::new(v_3, n_1),
            // Face 2
            Vertex::new(v_0, n_2),
            Vertex::new(v_3, n_2),
            Vertex::new(v_2, n_2),
            // Face 3
            Vertex::new(v_1, n_3),
            Vertex::new(v_2, n_3),
            Vertex::new(v_3, n_3),
        ];

        // These indices define faces by triangles. (each 3 represent a triangle, starting at index 0.
        // Indices are arranged CCW, from front of face
        // Note that because we're using "hard" lighting on faces, we can't repeat any vertices, since
        // they each have a different normal.
        #[rustfmt::skip]
            // let indices: &[u32] = &[
            let indices = vec![
            0, 1, 2,
            3, 4, 5,
            6, 7, 8,
            9, 10, 11,
        ];

        Mesh {
            vertices,
            indices,
            // vertex_buffer: Vec<usize>,
            // index_buffer: Vec<usize>,
            // num_elements: u32,
            material: 0,
        }
    }

    /// Create a cylinder
    pub fn new_cylinder(len: f32, radius: f32, num_sides: usize) -> Self {
        let angle_between_vertices = TAU / num_sides as f32;

        let mut circle_vertices = Vec::new();
        for i in 0..num_sides {
            circle_vertices.push(rotate_vec_2d(
                [radius, 0.],
                i as f32 * angle_between_vertices,
            ));
        }

        let half_len = len * 0.5;
        let mod_ = 2 * num_sides;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let mut i_vertex = 0;

        for vert in &circle_vertices {
            // The number of faces is the number of angles - 1.
            // Triangle 1: This top, this bottom, next top.
            indices.append(&mut vec![i_vertex, i_vertex + 1, (i_vertex + 2) % mod_]);
            // Triangle 2: This bottom, next bottom, next top.
            indices.append(&mut vec![
                i_vertex + 1,
                (i_vertex + 3) % mod_,
                (i_vertex + 2) % mod_,
            ]);

            // On edge face, top
            vertices.push(Vertex::new(
                [vert[0], half_len, vert[1]],
                Vec3::new(vert[0], 0., vert[1]).to_normalized(),
            ));
            i_vertex += 1;

            // On edge face, bottom
            vertices.push(Vertex::new(
                [vert[0], -half_len, vert[1]],
                Vec3::new(vert[0], 0., vert[1]).to_normalized(),
            ));
            i_vertex += 1;
        }

        // let mut vertices_top_face = Vec::new();
        // let mut vertices_bottom_face = Vec::new();
        let top_anchor = i_vertex;
        let bottom_anchor = i_vertex + 1;

        for (j, vert) in circle_vertices.iter().enumerate() {
            // We need num_sides - 2 triangles using this anchor-vertex algorithm.
            if j != 0 && j != num_sides - 1 {
                indices.append(&mut vec![top_anchor, i_vertex, i_vertex + 2]);
                // We need CCW triangles for both, so reverse order on the bottom face.
                indices.append(&mut vec![bottom_anchor, i_vertex + 3, i_vertex + 1]);
            }

            // Top face
            vertices.push(Vertex::new([vert[0], half_len, vert[1]], UP_VEC));
            i_vertex += 1;

            // Bottom face
            vertices.push(Vertex::new([vert[0], -half_len, vert[1]], -UP_VEC));
            i_vertex += 1;
        }

        Mesh {
            vertices,
            indices,
            material: 0,
        }
    }
}
