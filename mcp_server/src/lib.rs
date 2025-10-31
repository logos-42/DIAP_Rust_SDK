use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use diap_rs_sdk::*;
use std::sync::Arc;
use tokio::sync::Mutex;

#[pyclass]
struct DiapMcpService {
    auth_manager: Arc<Mutex<AgentAuthManager>>,
    ipfs_client: Arc<Mutex<Option<IpfsClient>>>,
}

#[pymethods]
impl DiapMcpService {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(Self {
            auth_manager: Arc::new(Mutex::new(
                pyo3_asyncio::tokio::get_runtime()
                    .block_on(async { AgentAuthManager::new().await })
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?
            )),
            ipfs_client: Arc::new(Mutex::new(None)),
        })
    }

    fn create_agent<'py>(&self, py: Python<'py>, name: String, ipfs_api: Option<String>) -> PyResult<&'py PyAny> {
        let auth_manager = self.auth_manager.clone();
        future_into_py(py, async move {
            let manager = auth_manager.lock().await;
            let (agent_info, keypair, peer_id) = manager.create_agent(&name, ipfs_api.as_deref())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Ok(Python::with_gil(|py| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("name", agent_info.name).unwrap();
                dict.set_item("did", keypair.did).unwrap();
                dict.set_item("peer_id", peer_id.to_string()).unwrap();
                dict.to_object(py)
            }))
        })
    }

    fn upload_to_ipfs<'py>(&self, py: Python<'py>, content: String) -> PyResult<&'py PyAny> {
        let ipfs_client = self.ipfs_client.clone();
        future_into_py(py, async move {
            let mut client_guard = ipfs_client.lock().await;
            if client_guard.is_none() {
                *client_guard = Some(IpfsClient::new(None));
            }
            
            let client = client_guard.as_ref().unwrap();
            let result = client.upload_json(&content).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Ok(Python::with_gil(|py| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("cid", result.cid).unwrap();
                dict.set_item("size", result.size).unwrap();
                dict.to_object(py)
            }))
        })
    }

    fn get_from_ipfs<'py>(&self, py: Python<'py>, cid: String) -> PyResult<&'py PyAny> {
        let ipfs_client = self.ipfs_client.clone();
        future_into_py(py, async move {
            let mut client_guard = ipfs_client.lock().await;
            if client_guard.is_none() {
                *client_guard = Some(IpfsClient::new(None));
            }
            
            let client = client_guard.as_ref().unwrap();
            let content = client.get_json(&cid).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Ok(content)
        })
    }
}

#[pymodule]
fn diap_mcp_server(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<DiapMcpService>()?;
    Ok(())
}
