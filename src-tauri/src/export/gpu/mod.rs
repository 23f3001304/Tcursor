// GPU detection + device/pipeline setup for the wgpu compositor (Task 6b).

/// Bytes per row must be a multiple of this for copy_texture_to_buffer.
pub fn align_up(v: u32, align: u32) -> u32 {
    ((v + align - 1) / align) * align
}

/// True if wgpu can find any adapter on this machine.
pub fn gpu_available() -> bool {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).is_some()
}

/// The fixed-output texture format. Bgra8Unorm keeps bytes in BGRA order so the
/// readback matches CpuCompositor without any channel swap.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// All device-lifetime GPU state. Device/Queue/etc. are Send+Sync in wgpu 22.
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sampler: wgpu::Sampler,
    pub bind_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub out_tex: wgpu::Texture,
    pub out_view: wgpu::TextureView,
    pub readback: wgpu::Buffer,
    pub padded_bpr: u32,
}

impl Gpu {
    /// Create the device/pipeline/output-texture/readback buffer. None if no adapter.
    pub fn new(out_w: u32, out_h: u32) -> Option<Gpu> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        // Raise texture/buffer size limits to the adapter's (downlevel defaults
        // cap 2D textures at 2048, too small for 4K screen frames).
        let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("tcursor-gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .ok()?;

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("samp"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_layout = make_bind_layout(&device);
        let pipeline = make_pipeline(&device, &bind_layout);

        let out_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("out"),
            size: wgpu::Extent3d { width: out_w, height: out_h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let out_view = out_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let padded_bpr = align_up(out_w * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded_bpr * out_h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Some(Gpu { device, queue, sampler, bind_layout, pipeline, out_tex, out_view, readback, padded_bpr })
    }

    /// Create a persistent sampled texture sized w*h.
    pub fn create_tex(&self, label: &str, w: u32, h: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Upload BGRA bytes into an existing persistent texture using write_texture (zero allocation).
    pub fn update_tex(&self, tex: &wgpu::Texture, data: &[u8], w: u32, h: u32) {
        if data.is_empty() || w == 0 || h == 0 { return; }
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }

    /// Upload BGRA bytes into a fresh sampled texture sized w*h.
    pub fn upload_tex(&self, label: &str, data: &[u8], w: u32, h: u32) -> wgpu::Texture {
        let tex = self.create_tex(label, w, h);
        self.update_tex(&tex, data, w, h);
        tex
    }
}

mod gpu_pipeline;
use gpu_pipeline::{make_bind_layout, make_pipeline};

pub mod gpu_compositor;
pub mod gpu_uniforms;
pub mod compositor;
pub mod pool;
