use medi_rs::{MediCommand, Result, medi_handler, medi_module, mediator};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct UserRepository {
    users: Arc<Mutex<Vec<String>>>,
}
impl UserRepository {
    fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn save(&self, name: String) {
        self.users.lock().unwrap().push(name);
    }
    fn len(&self) -> usize {
        self.users.lock().unwrap().len()
    }
}
#[derive(MediCommand)]
#[medi_command(error_type = medi_rs::Error)]
struct CreateUser {
    name: String,
}
#[medi_handler]
async fn create_user(repo: UserRepository, request: CreateUser) -> Result<()> {
    repo.save(request.name);
    Ok(())
}
medi_module! { manifest users_manifest; resources { UserRepository; } commands { CreateUser => create_user; } }
mediator! { pub struct UserMediator { event_queue_capacity: 1; event_workers: 1; modules: [users_manifest]; } }
#[tokio::main]
async fn main() -> Result<()> {
    let repo = UserRepository::new();
    UserMediator::new((repo.clone(),))
        .send(CreateUser { name: "Ada".into() })
        .await?;
    println!("stored users: {}", repo.len());
    Ok(())
}
