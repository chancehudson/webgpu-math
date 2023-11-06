@group(0)
@binding(0)
var<storage, read_write> inputs: array<u32>; // this is used as both input and output for convenience

/**
 * calculate (in0 * in1) % p
 * in0 and in1 MUST be less than p
 **/
fn mulmod(in0: u32, in1: u32, p: u32) -> u32 {
    var a: u32;
    var b: u32;

    if in0 > in1 {
        a = in1;
        b = in0;
    } else {
        a = in0;
        b = in1;
    }
    var r: u32 = 0u;
    // iterate over the bits of the smaller number
    // and update r as needed
    while a != 0u {
        if (a & 1u) == 1u {
            if b >= (p - r) {
                r -= p - b;
            } else {
                r += b;
            }
        }
        a >>= 1u;
        if b >= (p - b) {
            b -= p - b;
        } else {
            b <<= 1u;
        }
    }
    return r;
}

@compute
@workgroup_size(1)
fn main() {
    inputs[3] = mulmod(inputs[0], inputs[1], inputs[2]);
}
