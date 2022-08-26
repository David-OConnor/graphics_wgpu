struct VertexOutput {
    @builtin(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,

};

// todo: Rename transform A/R to indicate it's for the camera

struct Transform {
    // from camera to screen
    proj: mat4x4<f32>,
    // from screen to camera
    proj_inv: mat4x4<f32>,
    // from world to camera
    view: mat4x4<f32>,
    // camera position
    position: vec4<f32>,
}

@group(0)
@binding(0)
var<uniform> transform: Transform;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
//    @location(2) model: vec4<f32>,
    @builtin(vertex_index) in_vertex_index: u32
) -> VertexOutput {
    var result: VertexOutput;
    result.tex_coord = tex_coord;

    result.position = transform.proj * transform.view * model * (transform.position - position);
    return result;
}

@group(0)
@binding(1)
var r_color: texture_2d<u32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureLoad(r_color, vec2<i32>(vertex.tex_coord * 256.0), 0);
    let v = f32(tex.x) / 255.0;
    return vec4<f32>(1.0 - (v * 5.0), 1.0 - (v * 15.0), 1.0 - (v * 50.0), 1.0);
}

//@fragment
//fn fs_wire(vertex: VertexOutput) -> @location(0) vec4<f32> {
//    return vec4<f32>(0.0, 0.5, 0.0, 0.5);
//}