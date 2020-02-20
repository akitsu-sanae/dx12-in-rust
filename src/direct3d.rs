use std::ptr::{null, null_mut};
use winapi::shared::minwindef::{BOOL, UINT};
use winapi::{
    ctypes::c_void,
    shared::{
        dxgi1_2::IDXGISwapChain1, dxgi1_5::IDXGISwapChain4, dxgi1_6::IDXGIFactory6, winerror::S_OK,
    },
    um::{
        d3d12::{
            ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12Device, ID3D12GraphicsCommandList,
        },
        unknwnbase::IUnknown,
    },
    Interface,
};

use crate::window::Window;

pub struct Direct3D {
    device: *mut ID3D12Device,
    factory: *mut IDXGIFactory6,
    swapchain: *mut IDXGISwapChain4,

    command_manager: CommandManager,
}

fn create_factory() -> Result<*mut IDXGIFactory6, String> {
    use winapi::shared::dxgi1_3::{CreateDXGIFactory2, DXGI_CREATE_FACTORY_DEBUG};
    let mut factory: *mut IDXGIFactory6 = null_mut();
    if unsafe {
        CreateDXGIFactory2(
            DXGI_CREATE_FACTORY_DEBUG,
            &IDXGIFactory6::uuidof(),
            &mut factory as *mut *mut _ as *mut *mut c_void,
        )
    } == S_OK
    {
        Ok(factory)
    } else if unsafe {
        CreateDXGIFactory2(
            0,
            &IDXGIFactory6::uuidof(),
            &mut factory as *mut *mut _ as *mut *mut c_void,
        )
    } == S_OK
    {
        Ok(factory)
    } else {
        Err("failed: create DXGIFactory".to_string())
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
        if unsafe {
            D3D12CreateDevice(
                null_mut(),
                *level,
                &ID3D12Device::uuidof(),
                &mut device as *mut *mut _ as *mut *mut c_void,
            )
        } == S_OK
        {
            return Ok(device);
        }
    }
    Err("failed: create D3D12CreateDevice".to_string())
}

fn create_swapchain(
    factory: *mut IDXGIFactory6,
    command_manager: &CommandManager,
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
            command_manager.queue as *mut _ as *mut IUnknown,
            window.handle,
            &desc,
            null(),
            null_mut(),
            &mut swapchain as *mut *mut _ as *mut *mut IDXGISwapChain1,
        )
    };
    if result != S_OK {
        Ok(swapchain)
    } else {
        Err("failed: create IDXGIFactory6".to_string())
    }
}

impl Direct3D {
    pub fn create(window: &Window) -> Result<Direct3D, String> {
        let factory = create_factory()?;
        let device = create_device()?;
        let command_manager = CommandManager::create(device)?;
        let swapchain = create_swapchain(factory, &command_manager, window)?;
        Ok(Direct3D {
            factory: factory,
            device: device,
            command_manager: command_manager,
            swapchain: swapchain,
        })
    }

    pub fn update(&self) {
        use winapi::um::d3d12::{
            D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_TRANSITION_BARRIER,
        };
        let mut barrier = D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            ..unsafe { std::mem::zeroed() }
        };
        unsafe {
            *barrier.u.Transition_mut() = D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: unimplemented!(), // backBuffers[backbuffer_index],
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: D3D12_RESOURCE_STATE_PRESENT,
                StateAfter: D3D12_RESOURCE_STATE_RENDER_TARGET,
            };
        }
        unsafe { (*self.command_manager.list).ResourceBarrier(1, &barrier) };
    }
}

struct CommandManager {
    allocator: *mut ID3D12CommandAllocator,
    list: *mut ID3D12GraphicsCommandList,
    queue: *mut ID3D12CommandQueue,
}

impl CommandManager {
    pub fn create(dev: *mut ID3D12Device) -> Result<CommandManager, String> {
        use winapi::um::d3d12::{
            D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
            D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_COMMAND_QUEUE_PRIORITY_NORMAL,
        };
        let mut allocator: *mut ID3D12CommandAllocator = null_mut();
        let result = unsafe {
            (*dev).CreateCommandAllocator(
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                &ID3D12CommandAllocator::uuidof(),
                &mut allocator as *mut *mut _ as *mut *mut c_void,
            )
        };
        if result != S_OK {
            return Err("failed: create ID3D12CommandAllocator".to_string());
        }
        let mut list: *mut ID3D12GraphicsCommandList = null_mut();
        let result = unsafe {
            (*dev).CreateCommandList(
                0,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                allocator,
                null_mut(),
                &ID3D12GraphicsCommandList::uuidof(),
                &mut list as *mut *mut _ as *mut *mut c_void,
            )
        };
        if result != S_OK {
            return Err("failed: create ID3D12CommandList".to_string());
        }
        let queue_desc = D3D12_COMMAND_QUEUE_DESC {
            Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
            NodeMask: 0,
            Priority: D3D12_COMMAND_QUEUE_PRIORITY_NORMAL as i32,
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
        };
        let mut queue: *mut ID3D12CommandQueue = null_mut();
        let result = unsafe {
            (*dev).CreateCommandQueue(
                &queue_desc,
                &ID3D12CommandQueue::uuidof(),
                &mut queue as *mut *mut _ as *mut *mut c_void,
            )
        };
        if result != S_OK {
            return Err("failed: create ID3D12CommandQueue".to_string());
        }
        Ok(CommandManager {
            allocator: allocator,
            list: list,
            queue: queue,
        })
    }
}
