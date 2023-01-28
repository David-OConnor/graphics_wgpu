//! This module contains code specific to compute shader operations.

use wgpu::{self, util::DeviceExt};

// todo: Temp test for compute
#[derive(Clone, Copy)]
struct Cplx {
    real: f32,
    im: f32, // todo: How to f64 in shader?
}

impl Cplx {
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut result = [0; 8];

        result[0..4].clone_from_slice(&self.real.to_ne_bytes());
        result[4..8].clone_from_slice(&self.im.to_ne_bytes());

        result
    }
}

/// Temporary test data
fn create_test_data<'a>(compute_buf: &mut [u8]) {
    // Set up test input of complex numbers.
    let cplx_val = Cplx { real: 1.0, im: 0. };
    let mut compute_input = Vec::new();
    for _ in 0..10 {
        compute_input.push(cplx_val);
    }

    // Serialize these as a byte array.
    for (j, cplx_num) in compute_input.iter().enumerate() {
        let buf_this_val = cplx_num.to_bytes();
        for i in 0..8 {
            compute_buf[j * 8 + i] = buf_this_val[i];
        }
    }
}

pub(crate) fn setup(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Buffer) {
    let compute_buf = {
        let mut input_data = [0; 80];
        create_test_data(&mut input_data);
        input_data
    };

    let compute_buf_out = [0_u8; 80];

    // For our WIP compute functionality.
    let compute_storage_buf_input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Compute storage buffer"),
        contents: &compute_buf,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST // todo?
            | wgpu::BufferUsages::COPY_SRC, // todo?
    });

    let compute_storage_buf_output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Compute storage buffer output"),
        contents: &compute_buf_out,
        usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST // todo?
                | wgpu::BufferUsages::COPY_SRC, // todo?
    });

    // Gets the size in bytes of the buffer.
    let size = compute_buf.len() as wgpu::BufferAddress;
    // let size = 1;

    // Instantiates buffer without data.
    // `usage` of buffer specifies how it can be used:
    //   `BufferUsages::MAP_READ` allows it to be read (outside the shader).
    //   `BufferUsages::COPY_DST` allows it to be the destination of the copy.
    let compute_staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Compute staging buffer"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    (
        compute_storage_buf_input,
        compute_storage_buf_output,
        compute_staging_buf,
    )
}
