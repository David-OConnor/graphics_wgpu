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
var<storage, read_write> cplx_input: array<Cplx>; // this is used as both input and output for convenience


// Compute the norm squared of a complex number.
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
//    var i: u32 = 0u;
//    loop {
//        if (i == 10) {
//            break;
//        }
//    }
//    v_indices[global_id.x] = 10;
//    v_indices[global_id.x] = collatz_iterations(v_indices[global_id.x]);

}
