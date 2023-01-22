struct F64 {
    ms: f32,
    ls: f32,
}

// todo: f64 not working? SIPRV passthrough for it? For now, use f32s.
struct Cplx {
    real: f32,
    im: f32,
}

@group(0)
@binding(0)
var<storage, read> cplx_input: array<Cplx>;
//var<storage, read_write> cplx_input: array<Cplx>;

@group(0)
@binding(1)
var<storage, write> cplx_output: array<f32>;


// Multiply a complex number.
fn mul_cplx(v1: Cplx, v2: Cplx) -> Cplx {
    return Cplx (
        v1.real * v2.real - v1.im * v2.im,
        v1.real * v2.im + v1.im * v2.real
    );
}

// Compute the norm squared of a complex number.
fn norm_sq(v: Cplx) -> f32 {
    var conj = Cplx ( v.real, -v.im );
    return mul_cplx(conj, v).real;
}

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var i: i32 = 0;
    var len: i32 = 10; // todo: Pass in?

    loop {
        if (i == len) {
            break;
        }
        cplx_output[i] = norm_sq(cplx_input[i]);
    }
}
