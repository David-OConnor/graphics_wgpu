struct Camera {
    proj_view: mat4x4<f32>,
    position: vec3<f32>,
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


struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>, // unused
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
}

// These are matrix columns; can't pass matrices directly for vertex attributes.
struct InstanceIn {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
    @location(12) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tangent_position: vec3<f32>,
    @location(2) tangent_light_position: vec3<f32>,
    @location(3) tangent_view_position: vec3<f32>,
    // Experimenting
    @location(4) normal: vec3<f32>,
    @location(5) color: vec4<f32>,
//    @location(6) world_normal: vec3<f32>,
//    @location(7) world_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexIn,
    instance: InstanceIn,
) -> VertexOut {
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

    // Pad the model position with 1., for use with the 4x4 transform mats.
    let world_posit = model_mat * vec4<f32>(model.position, 1.0);

    var result: VertexOut;

    result.clip_position = camera.proj_view * world_posit;

    result.tangent_position = tangent_mat * world_posit.xyz;
    result.tangent_view_position = tangent_mat * camera.position.xyz;
    result.normal = world_normal;

    result.color = instance.color;

//    result.world_normal = normal_mat * model.normal;
//    var world_position: vec4<f32> = model_mat * vec4<f32>(model.position, 1.0);
//    result.world_position = world_position.xyz;

    return result;
}

@fragment
fn fs_main(vertex: VertexOut) -> @location(0) vec4<f32> {
    // Ambient lighting
    // todo: Don't multiply this for every fragment; do it on the CPU.
    // todo: Why isn't the ambient intensity passed from the cpu working?
//    var ambient = lighting.ambient_color * lighting.ambient_intensity;
    var ambient = lighting.ambient_color * 0.05;

    // Note: We currently don't use the model color's alpha value.
    // todo: More elegant way of casting to vec3?
    var vertex_color = vec3<f32>(vertex.color[0], vertex.color[1], vertex.color[2]);

    // Diffuse lighting
    var diffuse_on_face = max(dot(vertex.normal, lighting.diffuse_dir), 0.);
    var diffuse = lighting.diffuse_color * diffuse_on_face * lighting.diffuse_intensity;

    // Specular lighting
//    let light_dir = normalize(vertex.tangent_light_position - vertex.tangent_position);
//    let view_dir = normalize(vertex.tangent_view_position - vertex.tangent_position);
//    let half_dir = normalize(view_dir + light_dir);

//    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
//    let specular_color = specular_strength * light.color;

    // todo: Vec4 with opacity?
//    let result = (ambient + diffuse + specular) * vertex_color;
//    let result = (ambient + diffuse) * vertex_color;

    // todo: How to mix light with face color?
    let result = (ambient + diffuse) * vertex_color;
//    let result = (ambient) * vertex_color;

    return vec4<f32>(result, 1.0);
}