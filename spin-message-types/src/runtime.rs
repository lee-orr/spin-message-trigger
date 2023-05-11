use tokio::runtime::Runtime;
use anyhow::Result;

pub fn runtime() -> Result<Runtime> {
    let rt = tokio::runtime::Builder::new_current_thread().build()?;
    Ok(rt)
}