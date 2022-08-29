struct Camera {
    proj_view: mat4x4<f32>,
    proj_inv: mat4x4<f32>,
    position: vec4<f32>,
}

struct Lighting {
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    diffuse_color: vec3<f32>,
    diffuse_intensity: f32,
    diffuse_dir: vec3<f32>,
}

struct PointLight {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

// todo: with temp: 64. Without: 48. Why is vec3 16 bytes?

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var<uniform> lighting: Lighting;

//@group(2) @binding(0)
//var<uniform> point_light: PointLight;


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
    @location(1) tangent_position: vec3<f32>,
    @location(2) tangent_light_position: vec3<f32>,
    @location(3) tangent_view_position: vec3<f32>,
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

    let normal_mat = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    // Construct the tangent matrix
    let world_normal = normalize(normal_mat * model.normal);
    let world_tangent = normalize(normal_mat * model.tangent);
    let world_bitangent = normalize(normal_mat * model.bitangent);
    let tangent_mat = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal,
    ));


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
    var ambient = lighting.ambient_color * lighting.ambient_intensity;

    // todo: How do we pass vnormal from the vector?
    var v_normal = vec3<f32>(1., 0., 0.);

//    let light_dir = normalize(in.tangent_light_position - in.tangent_position);
//    let view_dir = normalize(in.tangent_view_position - in.tangent_position);
//    let half_dir = normalize(view_dir + light_dir);

    var diffuse_on_face = max(dot(v_normal, lighting.diffuse_dir), 0.);
    var diffuse = lighting.diffuse_color * diffuse_on_face * lighting.diffuse_intensity;

// todo: Put this in once the rest works.
//    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
//    let specular_color = specular_strength * light.color;
    var specular = vec3<f32>(0., 0., 0.); // todo placeholder


    // todo: Vec4 with opacity?
    var model_color = vec3<f32>(0., 0., 1.); // todo: Temp. Pass in from program.

    let result = (ambient + diffuse + specular) * model_color;

    return vec4<f32>(result, 1.0); // todo: Alpha instead of 1. A/R
}