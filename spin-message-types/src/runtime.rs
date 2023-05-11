use anyhow::Result;
use tokio::runtime::Runtime;

pub fn runtime() -> Result<Runtime> {
    let rt = tokio::runtime::Builder::new_current_thread().build()?;
    Ok(rt)
}
