struct CameraUniform {
    // from camera to screen
    proj_view: mat4x4<f32>,
    // from screen to camera
    proj_inv: mat4x4<f32>,
    // from world to camera
//    view: mat4x4<f32>,
    // camera position
    position: vec4<f32>,
}

@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var result: VertexOutput;
    result.tex_coords = model.tex_coords;

    // Pad the model position with 1., for use with the 4x4 transform mats.
    var model_posit = vec4<f32>(model.position, 1.0);

    result.clip_position = camera.proj_view * model_posit * (camera.position - model_posit);
    return result;
}

@group(0)
@binding(1)
var r_color: texture_2d<u32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
//    let tex = textureLoad(r_color, vec2<i32>(vertex.tex_coord * 256.0), 0);
//    let v = f32(tex.x) / 255.0;
//    return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);

    let v = 100.; // todo?

    return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);

}