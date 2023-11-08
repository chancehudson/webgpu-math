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
    var scratch2: array<u32, tuple_size>;
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

@compute
@workgroup_size(64)
fn main() {
    var in0: array<u32, tuple_size>;
    var in1: array<u32, tuple_size>;
    var p: array<u32, tuple_size>;

    for (var i: u32 = 0u; i < tuple_size; i++) {
        in0[i] = inputs[0][i];
        in1[i] = inputs[1][i];
        p[i] = inputs[2][i];
    }
    // shr(&in0, 1u);
    // inputs[3] = in0;
    inputs[3] = mulmod(&in0, &in1, &p);
    // sub(&in0, &in1, &p);
    // inputs[3] = p;
}
