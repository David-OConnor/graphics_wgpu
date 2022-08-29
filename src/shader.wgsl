struct Camera {
    // from camera to screen
    proj_view: mat4x4<f32>,
    // from screen to camera
    proj_inv: mat4x4<f32>,
    // from world to camera
//    view: mat4x4<f32>,
    // camera position
    position: vec4<f32>,
}

struct Lighting {
    ambient_color: vec3<f32>,
    ambient_brightness: f32,
    diffuse_color: vec3<f32>,
    diffuse_brightness: f32,
    diffuse_dir: vec3<f32>,
    temp: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> lighting: Lighting;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>, // unused
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var result: VertexOutput;
    // todo: Why do we construct the matrix from parts instead of passing whole?
    var model_mat = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

//    result.tex_coords = model.tex_coords;

    // Pad the model position with 1., for use with the 4x4 transform mats.
    var model_posit = vec4<f32>(model.position, 1.0);

    result.clip_position = camera.proj_view * model_mat * model_posit;

    // todo: How?
//    v_normal = transpose(inverse(mat3(uniforms.r_model))) * -normal;

    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    // todo: Are these already normalized? dot vs * ?
//    var brightness = dot(normalize(v_normal), normalize(diffuse_direction));

    // todo: How do we pass vnormal from the vector?
    var v_normal = vec3<f32>(1., 0., 0.);
    var brightness = dot(v_normal, lighting.diffuse_dir);

    // todo: Vec4 with opacity?
    var model_color = vec3<f32>(0., 0., 1.); // todo: Temp. Pass in from program.
//    var regular_color = vec3(face_color2);

    return vec4<f32>(mix(lighting.ambient_color, model_color, brightness), 1.0);
}