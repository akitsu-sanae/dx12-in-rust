pub mod command;

use std::mem::{size_of_val, zeroed};
use std::ptr::{null, null_mut};
use widestring::U16CString;
use winapi::{
    ctypes::c_void,
    shared::{
        dxgi::DXGI_SWAP_CHAIN_DESC,
        dxgi1_2::IDXGISwapChain1,
        dxgi1_5::IDXGISwapChain4,
        dxgi1_6::IDXGIFactory6,
        minwindef::{BOOL, UINT},
    },
    um::{d3d12::*, unknwnbase::IUnknown},
    Interface,
};

use crate::{math, util::*, window::Window};

pub struct Direct3D {
    device: *mut ID3D12Device,
    swapchain: *mut IDXGISwapChain4,
    rtv_heaps: *mut ID3D12DescriptorHeap,
    back_buffers: Vec<*mut ID3D12Resource>,
    command_manager: command::CommandManager,

    pipeline_state: *mut ID3D12PipelineState,
    root_signature: *mut ID3D12RootSignature,

    viewport: D3D12_VIEWPORT,
    scissorrect: D3D12_RECT,
    frame: usize,
    vb_view: D3D12_VERTEX_BUFFER_VIEW,
    ib_view: D3D12_INDEX_BUFFER_VIEW,
}

fn create_factory() -> Result<*mut IDXGIFactory6, String> {
    use winapi::shared::dxgi1_3::{CreateDXGIFactory2, DXGI_CREATE_FACTORY_DEBUG};
    let mut factory: *mut IDXGIFactory6 = null_mut();
    let result = unsafe {
        CreateDXGIFactory2(
            DXGI_CREATE_FACTORY_DEBUG,
            &IDXGIFactory6::uuidof(),
            &mut factory as *mut *mut _ as *mut *mut c_void,
        )
    };
    if is_succeeded(result) {
        Ok(factory)
    } else {
        let result = unsafe {
            CreateDXGIFactory2(
                0,
                &IDXGIFactory6::uuidof(),
                &mut factory as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_succeeded(result) {
            Ok(factory)
        } else {
            Err("failed: create DXGIFactory".to_string())
        }
    }
}

fn create_device() -> Result<*mut ID3D12Device, String> {
    let mut device: *mut ID3D12Device = null_mut();
    use winapi::um::d3dcommon::*;
    let feature_levels = [
        D3D_FEATURE_LEVEL_12_1,
        D3D_FEATURE_LEVEL_12_0,
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
    ];
    for level in feature_levels.iter() {
        let result = unsafe {
            D3D12CreateDevice(
                null_mut(),
                *level,
                &ID3D12Device::uuidof(),
                &mut device as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_succeeded(result) {
            return Ok(device);
        }
    }
    Err("failed: create D3D12CreateDevice".to_string())
}

fn create_swapchain(
    factory: *mut IDXGIFactory6,
    command_manager: &command::CommandManager,
    window: &Window,
) -> Result<*mut IDXGISwapChain4, String> {
    use winapi::shared::{
        dxgi::{DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH, DXGI_SWAP_EFFECT_FLIP_DISCARD},
        dxgi1_2::{DXGI_ALPHA_MODE_UNSPECIFIED, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1},
        dxgiformat::DXGI_FORMAT_R8G8B8A8_UNORM,
        dxgitype::{DXGI_SAMPLE_DESC, DXGI_USAGE_BACK_BUFFER},
    };
    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: window.width as UINT,
        Height: window.height as UINT,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        Stereo: false as BOOL,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1 as UINT,
            Quality: 0 as UINT,
        },
        BufferUsage: DXGI_USAGE_BACK_BUFFER,
        BufferCount: 2 as UINT,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_UNSPECIFIED,
        Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH,
    };

    let mut swapchain: *mut IDXGISwapChain4 = null_mut();
    let result = unsafe {
        (*factory).CreateSwapChainForHwnd(
            command_manager.get_queue() as *mut _ as *mut IUnknown,
            window.handle,
            &desc,
            null(),
            null_mut(),
            &mut swapchain as *mut *mut _ as *mut *mut IDXGISwapChain1,
        )
    };
    if is_succeeded(result) {
        Ok(swapchain)
    } else {
        Err("failed: create IDXGIFactory6".to_string())
    }
}

fn create_rtv_heaps(dev: *mut ID3D12Device) -> Result<*mut ID3D12DescriptorHeap, String> {
    let desc = D3D12_DESCRIPTOR_HEAP_DESC {
        Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV, // render target view
        NumDescriptors: 2,
        Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
        NodeMask: 0,
    };
    let mut rtv_heaps: *mut ID3D12DescriptorHeap = null_mut();
    let result = unsafe {
        (*dev).CreateDescriptorHeap(
            &desc,
            &ID3D12DescriptorHeap::uuidof(),
            &mut rtv_heaps as *mut *mut _ as *mut *mut c_void,
        )
    };
    if is_succeeded(result) {
        Ok(rtv_heaps)
    } else {
        Err("failed: create ID3D12DescriptorHeap".to_string())
    }
}

fn create_back_buffers(
    device: *mut ID3D12Device,
    swapchain: *mut IDXGISwapChain4,
    rtv_heaps: *mut ID3D12DescriptorHeap,
) -> Result<Vec<*mut ID3D12Resource>, String> {
    let mut swapchain_desc: DXGI_SWAP_CHAIN_DESC = unsafe { zeroed() };
    let result = unsafe { (*swapchain).GetDesc(&mut swapchain_desc) };
    if is_failed(result) {
        return Err("failed: get swapchain descriptor".to_string());
    }
    let mut back_buffers: Vec<*mut ID3D12Resource> = vec![];
    back_buffers.resize(swapchain_desc.BufferCount as usize, null_mut());
    let mut handle: D3D12_CPU_DESCRIPTOR_HANDLE =
        unsafe { (*rtv_heaps).GetCPUDescriptorHandleForHeapStart() };
    for i in { 0..swapchain_desc.BufferCount } {
        let result = unsafe {
            (*swapchain).GetBuffer(
                i,
                &ID3D12Resource::uuidof(),
                &mut back_buffers[i as usize] as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_failed(result) {
            return Err("failed: get buffer".to_string());
        }
        unsafe { (*device).CreateRenderTargetView(back_buffers[i as usize], null(), handle) }
        handle.ptr +=
            unsafe { (*device).GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) }
                as usize;
    }
    Ok(back_buffers)
}

impl Direct3D {
    pub fn create(window: &Window) -> Result<Direct3D, String> {
        let factory = create_factory()?;
        let device = create_device()?;
        let command_manager = command::CommandManager::create(device)?;
        let swapchain = create_swapchain(factory, &command_manager, window)?;
        let rtv_heaps = create_rtv_heaps(device)?;
        let back_buffers = create_back_buffers(device, swapchain, rtv_heaps)?;

        use winapi::shared::{
            dxgiformat::{
                DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R8G8B8A8_UNORM,
                DXGI_FORMAT_UNKNOWN,
            },
            dxgitype::DXGI_SAMPLE_DESC,
        };
        use winapi::um::d3dcommon::ID3DBlob;
        let vertices = [
            math::Vec3::new(-1.0f32, -1.0f32, 0.0f32),
            math::Vec3::new(-1.0f32, 1.0f32, 0.0f32),
            math::Vec3::new(1.0f32, -1.0f32, 0.0f32),
        ];
        let heapprop = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_UPLOAD,
            CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
            MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
            CreationNodeMask: 0,
            VisibleNodeMask: 0,
        };

        let resource_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: 0,
            Width: size_of_val(&vertices) as u64,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Flags: D3D12_RESOURCE_FLAG_NONE,
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        };

        let mut vertex_buffer: *mut ID3D12Resource = null_mut();
        let result = unsafe {
            (*device).CreateCommittedResource(
                &heapprop,
                D3D12_HEAP_FLAG_NONE,
                &resource_desc,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                null(),
                &ID3D12Resource::uuidof(),
                &mut vertex_buffer as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_failed(result) {
            return Err("failed: CreateCommittedResource".to_string());
        }

        let mut vertex_map: *mut math::Vec3<f32> = null_mut();
        let result = unsafe {
            (*vertex_buffer).Map(
                0,
                null(),
                &mut vertex_map as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_failed(result) {
            return Err("failed: resource map".to_string());
        }
        unsafe {
            std::ptr::copy(vertices.as_ptr(), vertex_map, size_of_val(&vertices));
            (*vertex_buffer).Unmap(0, null());
        }

        let vb_view = D3D12_VERTEX_BUFFER_VIEW {
            BufferLocation: unsafe { (*vertex_buffer).GetGPUVirtualAddress() },
            SizeInBytes: size_of_val(&vertices) as u32,
            StrideInBytes: size_of_val(&vertices[0]) as u32,
        };

        let indices = [0u16, 1u16, 2u16, 2u16, 1u16, 3u16];

        let mut idx_buffer: *mut ID3D12Resource = null_mut();
        let mut resource_desc = resource_desc;
        resource_desc.Width = size_of_val(&indices) as u64;
        let result = unsafe {
            (*device).CreateCommittedResource(
                &heapprop,
                D3D12_HEAP_FLAG_NONE,
                &resource_desc,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                null(),
                &ID3D12Resource::uuidof(),
                &mut idx_buffer as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_failed(result) {
            return Err("failed: CreateCommittedResource".to_string());
        }

        let mut mapped_idx: *mut u16 = null_mut();
        unsafe {
            (*idx_buffer).Map(
                0,
                null(),
                &mut mapped_idx as *mut *mut _ as *mut *mut c_void,
            );
            std::ptr::copy(indices.as_ptr(), mapped_idx, size_of_val(&indices));
            (*idx_buffer).Unmap(0, null());
        }

        let ib_view = D3D12_INDEX_BUFFER_VIEW {
            BufferLocation: unsafe { (*idx_buffer).GetGPUVirtualAddress() },
            Format: DXGI_FORMAT_R16_UINT,
            SizeInBytes: size_of_val(&indices) as u32,
        };

        use winapi::um::d3dcompiler::{
            D3DCompileFromFile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION,
            D3D_COMPILE_STANDARD_FILE_INCLUDE,
        };
        let mut vs_blob: *mut ID3DBlob = null_mut();
        let mut ps_blob: *mut ID3DBlob = null_mut();
        let mut error_blob: *mut ID3DBlob = null_mut();
        let result = unsafe {
            D3DCompileFromFile(
                U16CString::from_str("resource/VertexShader.hlsl")
                    .unwrap()
                    .as_ptr(),
                null(),
                D3D_COMPILE_STANDARD_FILE_INCLUDE,
                "main\0".as_ptr() as *const _,
                "vs_5_0\0".as_ptr() as *const _,
                D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION,
                0,
                &mut vs_blob,
                &mut error_blob,
            )
        };
        if is_failed(result) {
            // TODO: dump more detail
            return Err("failed: compile vertex shader file".to_string());
        }
        let result = unsafe {
            D3DCompileFromFile(
                U16CString::from_str("resource/PixelShader.hlsl")
                    .unwrap()
                    .as_ptr(),
                null(),
                D3D_COMPILE_STANDARD_FILE_INCLUDE,
                "main\0".as_ptr() as *const _,
                "ps_5_0\0".as_ptr() as *const _,
                D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION,
                0,
                &mut ps_blob,
                &mut error_blob,
            )
        };
        if is_failed(result) {
            // TODO: dump more detail
            return Err("failed: compile pixel shader file".to_string());
        }

        let input_layout = [D3D12_INPUT_ELEMENT_DESC {
            SemanticName: U16CString::from_str("POSITION").unwrap().as_ptr() as *const _,
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D12_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        }];

        let mut graphics_pipeline: D3D12_GRAPHICS_PIPELINE_STATE_DESC = unsafe { zeroed() };
        graphics_pipeline.pRootSignature = null_mut();
        unsafe {
            graphics_pipeline.VS.pShaderBytecode = (*vs_blob).GetBufferPointer();
            graphics_pipeline.VS.BytecodeLength = (*vs_blob).GetBufferSize();
            graphics_pipeline.PS.pShaderBytecode = (*ps_blob).GetBufferPointer();
            graphics_pipeline.PS.BytecodeLength = (*vs_blob).GetBufferSize();
        }
        graphics_pipeline.SampleMask = D3D12_DEFAULT_SAMPLE_MASK;
        graphics_pipeline.BlendState.AlphaToCoverageEnable = 0;
        graphics_pipeline.BlendState.IndependentBlendEnable = 0;

        let mut render_target_blend_desc: D3D12_RENDER_TARGET_BLEND_DESC = unsafe { zeroed() };
        render_target_blend_desc.BlendEnable = 0;
        render_target_blend_desc.RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL as u8;

        render_target_blend_desc.LogicOpEnable = 0;

        graphics_pipeline.BlendState.RenderTarget[0] = render_target_blend_desc;

        graphics_pipeline.RasterizerState.MultisampleEnable = 0;
        graphics_pipeline.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
        graphics_pipeline.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
        graphics_pipeline.RasterizerState.DepthClipEnable = 1;

        graphics_pipeline.RasterizerState.FrontCounterClockwise = 0;
        graphics_pipeline.RasterizerState.DepthBias = D3D12_DEFAULT_DEPTH_BIAS as i32;
        graphics_pipeline.RasterizerState.DepthBiasClamp = D3D12_DEFAULT_DEPTH_BIAS_CLAMP;
        graphics_pipeline.RasterizerState.SlopeScaledDepthBias =
            D3D12_DEFAULT_SLOPE_SCALED_DEPTH_BIAS;
        graphics_pipeline.RasterizerState.AntialiasedLineEnable = 0;
        graphics_pipeline.RasterizerState.ForcedSampleCount = 0;
        graphics_pipeline.RasterizerState.ConservativeRaster =
            D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF;

        graphics_pipeline.DepthStencilState.DepthEnable = 0;
        graphics_pipeline.DepthStencilState.StencilEnable = 0;

        graphics_pipeline.InputLayout.pInputElementDescs = input_layout.as_ptr();
        graphics_pipeline.InputLayout.NumElements = input_layout.len() as u32;

        graphics_pipeline.IBStripCutValue = D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED;
        graphics_pipeline.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;

        graphics_pipeline.NumRenderTargets = 1;
        graphics_pipeline.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;

        graphics_pipeline.SampleDesc.Count = 1;
        graphics_pipeline.SampleDesc.Quality = 0;

        let mut root_signature: *mut ID3D12RootSignature = null_mut();

        let mut root_signature_desc: D3D12_ROOT_SIGNATURE_DESC = unsafe { zeroed() };
        root_signature_desc.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;

        let mut root_sig_blob: *mut ID3DBlob = null_mut();
        let result = unsafe {
            D3D12SerializeRootSignature(
                &root_signature_desc,
                D3D_ROOT_SIGNATURE_VERSION_1_0,
                &mut root_sig_blob,
                &mut error_blob,
            )
        };
        if is_failed(result) {
            return Err("failed: D3D12SerializeRootSignature".to_string());
        }
        let result = unsafe {
            (*device).CreateRootSignature(
                0,
                (*root_sig_blob).GetBufferPointer(),
                (*root_sig_blob).GetBufferSize(),
                &ID3D12RootSignature::uuidof(),
                &mut root_signature as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_failed(result) {
            return Err("failed: CreateRootSignature".to_string());
        }
        unsafe { (*root_sig_blob).Release() };

        graphics_pipeline.pRootSignature = root_signature;

        let mut pipeline_state: *mut ID3D12PipelineState = null_mut();
        let result = unsafe {
            (*device).CreateGraphicsPipelineState(
                &graphics_pipeline,
                &ID3D12PipelineState::uuidof(),
                &mut pipeline_state as *mut *mut _ as *mut *mut c_void,
            )
        };
        if is_failed(result) {
            return Err("failed: CreateGraphicsPipelineState".to_string());
        }

        let viewport = D3D12_VIEWPORT {
            Width: window.width as f32,
            Height: window.height as f32,
            TopLeftX: 0.0f32,
            TopLeftY: 0.0f32,
            MaxDepth: 1.0f32,
            MinDepth: 0.0f32,
        };

        let scissorrect = D3D12_RECT {
            top: 0,
            left: 0,
            right: window.width as i32,
            bottom: window.height as i32,
        };

        Ok(Direct3D {
            device: device,
            swapchain: swapchain,
            rtv_heaps: rtv_heaps,
            back_buffers: back_buffers,
            command_manager: command_manager,

            viewport: viewport,
            scissorrect: scissorrect,
            pipeline_state: pipeline_state,
            root_signature: root_signature,
            frame: 0,
            vb_view: vb_view,
            ib_view: ib_view,
        })
    }

    pub fn update(&mut self) {
        use winapi::um::d3dcommon::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
        let backbuffer_idx = unsafe { (*self.swapchain).GetCurrentBackBufferIndex() } as usize;
        self.command_manager.resource_barrier(
            self.back_buffers[backbuffer_idx],
            D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
        unsafe {
            self.command_manager
                .get_list()
                .SetPipelineState(self.pipeline_state)
        };
        let mut rtv_handle = unsafe { (*self.rtv_heaps).GetCPUDescriptorHandleForHeapStart() };
        rtv_handle.ptr += backbuffer_idx
            * unsafe {
                (*self.device).GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV)
            } as usize;
        self.command_manager.set_rtv(&rtv_handle);

        let r = (0xff & self.frame >> 16) as f32 / 255.0f32;
        let g = (0xff & self.frame >> 8) as f32 / 255.0f32;
        let b = (0xff & self.frame >> 0) as f32 / 255.0f32;

        self.command_manager
            .clear_render_target_view(rtv_handle, r, g, b, 1.0f32);

        self.frame += 1;
        unsafe {
            self.command_manager
                .get_list()
                .RSSetViewports(1, &self.viewport);
            self.command_manager
                .get_list()
                .RSSetScissorRects(1, &self.scissorrect);
            self.command_manager
                .get_list()
                .SetGraphicsRootSignature(self.root_signature);
            self.command_manager
                .get_list()
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.command_manager
                .get_list()
                .IASetVertexBuffers(0, 1, &self.vb_view);
            self.command_manager
                .get_list()
                .IASetIndexBuffer(&self.ib_view);

            self.command_manager
                .get_list()
                .DrawIndexedInstanced(6, 1, 0, 0, 0);
        }

        self.command_manager.resource_barrier(
            self.back_buffers[backbuffer_idx],
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_PRESENT,
        );

        self.command_manager.run();

        unsafe { (*self.swapchain).Present(1, 0) };
    }
}
