//! This module contains code specific to compute shader operations.

use wgpu::{self, util::DeviceExt};

use futures_intrusive; // todo get rid of this once you can. For converting compute buf to [u8];

// todo: Temp test for compute
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cplx {
    real: f32,
    im: f32, // todo: How to f64 in shader?
}

impl Cplx {
    pub fn new(real: f32, im: f32) -> Self {
        Self { real, im }
    }
}

impl Cplx {
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut result = [0; 8];

        result[0..4].clone_from_slice(&self.real.to_ne_bytes());
        result[4..8].clone_from_slice(&self.im.to_ne_bytes());

        result
    }

    pub fn from_bytes(buf: &[u8]) -> Self {
        Self {
            real: f32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]),
            im: f32::from_ne_bytes([buf[4], buf[5], buf[6], buf[7]]),
        }
    }
}

/// Temporary test data
fn create_test_data<'a>(compute_buf: &mut [u8]) {
    // Set up test input of complex numbers.
    let compute_input = vec![
        Cplx::new(1., 1.),
        Cplx::new(2., 2.),
        Cplx::new(3., 3.),
        Cplx::new(4., 4.),
        Cplx::new(5., 4.),
        Cplx::new(6., 0.),
        Cplx::new(-2., 2.),
        Cplx::new(3., 4.),
        Cplx::new(1., 0.),
        Cplx::new(0., 1.),
    ];

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

    // Gets the size in bytes of the buffer.
    let size = compute_buf.len() as wgpu::BufferAddress;

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

    // For our WIP compute functionality.
    let compute_storage_buf_input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Compute storage buffer input"),
        contents: &compute_buf,
        usage: wgpu::BufferUsages::STORAGE
    });

    let compute_storage_buf_output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Compute storage buffer output"),
        contents: &compute_buf_out,
        usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
    });

    (
        compute_storage_buf_input,
        compute_storage_buf_output,
        compute_staging_buf,
    )
}

pub(crate) fn create_bindgroups(
    device: &wgpu::Device,
    storage_buf_input: &wgpu::Buffer,
    storage_buf_output: &wgpu::Buffer,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            // Input
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    // The dynamic field indicates whether this buffer will change size or
                    // not. This is useful if we want to store an array of things in our uniforms.
                    has_dynamic_offset: false,
                    min_binding_size: None,
                    // todo: Setting size here may be more efficient, since it runs at draw time if None
                    // min_binding_size: wgpu::BufferSize::new((80) as _),
                },
                count: None,
            },
            // Output
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
        label: Some("Compute bind group layout"),
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buf_input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: storage_buf_output.as_entire_binding(),
            },
        ],
        label: Some("Compute bind group"),
    });

    (layout, bind_group)
}

// pub fn buf_to_cpu(
//     // self,
//     buf: &wgpu::Buffer,
//     device: &wgpu::Device,
//     queue: &wgpu::Queue,
// ) -> Result<Tensor, GpuError> {
//     let buffer_slice = buf.slice(..);
//
//     // let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
//
//     wgpu::util::DownloadBuffer::read_buffer(device, queue, &buffer_slice, move |buffer| {
//         // tx.send(match buffer {
//         //     Ok(bytes) => Ok(Self::read_to_host(self.shape, self.dt, &bytes)),
//         //     Err(error) => Err(GpuError::BufferAsyncError(error)),
//         // })
//         // .unwrap();
//     });
//
//     device.poll(wgpu::Maintain::Wait);
//     // rx.receive().await.unwrap()
// }

/// Convert a WGPU buffer to a byte array; intended to return data after a compute pass.
/// Buf is, eg, the staging buffer.
pub(crate) fn buf_to_vec(buf: &wgpu::Buffer, device: &wgpu::Device) -> Vec<u8> {
    let buffer_slice = buf.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device.poll(wgpu::Maintain::Wait);

    let data = buffer_slice.get_mapped_range();
    let result = data.to_vec();

    // With the current interface, we have to make sure all mapped views are
    // dropped before we unmap the buffer.
    drop(data);

    buf.unmap(); // Unmaps buffer from memory
                 // If you are familiar with C++ these 2 lines can be thought of similarly to:
                 //   delete myPointer;
                 //   myPointer = NULL;
                 // It effectively frees the memory

    result
}
