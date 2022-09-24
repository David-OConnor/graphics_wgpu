//! Experimental module for overlaying an `egui` UI over WGPU
//! Based off [this example](https://github.com/hasenbanck/egui_wgpu_backend)

use crate::init_graphics::GraphicsState;

use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
    fmt::Formatter,
    num::NonZeroU32,
};

use bytemuck::{Pod, Zeroable};

use egui::epaint;
pub use wgpu;
use wgpu::util::DeviceExt;

/// Error that the backend can return.
#[derive(Debug)]
pub enum BackendError {
    /// The given `egui::TextureId` was invalid.
    InvalidTextureId(String),
    /// Internal implementation error.
    Internal(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::InvalidTextureId(msg) => {
                write!(f, "invalid TextureId: `{:?}`", msg)
            }
            BackendError::Internal(msg) => {
                write!(f, "internal error: `{:?}`", msg)
            }
        }
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl GraphicsState {
    pub fn get_texture_bind_group(
        &self,
        texture_id: egui::TextureId,
    ) -> Result<&wgpu::BindGroup, BackendError> {
        self.textures
            .get(&texture_id)
            .ok_or_else(|| {
                BackendError::Internal(format!("Texture {:?} used but not live", texture_id))
            })
            .map(|x| &x.1)
    }

    /// Updates the texture used by egui for the fonts etc. Should be called before `execute()`.
    pub fn add_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &egui::TexturesDelta,
    ) -> Result<(), BackendError> {
        for (texture_id, image_delta) in textures.set.iter() {
            let image_size = image_delta.image.size();

            let origin = match image_delta.pos {
                Some([x, y]) => wgpu::Origin3d {
                    x: x as u32,
                    y: y as u32,
                    z: 0,
                },
                None => wgpu::Origin3d::ZERO,
            };

            let alpha_srgb_pixels: Option<Vec<_>> = match &image_delta.image {
                egui::ImageData::Color(_) => None,
                egui::ImageData::Font(a) => Some(a.srgba_pixels(1.0).collect()),
            };

            let image_data: &[u8] = match &image_delta.image {
                egui::ImageData::Color(c) => bytemuck::cast_slice(c.pixels.as_slice()),
                egui::ImageData::Font(_) => {
                    // The unwrap here should never fail as alpha_srgb_pixels will have been set to
                    // `Some` above.
                    bytemuck::cast_slice(
                        alpha_srgb_pixels
                            .as_ref()
                            .expect("Alpha texture should have been converted already")
                            .as_slice(),
                    )
                }
            };

            let image_size = wgpu::Extent3d {
                width: image_size[0] as u32,
                height: image_size[1] as u32,
                depth_or_array_layers: 1,
            };

            let image_data_layout = wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * image_size.width),
                rows_per_image: None,
            };

            let label_base = match texture_id {
                egui::TextureId::Managed(m) => format!("egui_image_{}", m),
                egui::TextureId::User(u) => format!("egui_user_image_{}", u),
            };

            match self.textures.entry(*texture_id) {
                Entry::Occupied(mut o) => match image_delta.pos {
                    None => {
                        let (texture, bind_group) = create_texture_and_bind_group(
                            device,
                            queue,
                            &label_base,
                            origin,
                            image_data,
                            image_data_layout,
                            image_size,
                            &self.bind_groups.layout_texture,
                        );

                        let (texture, _) = o.insert((Some(texture), bind_group));

                        if let Some(texture) = texture {
                            texture.destroy();
                        }
                    }
                    Some(_) => {
                        if let Some(texture) = o.get().0.as_ref() {
                            queue.write_texture(
                                wgpu::ImageCopyTexture {
                                    texture,
                                    mip_level: 0,
                                    origin,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                image_data,
                                image_data_layout,
                                image_size,
                            );
                        } else {
                            return Err(BackendError::InvalidTextureId(format!(
                                "Update of unmanaged texture {:?}",
                                texture_id
                            )));
                        }
                    }
                },
                Entry::Vacant(v) => {
                    let (texture, bind_group) = create_texture_and_bind_group(
                        device,
                        queue,
                        &label_base,
                        origin,
                        image_data,
                        image_data_layout,
                        image_size,
                        &self.bind_groups.layout_texture,
                    );

                    v.insert((Some(texture), bind_group));
                }
            }
        }

        Ok(())
    }

    /// Remove the textures egui no longer needs. Should be called after `execute()`
    pub fn remove_textures(&mut self, textures: egui::TexturesDelta) -> Result<(), BackendError> {
        for texture_id in textures.free {
            let (texture, _binding) = self.textures.remove(&texture_id).ok_or_else(|| {
                // This can happen due to a bug in egui, or if the user doesn't call `add_textures`
                // when required.
                BackendError::InvalidTextureId(format!(
                    "Attempted to remove an unknown texture {:?}",
                    texture_id
                ))
            })?;

            if let Some(texture) = texture {
                texture.destroy();
            }
        }

        Ok(())
    }

    /// Registers a `wgpu::Texture` with a `egui::TextureId`.
    ///
    /// This enables the application to reference the texture inside an image ui element.
    /// This effectively enables off-screen rendering inside the egui UI. Texture must have
    /// the texture format `TextureFormat::Rgba8UnormSrgb` and
    /// Texture usage `TextureUsage::SAMPLED`.
    pub fn egui_texture_from_wgpu_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
    ) -> egui::TextureId {
        self.egui_texture_from_wgpu_texture_with_sampler_options(
            device,
            texture,
            wgpu::SamplerDescriptor {
                label: Some(
                    format!(
                        "egui_user_image_{}_texture_sampler",
                        self.next_user_texture_id
                    )
                    .as_str(),
                ),
                mag_filter: texture_filter,
                min_filter: texture_filter,
                ..Default::default()
            },
        )
    }

    /// Registers a `wgpu::Texture` with an existing `egui::TextureId`.
    ///
    /// This enables applications to reuse `TextureId`s.
    pub fn update_egui_texture_from_wgpu_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
        id: egui::TextureId,
    ) -> Result<(), BackendError> {
        self.update_egui_texture_from_wgpu_texture_with_sampler_options(
            device,
            texture,
            wgpu::SamplerDescriptor {
                label: Some(
                    format!(
                        "egui_user_image_{}_texture_sampler",
                        self.next_user_texture_id
                    )
                    .as_str(),
                ),
                mag_filter: texture_filter,
                min_filter: texture_filter,
                ..Default::default()
            },
            id,
        )
    }

    /// Registers a `wgpu::Texture` with a `egui::TextureId` while also accepting custom
    /// `wgpu::SamplerDescriptor` options.
    ///
    /// This allows applications to specify individual minification/magnification filters as well as
    /// custom mipmap and tiling options.
    ///
    /// The `Texture` must have the format `TextureFormat::Rgba8UnormSrgb` and usage
    /// `TextureUsage::SAMPLED`. Any compare function supplied in the `SamplerDescriptor` will be
    /// ignored.
    pub fn egui_texture_from_wgpu_texture_with_sampler_options(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler_descriptor: wgpu::SamplerDescriptor,
    ) -> egui::TextureId {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: None,
            ..sampler_descriptor
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(
                format!(
                    "egui_user_image_{}_texture_bind_group",
                    self.next_user_texture_id
                )
                .as_str(),
            ),
            layout: &self.bind_groups.layout_texture,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let id = egui::TextureId::User(self.next_user_texture_id);
        self.textures.insert(id, (None, bind_group));
        self.next_user_texture_id += 1;

        id
    }

    /// Registers a `wgpu::Texture` with an existing `egui::TextureId` while also accepting custom
    /// `wgpu::SamplerDescriptor` options.
    ///
    /// This allows applications to reuse `TextureId`s created with custom sampler options.
    pub fn update_egui_texture_from_wgpu_texture_with_sampler_options(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        sampler_descriptor: wgpu::SamplerDescriptor,
        id: egui::TextureId,
    ) -> Result<(), BackendError> {
        if let egui::TextureId::Managed(_) = id {
            return Err(BackendError::InvalidTextureId(
                "ID was not of type `TextureId::User`".to_string(),
            ));
        }

        let (_user_texture, user_texture_binding) =
            self.textures.get_mut(&id).ok_or_else(|| {
                BackendError::InvalidTextureId(format!(
                    "user texture for TextureId {:?} could not be found",
                    id
                ))
            })?;

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: None,
            ..sampler_descriptor
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(
                format!("egui_user_{}_texture_bind_group", self.next_user_texture_id).as_str(),
            ),
            layout: &self.bind_groups.layout_texture,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        *user_texture_binding = bind_group;

        Ok(())
    }

    /// Executes the egui render pass onto an existing wgpu renderpass.
    pub fn execute_with_renderpass<'rpass>(
        &'rpass self,
        rpass: &mut wgpu::RenderPass<'rpass>,
        paint_jobs: &[egui::epaint::ClippedPrimitive],
        screen_descriptor: &ScreenDescriptor,
    ) -> Result<(), BackendError> {
        rpass.set_pipeline(&self.render_pipeline);

        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);

        let scale_factor = screen_descriptor.scale_factor;
        let physical_width = screen_descriptor.physical_width;
        let physical_height = screen_descriptor.physical_height;

        for (
            (
                egui::ClippedPrimitive {
                    clip_rect,
                    primitive,
                },
                vertex_buffer,
            ),
            index_buffer,
        ) in paint_jobs
            .iter()
            .zip(self.vertex_buf.iter())
            .zip(self.index_buf.iter())
        {
            // Transform clip rect to physical pixels.
            let clip_min_x = scale_factor * clip_rect.min.x;
            let clip_min_y = scale_factor * clip_rect.min.y;
            let clip_max_x = scale_factor * clip_rect.max.x;
            let clip_max_y = scale_factor * clip_rect.max.y;

            // Make sure clip rect can fit within an `u32`.
            let clip_min_x = clip_min_x.clamp(0.0, physical_width as f32);
            let clip_min_y = clip_min_y.clamp(0.0, physical_height as f32);
            let clip_max_x = clip_max_x.clamp(clip_min_x, physical_width as f32);
            let clip_max_y = clip_max_y.clamp(clip_min_y, physical_height as f32);

            let clip_min_x = clip_min_x.round() as u32;
            let clip_min_y = clip_min_y.round() as u32;
            let clip_max_x = clip_max_x.round() as u32;
            let clip_max_y = clip_max_y.round() as u32;

            let width = (clip_max_x - clip_min_x).max(1);
            let height = (clip_max_y - clip_min_y).max(1);

            {
                // Clip scissor rectangle to target size.
                let x = clip_min_x.min(physical_width);
                let y = clip_min_y.min(physical_height);
                let width = width.min(physical_width - x);
                let height = height.min(physical_height - y);

                // Skip rendering with zero-sized clip areas.
                if width == 0 || height == 0 {
                    continue;
                }

                rpass.set_scissor_rect(x, y, width, height);
            }

            if let epaint::Primitive::Mesh(mesh) = primitive {
                let bind_group = self.get_texture_bind_group(mesh.texture_id)?;
                rpass.set_bind_group(1, bind_group, &[]);

                rpass.set_index_buffer(index_buffer.buffer.slice(..), wgpu::IndexFormat::Uint32);
                rpass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));
                rpass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
            }
        }

        Ok(())
    }
}

/// Create a texture and bind group from existing data
#[allow(clippy::too_many_arguments)]
fn create_texture_and_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label_base: &str,
    origin: wgpu::Origin3d,
    image_data: &[u8],
    image_data_layout: wgpu::ImageDataLayout,
    image_size: wgpu::Extent3d,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(format!("{}_texture", label_base).as_str()),
        size: image_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin,
            aspect: wgpu::TextureAspect::All,
        },
        image_data,
        image_data_layout,
        image_size,
    );

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(format!("{}_sampler", label_base).as_str()),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(format!("{}_texture_bind_group", label_base).as_str()),
        layout: texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    (texture, bind_group)
}
