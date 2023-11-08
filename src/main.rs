use std::{borrow::Cow};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};
use num_bigint::{BigUint};

const LIMB_COUNT: usize = 4;

#[cfg(test)]
mod tests;

#[cfg(not(test))]
fn main() {
    env_logger::init();
    pollster::block_on(run());
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Input {
    values: [[u32; LIMB_COUNT]; 4]
}

fn rand() -> Input {
    let mut out: [[u32; LIMB_COUNT]; 4] = [[0; 4]; 4];
    for i in 0..4 {
        for j in 0..LIMB_COUNT {
            out[i][j] = rand::random::<u32>();
        }
    }
    out[2][0] = 0xffffffff;
    out[2][1] = 0xffffffff;
    out[2][2] = 0xffffffff;
    out[2][3] = 0xffffffdf;
    Input {
        values: out
    }
}

#[cfg_attr(test, allow(dead_code))]
async fn run() {
    let input = rand();

    let steps = execute_gpu(&input).await.unwrap();
    let in0 = BigUint::new(input.values[0].to_vec());
    let in1 = BigUint::new(input.values[1].to_vec());
    let in2 = BigUint::new(input.values[2].to_vec());
    let expected: BigUint = (in0 * in1) % in2; //BigUint::from(2_u32).pow(128);
    println!("{}", expected.to_u32_digits().iter().map(|&v| v.to_string()).collect::<Vec<String>>().join(", "));

    let disp_steps: Vec<String> = steps
        .values
        .iter()
        .map(|&n| n.iter().map(|&v| v.to_string()).collect::<Vec<String>>().join(", "))
        .collect();

    println!("Steps: [{}]", disp_steps.join(", "));
    #[cfg(target_arch = "wasm32")]
    log::info!("Steps: [{}]", disp_steps.join(", "));
}

#[cfg_attr(test, allow(dead_code))]
async fn execute_gpu(numbers: &Input) -> Option<Input> {
    // Instantiates instance of WebGPU
    let instance = wgpu::Instance::default();

    // `request_adapter` instantiates the general connection to the GPU
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;

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

    execute_gpu_inner(&device, &queue, numbers).await
}

async fn execute_gpu_inner(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    numbers: &Input,
) -> Option<Input> {
    // Loads the shader from WGSL
    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    // Gets the size in bytes of the buffer.
    let size = std::mem::size_of_val(numbers) as wgpu::BufferAddress;

    // Instantiates buffer without data.
    // `usage` of buffer specifies how it can be used:
    //   `BufferUsages::MAP_READ` allows it to be read (outside the shader).
    //   `BufferUsages::COPY_DST` allows it to be the destination of the copy.
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Instantiates buffer with data (`numbers`).
    // Usage allowing the buffer to be:
    //   A storage buffer (can be bound within a bind group and thus available to a shader).
    //   The destination of a copy.
    //   The source of a copy.
    let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Storage Buffer"),
        contents: bytemuck::cast_ref::<Input, [u8; 64]>(numbers),
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    // A bind group defines how buffers are accessed by shaders.
    // It is to WebGPU what a descriptor set is to Vulkan.
    // `binding` here refers to the `binding` of a buffer in the shader (`layout(set = 0, binding = 0) buffer`).

    // A pipeline specifies the operation of a shader

    // Instantiates the pipeline.
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &cs_module,
        entry_point: "main",
    });

    // Instantiates the bind group, once again specifying the binding of buffers.
    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
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
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.insert_debug_marker("compute collatz iterations");
        cpass.dispatch_workgroups(1, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed
    }
    // Sets adds copy operation to command encoder.
    // Will copy data from storage buffer on GPU to staging buffer on CPU.
    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);

    // Submits command encoder for processing
    queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_slice = staging_buffer.slice(..);
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
        let mut bytes: [u8; 64] = [0; 64];
        for (i, c) in output.iter().enumerate() {
            bytes[i] = *c;
        }

        let result = bytemuck::cast_ref::<[u8; 64], Input>(&bytes).clone();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory

        // Returns data from buffer
        Some(result)
    } else {
        panic!("failed to run compute on gpu!")
    }
}
