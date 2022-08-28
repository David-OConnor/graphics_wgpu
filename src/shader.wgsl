struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>;
    @location(6) model_matrix_1: vec4<f32>;
    @location(7) model_matrix_2: vec4<f32>;
    @location(8) model_matrix_3: vec4<f32>;
};

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

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>, // unused
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
    // todo: let vs var?
    // todo: Why do we construct the matrix from parts instead of passing whole?
    let model_mat = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

//    result.tex_coords = model.tex_coords;

    // Pad the model position with 1., for use with the 4x4 transform mats.
    var model_posit = vec4<f32>(model.position, 1.0);

    result.clip_position = camera.proj_view * model_mat * model_posit;
//    result.clip_position = camera.proj_view * model_mat * model_posit * (camera.position - model_posit);
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let v = 100.; // todo?

    return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);

}