// Main library entry point for Rohas Rust application
// This file sets up the module structure

#[path = "generated/lib.rs"]
pub mod generated;

// Re-export generated types for convenience
pub use generated::*;

pub mod handlers;

/// Initialize and register all handlers with the Rust runtime.
/// This function should be called during engine startup.
/// It will automatically register all handlers using the global registry.
pub async fn init_handlers(runtime: std::sync::Arc<rohas_runtime::RustRuntime>) -> rohas_runtime::Result<()> {
    generated::register_all_handlers(runtime).await
}

/// C-compatible FFI function for automatic handler registration.
/// This is called automatically by the engine.
/// Returns 0 on success, non-zero on error.
#[no_mangle]
pub extern "C" fn rohas_set_runtime(runtime_ptr: *mut std::ffi::c_void) -> i32 {
    use std::sync::Arc;
    
    if runtime_ptr.is_null() {
        return 1; // Error: null pointer
    }
    
    // Safety: The engine passes a valid Arc<RustRuntime> pointer that was created with Arc::into_raw.
    // We reconstruct the Arc temporarily to clone it, then forget it so the engine retains ownership.
    unsafe {
        // Convert the raw pointer back to Arc<RustRuntime>
        // The engine created this with Arc::into_raw, so we reconstruct it temporarily
        let runtime: Arc<rohas_runtime::RustRuntime> = Arc::from_raw(runtime_ptr as *const rohas_runtime::RustRuntime);
        
        // Clone the Arc - this increments the reference count
        let runtime_clone = runtime.clone();
        
        // Forget the reconstructed Arc - we don't want to drop it here since the engine still owns it
        // The engine will manage the original Arc's lifetime
        std::mem::forget(runtime);
        
        // Call the generated set_runtime function which will register all handlers
        // This will store the cloned Arc in a OnceLock and register handlers synchronously
        // Note: If registration fails, set_runtime will panic (via .expect())
        generated::set_runtime(runtime_clone);
        
        0 // Success
    }
}
