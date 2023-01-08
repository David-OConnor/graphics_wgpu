@group(0)
@binding(0)
var<storage, read_write> v_indices: array<u32>; // this is used as both input and output for convenience


//fn my_func(n_base: u32) -> u32{
//    return 1;
//}

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
