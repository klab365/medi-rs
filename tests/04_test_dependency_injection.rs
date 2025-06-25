use medi_rs::{
    BusBuilder, FromResources, {IntoCommand, Result},
};
use medi_rs_macros::{MediCommand, MediRessource};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn send_should_work_with_dependencyinjection() {
    let repo = Arc::new(InMemoryUserRepository::new());
    let state = AppStateDyn::new(repo.clone());
    let bus = BusBuilder::default()
        .add_req_handler(create_user_dyn)
        .append_resources(state.clone())
        .build()
        .unwrap();

    bus.send(CreateUser { name: "John".into() }).await.unwrap();

    let users = repo.0.lock().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "John");
}

#[tokio::test]
async fn send_should_work_with_generic_dependencyinjection() {
    let repo = Arc::new(InMemoryUserRepository::new());
    let state = AppStateGeneric::new(repo.clone());
    let bus = BusBuilder::default()
        .add_req_handler(create_user_generic)
        .append_resources(state.clone())
        .build()
        .unwrap();

    bus.send(CreateUser { name: "John".into() }).await.unwrap();

    let users = repo.0.lock().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "John");
}

async fn create_user_dyn(state: AppStateDyn, req: CreateUser) -> Result<()> {
    let user = User { name: req.name };
    state.user_repository.save(user)?;
    Ok(())
}

async fn create_user_generic(state: AppStateGeneric<InMemoryUserRepository>, req: CreateUser) -> Result<()> {
    let user = User { name: req.name };
    state.user_repository.save(user)?;
    Ok(())
}

/// Request to create a user
#[derive(MediCommand)]
struct CreateUser {
    name: String,
}

struct User {
    name: String,
}

trait UserRepository: Send + Sync {
    fn save(&self, user: User) -> Result<()>;
}

#[derive(Clone)]
struct InMemoryUserRepository(Arc<Mutex<Vec<User>>>);

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}

impl UserRepository for InMemoryUserRepository {
    fn save(&self, user: User) -> Result<()> {
        self.0.lock().unwrap().push(user);
        Ok(())
    }
}

#[derive(Clone, MediRessource)]
struct AppStateDyn {
    user_repository: Arc<dyn UserRepository>,
}
impl AppStateDyn {
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }
}

#[derive(Clone, MediRessource)]
struct AppStateGeneric<T: UserRepository> {
    user_repository: Arc<T>,
}
impl<T: UserRepository> AppStateGeneric<T> {
    pub fn new(user_repository: Arc<T>) -> Self {
        Self { user_repository }
    }
}
