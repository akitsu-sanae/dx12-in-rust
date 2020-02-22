use std::ptr::{null, null_mut};
use winapi::{
    ctypes::c_void,
    shared::{basetsd::UINT64, winerror::S_OK},
    um::d3d12::{
        ID3D12CommandAllocator, ID3D12CommandQueue, ID3D12Device, ID3D12Fence,
        ID3D12GraphicsCommandList, ID3D12Resource, D3D12_CPU_DESCRIPTOR_HANDLE,
        D3D12_RESOURCE_STATES,
    },
    Interface,
};

pub struct CommandManager {
    allocator: *mut ID3D12CommandAllocator,
    list: *mut ID3D12GraphicsCommandList,
    queue: *mut ID3D12CommandQueue,
    fence: *mut ID3D12Fence,
    fence_val: UINT64,
}

impl CommandManager {
    pub fn create(device: *mut ID3D12Device) -> Result<CommandManager, String> {
        let allocator = create_allocator(device)?;
        let list = create_list(device, allocator)?;
        let queue = create_queue(device)?;
        let fence_val: UINT64 = 0;
        let fence = create_fence(device, fence_val)?;

        Ok(CommandManager {
            allocator: allocator,
            list: list,
            queue: queue,
            fence: fence,
            fence_val: fence_val,
        })
    }

    pub fn get_queue(&self) -> *mut ID3D12CommandQueue {
        self.queue
    }

    pub fn resource_barrier(
        &self,
        backbuffer: *mut ID3D12Resource,
        before: D3D12_RESOURCE_STATES,
        after: D3D12_RESOURCE_STATES,
    ) {
        use winapi::um::d3d12::{
            D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            D3D12_RESOURCE_TRANSITION_BARRIER,
        };
        unsafe {
            let mut barrier_desc = D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                ..std::mem::zeroed()
            };

            *barrier_desc.u.Transition_mut() = D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: backbuffer,
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter: after,
            };
            (*self.list).ResourceBarrier(1, &barrier_desc)
        }
    }

    pub fn set_rtv(&self, rtv_handle: *const D3D12_CPU_DESCRIPTOR_HANDLE) {
        unsafe { (*self.list).OMSetRenderTargets(1, rtv_handle, 0, null()) };
    }

    pub fn clear_render_target_view(
        &self,
        rtv_handle: D3D12_CPU_DESCRIPTOR_HANDLE,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    ) {
        let color = [r, g, b, a];
        unsafe { (*self.list).ClearRenderTargetView(rtv_handle, &color as *const _, 0, null()) }
    }

    pub fn run(&mut self) {
        unsafe {
            (*self.list).Close();
            let command_lists = vec![self.list];
            (*self.queue).ExecuteCommandLists(1, command_lists.as_ptr() as *const _);
            self.fence_val += 1;
            (*self.queue).Signal(self.fence, self.fence_val);
            (*self.allocator).Reset();
            (*self.list).Reset(self.allocator, null_mut());
        }
    }
}

fn create_allocator(device: *mut ID3D12Device) -> Result<*mut ID3D12CommandAllocator, String> {
    use winapi::um::d3d12::D3D12_COMMAND_LIST_TYPE_DIRECT;
    let mut allocator: *mut ID3D12CommandAllocator = null_mut();
    let result = unsafe {
        (*device).CreateCommandAllocator(
            D3D12_COMMAND_LIST_TYPE_DIRECT,
            &ID3D12CommandAllocator::uuidof(),
            &mut allocator as *mut *mut _ as *mut *mut c_void,
        )
    };
    if result == S_OK {
        Ok(allocator)
    } else {
        Err("failed: create ID3D12CommandAllocator".to_string())
    }
}

fn create_list(
    device: *mut ID3D12Device,
    allocator: *mut ID3D12CommandAllocator,
) -> Result<*mut ID3D12GraphicsCommandList, String> {
    use winapi::um::d3d12::D3D12_COMMAND_LIST_TYPE_DIRECT;
    let mut list: *mut ID3D12GraphicsCommandList = null_mut();
    let result = unsafe {
        (*device).CreateCommandList(
            0,
            D3D12_COMMAND_LIST_TYPE_DIRECT,
            allocator,
            null_mut(),
            &ID3D12GraphicsCommandList::uuidof(),
            &mut list as *mut *mut _ as *mut *mut c_void,
        )
    };
    if result == S_OK {
        Ok(list as *mut _)
    } else {
        Err("failed: create ID3D12CommandList".to_string())
    }
}

fn create_queue(device: *mut ID3D12Device) -> Result<*mut ID3D12CommandQueue, String> {
    use winapi::um::d3d12::{
        D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
        D3D12_COMMAND_QUEUE_PRIORITY_NORMAL,
    };
    let queue_desc = D3D12_COMMAND_QUEUE_DESC {
        Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
        NodeMask: 0,
        Priority: D3D12_COMMAND_QUEUE_PRIORITY_NORMAL as i32,
        Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
    };
    let mut queue: *mut ID3D12CommandQueue = null_mut();
    let result = unsafe {
        (*device).CreateCommandQueue(
            &queue_desc,
            &ID3D12CommandQueue::uuidof(),
            &mut queue as *mut *mut _ as *mut *mut c_void,
        )
    };
    if result == S_OK {
        Ok(queue)
    } else {
        Err("failed: create ID3D12CommandQueue".to_string())
    }
}

fn create_fence(device: *mut ID3D12Device, fence_val: UINT64) -> Result<*mut ID3D12Fence, String> {
    use winapi::um::d3d12::D3D12_FENCE_FLAG_NONE;
    let mut fence: *mut ID3D12Fence = null_mut();
    let result = unsafe {
        (*device).CreateFence(
            fence_val,
            D3D12_FENCE_FLAG_NONE,
            &ID3D12Fence::uuidof(),
            &mut fence as *mut *mut _ as *mut *mut c_void,
        )
    };
    if result == S_OK {
        Ok(fence)
    } else {
        Err("fail: create fence".to_string())
    }
}
