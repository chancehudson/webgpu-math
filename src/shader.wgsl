// 128 bit numbers = 4 * 32 bit values
// v[0] = least significant
// v[tuple_size - 1] = most significant
const tuple_size = 4u;
const tuple_size_double = 2u * tuple_size;

const iterations = 512u;

@group(0)
@binding(0)
var<storage, read> input0: array<array<u32, tuple_size>, iterations>;

@group(0)
@binding(1)
var<storage, read> input1: array<array<u32, tuple_size>, iterations>;

@group(0)
@binding(2)
var<storage, read> input2: array<array<u32, tuple_size>, iterations>;

@group(0)
@binding(3)
var<storage, read_write> outputs: array<array<u32, tuple_size>, iterations>;

// limb based addition, not modular, overflows wrap
// break each limb into 16 bit sections and add
// take the upper bits as the carry
fn add(
    in0: ptr<function, array<u32, tuple_size>>,
    in1: ptr<function, array<u32, tuple_size>>,
    out: ptr<function, array<u32, tuple_size>>
) {
    var carry: u32;

    for (var i: u32 = 0u; i < tuple_size; i++) {
        (*out)[i] = (*in0)[i] + (*in1)[i] + carry;
        carry = u32(((*out)[i] < (*in0)[i]) || ((*out)[i] < (*in0)[i]));
    }
}

// negate and add
fn sub(
    in0: ptr<function, array<u32, tuple_size>>,
    in1: ptr<function, array<u32, tuple_size>>,
    out: ptr<function, array<u32, tuple_size>>
) {
    var negated: array<u32, tuple_size>;
    for (var i: u32 = 0u; i < tuple_size; i++) {
        negated[i] = ~(*in1)[i];
    }
    add(in0, &negated, out);
}

fn gte(
    in0: ptr<function, array<u32, tuple_size>>,
    in1: ptr<function, array<u32, tuple_size>>
) -> bool {
    let start: u32 = tuple_size - 1u;
    for (var i: u32 = start; i >= 0u; i++) {
        if (*in0)[i] > (*in1)[i] {
            return true;
        } else if (*in0)[i] < (*in1)[i] {
            return false;
        }
    }
    return true;
}

fn is_zero(v: ptr<function, array<u32, tuple_size>>) -> bool {
    // this should be unrolled at compile time
    for (var i: u32 = 0u; i < tuple_size; i++) {
        if (*v)[i] != 0u {
            return false;
        }
    }
    return true;
}

fn is_odd(v: ptr<function, array<u32, tuple_size>>) -> bool {
    return ((*v)[0] & 1u) == 1u;
}

fn shl(v: ptr<function, array<u32, tuple_size>>, shift: u32) {
    for (var i: u32 = 0u; i < tuple_size - 1u; i++) {
        var _i = tuple_size - 1u - i;
        (*v)[_i] <<= 1u;
        (*v)[_i] += (*v)[_i - 1u] >> 31u;
    }
    (*v)[0u] <<= 1u;
}

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
 * https://en.wikipedia.org/wiki/Ancient_Egyptian_multiplication
 **/
fn mulmod(
    in0: ptr<function, array<u32, tuple_size>>,
    in1: ptr<function, array<u32, tuple_size>>,
    p: ptr<function, array<u32, tuple_size>>
) -> array<u32, tuple_size> {
    let a = in0;
    let b = in1;

    // use the smaller value to iterate fewer times

    // if gte(in0, in1) {
    //     // a = in1;
    //     // b = in0;
    // } else {
    //     // a = in0;
    //     // b = in1;
    // }

    var r: array<u32, tuple_size>;
    var scratch1: array<u32, tuple_size>;
    // var scratch2: array<u32, tuple_size>;
    // // iterate over the bits of the smaller number
    // // and update r as needed
    while !is_zero(a) {
        if is_odd(a) {
            sub(p, &r, &scratch1);
            if gte(b, &scratch1) {
                // r -= p - b;
                sub(p, b, &scratch1);
                sub(&r, &scratch1, &r);
            } else {
                // r += b
                add(&r, b, &r);
            }
        }
        shr(a, 1u);
        sub(p, b, &scratch1);
        if gte(b, &scratch1) {
            // b -= p - b;
            sub(b, &scratch1, b);
        } else {
            shl(b, 1u);
        }
    }
    return r;
}

// multiply and store result in large buffer
// then use barret reduction
fn mulmod_2(
    in0: ptr<function, array<u32, tuple_size>>,
    in1: ptr<function, array<u32, tuple_size>>,
    p: ptr<function, array<u32, tuple_size>>
) {
    var prod: array<u32, tuple_size_double>;

    for (var i: u32 = 0u; i < tuple_size; i++) {

    }

}

// @compute
// @workgroup_size(64)
// fn test_sub(
//     @builtin(global_invocation_id) global_id: vec3<u32>
// ) {
//     var in0: array<u32, tuple_size>;
//     var in1: array<u32, tuple_size>;
//     var p: array<u32, tuple_size>;
//     // let offset = tuple_size * global_id.x;

//     for (var i: u32 = 0u; i < tuple_size; i++) {
//         in0[i] = inputs[global_id.x * tuple_size][i];
//         in1[i] = inputs[global_id.x * tuple_size + 1u][i];
//         p[i] = inputs[global_id.x * tuple_size + 2u][i];
//     }
//     sub(&in0, &in1, &p);
//     outputs[global_id.x * tuple_size] = p;
// }

@compute
@workgroup_size(64)
fn test_add(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    var in0: array<u32, tuple_size>;
    var in1: array<u32, tuple_size>;
    var p: array<u32, tuple_size>;

    for (var i: u32 = 0u; i < tuple_size; i++) {
        in0[i] = input0[global_id.x][i];
        in1[i] = input1[global_id.x][i];
        p[i] = input2[global_id.x][i];
    }
    add(&in0, &in1, &p);
    outputs[global_id.x] = p;
}

@compute
@workgroup_size(64)
fn test_mulmod(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    var in0: array<u32, tuple_size>;
    var in1: array<u32, tuple_size>;
    var p: array<u32, tuple_size>;
    // let offset = tuple_size * global_id.x;

    for (var i: u32 = 0u; i < tuple_size; i++) {
        in0[i] = input0[global_id.x][i];
        in1[i] = input1[global_id.x][i];
        p[i] = input2[global_id.x][i];
    }
    outputs[global_id.x] = mulmod(&in0, &in1, &p);
}
