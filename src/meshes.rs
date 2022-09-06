//! This module generates meshes

use crate::types::{Mesh, Vertex};

use lin_alg2::f32::Vec3;

impl Mesh {
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
        let n_2 = Vec3::new(-1., 1., -1.).to_normalized();
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
}
