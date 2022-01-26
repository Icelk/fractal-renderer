fn maybe_watch(
    shader: RustGPUShader,
    on_watch: Option<Box<dyn FnMut(wgpu::ShaderModuleDescriptorSpirV<'static>) + Send + 'static>>,
) -> wgpu::ShaderModuleDescriptorSpirV<'static> {
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    {
        use spirv_builder::{CompileResult, MetadataPrintout, SpirvBuilder};
        use std::borrow::Cow;
        use std::path::PathBuf;
        // Hack: spirv_builder builds into a custom directory if running under cargo, to not
        // deadlock, and the default target directory if not. However, packages like `proc-macro2`
        // have different configurations when being built here vs. when building
        // rustc_codegen_spirv normally, so we *want* to build into a separate target directory, to
        // not have to rebuild half the crate graph every time we run. So, pretend we're running
        // under cargo by setting these environment variables.
        std::env::set_var("OUT_DIR", env!("OUT_DIR"));
        std::env::set_var("PROFILE", env!("PROFILE"));
        let crate_name = match shader {
            RustGPUShader::Simplest => "simplest-shader",
            RustGPUShader::Sky => "sky-shader",
            RustGPUShader::Compute => "compute-shader",
            RustGPUShader::Mouse => "mouse-shader",
        };
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let crate_path = [manifest_dir, "..", "..", "shaders", crate_name]
            .iter()
            .copied()
            .collect::<PathBuf>();
        let builder = SpirvBuilder::new(crate_path, "spirv-unknown-vulkan1.1")
            .print_metadata(MetadataPrintout::None);
        let initial_result = if let Some(mut f) = on_watch {
            builder
                .watch(move |compile_result| f(handle_compile_result(compile_result)))
                .expect("Configuration is correct for watching")
        } else {
            builder.build().unwrap()
        };
        fn handle_compile_result(
            compile_result: CompileResult,
        ) -> wgpu::ShaderModuleDescriptorSpirV<'static> {
            let module_path = compile_result.module.unwrap_single();
            let data = std::fs::read(module_path).unwrap();
            let spirv = Cow::Owned(wgpu::util::make_spirv_raw(&data).into_owned());
            wgpu::ShaderModuleDescriptorSpirV {
                label: None,
                source: spirv,
            }
        }
        handle_compile_result(initial_result)
    }
    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    {
        match shader {
            RustGPUShader::Simplest => wgpu::include_spirv_raw!(env!("simplest_shader.spv")),
            RustGPUShader::Sky => wgpu::include_spirv_raw!(env!("sky_shader.spv")),
            RustGPUShader::Compute => wgpu::include_spirv_raw!(env!("compute_shader.spv")),
            RustGPUShader::Mouse => wgpu::include_spirv_raw!(env!("mouse_shader.spv")),
        }
    }
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            wasm_bindgen_futures::spawn_local(future)
        } else {
            futures::executor::block_on(future)
        }
    }
}

pub fn start(options: &Options) {
    let shader_binary = crate::maybe_watch(options.shader, None);

    block_on(start_internal(options, shader_binary));
}

pub async fn start_internal(
    _options: &Options,
    shader_binary: wgpu::ShaderModuleDescriptorSpirV<'static>,
) {
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");
    drop(instance);
    drop(adapter);

    let timestamp_period = queue.get_timestamp_period();

    // Load the shaders from disk
    let module = unsafe { device.create_shader_module_spirv(&shader_binary) };

    let top = 2u32.pow(20);
    let src_range = 1..top;

    let src = src_range
        .clone()
        .flat_map(u32::to_ne_bytes)
        .collect::<Vec<_>>();

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            // XXX - some graphics cards do not support empty bind layout groups, so
            // create a dummy entry.
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(1).unwrap()),
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                },
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: "main_cs",
    });

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: src.len() as wgpu::BufferAddress,
        // Can be read to the CPU, and can be copied from the shader's storage buffer
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Collatz Conjecture Input"),
        contents: &src,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
    });

    let timestamp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Timestamps buffer"),
        size: 16,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: true,
    });
    timestamp_buffer.unmap();

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
    });

    let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: None,
        count: 2,
        ty: wgpu::QueryType::Timestamp,
    });

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.set_pipeline(&compute_pipeline);
        cpass.write_timestamp(&queries, 0);
        cpass.dispatch(src_range.len() as u32 / 64, 1, 1);
        cpass.write_timestamp(&queries, 1);
    }

    encoder.copy_buffer_to_buffer(
        &storage_buffer,
        0,
        &readback_buffer,
        0,
        src.len() as wgpu::BufferAddress,
    );
    encoder.resolve_query_set(&queries, 0..2, &timestamp_buffer, 0);

    queue.submit(Some(encoder.finish()));
    let buffer_slice = readback_buffer.slice(..);
    let timestamp_slice = timestamp_buffer.slice(..);
    let timestamp_future = timestamp_slice.map_async(wgpu::MapMode::Read);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);

    if let (Ok(()), Ok(())) = join(buffer_future, timestamp_future).await {
        let data = buffer_slice.get_mapped_range();
        let timing_data = timestamp_slice.get_mapped_range();
        let result = data
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>();
        let timings = timing_data
            .chunks_exact(8)
            .map(|b| u64::from_ne_bytes(b.try_into().unwrap()))
            .collect::<Vec<_>>();
        drop(data);
        readback_buffer.unmap();
        drop(timing_data);
        timestamp_buffer.unmap();
        let mut max = 0;
        for (src, out) in src_range.zip(result.iter().copied()) {
            if out == u32::MAX {
                println!("{}: overflowed", src);
                break;
            } else if out > max {
                max = out;
                // Should produce <https://oeis.org/A006877>
                println!("{}: {}", src, out);
            }
        }
        println!(
            "Took: {:?}",
            Duration::from_nanos(
                ((timings[1] - timings[0]) as f64 * f64::from(timestamp_period)) as u64
            )
        );
    }
}
