// todo: Implement point lights and multiple diffuse lights.

struct Camera {
    proj_view: mat4x4<f32>,
    position: vec3<f32>,
}

struct PointLight {
    position: vec4<f32>,
    diffuse_color: vec4<f32>,
    specular_color: vec4<f32>,
    diffuse_intensity: f32,
    specular_intensity: f32,
}

// Note: Don't us vec3 in uniforms due to alignment issues.
struct Lighting {
    ambient_color: vec4<f32>,
    ambient_intensity: f32,
    point_lights: array<PointLight>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
// We use a storage buffer, since our lighting size is unknown by the shader;
// this is due to the dynamic-sized point light array.
var<storage> lighting: Lighting;

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
    @location(13) shinyness: f32,
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) shinyness: f32,
    //    @location(1) tangent_position: vec3<f32>,
    //    @location(2) tangent_light_position: vec3<f32>,
    //    @location(3) tangent_view_position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexIn,
    instance: InstanceIn,
) -> VertexOut {
    var model_mat = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var normal_mat = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    var world_normal = normalize(normal_mat * model.normal);
//    var world_tangent = normalize(normal_mat * model.tangent);
//    var world_bitangent = normalize(normal_mat * model.bitangent);

// We use the tangent matrix, and tangent out values for normal mapping.
// This is currently unimplemented.
// Construct the tangent matrix
//    var tangent_mat = transpose(mat3x3<f32>(
//        world_tangent,
//        world_bitangent,
//        world_normal,
//    ));

    // Pad the model position with 1., for use with the 4x4 transform mats.
    var world_posit = model_mat * vec4<f32>(model.position, 1.0);

    var result: VertexOut;

    result.position = camera.proj_view * world_posit;

//    result.tangent_position = tangent_mat * world_posit.xyz;
//    result.tangent_view_position = tangent_mat * camera.position.xyz;
    result.normal = world_normal;

    result.color = instance.color;
    result.shinyness = instance.shinyness;

    return result;
}

/// Blinn-Phong shader.
@fragment
fn fs_main(vertex: VertexOut) -> @location(0) vec4<f32> {
    // Ambient lighting
    // todo: Don't multiply ambient for every fragment; do it on the CPU.
    var ambient = lighting.ambient_color * lighting.ambient_intensity;

    // These values include color and intensity
    // todo: More concise constructor?
    var diffuse = vec4<f32>(0., 0., 0., 0.);
    var specular = vec4<f32>(0., 0., 0., 0.);

//    for (var i=0; i < arrayLength(lighting.point_lights); i++) { // todo glitching
    for (var i=0; i < 1; i++) {
        var light = lighting.point_lights[i];

        var diff = vertex.position.xyz - light.position.xyz;

        var light_dir = normalize(diff);
        // This expr applies the inverse square to find falloff with distance.
        var dist_intensity = 1. / (pow(diff.x, 2.) + pow(diff.y, 2.) + pow(diff.z, 2.));

        // Diffuse lighting
        var diffuse_on_face = max(dot(vertex.normal, light_dir), 0.);
        var diffuse = light.diffuse_color * diffuse_on_face * light.diffuse_intensity * dist_intensity;

        // Specular lighting.

        // Lambert's cosine law
        var specular = vec4<f32>(0., 0., 0., 0.);

        if (diffuse_on_face > 0.0) {
            var view_dir = normalize(camera.position.xyz - vertex.position.xyz);

            // Blinn half vector
            var half_dir = normalize(view_dir + light_dir);

            var specular_coeff = pow(max(dot(vertex.normal, half_dir), 0.), vertex.shinyness);

            specular = light.specular_color * specular_coeff * light.specular_intensity * dist_intensity;
        }
    }

    return (ambient + diffuse + specular) * vertex.color;
}