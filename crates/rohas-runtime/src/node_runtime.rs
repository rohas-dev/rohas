use crate::error::Result;
use crate::handler::{HandlerContext, HandlerResult};
use crate::node_rpc_runtime::NodeRpcRuntime;
use std::path::{Path, PathBuf};
use tracing::info;

pub struct NodeRuntime {
    rpc_runtime: NodeRpcRuntime,
}

impl NodeRuntime {
    pub fn new() -> Result<Self> {
        let project_root = std::env::current_dir().unwrap_or_default();
        let max_workers = std::env::var("ROHAS_NODE_WORKERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10); // Default to 10 workers

        info!("Initializing Node.js RPC runtime with {} workers", max_workers);

        Ok(Self {
            rpc_runtime: NodeRpcRuntime::new(project_root, max_workers)?,
        })
    }

    pub fn set_project_root(&mut self, root: PathBuf) {
        self.rpc_runtime.set_project_root(root);
    }

    pub async fn execute_handler(
        &self,
        handler_path: &Path,
        context: HandlerContext,
    ) -> Result<HandlerResult> {
        self.rpc_runtime.execute_handler(handler_path, context).await
    }

    pub async fn load_module(&self, module_path: &Path) -> Result<()> {
        self.rpc_runtime.load_module(module_path).await
    }

    pub async fn reload_module(&self, module_name: &str) -> Result<()> {
        self.rpc_runtime.reload_module(module_name).await
    }

    pub async fn clear_cache(&self) -> Result<()> {
        self.rpc_runtime.clear_cache().await
    }

    pub async fn get_loaded_modules(&self) -> Vec<String> {
        self.rpc_runtime.get_loaded_modules().await
    }
}

impl Default for NodeRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Node.js runtime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_runtime_creation() {
        let runtime = NodeRuntime::new();
        assert!(runtime.is_ok());
    }
}
