// 128 bit numbers = 4 * 32 bit values
// v[0] = least significant
// v[tuple_size - 1] = most significant
const tuple_size = 4u;

const upper_bitmask: u32 = 0xFFFF0000u;
const lower_bitmask: u32 = 0x0000FFFFu;

@group(0)
@binding(0)
var<storage, read_write> inputs: array<array<u32, tuple_size>, 4>; // this is used as both input and output for convenience

// limb based addition, not modular, overflows wrap
// break each limb into 16 bit sections and add
// take the upper bits as the carry
fn add(
    in0: ptr<function, array<u32, tuple_size>>,
    in1: ptr<function, array<u32, tuple_size>>,
    out: ptr<function, array<u32, tuple_size>>
) {
    var carry: u32;
    var upper: u32;
    var lower: u32;

    for (var i: u32 = 0u; i < tuple_size; i++) {
        // add the lower bits first
        // then add the higher bits
        upper = (((*in0)[i] & upper_bitmask) >> 16u) + (((*in1)[i] & upper_bitmask) >> 16u);
        lower = ((*in0)[i] & lower_bitmask) + ((*in1)[i] & lower_bitmask) + carry;

        // then determine the lower carry
        carry = (lower & upper_bitmask) >> 16u;
        // add the carry to the upper
        upper += carry;

        // determine the total carry for this limb
        carry = (upper & upper_bitmask) >> 16u;

        // combine upper and lower to form final limb
        (*out)[i] = ((upper & lower_bitmask) << 16u) + (lower & lower_bitmask);
    }
}

// negate and add
fn sub(
    out: ptr<function, array<u32, tuple_size>>
) {
    var in0: array<u32, tuple_size> = array(0u, 0u, 0u, 0u);
    var in1: array<u32, tuple_size> = array(0u, 0u, 0u, 0u);
    for (var i: u32 = 0u; i < tuple_size; i++) {
        in0[i] = inputs[0][i];
        in1[i] = inputs[1][i];
    }
    var carry: u32;
    var upper: u32;
    var lower: u32;
    var negated: array<u32, tuple_size>;

    for (var i: u32 = 0u; i < tuple_size; i++) {
        negated[i] = ~in1[i];
    }
    add(&in0, &in1, out);
}

// fn gt(
//     in0: array<u32, tuple_size>,
//     in1: array<u32, tuple_size>
// ) -> bool {
//     let start: u32 = tuple_size - 1u;
//     for (var i: u32 = start; i >= 0u; i++) {
//         if in0[i] > in1[i] {
//             return true;
//         } else if in0[i] < in1[i] {
//             return false;
//         }
//     }
//     return false;
// }

// fn is_zero(v: array<u32, tuple_size>) -> bool {
//     // this should be unrolled at compile time
//     for (var i: u32 = 0u; i < tuple_size; i++) {
//         if v[i] != 0u {
//             return false;
//         }
//     }
//     return true;
// }

// fn is_one(v: array<u32, tuple_size>) -> bool {
//     if (v[0] & 1u) != 1u {
//         return false;
//     }
//     for (var i: u32 = 1u; i < tuple_size; i++) {
//         if v[i] != 0u {
//             return false;
//         }
//     }
//     return true;
// }

// fn shl(v: array<u32, tuple_size>, shift: u32) -> array<u32, tuple_size> {
//     var out: array<u32, tuple_size>;
//     for (var i: u32 = 0u; i < tuple_size; i++) {
//         out[i] = v[i] << shift;
//     }
//     return out;
// }

// operate in place
fn shr(v: ptr<function, array<u32, tuple_size>>, shift: u32) {
    for (var i: u32 = 0u; i < tuple_size - 1u; i++) {
        (*v)[i] >>= 1u;
        (*v)[i] += (*v)[i + 1u] << 31u;
    }
    (*v)[tuple_size - 1u] >>= 1u;
}

/**
 * calculate (in0 * in1) % p
 * in0 and in1 MUST be less than p
 **/
// fn mulmod(
//     in0: array<u32, tuple_size>,
//     in1: array<u32, tuple_size>,
//     p: array<u32, tuple_size>
// ) -> array<u32, tuple_size> {
//     var a: array<u32, tuple_size>;
//     var b: array<u32, tuple_size>;

//     // use the smaller value to iterate fewer times
//     if gt(in0, in1) {
//         a = in1;
//         b = in0;
//     } else {
//         a = in0;
//         b = in1;
//     }
//     var r: array<u32, tuple_size>;
//     // // iterate over the bits of the smaller number
//     // // and update r as needed
//     while !is_zero(a) {
//         if is_one(a) {
//             if b >= (p - r) {
//                 r -= p - b;
//             } else {
//                 r += b;
//             }
//         }
//         a = shr(a);
//         if b >= (p - b) {
//             b -= p - b;
//         } else {
//             b = shl(b);
//         }
//     }
//     return r;
// }

@compute
@workgroup_size(1)
fn main() {
    var out: array<u32, tuple_size>;
    let out_ptr = &out;
    sub(/*&inputs[0], &inputs[1],*/ out_ptr);//, inputs[2]);
    for (var i: u32 = 0u; i < tuple_size; i++) {
        inputs[3][i] = (*out_ptr)[i];
    }
    // inputs[3] = *out_ptr;
}
