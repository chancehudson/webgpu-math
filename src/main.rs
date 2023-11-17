use std::{borrow::Cow};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use num_bigint::{BigUint};
use std::time::{Instant, Duration};
use tinytemplate::TinyTemplate;
use serde::{Serialize};

// number of multiplications to do
const ITERATIONS: usize = 4000;
// const INPUT_COUNT: usize = 3;
const LIMB_COUNT: usize = 4;

fn prime() -> (BigUint, BigUint, [u32; LIMB_COUNT], [u32; LIMB_COUNT*2]) {
    let p = BigUint::from(1_u32) + BigUint::from(407_u32) * BigUint::from(2_u32).pow(119_u32);
    let r = BigUint::from(2_u32).pow(256) / p.clone();
    let mut p_slice: [u32; LIMB_COUNT] = [0; LIMB_COUNT];
    for (i, v) in p.to_u32_digits().iter().enumerate() {
        p_slice[i] = v.clone();
    }
    let mut r_slice: [u32; LIMB_COUNT*2] = [0; LIMB_COUNT*2];
    for (i, v) in r.to_u32_digits().iter().enumerate() {
        r_slice[i] = v.clone();
    }
    // for v in p.to_u32_digits() {
    //     println!("{}", v);
    // }
    // println!("break");
    // for v in r.to_u32_digits() {
    //     println!("{}", v);
    // }
    (p, r, p_slice, r_slice)
}

#[cfg(test)]
mod tests;

#[cfg(not(test))]
fn main() {
    env_logger::init();
    prime();
    pollster::block_on(run());
}

#[derive(Serialize)]
struct Context {
    tuple_arr: Vec<u32>,
    tuple_arr_reverse: Vec<usize>,
    tuple_arr_double: Vec<u32>,
    input0: Vec<Vec<u32>>,
    iterations: usize
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Input {
    value0: [[u32; LIMB_COUNT]; ITERATIONS],
    value1: [[u32; LIMB_COUNT]; ITERATIONS],
    value2: [[u32; LIMB_COUNT]; ITERATIONS]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct InputSingle {
    value: [[u32; LIMB_COUNT]; ITERATIONS],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Output {
    values: [[u32; LIMB_COUNT]; ITERATIONS]
}

fn zero_out() -> Output {
    let mut out: [[u32; LIMB_COUNT]; ITERATIONS] = [[0; LIMB_COUNT]; ITERATIONS];
    for i in 0..ITERATIONS {
        for j in 0..LIMB_COUNT {
            out[i][j] = 0;
        }
    }
    Output {
        values: out
    }
}

fn rand() -> Input {
    let mut in0: [[u32; LIMB_COUNT]; ITERATIONS] = [[0; LIMB_COUNT]; ITERATIONS];
    let mut in1: [[u32; LIMB_COUNT]; ITERATIONS] = [[0; LIMB_COUNT]; ITERATIONS];
    let mut in2: [[u32; LIMB_COUNT]; ITERATIONS] = [[0; LIMB_COUNT]; ITERATIONS];
    for i in 0..ITERATIONS {
        // 4 fields for each multiplication
        // in0, in1, p, out
        for j in 0..LIMB_COUNT {
            // in0[i][j] = 0;
            // in1[i][j] = 0;
            // if j == 0 || j == 1 {
            //     in0[i][j] = rand::random::<u32>();
            //     in1[i][j] = rand::random::<u32>();
            // }
            in0[i][j] = rand::random::<u32>();
            in1[i][j] = rand::random::<u32>();
            if j == 3 {
                in0[i][j] = 0x000fffff;
                in1[i][j] = 0x000fffff;
            }
            if j == 0 {
                in2[i][j] = 0xfffffffe;
            } else {
                in2[i][j] = 0xffffffff;
            }
        }
    }
    Input {
        value0: in0,
        value1: in1,
        value2: in2,
    }
}

#[cfg_attr(test, allow(dead_code))]
async fn run() {
    // Instantiates instance of WebGPU
    let instance = wgpu::Instance::default();

    // `request_adapter` instantiates the general connection to the GPU
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await.unwrap();

    // `request_device` instantiates the feature specific connection to the GPU, defining some parameters,
    //  `features` being the available features.
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    // Loads the shader from WGSL
    let input = rand();
    let mut tt = TinyTemplate::new();
    tt.add_template("shader", include_str!("shader.wgsl")).unwrap();
    let context = Context {
        tuple_arr: [0; LIMB_COUNT].to_vec(),
        tuple_arr_reverse: core::array::from_fn::<usize, LIMB_COUNT, _>(|i| LIMB_COUNT - 1 - i).to_vec(),
        tuple_arr_double: [0; LIMB_COUNT*2].to_vec(),
        input0: input.value0.iter().map(|v| v.to_vec()).collect(),
        iterations: ITERATIONS
    };

    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&tt.render("shader", &context).unwrap())),
    });

    // #[cfg(target_arch = "wasm32")]
    // log::info!("Steps: [{}]", disp_steps.join(", "));

    // execute the add function
    {
        // let input = rand();
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &cs_module,
            entry_point: "test_add",
        });
        let (gpu_out, time) = execute_gpu(&pipeline, &device, &queue, &input).await.unwrap();
        println!("gpu: {:.2?}", time);
        let cpu_start = Instant::now();
        let mut out = Vec::new();
        for i in 0..ITERATIONS {
            let in0 = BigUint::new(input.value0[i].to_vec());
            let in1 = BigUint::new(input.value1[i].to_vec());
            // let in2 = BigUint::new(input.value2[i].to_vec());
            // let expected: BigUint = (in0 * in1) % in2; //BigUint::from(2_u32).pow(128);
            let expected: BigUint = (in0 + in1) % BigUint::from(2_u32).pow(128);
            // println!("{}", expected.to_u32_digits().iter().map(|&v| v.to_string()).collect::<Vec<String>>().join(", "));
            // println!("{}", expected.to_string());
            out.push(expected);
        }
        println!("cpu: {:.2?}", cpu_start.elapsed());
        for (i, v) in out.iter().enumerate() {
            if &gpu_out[i] != v {
                println!("output mismatch for index {}. Expected {} got {}", i, v.to_string(), gpu_out[i].to_string());
            }
        }

    }

    // execute the mul function
    {
        // let input = rand();
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &cs_module,
            entry_point: "test_mul",
        });
        let (gpu_out, time) = execute_gpu(&pipeline, &device, &queue, &input).await.unwrap();
        println!("gpu: {:.2?}", time);
        let cpu_start = Instant::now();
        let mut out = Vec::new();
        let (p, _r, _, _) = prime();
        for i in 0..ITERATIONS {
            let in0 = BigUint::new(input.value0[i].to_vec());
            let in1 = BigUint::new(input.value1[i].to_vec());
            // let in2 = BigUint::new(input.value2[i].to_vec());
            // let expected: BigUint = (in0 * in1) % in2; //BigUint::from(2_u32).pow(128);
            let expected: BigUint = (in0.clone() * in1.clone()) % p.clone(); // BigUint::from(2_u32).pow(128);
            // println!("{}", expected.to_u32_digits().iter().map(|&v| v.to_string()).collect::<Vec<String>>().join(", "));
            // println!("{}", expected.to_string());
            out.push(expected.clone());
            // let m = (r.clone() * (in0.clone() * in1.clone())) / BigUint::from(2_u32).pow(256);
            // let z = p.clone() * m;
            // let f = ((in0.clone() * in1.clone()) - z.clone()) % BigUint::from(2_u32).pow(256);
            // out.push(f.clone())
            // println!("{} {}", f.to_string(), expected.to_string());

    // var m = mul_r(v);
    // var z = mul(p, &m);
    // var f: array<u32, tuple_size_double>;
    // sub_double(v, &z, &f);
        }
        println!("cpu: {:.2?}", cpu_start.elapsed());
        for (i, v) in out.iter().enumerate() {
            // println!("{} {}", v.to_str_radix(16), gpu_out[i].to_str_radix(16));
            // let lower = v & BigUint::from(2_u32).pow(128) - BigUint::from(1_u32);
            if &gpu_out[i] != v {
                println!("output mismatch for index {}. Expected {} got {}", i, v.to_str_radix(16), gpu_out[i].to_str_radix(16));
            let in0 = BigUint::new(input.value0[i].to_vec());
            let in1 = BigUint::new(input.value1[i].to_vec());
            println!("{} {}", in0.to_string(), in1.to_string());
            }
        }

    }
}

async fn execute_gpu(
    compute_pipeline: &wgpu::ComputePipeline,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &Input,
) -> Option<(Vec<BigUint>, Duration)> {
    let gpu_start = Instant::now();
    // Gets the size in bytes of the buffer.
    let size = std::mem::size_of_val(&zero_out()) as wgpu::BufferAddress;

    // Instantiates buffer without data.
    // `usage` of buffer specifies how it can be used:
    //   `BufferUsages::MAP_READ` allows it to be read (outside the shader).
    //   `BufferUsages::COPY_DST` allows it to be the destination of the copy.
    let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Instantiates buffer with data (`data`).
    // Usage allowing the buffer to be:
    //   A storage buffer (can be bound within a bind group and thus available to a shader).
    //   The destination of a copy.
    //   The source of a copy.
    let input0_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Storage Buffer"),
        contents: bytemuck::cast_ref::<InputSingle, [u8; 4*ITERATIONS*LIMB_COUNT]>(&InputSingle { value: data.value0 }),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST,
    });
    let input1_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Storage Buffer"),
        contents: bytemuck::cast_ref::<InputSingle, [u8; 4*ITERATIONS*LIMB_COUNT]>(&InputSingle { value: data.value1 }),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST,
    });
    let input2_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Storage Buffer"),
        contents: bytemuck::cast_ref::<InputSingle, [u8; 4*ITERATIONS*LIMB_COUNT]>(&InputSingle { value: data.value2 }),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST,
    });

    let output_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Output Buffer"),
        contents: bytemuck::cast_ref::<Output, [u8; 4*ITERATIONS*LIMB_COUNT]>(&zero_out()),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    // A bind group defines how buffers are accessed by shaders.
    // It is to WebGPU what a descriptor set is to Vulkan.
    // `binding` here refers to the `binding` of a buffer in the shader (`layout(set = 0, binding = 0) buffer`).

    // A pipeline specifies the operation of a shader

    // Instantiates the pipeline.

    // Instantiates the bind group, once again specifying the binding of buffers.
    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: input0_storage_buffer.as_entire_binding(),
        }, wgpu::BindGroupEntry {
            binding: 1,
            resource: input1_storage_buffer.as_entire_binding(),
        }, wgpu::BindGroupEntry {
            binding: 2,
            resource: input2_storage_buffer.as_entire_binding(),
        }, wgpu::BindGroupEntry {
            binding: 3,
            resource: output_storage_buffer.as_entire_binding(),
        }
        ],
    });

    // A command encoder executes one or many pipelines.
    // It is to WebGPU what a command buffer is to Vulkan.
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.insert_debug_marker("modular multiplication");
        cpass.dispatch_workgroups(64, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed
    }
    // Sets adds copy operation to command encoder.
    // Will copy data from storage buffer on GPU to staging buffer on CPU.
    encoder.copy_buffer_to_buffer(&output_storage_buffer, 0, &output_staging_buffer, 0, size);

    // Submits command encoder for processing
    queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_slice = output_staging_buffer.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Ok(Ok(())) = receiver.recv_async().await {
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let output = data.as_ref();

        // manually parse into an input type
        let mut bytes: [u8; 4*ITERATIONS*LIMB_COUNT] = [0; 4*ITERATIONS*LIMB_COUNT];
        for (i, c) in output.iter().enumerate() {
            bytes[i] = *c;
        }

        let result = bytemuck::cast_ref::<[u8; 4*ITERATIONS*LIMB_COUNT], Output>(&bytes).clone();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        output_staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory

        let finish = gpu_start.elapsed();
        Some((result
            .values
            .iter()
            .map(|&n| BigUint::new(n.to_vec()))
            .collect::<Vec<BigUint>>(), finish))

    } else {
        panic!("failed to run compute on gpu!")
    }
}
