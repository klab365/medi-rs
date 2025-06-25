use medi_rs::{Bus, BusBuilder, FromResources, IntoCommand, IntoEvent, Result};
use medi_rs_macros::{MediCommand, MediEvent, MediRessource};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn send_should_work_with_dependencyinjection() {
    let repo = Arc::new(InMemoryUserRepository::new());
    let state = AppStateDyn::new(repo.clone());
    let bus = BusBuilder::default()
        .add_req_handler(create_user_dyn)
        .add_event_handler(user_created)
        .append_resources(state.clone())
        .build()
        .unwrap();

    bus.send(CreateUser { name: "John".into() }).await.unwrap();

    let users = repo.0.lock().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "John");
}

async fn create_user_dyn(state: AppStateDyn, bus: Bus, req: CreateUser) -> Result<()> {
    let user = User { name: req.name.clone() };
    state.user_repository.save(user).await?;

    let event = UserCreatedEvent { name: req.name };
    bus.publish(event).await?;

    Ok(())
}

async fn user_created(state: AppStateDyn, user: UserCreatedEvent) -> Result<()> {
    // Here you could handle the event, e.g., log it or notify other services
    println!("User created: {}", user.name);
    state.user_repository.save(User { name: user.name }).await?;
    Ok(())
}

/// Request to create a user
#[derive(MediCommand)]
struct CreateUser {
    name: String,
}

#[derive(Clone, MediEvent)]
struct UserCreatedEvent {
    name: String,
}

struct User {
    name: String,
}

#[async_trait::async_trait]
trait UserRepository: Send + Sync {
    async fn save(&self, user: User) -> Result<()>;
}

#[derive(Clone)]
struct InMemoryUserRepository(Arc<Mutex<Vec<User>>>);

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}

#[async_trait::async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn save(&self, user: User) -> Result<()> {
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
