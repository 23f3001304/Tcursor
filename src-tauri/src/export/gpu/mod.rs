// GPU detection + device/pipeline setup for the wgpu compositor (Task 6b).
use std::sync::{Arc, OnceLock};

/// Bytes per row must be a multiple of this for copy_texture_to_buffer.
pub fn align_up(v: u32, align: u32) -> u32 {
    ((v + align - 1) / align) * align
}

/// The expensive, dims-INDEPENDENT GPU pair - device + queue - built ONCE per process and shared
/// by every `Gpu`/`GpuFx`. `request_device` costs tens-to-hundreds of ms (driver init) and nothing
/// about the device depends on output dims or aspect, so an aspect change, a new export, and the
/// first preview all reuse this instead of paying it again. This is the aspect-change-lag fix:
/// `build_renderer` used to spin up TWO fresh devices (compositor + fx) every time the frame resized.
struct DeviceCore { device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue> }

/// Get-or-create the shared device core. Caches `None` too, so a machine with no usable adapter
/// settles onto the CPU compositor once instead of retrying device creation on every frame.
fn device_core() -> Option<Arc<DeviceCore>> {
    static CORE: OnceLock<Option<Arc<DeviceCore>>> = OnceLock::new();
    CORE.get_or_init(|| {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        // Raise texture/buffer size limits to the adapter's (downlevel defaults cap 2D textures at
        // 2048, too small for 4K screen frames).
        let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("tcursor-gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )).ok()?;
        Some(Arc::new(DeviceCore { device: Arc::new(device), queue: Arc::new(queue) }))
    }).clone()
}

/// The shared device + queue as `Arc` handles all pointing at the SAME underlying device (wgpu's
/// own `Device`/`Queue` are not `Clone`, so we share them behind `Arc`). `None` if this machine has
/// no usable adapter. `Arc<Device>` derefs to `Device`, so call sites use `.device`/`.queue` as before.
pub fn shared_device() -> Option<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
    let core = device_core()?;
    Some((core.device.clone(), core.queue.clone()))
}

/// True if a usable GPU device could be created (warms the shared cache above).
pub fn gpu_available() -> bool {
    device_core().is_some()
}

/// The fixed-output texture format. Bgra8Unorm keeps bytes in BGRA order so the
/// readback matches CpuCompositor without any channel swap.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// All device-lifetime GPU state. Device/Queue/etc. are Send+Sync in wgpu 22.
pub struct Gpu {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub sampler: wgpu::Sampler,
    pub bind_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub out_tex: wgpu::Texture,
    pub out_view: wgpu::TextureView,
    pub readback: wgpu::Buffer,
    pub padded_bpr: u32,
}

impl Gpu {
    /// Build the pipeline/output-texture/readback buffer on the shared device. None if no adapter.
    pub fn new(out_w: u32, out_h: u32) -> Option<Gpu> {
        // Device/queue are the shared, process-global handles (see `shared_device`); only the
        // dims-dependent resources below (output texture + readback buffer) are per-instance, so a
        // frame resize / aspect change no longer recreates the whole device.
        let (device, queue) = shared_device()?;

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

    /// Create a persistent sampled BGRA texture sized w*h.
    pub fn create_tex(&self, label: &str, w: u32, h: u32) -> wgpu::Texture {
        self.create_tex_fmt(label, w, h, FORMAT)
    }

    /// Create a persistent sampled texture of an arbitrary `format` (e.g. `R8Unorm` for the nv12
    /// Y plane, `Rg8Unorm` for the interleaved UV plane).
    pub fn create_tex_fmt(&self, label: &str, w: u32, h: u32, format: wgpu::TextureFormat) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Upload BGRA bytes (4 bytes/px) into an existing persistent texture.
    pub fn update_tex(&self, tex: &wgpu::Texture, data: &[u8], w: u32, h: u32) {
        self.update_tex_bpp(tex, data, w, h, 4);
    }

    /// Upload `bpp`-bytes-per-pixel bytes into an existing persistent texture using write_texture
    /// (zero allocation). `bpp` is 1 for the nv12 Y plane (R8), 2 for the UV plane (Rg8), 4 for BGRA.
    pub fn update_tex_bpp(&self, tex: &wgpu::Texture, data: &[u8], w: u32, h: u32, bpp: u32) {
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
                bytes_per_row: Some(w * bpp),
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
