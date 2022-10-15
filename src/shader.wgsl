// Reference: https://www.w3.org/TR/WGSL

struct Camera {
    proj_view: mat4x4<f32>,
    position: vec4<f32>,
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
    // We use this as a workaround for array len not working.
    lights_len: i32,
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

// These are matrix columns; we can't pass matrices directly for vertex attributes.
struct InstanceIn {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
    @location(12) color: vec3<f32>, // Len 3: No alpha.
    @location(13) shinyness: f32,
}

struct VertexOut {
    @builtin(position) clip_posit: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) shinyness: f32,
    @location(4) world_posit: vec3<f32>, // todo: Experimenting
//        @location(1) tangent_position: vec3<f32>,
//        @location(2) tangent_light_position: vec3<f32>,
//        @location(3) tangent_view_position: vec3<f32>,
}

@vertex
fn vs_main(
    vertex_in: VertexIn,
    instance: InstanceIn,
) -> VertexOut {
    // The model matrix includes translation, rotation, and scale.
    var model_mat = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    // The normal matrix includes rotation only.
    var normal_mat = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    // "the transpose of the inverse of the upper-left 3x3 part of the model matrix"
//    var model_mat_3 = mat3x3<f32>(
//        instance.model_matrix_0.xyz,
//        instance.model_matrix_1.xyz,
//        instance.model_matrix_2.xyz,
//    );

    // todo: Constructing normal mat here to troubleshoot
//    var normal_mat = model_mat_3;

    // Note that the normal matrix is just the 3x3 rotation matrix, unless
    // non-uniform scaling is used; that's when we need the inverse transpose.
    // In either case, you should probably do that on the CPU.
//    var normal_mat = inverse(transpose(model_mat_3));


    // todo: Is this right?
    // We use the tangent matrix, and tangent out values for normal mapping.
    // This is currently unimplemented.
    var world_normal = normalize(normal_mat * vertex_in.normal);
    var world_tangent = normalize(normal_mat * vertex_in.tangent);
    var world_bitangent = normalize(normal_mat * vertex_in.bitangent);

// Construct the tangent matrix
    var tangent_mat = transpose(mat3x3<f32>(
        world_tangent,
        world_bitangent,
        world_normal,
    ));

    // Pad the model position with 1., for use with the 4x4 transform mats.
    var world_posit = model_mat * vec4<f32>(vertex_in.position, 1.0);

    var result: VertexOut;

    result.clip_posit = camera.proj_view * world_posit;

//    result.tangent_position = tangent_mat * world_posit.xyz;
//    result.tangent_view_position = tangent_mat * camera.position.xyz;
//    result.tangent_light_position = tangent_matrix * light.position;
    result.normal = world_normal;

    result.color = vec4<f32>(instance.color, 1.);
    result.shinyness = instance.shinyness;
    result.world_posit = world_posit.xyz;

    return result;
}

/// Fragment shader, which is mostly lighting calculations.
@fragment
fn fs_main(vertex: VertexOut) -> @location(0) vec4<f32> {
    // Ambient lighting
    // todo: Don't multiply ambient for every fragment; do it on the CPU.
    var ambient = lighting.ambient_color * lighting.ambient_intensity;

    // todo: Emmissive term?

//    let tangent_normal = object_normal.xyz * 2.0 - 1.0;

    // These values include color and intensity
    var diffuse = vec4<f32>(0., 0., 0., 0.);
    var specular = vec4<f32>(0., 0., 0., 0.);

    // todo: arrayLength on this variable is not working. Use size passed from CPU in the
    // todo meanwhile.
//    for (var i=0; i < arrayLength(lighting.point_lights); i++) {
    for (var i=0; i < lighting.lights_len; i++) {
        var light = lighting.point_lights[i];

        // Direction from light to the vertex; we use this to calculate attentiation,
        // and diffuse-lighting cosine loss.

        var light_to_vert_diff =  vertex.world_posit.xyz - light.position.xyz;

        var light_to_vert_dir = normalize(light_to_vert_diff);

        // This expr applies the inverse square to find falloff with distance.
        // Note that we use the word "attenuation" in perhaps the inverse of how we usually use it; 1.0
        // is full intensity here.
        var dist_attenuation = 1. / (pow(light_to_vert_diff.x, 2.) + pow(light_to_vert_diff.y, 2.) + pow(light_to_vert_diff.z, 2.));

        // Diffuse lighting. This is essentially cosine los.
        var diffuse_attenuation = max(dot(vertex.normal, -light_to_vert_dir), 0.);
        diffuse += light.diffuse_color * diffuse_attenuation * light.diffuse_intensity * dist_attenuation;

        // Specular lighting.
        var specular_this_light = vec4<f32>(0., 0., 0., 0.);

        if (diffuse_attenuation > 0.0) {
            var view_dir = normalize(camera.position.xyz - vertex.world_posit.xyz);

//          // Blinn half vector
            var half_dir = normalize(view_dir + light_to_vert_dir);

            var specular_coeff = pow(max(dot(vertex.normal, half_dir), 0.), vertex.shinyness);

            specular_this_light = light.specular_color * specular_coeff * light.specular_intensity * dist_attenuation;
            specular += specular_this_light;
        }
    }

//    return (ambient + diffuse + specular) * vertex.color;
    var result = (ambient + diffuse) * vertex.color;

    return vec4<f32>(result.rgb, 1.); // TS by removing alpha channel.
}