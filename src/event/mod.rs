
use uuid::Uuid;

/// IntoEvent trait will be used to mark event types for the bus
/// Each event should have an unique id
pub trait IntoEvent: Clone
where
    Self: Send + Sync + 'static,
{
    /// Get the id of the event
    /// Each event should have an unique id
    fn get_id(&self) -> Uuid {
        Uuid::new_v4()
    }
}
