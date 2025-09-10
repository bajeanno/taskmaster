use super::Result;
use mockall::automock;

#[automock]
pub trait Api {
    async fn list_tasks(&self) -> Result<Vec<String>>;
}
