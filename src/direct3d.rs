pub mod command;

use std::ptr::{null, null_mut};
use winapi::{
    ctypes::c_void,
    shared::{
        dxgi::DXGI_SWAP_CHAIN_DESC,
        dxgi1_2::IDXGISwapChain1,
        dxgi1_5::IDXGISwapChain4,
        dxgi1_6::IDXGIFactory6,
        minwindef::{BOOL, UINT},
        winerror::S_OK,
    },
    um::{
        d3d12::{
            ID3D12DescriptorHeap, ID3D12Device, ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE,
            D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
        },
        unknwnbase::IUnknown,
    },
    Interface,
};

use crate::window::Window;

pub struct Direct3D {
    device: *mut ID3D12Device,
    swapchain: *mut IDXGISwapChain4,
    rtv_heaps: *mut ID3D12DescriptorHeap,
    back_buffers: Vec<*mut ID3D12Resource>,
    command_manager: command::CommandManager,
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
    if result == S_OK {
        Ok(factory)
    } else {
        let result = unsafe {
            CreateDXGIFactory2(
                0,
                &IDXGIFactory6::uuidof(),
                &mut factory as *mut *mut _ as *mut *mut c_void,
            )
        };
        if result == S_OK {
            Ok(factory)
        } else {
            Err("failed: create DXGIFactory".to_string())
        }
    }
}

fn create_device() -> Result<*mut ID3D12Device, String> {
    let mut device: *mut ID3D12Device = null_mut();
    use winapi::um::{d3d12::D3D12CreateDevice, d3dcommon::*};
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
        if result == S_OK {
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
    if result == S_OK {
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
    if result == S_OK {
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
    let mut swapchain_desc: DXGI_SWAP_CHAIN_DESC = unsafe { std::mem::zeroed() };
    let result = unsafe { (*swapchain).GetDesc(&mut swapchain_desc) };
    if result != S_OK {
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
        if result != S_OK {
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

        Ok(Direct3D {
            device: device,
            swapchain: swapchain,
            rtv_heaps: rtv_heaps,
            back_buffers: back_buffers,
            command_manager: command_manager,
        })
    }

    pub fn update(&mut self) {
        use winapi::um::d3d12::{D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET};
        let backbuffer_idx = unsafe { (*self.swapchain).GetCurrentBackBufferIndex() } as usize;
        self.command_manager.resource_barrier(
            self.back_buffers[backbuffer_idx],
            D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );

        let mut rtv_handle = unsafe { (*self.rtv_heaps).GetCPUDescriptorHandleForHeapStart() };
        rtv_handle.ptr += backbuffer_idx
            * unsafe {
                (*self.device).GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV)
            } as usize;
        self.command_manager.set_rtv(&rtv_handle);

        self.command_manager
            .clear_render_target_view(rtv_handle, 1.0f32, 1.0f32, 0.0f32, 1.0f32);

        self.command_manager.resource_barrier(
            self.back_buffers[backbuffer_idx],
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_PRESENT,
        );

        self.command_manager.run();

        unsafe { (*self.swapchain).Present(1, 0) };
    }
}
